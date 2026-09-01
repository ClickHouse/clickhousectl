pub mod cli;
pub mod config;
pub mod discovery;
pub mod docker;
pub mod output;
pub mod postgres;
pub mod server;
pub mod symlink;

use cli::{ClientVersionArg, InstallVersionArg, LocalCommands, ServerCommands, ServerVersionArg};

use crate::error::{
    BinaryLaunchProblem, Error, ManagedClientError, ManagedClientErrorKind, ManagedClientSelection,
    ProjectServerCommand, ProjectServerNotFound, ProjectServerStateMissing, Result,
};
use crate::{init, paths, version_manager};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub async fn run(cmd: LocalCommands, json: bool) -> Result<()> {
    match cmd {
        LocalCommands::Install { version, force } => install(version, force, json).await,
        LocalCommands::List { remote } => {
            if remote {
                list_available(json).await
            } else {
                list_installed(json)
            }
        }
        LocalCommands::Use { version, no_global } => {
            use_version(version.into_spec(), no_global, json).await
        }
        LocalCommands::Remove { version, force } => remove(&version, force, json),
        LocalCommands::Which => which(json),
        LocalCommands::Init => {
            let result = init::init()?;
            let mut paths = vec![".clickhouse/".to_string()];
            if result.clickhouse_scaffold_created {
                paths.push("clickhouse/".to_string());
            }
            if result.postgres_scaffold_created {
                paths.push("postgres/".to_string());
            }
            let out = output::InitOutput {
                paths,
                already_initialized: !result.clickhouse_dir_created,
            };
            output::print_output(&out, json);
            Ok(())
        }
        LocalCommands::Client {
            name,
            host,
            port,
            version,
            query,
            queries_file,
            args,
        } => run_client(name, host, port, version, query, queries_file, args),
        LocalCommands::Server { command } => run_server_commands(command, json).await,
        LocalCommands::Postgres { command } => postgres::run(command, json).await,
    }
}

async fn install_postgres(tag: &str, force: bool, json: bool) -> Result<()> {
    postgres::validate_pg_tag(tag)?;
    let docker = docker::connect().await?;
    if !force && docker::image_exists(&docker, tag).await? {
        let out = output::InstallOutput {
            version: format!("postgres@{tag}"),
            set_as_default: false,
        };
        if !json {
            eprintln!("postgres:{tag} is already pulled");
        }
        output::print_output(&out, json);
        return Ok(());
    }

    docker::pull_image(&docker, tag, json).await?;

    let out = output::InstallOutput {
        version: format!("postgres@{tag}"),
        set_as_default: false,
    };
    output::print_output(&out, json);
    Ok(())
}

async fn install(version: InstallVersionArg, force: bool, json: bool) -> Result<()> {
    let spec = match version {
        InstallVersionArg::ClickHouse(spec) => spec,
        InstallVersionArg::Postgres(tag) => return install_postgres(&tag, force, json).await,
    };
    let platform = version_manager::platform::Platform::detect()?;

    let version =
        version_manager::install::install_local_first(&spec, &platform, force, json).await?;

    // If this is the first installed version, set it as default
    let set_as_default = version_manager::get_default_version().is_err();
    if set_as_default {
        version_manager::set_default_version(&version)?;
        if !json {
            eprintln!("Set as default version");
        }
    }

    let out = output::InstallOutput {
        version,
        set_as_default,
    };
    // The version manager already emits outcome-aware human output: a real
    // install is confirmed there, while a no-op says that the existing build
    // is being reused. The generic display text always says "Installed", so
    // reserve it for structured output to avoid a duplicate or misleading
    // human confirmation.
    if json {
        output::print_output(&out, json);
    }

    Ok(())
}

fn list_installed(json: bool) -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    let out = output::ListInstalledOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let is_default = Some(&v) == default.as_ref();
                output::InstalledVersion {
                    version: v,
                    default: is_default,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn list_available(json: bool) -> Result<()> {
    if !json {
        eprintln!("Checking available versions on builds.clickhouse.com...");
    }
    let versions = version_manager::list_available_versions_from_builds().await?;

    let installed = version_manager::list_installed_versions().unwrap_or_default();

    let out = output::ListAvailableOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let prefix = format!("{}.", v);
                let is_installed = installed
                    .iter()
                    .any(|iv| iv.starts_with(&prefix) || iv == &v);
                output::AvailableVersion {
                    version: v,
                    installed: is_installed,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn use_version(
    spec: version_manager::VersionSpec,
    no_global: bool,
    json: bool,
) -> Result<()> {
    let platform = version_manager::platform::Platform::detect()?;

    let version =
        version_manager::install::ensure_installed_local_first(&spec, &platform, json).await?;

    version_manager::set_default_version(&version)?;

    if !no_global {
        // Best-effort: any failures are warned to stderr inside the helper
        // and never affect the command's exit status.
        let _ = symlink::ensure_global_symlink(&version);
    }

    let out = output::UseOutput { version };
    output::print_output(&out, json);
    Ok(())
}

fn remove(version: &str, force: bool, json: bool) -> Result<()> {
    let version_dir = paths::version_dir(version)?;

    if !version_dir.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    // Refuse to delete the default version before the (potentially slow)
    // running-server discovery below: removing it clears the default marker and
    // the global symlink, and the exact build is not always re-downloadable.
    // The raw marker is read rather than `get_default_version` so a marker whose
    // binary is already missing still guards. This is a cheap early exit over a
    // snapshot; the authoritative check is repeated under the commit lock.
    if version_manager::default_version_marker().as_deref() == Some(version) && !force {
        return Err(Error::VersionIsDefault {
            version: version.to_string(),
        });
    }

    // Refuse to pull the binary out from under a server running on this
    // version. The lookup spans every project, not just this one: a server
    // started from another directory is invisible to project-scoped metadata,
    // but deleting its binary breaks `local client` there just the same.
    let in_use = server::servers_using_version(version)?;
    if !in_use.is_empty() {
        if !force {
            return Err(Error::VersionInUse {
                version: version.to_string(),
                servers: server::describe_version_users(&in_use),
            });
        }
        for user in &in_use {
            // Servers in this project are stopped through their metadata so it
            // stays consistent; servers elsewhere can only be stopped by PID,
            // like `server stop --global`, except that a blocker which exited
            // on its own since discovery is already what --force wants.
            if user.current_project {
                server::kill_server(&user.name)?;
            } else {
                server::ensure_stopped_by_pid(user.pid)?;
            }
            if !json {
                println!("Stopped server '{}' in {}", user.name, user.project);
            }
        }
    }

    let versions_dir = paths::versions_dir()?;
    let staging = version_manager::atomic::InstallStaging::create(&versions_dir)?;
    let commit_lock = version_manager::atomic::CommitLock::acquire_blocking(&versions_dir)?;
    if !version_dir.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    // Re-read the marker under the commit lock and decide everything default-
    // related from this one read. `set_default_version` writes the marker under
    // the same lock, so a `local use` that raced the snapshot above has either
    // landed (and is seen here) or is blocked until this removal finishes:
    // without --force the guard must refuse again, and with --force the marker
    // is cleared only if it still names this version, never a different one.
    let was_default = version_manager::default_version_marker().as_deref() == Some(version);
    if was_default && !force {
        return Err(Error::VersionIsDefault {
            version: version.to_string(),
        });
    }
    if was_default && !json {
        eprintln!(
            "Warning: {version} is the default version; --force is clearing ~/.clickhouse/default \
             and removing the global `clickhouse` symlink at ~/.local/bin/clickhouse."
        );
    }

    version_manager::master::invalidate_version(
        &commit_lock,
        &versions_dir,
        staging.path(),
        version,
    )?;

    if was_default {
        let default_file = paths::default_file()?;
        let _ = std::fs::remove_file(default_file);
        // Only removes the symlink if it still points into this version's dir.
        let _ = symlink::remove_global_symlink_for(version);
    }

    std::fs::remove_dir_all(&version_dir)?;
    version_manager::atomic::sync_directory(&versions_dir)?;

    let out = output::RemoveOutput {
        version: version.to_string(),
        was_default,
    };
    output::print_output(&out, json);
    Ok(())
}

fn which(json: bool) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;
    let out = output::WhichOutput {
        version,
        binary_path: binary.display().to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn run_client(
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    version_spec: Option<ClientVersionArg>,
    query: Vec<String>,
    queries_file: Vec<String>,
    args: Vec<String>,
) -> Result<()> {
    // If --host or --port is set, connect directly (bypass local server lookup).
    // Otherwise, look up the named server for port and version.
    let (resolved_host, tcp_port, version, binary) = if host.is_some() || port.is_some() {
        let h = host.unwrap_or_else(|| "localhost".to_string());
        let p = port.unwrap_or(9000);
        let v = resolve_direct_client_version(version_spec)?;
        let binary = paths::binary_path(&v)?;
        if !binary.exists() {
            return Err(Error::VersionNotFound(v));
        }
        (h, p, v, binary)
    } else {
        let server_name = name.as_deref().unwrap_or("default");
        let selection = if name.is_some() {
            ManagedClientSelection::Named
        } else {
            ManagedClientSelection::Default
        };
        // Resolve symlinks so diagnostics identify the one physical directory
        // whose project-local state was inspected.
        let project_dir = init::canonical_project_dir()?;
        let managed_error = |kind, binary_version| {
            Error::ManagedClient(ManagedClientError {
                kind,
                project_dir: project_dir.clone(),
                selection,
                server_name: server_name.to_string(),
                binary_version,
            })
        };
        let project_state_error = |source| {
            managed_error(
                ManagedClientErrorKind::ProjectStateUnavailable(Box::new(source)),
                None,
            )
        };
        let metadata_lock = server::lock_metadata().map_err(&project_state_error)?;
        server::recover_current_project_servers_locked(&metadata_lock)
            .map_err(&project_state_error)?;
        let entry = server::server_entry_locked(server_name, &metadata_lock)
            .map_err(&project_state_error)?
            .ok_or_else(|| managed_error(ManagedClientErrorKind::ServerNotFound, None))?;
        if !entry.running {
            return Err(managed_error(
                ManagedClientErrorKind::ServerNotRunning,
                None,
            ));
        }
        let info = entry
            .info
            .ok_or_else(|| managed_error(ManagedClientErrorKind::ServerNotRunning, None))?;
        let binary = paths::binary_path(&info.version)?;
        if !binary.exists() {
            return Err(managed_error(
                ManagedClientErrorKind::BinaryNotFound,
                Some(info.version),
            ));
        }
        ("localhost".to_string(), info.tcp_port, info.version, binary)
    };

    ensure_repeated_query_supported(&version, query.len())?;
    // Before the telemetry pre-exec hook, so a binary that cannot be launched
    // is an ordinary error event rather than a censored handoff attempt (#471).
    ensure_launchable(&binary, &version)?;

    let mut cmd = Command::new(&binary);
    cmd.arg("client")
        .arg("--host")
        .arg(&resolved_host)
        .arg("--port")
        .arg(tcp_port.to_string());

    for q in &query {
        cmd.arg("--query").arg(q);
    }

    for f in &queries_file {
        cmd.arg("--queries-file").arg(f);
    }

    cmd.args(&args);
    // `exec()` replaces the process image on success, so `main`'s telemetry
    // tail never runs for this invocation; record the censored handoff attempt
    // now (#320, #471). Everything checkable about the launch is checked above
    // this line, so only a race can still fail below it — and the native
    // client, once launched, owns the terminal outright: it inherits stdin,
    // stdout and stderr unchanged, stays in clickhousectl's process group and
    // session on the same controlling TTY, and keeps this PID, so Ctrl-C,
    // Ctrl-Z, window resizes and its own exit status or fatal signal reach the
    // shell exactly as if it had been invoked directly. Preserving that is why
    // the handoff stays an `exec()` and the outcome is censored instead of
    // becoming a spawn-and-wait wrapper that would have to relay all of it.
    #[cfg(feature = "telemetry")]
    crate::telemetry::finalize_before_exec();
    let err = cmd.exec();
    Err(Error::Exec(err.to_string()))
}

/// Reject a selected ClickHouse binary that `exec()` is certain to refuse,
/// *before* the telemetry pre-exec hook runs (#471).
///
/// `exec()` returns only on failure, by which point the invocation has already
/// been recorded as a censored `exec_attempt`; a launch failure must therefore
/// be caught here to be observable as a failure at all. The deterministic ones
/// are: nothing at the path, a path that is not a regular file (a directory,
/// say), a path that cannot be stat-ed, and a regular file carrying no execute
/// bit. A binary unlinked or chmod-ed between this check and `exec()`, or one
/// with a bad executable format, is a race no pre-flight can close — the
/// correct exit code and message still reach the shell.
fn ensure_launchable(binary: &Path, version: &str) -> Result<()> {
    let problem = match std::fs::metadata(binary) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BinaryLaunchProblem::Missing,
        Err(_) => BinaryLaunchProblem::Unreadable,
        Ok(metadata) if !metadata.is_file() => BinaryLaunchProblem::NotAFile,
        // "Can anyone execute this", not "can the owner": a build installed
        // 0o711 or 0o111 is perfectly launchable.
        Ok(metadata) if metadata.permissions().mode() & 0o111 == 0 => {
            BinaryLaunchProblem::NotExecutable
        }
        Ok(_) => return Ok(()),
    };
    Err(Error::BinaryNotLaunchable {
        version: version.to_string(),
        problem,
        path: binary.display().to_string(),
    })
}

const REPEATED_QUERY_MIN_VERSION: &str = "23.9.1.1854";

fn ensure_repeated_query_supported(version: &str, query_count: usize) -> Result<()> {
    if query_count > 1
        && version_manager::list::compare_versions(version, REPEATED_QUERY_MIN_VERSION)
            == std::cmp::Ordering::Less
    {
        return Err(Error::RepeatedClientQueryUnsupported {
            version: version.to_string(),
            minimum: REPEATED_QUERY_MIN_VERSION,
        });
    }
    Ok(())
}

fn resolve_direct_client_version(version_spec: Option<ClientVersionArg>) -> Result<String> {
    if let Some(version_spec) = version_spec {
        let spec = version_spec.into_spec();
        return version_manager::resolve::try_resolve_local(&spec)?
            .ok_or_else(|| Error::ClientVersionNotInstalled(spec.to_string()));
    }

    match version_manager::get_default_version() {
        Ok(version) => Ok(version),
        Err(Error::NoDefaultVersion) => {
            let installed = version_manager::list_installed_versions()?;
            match installed.as_slice() {
                [] => Err(Error::NoClientVersionInstalled),
                [version] => Ok(version.clone()),
                _ => Err(Error::AmbiguousClientVersion),
            }
        }
        Err(Error::VersionNotFound(version)) => Err(Error::StaleDefaultVersion(version)),
        Err(error) => Err(error),
    }
}

fn clean_up_untracked_child(child: &mut std::process::Child, primary: Error) -> Error {
    let cleanup = match child.try_wait() {
        Ok(Some(_)) => Ok(()),
        Ok(None) => child.kill().and_then(|()| child.wait()).map(|_| ()),
        Err(error) => Err(error),
    };
    match cleanup {
        Ok(()) => primary,
        Err(error) => Error::Exec(format!(
            "{primary}; additionally failed to stop untracked PID {}: {error}",
            child.id()
        )),
    }
}

#[allow(clippy::too_many_arguments)]
async fn start_server(
    name: Option<String>,
    version_spec: Option<ServerVersionArg>,
    http_port: Option<u16>,
    tcp_port: Option<u16>,
    foreground: bool,
    no_wait: bool,
    config_file: Option<String>,
    args: Vec<String>,
    json: bool,
) -> Result<()> {
    // `--foreground` streams the server's stdout/stderr and never emits a JSON
    // summary, so it simply ignores `json` rather than erroring on `--json`.

    // Recover any orphaned servers so name resolution and collision checks
    // see processes that lost their metadata files.
    server::recover_current_project_servers()?;

    // Resolve server name and check for collisions before any downloads
    let server_name = server::resolve_name(name.as_deref())?;

    if name.is_some() && server::is_server_running(&server_name)? {
        return Err(Error::ServerAlreadyRunning(server_name));
    }

    let version = if let Some(spec) = version_spec {
        let spec = spec.into_spec();
        let platform = version_manager::platform::Platform::detect()?;
        version_manager::install::ensure_installed_local_first(&spec, &platform, json).await?
    } else {
        match version_manager::get_default_version() {
            Ok(v) => v,
            Err(Error::NoDefaultVersion) => {
                // No version specified and no default set: bootstrap `latest`.
                // Deliberately do NOT set it as the default, so unpinned users keep
                // tracking latest on each start. This branch is therefore hit on every
                // subsequent bare start too; `ensure_installed_local_first` returns the
                // already-installed build silently if `latest` still resolves to it,
                // otherwise it pulls the newer master build.
                let spec = version_manager::VersionSpec::Latest;
                let platform = version_manager::platform::Platform::detect()?;
                // Says "using", not "installing": on repeat starts the build is
                // usually already installed and nothing is downloaded. The install
                // path prints its own Resolving/Downloading/up-to-date messages.
                if !json {
                    eprintln!("No version specified and no default set; using latest");
                }
                version_manager::install::ensure_installed_local_first(&spec, &platform, json)
                    .await?
            }
            // A default pointing at a removed binary stays an error.
            Err(e) => return Err(e),
        }
    };
    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    // Metadata locking is always acquired after version installation locks.
    // Hold it from the final collision check through the process metadata
    // commit so concurrent start/stop/client commands see one lifecycle state.
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let server_name = server::resolve_name_locked(name.as_deref(), &metadata_lock)?;
    if name.is_some() && server::is_server_running_locked(&server_name, &metadata_lock)? {
        return Err(Error::ServerAlreadyRunning(server_name));
    }

    // Show running server count
    let running = server::advisory_running_server_count_locked(&metadata_lock);
    if !json && running > 0 {
        eprintln!(
            "Note: {} server{} already running (use `clickhousectl local server list` to see them)",
            running,
            if running == 1 { "" } else { "s" }
        );
    }

    let (http_port, tcp_port, auto_assigned) = server::resolve_ports(http_port, tcp_port)?;
    if !json && auto_assigned {
        eprintln!(
            "Note: default ports in use, auto-assigned HTTP:{} TCP:{}",
            http_port, tcp_port
        );
    }
    // Reject --config / --config-file / -C in passthrough args. Passing a raw
    // config path here would bypass the managed `--config` handling below and
    // could redirect where ClickHouse stores data, breaking the managed server
    // lifecycle (list, stop, remove, dotenv all rely on the data directory
    // living under .clickhouse/servers/<name>/). Individual --setting=value
    // flags are fine — they don't change the data directory.
    // `--config` also matches `--config-file` as a prefix.
    if args
        .iter()
        .any(|a| a.starts_with("--config") || a.starts_with("-C"))
    {
        return Err(Error::UnsupportedArgument(
            "--config / --config-file / -C cannot be passed through in trailing args. \
             Use `--config <NAME>` with a file in ~/.clickhouse/configs/ \
             (see `clickhousectl local server configs`). \
             Individual --setting=value flags are supported."
                .into(),
        ));
    }

    // Resolve a named config file before any process is spawned, so a bad name
    // fails fast with a helpful error.
    let resolved_config = match &config_file {
        Some(name) => Some(config::resolve_config(name)?),
        None => None,
    };

    let mut cmd = Command::new(&binary);
    cmd.arg("server");

    server::ensure_server_data_dir(&server_name)?;
    let data_dir = server::server_data_dir(&server_name);

    // Stage the named config as a config.d overlay inside the data dir. With no
    // --config-file, ClickHouse uses its built-in defaults and merges any
    // config.d/ next to its working directory, so a partial override file (e.g.
    // just <query_log>) takes effect without replacing the whole config.
    // Passing it as --config-file instead would replace the embedded defaults
    // and a partial file would fail to start. The forced --path=./ and port
    // flags below are command-line overrides that still win over the file, so
    // the managed lifecycle is preserved regardless of the file's contents.
    config::apply_config_overlay(&data_dir, resolved_config.as_deref())?;

    cmd.current_dir(&data_dir);
    cmd.args(init::server_flags());

    cmd.args(server::port_flags(http_port, tcp_port));
    cmd.args(&args);

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if !foreground {
        let log_path = server::server_log_path(&server_name);
        let log = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;
        cmd.stdout(log.try_clone()?);
        cmd.stderr(log);
        let mut child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
            engine: server::Engine::Clickhouse,
            container_id: None,
        };
        if let Err(error) = server::save_server_info_locked(&info, &metadata_lock) {
            return Err(clean_up_untracked_child(&mut child, error));
        }
        drop(metadata_lock);

        if no_wait {
            server::check_spawn_health(&mut child, &server_name, &log_path).await?;
        } else {
            server::wait_for_server_ready(
                &mut child,
                &server_name,
                http_port,
                tcp_port,
                &log_path,
                server::STARTUP_TIMEOUT,
            )
            .await?;
        }

        let out = output::ServerStartOutput {
            name: server_name,
            pid,
            http_port,
            tcp_port,
            version,
        };
        output::print_output(&out, json);
        Ok(())
    } else {
        let mut child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
            engine: server::Engine::Clickhouse,
            container_id: None,
        };
        if let Err(error) = server::save_server_info_locked(&info, &metadata_lock) {
            return Err(clean_up_untracked_child(&mut child, error));
        }
        drop(metadata_lock);

        eprintln!(
            "Server '{}' running (PID: {}, HTTP: {}, TCP: {})",
            server_name, pid, http_port, tcp_port
        );

        let status = child.wait().map_err(|e| Error::Exec(e.to_string()))?;
        server::mark_server_stopped(&server_name, pid)?;

        if !status.success()
            && let Some(code) = status.code()
        {
            return Err(Error::ChildExit(code));
        }
        Ok(())
    }
}

fn list_configs(json: bool) -> Result<()> {
    let dir = paths::configs_dir()?;
    let out = output::ServerConfigsOutput {
        dir: dir.display().to_string(),
        configs: config::list_configs()?,
    };
    output::print_output(&out, json);
    Ok(())
}

fn dotenv_server(
    name: Option<&str>,
    use_local: bool,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    json: bool,
) -> Result<()> {
    let server_name = name.unwrap_or("default");
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let entry = server::server_entry_locked(server_name, &metadata_lock)?
        .ok_or_else(|| Error::ServerNotFound(server_name.to_string()))?;
    if !entry.running {
        return Err(Error::ServerNotRunning(server_name.to_string()));
    }
    let info = entry
        .info
        .ok_or_else(|| Error::ServerNotRunning(server_name.to_string()))?;

    // Only write vars we actually know from server metadata.
    // User, password, and database are only included when explicitly provided.
    let mut vars: Vec<(&str, String)> = vec![
        ("CLICKHOUSE_HOST", "localhost".to_string()),
        ("CLICKHOUSE_PORT", info.tcp_port.to_string()),
        ("CLICKHOUSE_HTTP_PORT", info.http_port.to_string()),
    ];
    if let Some(u) = user {
        vars.push(("CLICKHOUSE_USER", u));
    }
    if let Some(p) = password {
        vars.push(("CLICKHOUSE_PASSWORD", p));
    }
    if let Some(d) = database {
        vars.push(("CLICKHOUSE_DATABASE", d));
    }

    let filename = if use_local { ".env.local" } else { ".env" };
    let path = std::path::Path::new(filename);

    let content = if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        update_dotenv(&existing, "CLICKHOUSE_", &vars)
    } else {
        vars.iter()
            .map(|(k, v)| format_dotenv_line("", k, v))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    std::fs::write(path, &content)?;

    let out = output::ServerDotenvOutput {
        file: filename.to_string(),
        server: server_name.to_string(),
        vars: vars
            .into_iter()
            .map(|(k, v)| output::DotenvVar {
                key: k.to_string(),
                value: v,
            })
            .collect(),
    };
    output::print_output(&out, json);
    Ok(())
}

/// Format a dotenv line. Values that are plain alphanumeric tokens are written
/// bare; anything containing spaces, `#`, quotes, backslashes, or newlines is
/// double-quoted with inner `"`, `\`, and newlines escaped.
pub(crate) fn format_dotenv_line(prefix: &str, key: &str, val: &str) -> String {
    let needs_quoting = val.is_empty()
        || val
            .bytes()
            .any(|b| b == b' ' || b == b'#' || b == b'"' || b == b'\'' || b == b'\\' || b == b'\n');

    if needs_quoting {
        let escaped = val
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        format!("{}{}=\"{}\"", prefix, key, escaped)
    } else {
        format!("{}{}={}", prefix, key, val)
    }
}

/// Extract a `<prefix>*` key from a dotenv line, handling optional `export`
/// prefix and whitespace around `=`. Returns the bare key (e.g. "CLICKHOUSE_HOST"
/// for prefix "CLICKHOUSE_") or None if the line isn't a matching assignment.
fn extract_dotenv_key<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let s = line.trim();
    let s = s
        .strip_prefix("export")
        .map(|rest| rest.trim_start())
        .unwrap_or(s);
    let eq_pos = s.find('=')?;
    let key = s[..eq_pos].trim_end();
    if key.starts_with(prefix) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Some(key)
    } else {
        None
    }
}

/// Update an existing .env file: replace `<prefix>*` vars in-place, append any
/// missing ones. Lines for the same prefix that aren't in `vars` are preserved
/// (e.g. a manually-set CLICKHOUSE_PASSWORD survives a host/port-only update).
pub(crate) fn update_dotenv(existing: &str, prefix: &str, vars: &[(&str, String)]) -> String {
    let mut result = String::new();
    let mut written: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for line in existing.lines() {
        if let Some(key) = extract_dotenv_key(line, prefix) {
            if let Some((_, val)) = vars.iter().find(|(k, _)| *k == key) {
                let line_prefix = if line.trim_start().starts_with("export") {
                    "export "
                } else {
                    ""
                };
                result.push_str(&format_dotenv_line(line_prefix, key, val));
                written.insert(key);
            } else {
                // A matching-prefix var we don't manage — keep as-is
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    for (key, val) in vars {
        if !written.contains(key) {
            result.push_str(&format_dotenv_line("", key, val));
            result.push('\n');
        }
    }

    result
}

async fn run_server_commands(command: ServerCommands, json: bool) -> Result<()> {
    match command {
        ServerCommands::Start {
            name,
            name_flag,
            version,
            http_port,
            tcp_port,
            foreground,
            no_wait,
            config_file,
            args,
        } => {
            start_server(
                name.or(name_flag),
                version,
                http_port,
                tcp_port,
                foreground,
                no_wait,
                config_file,
                args,
                json,
            )
            .await
        }
        ServerCommands::Configs => list_configs(json),
        ServerCommands::List { global } => {
            if global {
                list_servers_global(json)
            } else {
                list_servers_local(json)
            }
        }
        ServerCommands::Stop {
            name,
            name_flag,
            global,
            project,
        } => stop_server(
            ServerNameInput::from_args(name, name_flag),
            global,
            project,
            json,
        ),
        ServerCommands::StopAll { global } => {
            if global {
                stop_all_servers_global(json)
            } else {
                stop_all_servers_local(json)
            }
        }
        ServerCommands::Dotenv {
            name,
            local,
            user,
            password,
            database,
        } => dotenv_server(name.as_deref(), local, user, password, database, json),
        ServerCommands::Remove { name, name_flag } => {
            remove_server(ServerNameInput::from_args(name, name_flag), json)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ServerNameInput {
    Omitted,
    Positional(String),
    NameFlag(String),
}

impl ServerNameInput {
    fn from_args(positional: Option<String>, name_flag: Option<String>) -> Self {
        match (positional, name_flag) {
            (Some(name), None) => Self::Positional(name),
            (None, Some(name)) => Self::NameFlag(name),
            (None, None) => Self::Omitted,
            (Some(_), Some(_)) => unreachable!("clap rejects conflicting server name forms"),
        }
    }
}

fn stop_server(
    name_input: ServerNameInput,
    global: bool,
    project: Option<String>,
    json: bool,
) -> Result<()> {
    if global {
        return stop_server_global(name_input, project.as_deref(), json);
    }

    let explicit_name = match &name_input {
        ServerNameInput::Omitted => None,
        ServerNameInput::Positional(name) | ServerNameInput::NameFlag(name) => Some(name),
    };
    if let Some(name) = explicit_name {
        server::validate_server_name(name)?;
    }
    let project_dir = init::canonical_project_dir()?;
    let project_state_missing = !init::local_dir().is_dir();

    // Recover orphaned servers so we can stop processes
    // that lost their metadata files.
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let (name, selection, known) = match name_input {
        ServerNameInput::Positional(name) | ServerNameInput::NameFlag(name) => {
            (name, output::ServerSelection::Explicit, false)
        }
        ServerNameInput::Omitted => {
            let names = server::list_clickhouse_server_names_locked(&metadata_lock)?;
            if names.iter().any(|name| name == "default") {
                (
                    "default".to_string(),
                    output::ServerSelection::Implicit,
                    true,
                )
            } else {
                match names.as_slice() {
                    [] => {
                        let out = output::ServerStopNoopOutput {
                            stopped: false,
                            selection: output::ServerSelection::Implicit,
                            reason: "no_clickhouse_servers",
                            project_scope: project_state_missing
                                .then(|| output::exact_current_project_scope(&project_dir)),
                            guidance: if project_state_missing {
                                output::project_scope_guidance(Some(ProjectServerCommand::Stop))
                            } else {
                                Vec::new()
                            },
                        };
                        output::print_output(&out, json);
                        return Ok(());
                    }
                    [name] => (name.clone(), output::ServerSelection::Implicit, true),
                    names => {
                        return Err(Error::ServerStopSelectionRequired {
                            available: names.len(),
                        });
                    }
                }
            }
        }
    };

    match classify_stop(
        server::is_server_running_locked(&name, &metadata_lock)?,
        known || server::server_data_dir(&name).exists(),
    ) {
        StopOutcome::Stop => {
            if !json {
                println!("Stopping server '{}'...", name);
            }
            server::kill_server_locked(&name, &metadata_lock)?;
            let out = output::ServerStopOutput {
                name,
                already_stopped: false,
                selection: Some(selection),
            };
            output::print_output(&out, json);
            Ok(())
        }
        StopOutcome::AlreadyStopped => {
            // Server exists on disk but isn't running. `stop` is
            // idempotent: this is the desired end state, so succeed
            // instead of erroring.
            let out = output::ServerStopOutput {
                name,
                already_stopped: true,
                selection: Some(selection),
            };
            output::print_output(&out, json);
            Ok(())
        }
        // No such server in this project — surface the typo.
        StopOutcome::NotFound => Err(Error::ProjectServerNotFound(ProjectServerNotFound {
            command: ProjectServerCommand::Stop,
            project_dir,
            server_name: name,
        })),
    }
}

fn remove_server(name_input: ServerNameInput, json: bool) -> Result<()> {
    let explicit_name = match &name_input {
        ServerNameInput::Omitted => None,
        ServerNameInput::Positional(name) | ServerNameInput::NameFlag(name) => Some(name),
    };
    if let Some(name) = explicit_name {
        server::validate_server_name(name)?;
    }
    let project_dir = init::canonical_project_dir()?;
    let project_state_missing = !init::local_dir().is_dir();

    // Recover orphaned servers so we correctly detect a running
    // process even when its metadata file is missing.
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let (name, selection) = match name_input {
        ServerNameInput::Positional(name) | ServerNameInput::NameFlag(name) => {
            (name, output::ServerSelection::Explicit)
        }
        ServerNameInput::Omitted => {
            let names = server::list_clickhouse_server_names_locked(&metadata_lock)?;
            if names.iter().any(|name| name == "default") {
                ("default".to_string(), output::ServerSelection::Implicit)
            } else {
                if project_state_missing && names.is_empty() {
                    return Err(Error::ProjectServerStateMissing(
                        ProjectServerStateMissing {
                            command: ProjectServerCommand::Remove,
                            project_dir,
                        },
                    ));
                }
                return Err(Error::ServerRemoveSelectionRequired {
                    available: names.len(),
                });
            }
        }
    };

    if server::is_server_running_locked(&name, &metadata_lock)? {
        return Err(Error::ServerRunningCannotRemove(name));
    }
    let data_dir = server::server_data_dir(&name);
    if !data_dir.exists() {
        return Err(Error::ProjectServerNotFound(ProjectServerNotFound {
            command: ProjectServerCommand::Remove,
            project_dir,
            server_name: name,
        }));
    }
    // Remove the whole server directory (parent of data/)
    let server_dir = data_dir.parent().unwrap();
    std::fs::remove_dir_all(server_dir)?;
    server::try_remove_server_info_locked(&name, &metadata_lock)?;
    let out = output::ServerRemoveOutput {
        name,
        selection: Some(selection),
    };
    output::print_output(&out, json);
    Ok(())
}

/// What a project-scoped `server stop <name>` should do, given whether the
/// server is currently running and whether its data directory exists on disk.
#[derive(Debug, PartialEq, Eq)]
enum StopOutcome {
    /// Running — kill it.
    Stop,
    /// Exists on disk but not running — idempotent noop (success).
    AlreadyStopped,
    /// Unknown server name — error, so typos surface.
    NotFound,
}

fn classify_stop(running: bool, exists_on_disk: bool) -> StopOutcome {
    match (running, exists_on_disk) {
        (true, _) => StopOutcome::Stop,
        (false, true) => StopOutcome::AlreadyStopped,
        (false, false) => StopOutcome::NotFound,
    }
}

fn list_servers_local(json: bool) -> Result<()> {
    let project_dir = init::canonical_project_dir()?;
    let entries = server::list_all_servers()?;
    let running_count = entries.iter().filter(|e| e.running).count();
    let total = entries.len();

    let out = output::ServerListOutput {
        servers: entries
            .into_iter()
            .map(|e| {
                let running = e.running;
                let (display_name, pid, version, http_port, tcp_port, engine, container_id) =
                    match e.info {
                        Some(info) => {
                            let is_ch = info.engine == server::Engine::Clickhouse;
                            let pid = if is_ch && running {
                                Some(info.pid)
                            } else {
                                None
                            };
                            // ClickHouse resolves its version and ports on each
                            // start, so stopped entries expose identity only.
                            let version = if !is_ch || running {
                                Some(info.version)
                            } else {
                                None
                            };
                            let http_port = if is_ch && running {
                                Some(info.http_port)
                            } else {
                                None
                            };
                            let tcp_port = if !is_ch || running {
                                Some(info.tcp_port)
                            } else {
                                None
                            };
                            // For Postgres the disk key is `<name>-pg<major>`;
                            // show users the friendly name without the suffix.
                            let display = if is_ch {
                                e.name.clone()
                            } else {
                                postgres::user_name_from_key(&e.name).to_string()
                            };
                            (
                                display,
                                pid,
                                version,
                                http_port,
                                tcp_port,
                                info.engine.as_str().to_string(),
                                info.container_id,
                            )
                        }
                        None => (
                            e.name.clone(),
                            None,
                            None,
                            None,
                            None,
                            "clickhouse".to_string(),
                            None,
                        ),
                    };
                output::ServerListEntry {
                    name: display_name,
                    running,
                    pid,
                    version,
                    http_port,
                    tcp_port,
                    project: None,
                    engine,
                    container_id,
                }
            })
            .collect(),
        total_servers: total,
        total_running_servers: running_count,
        project_scope: Some(output::exact_current_project_scope(&project_dir)),
        guidance: if total == 0 {
            output::project_scope_guidance(None)
        } else {
            Vec::new()
        },
    };
    output::print_output(&out, json);
    Ok(())
}

fn list_servers_global(json: bool) -> Result<()> {
    let entries = server::list_all_servers_global();
    let total = entries.len();

    let out = output::ServerListOutput {
        servers: entries
            .into_iter()
            .map(|e| output::ServerListEntry {
                name: e.name,
                running: true,
                pid: Some(e.pid),
                version: e.version,
                http_port: e.http_port,
                tcp_port: e.tcp_port,
                project: Some(e.project),
                engine: e.engine.as_str().to_string(),
                container_id: e.container_id,
            })
            .collect(),
        total_servers: total,
        total_running_servers: total,
        project_scope: None,
        guidance: Vec::new(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn stop_server_global(
    name_input: ServerNameInput,
    project: Option<&str>,
    json: bool,
) -> Result<()> {
    let all = server::list_all_servers_global();
    let (name, selection) = global_stop_target(name_input)?;
    let matches: Vec<_> = all
        .iter()
        .filter(|entry| entry.name == name)
        .filter(|entry| project.is_none_or(|project| entry.project == project))
        .collect();

    if matches.is_empty() {
        return Err(Error::ServerNotFound(name));
    }

    if matches.len() > 1 {
        let projects: Vec<_> = matches.iter().map(|e| e.project.as_str()).collect();
        return Err(Error::ServerInMultipleProjects {
            name,
            projects: projects.join(", "),
        });
    }

    let entry = matches[0];
    if !json {
        println!("Stopping server '{}' in {}...", entry.name, entry.project);
    }
    server::kill_server_by_pid(entry.pid)?;
    let out = output::ServerStopOutput {
        name,
        already_stopped: false,
        selection: Some(selection),
    };
    output::print_output(&out, json);
    Ok(())
}

fn global_stop_target(name_input: ServerNameInput) -> Result<(String, output::ServerSelection)> {
    match name_input {
        ServerNameInput::Positional(name) | ServerNameInput::NameFlag(name) => {
            server::validate_server_name(&name)?;
            Ok((name, output::ServerSelection::Explicit))
        }
        ServerNameInput::Omitted => Ok(("default".to_string(), output::ServerSelection::Implicit)),
    }
}

fn stop_all_servers_local(json: bool) -> Result<()> {
    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let servers = server::list_running_servers_locked(&metadata_lock)?;
    let out = stop_servers(&servers, json, |name| {
        server::kill_server_locked(name, &metadata_lock)
    });
    if json {
        output::print_output(&out, json);
    } else if servers.is_empty() {
        println!("No running servers");
    } else {
        println!("Done");
    }
    Ok(())
}

pub(crate) fn stop_servers<F>(
    servers: &[server::ServerInfo],
    json: bool,
    mut stop: F,
) -> output::ServerStopAllOutput
where
    F: FnMut(&str) -> Result<()>,
{
    let servers = servers
        .iter()
        .map(|server| {
            let (name, version) = match server.engine {
                server::Engine::Clickhouse => (server.name.clone(), None),
                server::Engine::Postgres => (
                    postgres::user_name_from_key(&server.name).to_string(),
                    Some(server.version.clone()),
                ),
            };
            let engine = server.engine.as_str().to_string();
            if !json {
                match version.as_deref() {
                    Some(version) => print!("Stopping '{}' ({}, {})...", name, engine, version),
                    None => print!("Stopping '{}' ({})...", name, engine),
                }
                let _ = std::io::stdout().flush();
            }
            let result = stop(&server.name);
            if !json {
                match &result {
                    Ok(()) => println!(" stopped"),
                    Err(error) => println!(" error: {error}"),
                }
            }
            output::ServerStopEntry {
                name,
                engine,
                version,
                stopped: result.is_ok(),
                error: result.err().map(|error| error.to_string()),
            }
        })
        .collect();

    output::ServerStopAllOutput { servers }
}

fn stop_all_servers_global(json: bool) -> Result<()> {
    let servers = server::list_all_servers_global();
    let mut stop_entries = Vec::new();
    for s in &servers {
        if !json {
            print!(
                "Stopping '{}' ({}, {})...",
                s.name,
                s.engine.as_str(),
                s.project
            );
            let _ = std::io::stdout().flush();
        }
        match server::kill_server_by_pid(s.pid) {
            Ok(()) => {
                if !json {
                    println!(" stopped");
                }
                stop_entries.push(output::ServerStopEntry {
                    name: s.name.clone(),
                    engine: s.engine.as_str().to_string(),
                    version: None,
                    stopped: true,
                    error: None,
                });
            }
            Err(e) => {
                if !json {
                    println!(" error: {}", e);
                }
                stop_entries.push(output::ServerStopEntry {
                    name: s.name.clone(),
                    engine: s.engine.as_str().to_string(),
                    version: None,
                    stopped: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    if json {
        let out = output::ServerStopAllOutput {
            servers: stop_entries,
        };
        output::print_output(&out, json);
    } else if servers.is_empty() {
        println!("No running servers");
    } else {
        println!("Done");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_query_support_matches_the_native_client_contract() {
        // ClickHouse introduced repeatable --query in v23.9.1.1854. The pinned
        // release evidence is linked from README.md; these tests need no live binary.
        assert!(ensure_repeated_query_supported("23.8.1.2992", 1).is_ok());
        assert!(ensure_repeated_query_supported("23.9.1.1853", 2).is_err());
        assert!(ensure_repeated_query_supported(REPEATED_QUERY_MIN_VERSION, 2).is_ok());
        assert!(ensure_repeated_query_supported("25.12.9.61", 3).is_ok());
        assert!(ensure_repeated_query_supported("26.8.1.1760", usize::MAX).is_ok());
    }

    /// Write `contents` at `path` with the given mode, creating parents.
    fn write_with_mode(path: &Path, contents: &str, mode: u32) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(path, contents).expect("write file");
        let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(mode);
        std::fs::set_permissions(path, permissions).expect("set mode");
    }

    #[test]
    fn launchable_binary_passes_the_pre_exec_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("versions/25.12.9.61/clickhouse");
        write_with_mode(&binary, "#!/bin/sh\nexit 0\n", 0o755);
        assert!(ensure_launchable(&binary, "25.12.9.61").is_ok());
    }

    /// The classified problem for a path the pre-flight rejects.
    fn launch_problem(binary: &Path, version: &str) -> BinaryLaunchProblem {
        match ensure_launchable(binary, version).expect_err("path must be rejected") {
            Error::BinaryNotLaunchable { problem, .. } => problem,
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unlaunchable_binaries_fail_before_the_pre_exec_hook() {
        let dir = tempfile::tempdir().expect("tempdir");

        // No execute bit: `exec()` would return EACCES *after* the handoff had
        // already been recorded, so the check names the file and the repair.
        let not_executable = dir.path().join("versions/25.12.9.61/clickhouse");
        write_with_mode(&not_executable, "#!/bin/sh\nexit 0\n", 0o644);
        assert_eq!(
            launch_problem(&not_executable, "25.12.9.61"),
            BinaryLaunchProblem::NotExecutable
        );

        // A directory in the binary's place: `exists()` is true, so resolution
        // accepts it and only the pre-flight can reject it.
        let directory = dir.path().join("versions/26.8.1.1760/clickhouse");
        std::fs::create_dir_all(&directory).expect("create directory");
        assert_eq!(
            launch_problem(&directory, "26.8.1.1760"),
            BinaryLaunchProblem::NotAFile
        );

        // Missing entirely: a build removed after version resolution.
        let missing = dir.path().join("versions/27.1.2.3/clickhouse");
        assert_eq!(
            launch_problem(&missing, "27.1.2.3"),
            BinaryLaunchProblem::Missing
        );
    }

    #[test]
    fn launch_failure_message_names_the_build_the_path_and_the_repair() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("versions/25.12.9.61/clickhouse");
        write_with_mode(&binary, "#!/bin/sh\nexit 0\n", 0o644);
        let message = ensure_launchable(&binary, "25.12.9.61")
            .expect_err("a non-executable binary cannot be launched")
            .to_string();
        assert!(message.contains("not executable"), "{message}");
        assert!(message.contains(&binary.display().to_string()), "{message}");
        assert!(
            message.contains("clickhousectl local install --force 25.12.9.61"),
            "{message}"
        );
    }

    /// `install --force` renames a fresh binary over the path, which fails
    /// with EISDIR when a directory sits there; that reason has to recommend
    /// `local remove` first.
    #[test]
    fn directory_at_binary_path_recommends_remove_then_install() {
        let dir = tempfile::tempdir().expect("tempdir");
        let directory = dir.path().join("versions/26.8.1.1760/clickhouse");
        std::fs::create_dir_all(&directory).expect("create directory");
        let message = ensure_launchable(&directory, "26.8.1.1760")
            .expect_err("a directory cannot be launched")
            .to_string();
        assert!(message.contains("not a regular file"), "{message}");
        assert!(
            message.contains(
                "clickhousectl local remove 26.8.1.1760 && clickhousectl local install 26.8.1.1760"
            ),
            "{message}"
        );
        assert!(!message.contains("--force"), "{message}");
    }

    #[test]
    fn group_and_other_execute_bits_count_as_launchable() {
        // The check asks "can anyone execute this", not "can the owner": a
        // build installed with 0o711 or 0o111 must not be refused.
        let dir = tempfile::tempdir().expect("tempdir");
        for mode in [0o711, 0o151, 0o111] {
            let binary = dir.path().join(format!("mode-{mode:o}/clickhouse"));
            write_with_mode(&binary, "#!/bin/sh\nexit 0\n", mode);
            assert!(
                ensure_launchable(&binary, "25.12.9.61").is_ok(),
                "mode {mode:o} must be launchable"
            );
        }
    }

    fn server_info(name: &str, engine: server::Engine, version: &str) -> server::ServerInfo {
        server::ServerInfo {
            name: name.to_string(),
            pid: 1,
            version: version.to_string(),
            http_port: 0,
            tcp_port: 0,
            started_at: "test".to_string(),
            cwd: "/tmp/project".to_string(),
            engine,
            container_id: None,
        }
    }

    #[test]
    fn classify_stop_running_server_is_stopped() {
        // Running takes precedence regardless of on-disk state.
        assert_eq!(classify_stop(true, true), StopOutcome::Stop);
        assert_eq!(classify_stop(true, false), StopOutcome::Stop);
    }

    #[test]
    fn classify_stop_existing_but_stopped_is_idempotent_noop() {
        assert_eq!(classify_stop(false, true), StopOutcome::AlreadyStopped);
    }

    #[test]
    fn classify_stop_unknown_name_is_not_found() {
        assert_eq!(classify_stop(false, false), StopOutcome::NotFound);
    }

    #[test]
    fn server_name_input_preserves_cli_source() {
        assert_eq!(
            ServerNameInput::from_args(None, None),
            ServerNameInput::Omitted
        );
        assert_eq!(
            ServerNameInput::from_args(Some("positional".into()), None),
            ServerNameInput::Positional("positional".into())
        );
        assert_eq!(
            ServerNameInput::from_args(None, Some("flagged".into())),
            ServerNameInput::NameFlag("flagged".into())
        );
    }

    #[test]
    fn omitted_global_stop_keeps_the_literal_default_target() {
        assert_eq!(
            global_stop_target(ServerNameInput::Omitted).unwrap(),
            ("default".to_string(), output::ServerSelection::Implicit)
        );
    }

    #[test]
    fn stop_servers_attempts_and_reports_both_engines() {
        let servers = vec![
            server_info("default", server::Engine::Clickhouse, "25.12.9.61"),
            server_info("default-pg17", server::Engine::Postgres, "postgres:17"),
            server_info("default-pg18", server::Engine::Postgres, "postgres:18"),
        ];
        let mut attempts = Vec::new();

        let output = stop_servers(&servers, true, |name| {
            attempts.push(name.to_string());
            if name == "default" {
                Err(Error::Exec("process stop failed".to_string()))
            } else {
                Ok(())
            }
        });

        assert_eq!(attempts, ["default", "default-pg17", "default-pg18"]);
        assert_eq!(output.servers.len(), 3);
        assert_eq!(output.servers[0].name, "default");
        assert_eq!(output.servers[0].engine, "clickhouse");
        assert_eq!(output.servers[0].version, None);
        assert!(!output.servers[0].stopped);
        assert_eq!(
            output.servers[0].error.as_deref(),
            Some("Failed to execute ClickHouse: process stop failed")
        );
        assert_eq!(output.servers[1].name, "default");
        assert_eq!(output.servers[1].engine, "postgres");
        assert_eq!(output.servers[1].version.as_deref(), Some("postgres:17"));
        assert!(output.servers[1].stopped);
        assert_eq!(output.servers[1].error, None);
        assert_eq!(output.servers[2].name, "default");
        assert_eq!(output.servers[2].engine, "postgres");
        assert_eq!(output.servers[2].version.as_deref(), Some("postgres:18"));
        assert!(output.servers[2].stopped);
        assert_eq!(output.servers[2].error, None);
    }

    #[test]
    fn update_dotenv_postgres_prefix_isolates_clickhouse_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\nDATABASE_URL=x\n";
        let vars = vec![
            ("POSTGRES_HOST", "localhost".to_string()),
            ("POSTGRES_PORT", "5432".to_string()),
        ];
        let result = update_dotenv(existing, "POSTGRES_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
        assert!(result.contains("POSTGRES_HOST=localhost"));
        assert!(result.contains("POSTGRES_PORT=5432"));
    }

    #[test]
    fn extract_dotenv_key_postgres_prefix() {
        assert_eq!(
            extract_dotenv_key("POSTGRES_USER=postgres", "POSTGRES_"),
            Some("POSTGRES_USER")
        );
        assert_eq!(extract_dotenv_key("CLICKHOUSE_HOST=x", "POSTGRES_"), None);
    }

    #[test]
    fn update_dotenv_creates_fresh_content() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv("", "CLICKHOUSE_", &vars);
        assert_eq!(result, "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\n");
    }

    #[test]
    fn update_dotenv_replaces_existing_vars() {
        let existing =
            "CLICKHOUSE_HOST=oldhost\nDATABASE_URL=postgres://...\nCLICKHOUSE_PORT=1234\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
        assert!(result.contains("DATABASE_URL=postgres://..."));
        assert!(!result.contains("oldhost"));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_preserves_non_clickhouse_vars() {
        let existing = "FOO=bar\nBAZ=qux\n";
        let vars = vec![("CLICKHOUSE_HOST", "localhost".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("FOO=bar"));
        assert!(result.contains("BAZ=qux"));
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
    }

    #[test]
    fn update_dotenv_appends_missing_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
    }

    #[test]
    fn update_dotenv_handles_export_prefix() {
        let existing = "export CLICKHOUSE_HOST=oldhost\nexport CLICKHOUSE_PORT=1234\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("export CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(!result.contains("oldhost"));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_handles_spaces_around_equals() {
        let existing = "CLICKHOUSE_HOST = oldhost\n";
        let vars = vec![("CLICKHOUSE_HOST", "localhost".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(!result.contains("oldhost"));
    }

    #[test]
    fn update_dotenv_handles_export_with_spaces() {
        let existing = "export CLICKHOUSE_PORT = 1234\nDATABASE_URL=postgres://...\n";
        let vars = vec![("CLICKHOUSE_PORT", "9000".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(result.contains("DATABASE_URL=postgres://..."));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_preserves_unmanaged_clickhouse_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PASSWORD=secret\n";
        // Only updating HOST — PASSWORD should be left alone
        let vars = vec![("CLICKHOUSE_HOST", "newhost".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=newhost"));
        assert!(result.contains("CLICKHOUSE_PASSWORD=secret"));
    }

    #[test]
    fn extract_dotenv_key_simple() {
        assert_eq!(
            extract_dotenv_key("CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_with_export() {
        assert_eq!(
            extract_dotenv_key("export CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_with_spaces() {
        assert_eq!(
            extract_dotenv_key("CLICKHOUSE_HOST = localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
        assert_eq!(
            extract_dotenv_key("export CLICKHOUSE_HOST = localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_non_clickhouse() {
        assert_eq!(
            extract_dotenv_key("DATABASE_URL=postgres://...", "CLICKHOUSE_"),
            None
        );
        assert_eq!(extract_dotenv_key("export FOO=bar", "CLICKHOUSE_"), None);
    }

    #[test]
    fn extract_dotenv_key_comment_and_blank() {
        assert_eq!(
            extract_dotenv_key("# CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            None
        );
        assert_eq!(extract_dotenv_key("", "CLICKHOUSE_"), None);
    }

    #[test]
    fn format_dotenv_line_plain_value() {
        assert_eq!(format_dotenv_line("", "KEY", "value"), "KEY=value");
    }

    #[test]
    fn format_dotenv_line_with_prefix() {
        assert_eq!(
            format_dotenv_line("export ", "KEY", "value"),
            "export KEY=value"
        );
    }

    #[test]
    fn format_dotenv_line_quotes_spaces() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "my secret"),
            r#"CLICKHOUSE_PASSWORD="my secret""#
        );
    }

    #[test]
    fn format_dotenv_line_quotes_hash() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "pass#123"),
            r#"CLICKHOUSE_PASSWORD="pass#123""#
        );
    }

    #[test]
    fn format_dotenv_line_escapes_quotes_and_backslashes() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", r#"a"b\c"#),
            r#"CLICKHOUSE_PASSWORD="a\"b\\c""#
        );
    }

    #[test]
    fn format_dotenv_line_escapes_newlines() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "line1\nline2"),
            r#"CLICKHOUSE_PASSWORD="line1\nline2""#
        );
    }

    #[test]
    fn format_dotenv_line_quotes_empty_value() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", ""),
            r#"CLICKHOUSE_PASSWORD="""#
        );
    }

    #[test]
    fn update_dotenv_quotes_special_values() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PASSWORD", "my secret#123".to_string()),
        ];
        let result = update_dotenv("", "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="my secret#123""#));
    }

    #[test]
    fn update_dotenv_quotes_when_replacing_in_place() {
        let existing = "CLICKHOUSE_PASSWORD=old\n";
        let vars = vec![("CLICKHOUSE_PASSWORD", "new pass".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="new pass""#));
    }
}
