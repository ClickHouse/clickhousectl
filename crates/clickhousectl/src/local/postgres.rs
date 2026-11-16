//! Handlers for `clickhousectl local postgres ...` subcommands.
//!
//! An instance runs in one of two runtimes ([`PgRuntime`]) and keeps the one
//! it was created with: native, a postmaster from a downloaded bundle
//! (`local::pg_bundle`, `local::pg_native`), or Docker (`local::docker`).
//! `start` chooses for a new instance in [`select_start_runtime`]; every other
//! command reads the choice back with `ServerInfo::pg_runtime`. State is
//! reused from `local::server` — Postgres entries land in the same metadata
//! directory and show up alongside ClickHouse in `local status`.

use crate::error::{Error, PortKind, Result, StartupKind};
use crate::local::cli::PostgresCommands;
use crate::local::docker::{self, PostgresRunOpts};
use crate::local::output;
use crate::local::pg_bundle::{self, InstalledBundle, PgVersionRequest};
use crate::local::pg_native::{self, BundleIdentity, NativeState};
use crate::local::server::{self, Engine, PgRuntime, ServerInfo};
use rand::distr::{Alphanumeric, SampleString};
use std::collections::HashSet;
use std::future::Future;
use std::io::IsTerminal;
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;

const DEFAULT_PG_PORT: u16 = 5432;
const DEFAULT_USER: &str = "postgres";
const DEFAULT_DATABASE: &str = "postgres";
/// Default version when `--version` is not given: a major both runtimes
/// serve. Docker also takes any 17/18 image tag (`17.0`, `18-bookworm`, etc).
pub const DEFAULT_PG_TAG: &str = "18";
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(200);
const READINESS_LOG_LINES: usize = 50;
const READINESS_LOG_BYTES: usize = 16 * 1024;
const READINESS_LOG_TIMEOUT: Duration = Duration::from_secs(2);

/// Extract the major-version digits from a Postgres image tag. `17-alpine` →
/// `"17"`, `17.0` → `"17"`, `18-bookworm` → `"18"`. Validation is the caller's
/// responsibility (`validate_pg_tag`) — this only parses.
pub(crate) fn pg_major_from_tag(tag: &str) -> String {
    tag.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Accept Postgres image tags in the form `17|18[.<minor>][-<variant>]`.
/// The variant follows Docker's tag character grammar. Examples that pass:
/// `17`, `17.0`, `17-alpine`, `18-bookworm`, `18.1-alpine3.20`.
pub(crate) fn validate_pg_tag(tag: &str) -> Result<()> {
    let valid = tag.len() <= 128 && tag.is_ascii() && {
        let (version, variant) = match tag.split_once('-') {
            Some((version, variant)) => (version, Some(variant)),
            None => (tag, None),
        };
        let variant_valid = variant.is_none_or(|variant| {
            let mut chars = variant.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        });
        let (major, minor) = match version.split_once('.') {
            Some((major, minor)) => (major, Some(minor)),
            None => (version, None),
        };
        let minor_valid = minor
            .is_none_or(|minor| !minor.is_empty() && minor.chars().all(|c| c.is_ascii_digit()));
        matches!(major, "17" | "18") && minor_valid && variant_valid
    };

    if !valid {
        return Err(Error::PostgresUsage(format!(
            "invalid or unsupported postgres version '{}'. Use 17 or 18, optionally followed \
             by .<minor> and -<variant> (for example: 17, 17-alpine, 18.1, 18-bookworm).",
            tag
        )));
    }
    Ok(())
}

pub(crate) fn parse_pg_tag_arg(tag: &str) -> std::result::Result<String, String> {
    validate_pg_tag(tag)
        .map(|()| tag.to_string())
        .map_err(|error| error.to_string())
}

fn validate_pg_env_assignment(assignment: &str) -> std::result::Result<(&str, &str), String> {
    let Some((key, value)) = assignment.split_once('=') else {
        return Err(format!(
            "invalid environment variable '{assignment}': expected KEY=VALUE"
        ));
    };
    let mut chars = key.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "invalid environment variable key '{key}': use letters, digits, and underscores, and do not start with a digit"
        ));
    }
    match key {
        "POSTGRES_USER" => {
            Err("POSTGRES_USER is managed by clickhousectl; use --user instead of --env".into())
        }
        "POSTGRES_DB" => {
            Err("POSTGRES_DB is managed by clickhousectl; use --database instead of --env".into())
        }
        "PGDATA" => Err("PGDATA is managed by clickhousectl and cannot be set with --env".into()),
        _ => Ok((key, value)),
    }
}

pub(crate) fn parse_pg_env_arg(assignment: &str) -> std::result::Result<String, String> {
    validate_pg_env_assignment(assignment).map(|_| assignment.to_string())
}

pub(crate) fn validate_pg_start_env_args(
    password: Option<&str>,
    extra_env: &[String],
) -> std::result::Result<(), String> {
    let mut seen = HashSet::new();
    let mut has_password_env = false;
    for assignment in extra_env {
        let (key, _) = validate_pg_env_assignment(assignment)?;
        if !seen.insert(key) {
            return Err(format!(
                "environment variable '{key}' was provided more than once; pass each --env key only once"
            ));
        }
        has_password_env |= key == "POSTGRES_PASSWORD";
    }
    if password.is_some() && has_password_env {
        return Err(
            "POSTGRES_PASSWORD cannot be set with both --password and --env; choose one".into(),
        );
    }
    Ok(())
}

struct StartPreflight {
    host_port: Option<u16>,
    extra_env: Vec<String>,
    password_from_env: Option<String>,
}

fn validate_start_options(
    name: Option<&str>,
    version: Option<&str>,
    port: Option<u16>,
    password: Option<&str>,
    extra_env: Vec<String>,
) -> Result<StartPreflight> {
    if let Some(name) = name {
        server::validate_server_name(name)?;
    }
    if let Some(version) = version {
        validate_pg_tag(version)?;
    }

    validate_pg_start_env_args(password, &extra_env).map_err(Error::PostgresUsage)?;
    let password_from_env = extra_env.iter().find_map(|assignment| {
        assignment
            .strip_prefix("POSTGRES_PASSWORD=")
            .map(str::to_string)
    });
    let validated_env = extra_env
        .into_iter()
        .filter(|assignment| !assignment.starts_with("POSTGRES_PASSWORD="))
        .collect();

    let host_port = port.map(|port| resolve_port(Some(port))).transpose()?;
    Ok(StartPreflight {
        host_port,
        extra_env: validated_env,
        password_from_env,
    })
}

pub async fn run(cmd: PostgresCommands, json: bool) -> Result<()> {
    match cmd {
        PostgresCommands::Start {
            name,
            name_flag,
            version,
            port,
            user,
            password,
            database,
            env,
            docker,
            wait_timeout,
        } => {
            start(
                StartArgs {
                    name: name.or(name_flag),
                    version,
                    port,
                    user,
                    password,
                    database,
                    extra_env: env,
                    docker,
                    wait_timeout: Duration::from_secs(wait_timeout.into()),
                },
                json,
            )
            .await
        }
        PostgresCommands::Stop {
            name,
            name_flag,
            version,
        } => stop(name.or(name_flag), version.as_deref(), json).await,
        PostgresCommands::List => super::list_project_servers(output::ProjectList::Postgres, json),
        PostgresCommands::StopAll => stop_all(json).await,
        PostgresCommands::Remove {
            name,
            name_flag,
            version,
        } => remove(
            name.or(name_flag).as_deref().unwrap_or("default"),
            version.as_deref(),
            json,
        ),
        PostgresCommands::Client {
            name,
            name_flag,
            version,
            host,
            port,
            query,
            queries_file,
            args,
        } => {
            client(
                name.or(name_flag),
                version,
                host,
                port,
                query,
                queries_file,
                args,
            )
            .await
        }
        PostgresCommands::Dotenv {
            name,
            name_flag,
            version,
            local,
        } => dotenv(
            name.or(name_flag).as_deref(),
            version.as_deref(),
            local,
            json,
        ),
    }
}

struct StartArgs {
    name: Option<String>,
    version: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    extra_env: Vec<String>,
    docker: bool,
    wait_timeout: Duration,
}

/// What a start asks for that only the Docker runtime provides.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DockerOnlyFeature {
    /// `--env`: container environment variables.
    Env,
    /// A `--version` that is an image tag rather than `17`, `18` or
    /// `<major>.<minor>`.
    ImageTag(String),
}

impl DockerOnlyFeature {
    fn of(version: Option<&str>, has_extra_env: bool) -> Option<Self> {
        if has_extra_env {
            return Some(Self::Env);
        }
        version
            .filter(|tag| PgVersionRequest::parse(tag).is_none())
            .map(|tag| Self::ImageTag(tag.to_string()))
    }

    fn describe(&self) -> String {
        match self {
            Self::Env => "`--env` sets container environment variables".to_string(),
            Self::ImageTag(tag) => format!(
                "`--version {tag}` is a Docker image tag (the native runtime takes 17, 18 or \
                 <major>.<minor>)"
            ),
        }
    }
}

/// The one decision between runtimes for `start`.
///
/// An existing instance keeps the runtime it was created with, so a Docker
/// instance from before the native runtime existed resumes without the flag.
/// A new instance is native unless `--docker` is given. Asking for the other
/// runtime, or for a Docker-only feature without it, is a usage error that
/// names the fix.
fn select_start_runtime(
    prior: Option<PgRuntime>,
    docker: bool,
    docker_only: Option<&DockerOnlyFeature>,
    name: &str,
    major: &str,
) -> Result<PgRuntime> {
    use clap::error::ErrorKind;
    let recreate = format!(
        "remove it first with `clickhousectl local postgres remove {name} --version {major}` \
         (this deletes its data) to recreate it with `--docker`"
    );
    match (prior, docker, docker_only) {
        (Some(PgRuntime::Docker), _, _) | (None, true, _) => Ok(PgRuntime::Docker),
        (Some(PgRuntime::Native), true, _) => Err(Error::usage(
            ErrorKind::ArgumentConflict,
            format!(
                "Postgres instance '{name}' (major {major}) runs natively, and `--docker` cannot \
                 change an existing instance. Start it without `--docker`, or {recreate}"
            ),
        )),
        (Some(PgRuntime::Native), false, Some(feature)) => Err(Error::usage(
            ErrorKind::ArgumentConflict,
            format!(
                "{}, which needs the Docker runtime, and Postgres instance '{name}' (major \
                 {major}) runs natively. Drop the option, or {recreate}",
                feature.describe()
            ),
        )),
        (None, false, Some(feature)) => Err(Error::usage(
            ErrorKind::ArgumentConflict,
            format!(
                "{}, which needs the Docker runtime; add `--docker`",
                feature.describe()
            ),
        )),
        (Some(PgRuntime::Native) | None, false, None) => Ok(PgRuntime::Native),
    }
}

/// The error for an instance whose runtime changed between choosing a
/// runtime and acting on it: another command removed and recreated it.
fn runtime_changed(name: &str) -> Error {
    Error::PostgresUsage(format!(
        "Postgres instance '{name}' was changed by another command while this one was starting \
         it; run the command again"
    ))
}

async fn start(args: StartArgs, json: bool) -> Result<()> {
    let StartArgs {
        name,
        version,
        port,
        user,
        password,
        database,
        extra_env,
        docker,
        wait_timeout,
    } = args;
    let docker_only = DockerOnlyFeature::of(version.as_deref(), !extra_env.is_empty());
    let has_extra_env = !extra_env.is_empty();
    let preflight = validate_start_options(
        name.as_deref(),
        version.as_deref(),
        port,
        password.as_deref(),
        extra_env,
    )?;

    // Look at what exists under the project lock, then release it: both
    // runtimes resolve their target again before changing anything.
    let (user_name, major, prior) = {
        let metadata_lock = server::lock_metadata()?;
        server::recover_current_project_servers_locked(&metadata_lock)?;
        let user_name = match name {
            Some(name) => name,
            None => default_pg_name_locked(&metadata_lock)?,
        };
        let (_, major) =
            resolve_pg_start_version_locked(&user_name, version.as_deref(), &metadata_lock)?;
        let key = server::pg_instance_key(&user_name, &major);
        let prior = server::load_info_locked(&key, &metadata_lock)?;
        (user_name, major, prior)
    };
    let prior_runtime = prior.as_ref().and_then(ServerInfo::pg_runtime);
    let runtime = select_start_runtime(
        prior_runtime,
        docker,
        docker_only.as_ref(),
        &user_name,
        &major,
    )?;

    let request = StartRequest {
        name: user_name,
        version,
        port,
        user,
        password,
        database,
        preflight,
        has_extra_env,
        wait_timeout,
    };
    match runtime {
        PgRuntime::Docker => {
            if !docker && !json {
                eprintln!(
                    "Note: postgres '{}' (major {major}) is a Docker instance; resuming it in \
                     Docker.",
                    request.name
                );
            }
            start_docker(request, json).await
        }
        PgRuntime::Native => start_native(request, json).await,
    }
}

/// A validated start, for the runtime [`select_start_runtime`] chose.
struct StartRequest {
    /// The instance name, already defaulted.
    name: String,
    version: Option<String>,
    /// The port the user asked for, if any; `preflight.host_port` is the
    /// same port once checked.
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    preflight: StartPreflight,
    has_extra_env: bool,
    wait_timeout: Duration,
}

async fn start_docker(request: StartRequest, json: bool) -> Result<()> {
    let StartRequest {
        name: user_name,
        version,
        port,
        user,
        password,
        database,
        preflight,
        has_extra_env,
        wait_timeout,
    } = request;
    let host_port = preflight.host_port;
    let extra_env = preflight.extra_env;
    let password_from_env = preflight.password_from_env;

    let docker = docker::connect().await?;
    let project_cwd = crate::init::project_root()?.dir.display().to_string();

    loop {
        // Resolve an optimistic target, then release the project-wide lock
        // before potentially slow fresh-image inspection and pulling.
        let metadata_lock = server::lock_metadata()?;
        server::recover_current_project_servers_locked(&metadata_lock)?;
        let (tag, major) =
            resolve_pg_start_version_locked(&user_name, version.as_deref(), &metadata_lock)?;
        let key = server::pg_instance_key(&user_name, &major);
        let prior = server::load_info_locked(&key, &metadata_lock)?;
        drop(metadata_lock);
        if prior
            .as_ref()
            .is_some_and(|prior| prior.pg_runtime() != Some(PgRuntime::Docker))
        {
            return Err(runtime_changed(&user_name));
        }

        if prior.is_none() {
            if !docker::image_exists(&docker, &tag).await? {
                docker::pull_image(&docker, &tag, json).await?;
            }
            docker::ensure_name_free(&docker, &user_name, &major, &project_cwd).await?;
        }

        // The optimistic target may have changed while Docker work was in
        // progress. Re-resolve it before any state-determining mutation.
        let metadata_lock = server::lock_metadata()?;
        let (current_tag, current_major) =
            resolve_pg_start_version_locked(&user_name, version.as_deref(), &metadata_lock)?;
        let current_key = server::pg_instance_key(&user_name, &current_major);
        let current = server::load_info_locked(&current_key, &metadata_lock)?;
        if current_tag != tag || current_key != key || current != prior {
            drop(metadata_lock);
            continue;
        }
        crate::init::ensure_runtime_gitignore()?;

        // Resume path: an instance for this exact (name, major) already exists.
        if let Some(prior) = prior {
            let cid = prior.container_id.as_deref().unwrap_or("");
            let inspected = if cid.is_empty() {
                None
            } else {
                docker::inspect_container(&docker, cid).await?
            };
            let Some(inspected) = inspected else {
                return Err(Error::PostgresUsage(format!(
                    "server '{}' (postgres:{}) has metadata but the container is gone. \
                     Run `clickhousectl local postgres remove {}` to clear the data dir \
                     and start fresh.",
                    user_name, major, user_name
                )));
            };
            if docker::inspected_container_running(&inspected)? {
                return Err(Error::ServerAlreadyRunning(user_name));
            }
            if !json
                && (port.is_some()
                    || user.is_some()
                    || password.is_some()
                    || database.is_some()
                    || has_extra_env)
            {
                eprintln!(
                    "Note: postgres:{major} '{}' already exists; resuming with stored settings. \
                     Run `local postgres remove {}` to start over.",
                    user_name, user_name
                );
            }
            return resume_existing(&docker, prior, wait_timeout, json, metadata_lock).await;
        }

        // Fresh create.
        let host_port = match host_port {
            Some(port) => port,
            None => resolve_port(None)?,
        };

        let instance_dir = server::servers_dir_join(&key)?;
        let remove_fresh_data_on_failure = fresh_instance_dir_is_disposable(&instance_dir);
        server::ensure_pg_data_dir(&user_name, &major)?;
        let data_dir = server::pg_data_dir(&user_name, &major)?;

        let user = user.unwrap_or_else(|| DEFAULT_USER.to_string());
        let database = database.unwrap_or_else(|| DEFAULT_DATABASE.to_string());

        let password = password_from_env
            .or(password)
            .unwrap_or_else(generate_password);

        let opts = PostgresRunOpts {
            user_name: &user_name,
            major: &major,
            tag: &tag,
            host_port,
            data_dir: &data_dir,
            project_cwd: &project_cwd,
            user: &user,
            password: &password,
            database: &database,
            extra_env,
        };

        let container_id = docker::create_postgres(&docker, opts).await?;

        let info = ServerInfo {
            name: key.clone(),
            pid: 0,
            version: format!("postgres:{tag}"),
            last_version: None,
            http_port: 0,
            tcp_port: host_port,
            started_at: server::now_timestamp(),
            cwd: project_cwd.clone(),
            engine: Engine::Postgres,
            container_id: Some(container_id.clone()),
        };
        let startup_result = async {
            docker::start_existing(&docker, &container_id).await?;
            ensure_port_published(&docker, &container_id, &user_name, host_port).await?;
            server::save_server_info_locked(&info, &metadata_lock)
        }
        .await;

        if let Err(primary) = startup_result {
            return Err(rollback_failed_fresh_start(
                &docker,
                &container_id,
                &info,
                remove_fresh_data_on_failure,
                primary,
                &metadata_lock,
            )
            .await);
        }
        drop(metadata_lock);

        if let Err(failure) = wait_for_postgres_ready(&docker, &container_id, wait_timeout).await {
            let primary =
                postgres_readiness_error(&docker, &container_id, &user_name, wait_timeout, failure)
                    .await;
            let metadata_lock = server::lock_metadata()?;
            return Err(rollback_failed_fresh_start(
                &docker,
                &container_id,
                &info,
                remove_fresh_data_on_failure,
                primary,
                &metadata_lock,
            )
            .await);
        }

        let out = output::PostgresStartOutput {
            name: user_name,
            runtime: output::PostgresRuntime::Docker,
            container_id: Some(container_id),
            pid: None,
            image: format!("postgres:{tag}"),
            extensions: Default::default(),
            port: host_port,
            user,
            password,
            database,
        };
        output::print_output(&out, json);
        return Ok(());
    }
}

/// A password `initdb --pwfile` can carry: one non-empty line.
fn validate_native_password(password: &str) -> Result<()> {
    if password.is_empty() || password.contains(['\n', '\r']) {
        return Err(Error::usage(
            clap::error::ErrorKind::InvalidValue,
            "--password must be a single non-empty line",
        ));
    }
    Ok(())
}

async fn start_native(request: StartRequest, json: bool) -> Result<()> {
    let StartRequest {
        name: user_name,
        version,
        port,
        user,
        password,
        database,
        preflight,
        wait_timeout,
        ..
    } = request;
    pg_native::reject_root()?;
    if let Some(password) = &password {
        validate_native_password(password)?;
    }
    let project_cwd = crate::init::project_root()?.dir.display().to_string();

    loop {
        // Resolve an optimistic target, then release the project-wide lock
        // before a possible bundle download.
        let metadata_lock = server::lock_metadata()?;
        let (tag, major) =
            resolve_pg_start_version_locked(&user_name, version.as_deref(), &metadata_lock)?;
        let key = server::pg_instance_key(&user_name, &major);
        let prior = server::load_info_locked(&key, &metadata_lock)?;
        drop(metadata_lock);
        if prior
            .as_ref()
            .is_some_and(|prior| prior.pg_runtime() != Some(PgRuntime::Native))
        {
            return Err(runtime_changed(&user_name));
        }

        let bundle = match &prior {
            // Data directories work across minors of a major, so an existing
            // instance takes the newest installed bundle and needs no network.
            Some(_) => pg_bundle::bundle_for_existing(&major, json).await?,
            None => {
                let request = PgVersionRequest::parse(&tag).ok_or_else(|| {
                    Error::PostgresUsage(format!(
                        "'{tag}' is not a version the native runtime serves; use 17, 18 or \
                         <major>.<minor>, or add `--docker`"
                    ))
                })?;
                pg_bundle::ensure_bundle(&request, false, json).await?
            }
        };

        let metadata_lock = server::lock_metadata()?;
        let (current_tag, current_major) =
            resolve_pg_start_version_locked(&user_name, version.as_deref(), &metadata_lock)?;
        let current_key = server::pg_instance_key(&user_name, &current_major);
        let current = server::load_info_locked(&current_key, &metadata_lock)?;
        if current_tag != tag || current_key != key || current != prior {
            drop(metadata_lock);
            continue;
        }
        crate::init::ensure_runtime_gitignore()?;

        let started = match prior {
            Some(prior) => {
                if !json
                    && (port.is_some()
                        || user.is_some()
                        || password.is_some()
                        || database.is_some())
                {
                    eprintln!(
                        "Note: postgres '{user_name}' (major {major}) already exists; resuming \
                         with stored settings. Run `local postgres remove {user_name}` to start \
                         over."
                    );
                }
                resume_native(prior, &bundle, wait_timeout, metadata_lock).await?
            }
            None => {
                let host_port = match preflight.host_port {
                    Some(port) => port,
                    None => resolve_port(None)?,
                };
                let state = NativeState {
                    postgres_major: major.clone(),
                    bundle: BundleIdentity::of(&bundle),
                    user: user.unwrap_or_else(|| DEFAULT_USER.to_string()),
                    password: password.unwrap_or_else(generate_password),
                    database: database.unwrap_or_else(|| DEFAULT_DATABASE.to_string()),
                };
                let info = ServerInfo {
                    name: key,
                    pid: 0,
                    version: format!("postgres:{}", bundle.spec.postgres_version),
                    last_version: None,
                    http_port: 0,
                    tcp_port: host_port,
                    started_at: server::now_timestamp(),
                    cwd: project_cwd,
                    engine: Engine::Postgres,
                    container_id: None,
                };
                create_native(info, state, &bundle, wait_timeout, metadata_lock).await?
            }
        };

        let NativeStarted { info, state } = started;
        let out = output::PostgresStartOutput {
            name: user_name,
            runtime: output::PostgresRuntime::Native,
            container_id: None,
            pid: Some(info.pid),
            image: info.version,
            extensions: bundle.spec.extensions.clone(),
            port: info.tcp_port,
            user: state.user,
            password: state.password,
            database: state.database,
        };
        output::print_output(&out, json);
        return Ok(());
    }
}

/// A native instance that is running and ready.
struct NativeStarted {
    info: ServerInfo,
    state: NativeState,
}

/// Initialize and start a new native instance. `info` describes it with no
/// PID yet. Any failure stops the postmaster and removes what this attempt
/// created, so a retry starts clean.
async fn create_native(
    mut info: ServerInfo,
    state: NativeState,
    bundle: &InstalledBundle,
    wait_timeout: Duration,
    metadata_lock: server::MetadataLock,
) -> Result<NativeStarted> {
    let instance_dir = metadata_lock.servers_dir().join(&info.name);
    let display_name = user_name_from_key(&info.name).to_string();
    if !fresh_instance_dir_is_disposable(&instance_dir) {
        // Data without metadata has no known credentials to resume with.
        return Err(Error::PostgresUsage(format!(
            "'{}' already holds files, but no Postgres instance is recorded for it. Move or \
             delete that directory, then start again.",
            instance_dir.display()
        )));
    }
    std::fs::create_dir_all(instance_dir.join("data"))?;

    let spawned = async {
        pg_native::initdb(&pg_native::InitOptions {
            bundle,
            instance_dir: &instance_dir,
            name: &display_name,
            user: &state.user,
            password: &state.password,
        })
        .await?;
        pg_native::save_state(&instance_dir, &state)?;
        pg_native::spawn_postmaster(bundle, &instance_dir, info.tcp_port)
    }
    .await;
    let mut child = match spawned {
        Ok(child) => child,
        Err(primary) => {
            return Err(rollback_native_create(
                None,
                &info.name,
                primary,
                &metadata_lock,
            ));
        }
    };
    info.pid = child.id();
    if let Err(primary) = server::save_server_info_locked(&info, &metadata_lock) {
        return Err(rollback_native_create(
            Some(&mut child),
            &info.name,
            primary,
            &metadata_lock,
        ));
    }
    drop(metadata_lock);

    let ready = async {
        pg_native::wait_ready(
            &mut child,
            bundle,
            &instance_dir,
            &display_name,
            info.tcp_port,
            wait_timeout,
        )
        .await?;
        if state.database != DEFAULT_DATABASE {
            pg_native::create_database(
                bundle,
                info.tcp_port,
                &state.user,
                &state.password,
                &state.database,
            )
            .await?;
        }
        Ok(())
    }
    .await;
    if let Err(primary) = ready {
        // Stop the postmaster before anything that can fail, such as taking
        // the lock again.
        let stopped = pg_native::stop_child(&mut child);
        let metadata_lock = server::lock_metadata()?;
        let primary = match stopped {
            Ok(()) => primary,
            Err(cleanup) => {
                return Err(Error::PostgresStartupRollback {
                    primary: Box::new(primary),
                    cleanup: format!(
                        "could not stop the postmaster (PID {}): {cleanup}; its data and \
                         metadata were kept, run `clickhousectl local postgres remove {}` to \
                         clean up",
                        info.pid, display_name
                    ),
                });
            }
        };
        return Err(rollback_native_create(
            None,
            &info.name,
            primary,
            &metadata_lock,
        ));
    }
    Ok(NativeStarted { info, state })
}

/// Undo a failed [`create_native`]: stop the postmaster if it was spawned,
/// then delete the instance directory and metadata this attempt created.
/// When the postmaster cannot be stopped, both are kept so `remove` can
/// finish the job.
fn rollback_native_create(
    child: Option<&mut Child>,
    key: &str,
    primary: Error,
    metadata_lock: &server::MetadataLock,
) -> Error {
    let instance_dir = metadata_lock.servers_dir().join(key);
    let cleanup = || -> std::result::Result<(), String> {
        if let Some(child) = child {
            pg_native::stop_child(child).map_err(|error| {
                format!(
                    "could not stop the postmaster: {error}; run `clickhousectl local postgres \
                     remove {}` to clean up",
                    user_name_from_key(key)
                )
            })?;
        }
        match std::fs::remove_dir_all(&instance_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "failed to remove '{}': {error}",
                    instance_dir.display()
                ));
            }
        }
        server::try_remove_server_info_locked(key, metadata_lock)
            .map_err(|error| format!("failed to remove metadata: {error}"))
    };
    match cleanup() {
        Ok(()) => primary,
        Err(cleanup) => Error::PostgresStartupRollback {
            primary: Box::new(primary),
            cleanup,
        },
    }
}

/// Start an existing, stopped native instance on its stored port with its
/// stored credentials. A failure after the postmaster is spawned stops it
/// and leaves the instance recorded as stopped.
async fn resume_native(
    prior: ServerInfo,
    bundle: &InstalledBundle,
    wait_timeout: Duration,
    metadata_lock: server::MetadataLock,
) -> Result<NativeStarted> {
    let instance_dir = metadata_lock.servers_dir().join(&prior.name);
    let display_name = user_name_from_key(&prior.name).to_string();
    if pg_native::is_running(&instance_dir) {
        return Err(Error::ServerAlreadyRunning(display_name));
    }
    let state = pg_native::load_state(&instance_dir)?
        .filter(|_| pg_native::is_initialized(&instance_dir))
        .ok_or_else(|| {
            Error::PostgresUsage(format!(
                "server '{display_name}' ({}) has metadata but its data directory is gone or \
                 incomplete. Run `clickhousectl local postgres remove {display_name}` to clear \
                 it and start fresh.",
                prior.version
            ))
        })?;
    ensure_resume_port_free(&display_name, prior.tcp_port)?;

    let mut child = pg_native::spawn_postmaster(bundle, &instance_dir, prior.tcp_port)?;
    let stopped = ServerInfo {
        pid: 0,
        ..prior.clone()
    };
    let info = ServerInfo {
        pid: child.id(),
        version: format!("postgres:{}", bundle.spec.postgres_version),
        started_at: server::now_timestamp(),
        ..prior
    };
    let state = NativeState {
        bundle: BundleIdentity::of(bundle),
        ..state
    };
    let recorded = server::save_server_info_locked(&info, &metadata_lock)
        .and_then(|()| pg_native::save_state(&instance_dir, &state));
    let ready = match recorded {
        Ok(()) => {
            drop(metadata_lock);
            pg_native::wait_ready(
                &mut child,
                bundle,
                &instance_dir,
                &display_name,
                info.tcp_port,
                wait_timeout,
            )
            .await
        }
        Err(error) => {
            drop(metadata_lock);
            Err(error)
        }
    };
    if let Err(primary) = ready {
        let stopped_child = pg_native::stop_child(&mut child);
        // Best effort: liveness comes from the postmaster's own PID file, so
        // a PID left behind here is display-only.
        if let Ok(lock) = server::lock_metadata() {
            let _ = server::save_server_info_locked(&stopped, &lock);
        }
        return Err(match stopped_child {
            Ok(()) => primary,
            Err(cleanup) => Error::PostgresStartupRollback {
                primary: Box::new(primary),
                cleanup: format!(
                    "could not stop the resumed postmaster (PID {}): {cleanup}",
                    info.pid
                ),
            },
        });
    }
    Ok(NativeStarted { info, state })
}

/// A fresh attempt owns an absent or empty instance directory, including one
/// containing only an empty `data/` from an earlier pre-container step.
fn fresh_instance_dir_is_disposable(path: &Path) -> bool {
    let mut entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Err(_) => return false,
    };
    let entry = match entries.next() {
        None => return true,
        Some(Ok(entry)) => entry,
        Some(Err(_)) => return false,
    };
    if entries.next().is_some()
        || entry.file_name() != "data"
        || !entry.file_type().is_ok_and(|file_type| file_type.is_dir())
    {
        return false;
    }
    match std::fs::read_dir(entry.path()) {
        Ok(mut data_entries) => data_entries.next().is_none(),
        Err(_) => false,
    }
}

async fn rollback_failed_fresh_start(
    docker: &bollard::Docker,
    container_id: &str,
    info: &ServerInfo,
    remove_fresh_data_on_failure: bool,
    primary: Error,
    metadata_lock: &server::MetadataLock,
) -> Error {
    let instance_dir = metadata_lock.servers_dir().join(&info.name);
    let metadata_path = metadata_lock
        .servers_dir()
        .join(format!("{}.json", info.name));
    let mut diagnostics = Vec::new();

    let container_removed = match docker::remove_container(docker, container_id).await {
        Ok(()) => true,
        Err(error) => {
            diagnostics.push(format!(
                "failed to remove container '{container_id}': {error}"
            ));
            false
        }
    };

    let instance_removed = if remove_fresh_data_on_failure && container_removed {
        match docker::remove_host_dir_blocking(&instance_dir) {
            Ok(()) if !instance_dir.exists() => true,
            Ok(()) => {
                diagnostics.push(format!(
                    "failed to remove fresh Postgres data '{}': path still exists",
                    instance_dir.display()
                ));
                false
            }
            Err(error) => {
                diagnostics.push(format!(
                    "failed to remove fresh Postgres data '{}': {error}",
                    instance_dir.display()
                ));
                false
            }
        }
    } else {
        let reason = if remove_fresh_data_on_failure {
            "the container could not be removed"
        } else {
            "the directory contained data before this start attempt"
        };
        diagnostics.push(format!(
            "retained Postgres data '{}' because {reason}",
            instance_dir.display()
        ));
        false
    };

    if container_removed && instance_removed {
        match server::try_remove_server_info_locked(&info.name, metadata_lock) {
            Ok(()) => return primary,
            Err(error) => diagnostics.push(format!(
                "failed to remove metadata '{}': {error}",
                metadata_path.display()
            )),
        }
    } else {
        match server::save_server_info_locked(info, metadata_lock) {
            Ok(()) => diagnostics.push(format!(
                "recovery metadata retained at '{}'; run `clickhousectl local postgres remove {}` to clean up",
                metadata_path.display(),
                user_name_from_key(&info.name)
            )),
            Err(error) => diagnostics.push(format!(
                "failed to preserve recovery metadata '{}': {error}",
                metadata_path.display()
            )),
        }
    }

    Error::PostgresStartupRollback {
        primary: Box::new(primary),
        cleanup: diagnostics.join("; "),
    }
}

/// Default user-facing name when NAME is omitted: `"default"` if no
/// postgres "default" is running, otherwise a random adjective-noun.
fn default_pg_name_locked(metadata_lock: &server::MetadataLock) -> Result<String> {
    default_pg_name_locked_with(metadata_lock, docker::is_container_running_blocking)
}

fn default_pg_name_locked_with(
    metadata_lock: &server::MetadataLock,
    is_container_running: impl Fn(&str) -> Result<bool>,
) -> Result<String> {
    for info in server::find_pg_instances_locked("default", metadata_lock)? {
        let running = match info.container_id.as_deref() {
            Some(id) => is_container_running(id)?,
            None => pg_native::is_running(&metadata_lock.servers_dir().join(&info.name)),
        };
        if running {
            return server::generate_random_name_locked(metadata_lock);
        }
    }
    Ok("default".into())
}

/// Resolve the image tag for a user-facing name. The caller invokes this both
/// before Docker image preparation and under the final metadata lock.
fn resolve_pg_start_version_locked(
    user_name: &str,
    version: Option<&str>,
    metadata_lock: &server::MetadataLock,
) -> Result<(String, String)> {
    if let Some(version) = version {
        return Ok((version.to_string(), pg_major_from_tag(version)));
    }

    let existing = server::find_pg_instances_locked(user_name, metadata_lock)?;
    match existing.as_slice() {
        [] => Ok((
            DEFAULT_PG_TAG.to_string(),
            pg_major_from_tag(DEFAULT_PG_TAG),
        )),
        [info] => {
            let stored_tag = info
                .version
                .strip_prefix("postgres:")
                .unwrap_or(&info.version);
            Ok((stored_tag.to_string(), pg_major_from_tag(stored_tag)))
        }
        _ => {
            let versions: Vec<&str> = existing.iter().map(|info| info.version.as_str()).collect();
            Err(Error::PostgresUsage(format!(
                "multiple postgres instances named '{}' ({}); pass --version to select one",
                user_name,
                versions.join(", ")
            )))
        }
    }
}

/// Lock project metadata, recover orphans, and resolve the target under that
/// lock. A project with neither server state nor an orphan has no target, so
/// this then creates no `.clickhouse/servers/`.
fn lock_and_resolve_pg_target(
    user_name: &str,
    version: Option<&str>,
) -> Result<(server::ServerInfo, server::MetadataLock)> {
    let Some(metadata_lock) = server::lock_recovered_metadata()? else {
        return Err(missing_pg_target(user_name, version));
    };
    let target = resolve_pg_target_locked(user_name, version, &metadata_lock)?;
    Ok((target, metadata_lock))
}

/// Lock project metadata, pick the instance name by the shared rule
/// ([`super::select_server`], #1064), then its major version with `--version`
/// as [`resolve_pg_target_locked`] does. `Ok(None)` means NAME was omitted and
/// this project has no Postgres instances. `usage` is the command with a
/// `<name>` placeholder, suggested when several names leave it ambiguous.
fn lock_and_select_pg_target(
    explicit: Option<String>,
    version: Option<&str>,
    usage: &'static str,
) -> Result<Option<PgSelection>> {
    if let Some(name) = &explicit {
        server::validate_server_name(name)?;
    }
    if let Some(version) = version {
        validate_pg_tag(version)?;
    }
    let metadata_lock = server::lock_recovered_metadata()?;
    let names = match (&explicit, &metadata_lock) {
        (None, Some(lock)) => server::list_postgres_instance_names_locked(lock)?,
        _ => Vec::new(),
    };
    let Some((name, selection)) = super::select_server(explicit, &names, Engine::Postgres, usage)?
    else {
        return Ok(None);
    };
    let Some(metadata_lock) = metadata_lock else {
        return Err(missing_pg_target(&name, version));
    };
    let target = resolve_pg_target_locked(&name, version, &metadata_lock)?;
    Ok(Some(PgSelection {
        name,
        selection,
        target,
        metadata_lock,
    }))
}

/// The instance a command acts on, and how its name was chosen.
struct PgSelection {
    /// The user-facing name, without the `-pg<major>` suffix.
    name: String,
    selection: output::ServerSelection,
    target: server::ServerInfo,
    metadata_lock: server::MetadataLock,
}

/// [`lock_and_select_pg_target`] for commands that need an instance: with
/// none, `default` is reported missing, as for an explicit name.
fn lock_and_require_pg_target(
    explicit: Option<String>,
    version: Option<&str>,
    usage: &'static str,
) -> Result<PgSelection> {
    lock_and_select_pg_target(explicit, version, usage)?
        .ok_or_else(|| missing_pg_target("default", version))
}

/// The error for a `[NAME] [--version <V>]` selection that matches nothing.
fn missing_pg_target(user_name: &str, version: Option<&str>) -> Error {
    match version {
        Some(v) => match validate_pg_tag(v) {
            Ok(()) => {
                Error::ServerNotFound(format!("{user_name} (postgres:{})", pg_major_from_tag(v)))
            }
            Err(error) => error,
        },
        None => Error::ServerNotFound(user_name.to_string()),
    }
}

/// Resolve `[NAME] [--version <V>]` to a single Postgres instance on disk.
/// If `version` is given, target the (X, major(V)) pair directly. Otherwise:
/// 0 instances → ServerNotFound; 1 → use it; >1 → ask for `--version`.
fn resolve_pg_target_locked(
    user_name: &str,
    version: Option<&str>,
    metadata_lock: &server::MetadataLock,
) -> Result<server::ServerInfo> {
    if let Some(v) = version {
        validate_pg_tag(v)?;
        let major = pg_major_from_tag(v);
        let key = server::pg_instance_key(user_name, &major);
        return server::load_info_locked(&key, metadata_lock)?
            .filter(|i| i.engine == Engine::Postgres)
            .ok_or_else(|| missing_pg_target(user_name, version));
    }
    let instances = server::find_pg_instances_locked(user_name, metadata_lock)?;
    match instances.len() {
        0 => Err(missing_pg_target(user_name, None)),
        1 => Ok(instances.into_iter().next().unwrap()),
        _ => {
            let versions: Vec<String> = instances.iter().map(|i| i.version.clone()).collect();
            Err(Error::PostgresUsage(format!(
                "multiple postgres instances named '{}' ({}); pass --version to select one",
                user_name,
                versions.join(", ")
            )))
        }
    }
}

/// Resume an existing stopped Postgres container. Reads credentials from the
/// container's persisted env (the source of truth — PGDATA was initialized
/// for them) and refreshes the metadata.
async fn resume_existing(
    docker: &bollard::Docker,
    prior: ServerInfo,
    wait_timeout: Duration,
    json: bool,
    metadata_lock: server::MetadataLock,
) -> Result<()> {
    let container_id = prior.container_id.clone().expect("checked by caller");
    let display_name = user_name_from_key(&prior.name).to_string();

    // The container keeps the host port it was created with. Starting it
    // while another process holds that port either fails opaquely or starts
    // it unpublished, so check first and confirm the binding after (#1227).
    ensure_resume_port_free(&display_name, prior.tcp_port)?;
    docker::start_existing(docker, &container_id).await?;
    if let Err(primary) =
        ensure_port_published(docker, &container_id, &display_name, prior.tcp_port).await
    {
        drop(metadata_lock);
        return Err(stop_failed_resume(docker, &container_id, primary, "port check").await);
    }

    let info = ServerInfo {
        started_at: server::now_timestamp(),
        ..prior
    };
    if let Err(primary) = server::save_server_info_locked(&info, &metadata_lock) {
        drop(metadata_lock);
        return Err(stop_failed_resume(docker, &container_id, primary, "metadata failure").await);
    }
    drop(metadata_lock);

    if let Err(failure) = wait_for_postgres_ready(docker, &container_id, wait_timeout).await {
        let error =
            postgres_readiness_error(docker, &container_id, &display_name, wait_timeout, failure)
                .await;
        let _ = docker::stop_container(docker, &container_id).await;
        return Err(error);
    }

    let (user, password, database) = read_pg_env(docker, &container_id).await;

    let out = output::PostgresStartOutput {
        name: display_name,
        runtime: output::PostgresRuntime::Docker,
        container_id: Some(container_id),
        pid: None,
        image: info.version,
        extensions: Default::default(),
        port: info.tcp_port,
        user,
        password,
        database,
    };
    output::print_output(&out, json);
    Ok(())
}

/// A stopped container cannot move to another host port, so a resume whose
/// stored port is taken fails instead of starting unpublished (#1227).
fn ensure_resume_port_free(name: &str, port: u16) -> Result<()> {
    if server::is_port_available(port) {
        Ok(())
    } else {
        Err(Error::PostgresResumePortInUse {
            name: name.to_string(),
            port,
        })
    }
}

/// Confirm Docker published `port` for a container it just started (#1227).
async fn ensure_port_published(
    docker: &bollard::Docker,
    container_id: &str,
    name: &str,
    port: u16,
) -> Result<()> {
    let published = docker::inspect_container(docker, container_id)
        .await?
        .is_some_and(|inspected| docker::inspected_postgres_port_published(&inspected, port));
    if published {
        Ok(())
    } else {
        Err(Error::PostgresPortNotPublished {
            name: name.to_string(),
            port,
        })
    }
}

/// Stop a resumed container after a later resume step failed, keeping the
/// step's error as the primary one.
async fn stop_failed_resume(
    docker: &bollard::Docker,
    container_id: &str,
    primary: Error,
    step: &str,
) -> Error {
    match docker::stop_container(docker, container_id).await {
        Ok(()) => primary,
        Err(cleanup) => Error::PostgresStartupRollback {
            primary: Box::new(primary),
            cleanup: format!(
                "could not stop resumed container '{container_id}' after {step}: {cleanup}"
            ),
        },
    }
}

/// Extract the user-facing name from a disk key. `dev-pg16` → `dev`;
/// anything that doesn't match the suffix shape passes through unchanged.
pub(crate) fn user_name_from_key(key: &str) -> &str {
    if let Some(idx) = key.rfind("-pg") {
        let suffix = &key[idx + 3..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return &key[..idx];
        }
    }
    key
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ContainerReadinessState {
    Pending,
    Running,
    Exited {
        status: String,
        exit_code: Option<i64>,
        oom_killed: bool,
    },
}

#[derive(Debug)]
enum ReadinessFailure {
    Exited {
        status: String,
        exit_code: Option<i64>,
        oom_killed: bool,
    },
    Probe(Error),
    TimedOut {
        last_probe_error: Option<String>,
    },
}

trait ReadinessProbe {
    async fn container_state(&mut self) -> Result<ContainerReadinessState>;
    async fn postgres_is_ready(&mut self) -> Result<bool>;
}

struct DockerReadinessProbe<'a> {
    docker: &'a bollard::Docker,
    container_id: &'a str,
}

impl ReadinessProbe for DockerReadinessProbe<'_> {
    async fn container_state(&mut self) -> Result<ContainerReadinessState> {
        let state = docker::container_state(self.docker, self.container_id).await?;
        if state.running {
            Ok(ContainerReadinessState::Running)
        } else if state.exited {
            Ok(ContainerReadinessState::Exited {
                status: state.status,
                exit_code: state.exit_code,
                oom_killed: state.oom_killed,
            })
        } else {
            Ok(ContainerReadinessState::Pending)
        }
    }

    async fn postgres_is_ready(&mut self) -> Result<bool> {
        docker::postgres_is_ready(self.docker, self.container_id).await
    }
}

async fn poll_postgres_readiness<P, S, SFut>(
    probe: &mut P,
    max_checks: usize,
    mut sleep: S,
    last_probe_error: &mut Option<String>,
) -> std::result::Result<(), ReadinessFailure>
where
    P: ReadinessProbe,
    S: FnMut() -> SFut,
    SFut: Future<Output = ()>,
{
    for check in 0..max_checks {
        match probe
            .container_state()
            .await
            .map_err(ReadinessFailure::Probe)?
        {
            ContainerReadinessState::Exited {
                status,
                exit_code,
                oom_killed,
            } => {
                return Err(ReadinessFailure::Exited {
                    status,
                    exit_code,
                    oom_killed,
                });
            }
            ContainerReadinessState::Running => match probe.postgres_is_ready().await {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(error) => {
                    *last_probe_error = Some(error.to_string());
                    if let Ok(ContainerReadinessState::Exited {
                        status,
                        exit_code,
                        oom_killed,
                    }) = probe.container_state().await
                    {
                        return Err(ReadinessFailure::Exited {
                            status,
                            exit_code,
                            oom_killed,
                        });
                    }
                }
            },
            ContainerReadinessState::Pending => {}
        }

        if check + 1 < max_checks {
            sleep().await;
        }
    }
    Err(ReadinessFailure::TimedOut {
        last_probe_error: last_probe_error.take(),
    })
}

async fn wait_for_postgres_ready_with_probe<P: ReadinessProbe>(
    probe: &mut P,
    timeout: Duration,
) -> std::result::Result<(), ReadinessFailure> {
    let mut last_probe_error = None;
    match tokio::time::timeout(
        timeout,
        poll_postgres_readiness(
            probe,
            usize::MAX,
            || tokio::time::sleep(READINESS_POLL_INTERVAL),
            &mut last_probe_error,
        ),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(ReadinessFailure::TimedOut { last_probe_error }),
    }
}

async fn wait_for_postgres_ready(
    docker: &bollard::Docker,
    container_id: &str,
    timeout: Duration,
) -> std::result::Result<(), ReadinessFailure> {
    let mut probe = DockerReadinessProbe {
        docker,
        container_id,
    };
    wait_for_postgres_ready_with_probe(&mut probe, timeout).await
}

async fn postgres_readiness_error(
    docker: &bollard::Docker,
    container_id: &str,
    display_name: &str,
    timeout: Duration,
    failure: ReadinessFailure,
) -> Error {
    let logs = collect_postgres_readiness_logs(
        container_id,
        READINESS_LOG_TIMEOUT,
        docker::container_logs_tail(
            docker,
            container_id,
            READINESS_LOG_LINES,
            READINESS_LOG_BYTES,
        ),
    )
    .await;
    format_postgres_readiness_error(display_name, timeout, failure, &logs)
}

async fn collect_postgres_readiness_logs<F>(
    container_id: &str,
    timeout: Duration,
    logs: F,
) -> String
where
    F: Future<Output = Result<String>>,
{
    match tokio::time::timeout(timeout, logs).await {
        Ok(Ok(logs)) if logs.trim().is_empty() || logs == "(no container logs)" => format!(
            "No container logs were available. Run `docker logs {container_id}` for current diagnostics."
        ),
        Ok(Ok(logs)) => logs,
        Ok(Err(error)) => format!(
            "Could not read container logs ({error}). Run `docker logs {container_id}` for diagnostics."
        ),
        Err(_) => format!(
            "Timed out reading container logs. Run `docker logs {container_id}` for diagnostics."
        ),
    }
}

fn format_postgres_readiness_error(
    display_name: &str,
    timeout: Duration,
    failure: ReadinessFailure,
    logs: &str,
) -> Error {
    let diagnostics = |summary: &str| {
        format!(
            "{summary}\n--- last {READINESS_LOG_LINES} container log lines (maximum {READINESS_LOG_BYTES} bytes) ---\n{logs}"
        )
    };
    match failure {
        ReadinessFailure::Exited {
            status,
            exit_code,
            oom_killed,
        } => {
            let exit_code = exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let oom = if oom_killed { "; out of memory" } else { "" };
            let summary = format!(
                "Postgres container '{display_name}' exited before PostgreSQL became ready \
                 (status: {status}, exit code: {exit_code}{oom})."
            );
            Error::StartupExit {
                kind: StartupKind::Postgres,
                name: display_name.to_string(),
                details: format!("Docker error: {}", diagnostics(&summary)),
            }
        }
        ReadinessFailure::Probe(error) => {
            let summary = format!(
                "Could not check PostgreSQL readiness in container '{display_name}': {error}."
            );
            Error::DockerError(diagnostics(&summary))
        }
        ReadinessFailure::TimedOut { last_probe_error } => {
            let probe_context = last_probe_error
                .map(|error| format!(" Last readiness probe error: {error}."))
                .unwrap_or_default();
            let summary = format!(
                "PostgreSQL in container '{display_name}' did not become ready within {} seconds.{probe_context}",
                timeout.as_secs()
            );
            Error::StartupTimeout {
                kind: StartupKind::Postgres,
                name: display_name.to_string(),
                seconds: timeout.as_secs(),
                details: format!("Docker error: {}", diagnostics(&summary)),
            }
        }
    }
}

fn resolve_port(explicit: Option<u16>) -> Result<u16> {
    match explicit {
        Some(port) if server::is_port_available(port) => {
            return Ok(port);
        }
        Some(port) => {
            return Err(Error::PortInUse {
                kind: PortKind::Postgres,
                port,
            });
        }
        None => {}
    }
    if server::is_port_available(DEFAULT_PG_PORT) {
        return Ok(DEFAULT_PG_PORT);
    }
    for p in (DEFAULT_PG_PORT + 1)..=(DEFAULT_PG_PORT + 100) {
        if server::is_port_available(p) {
            return Ok(p);
        }
    }
    Err(Error::PortUnavailable(PortKind::Postgres))
}

fn generate_password() -> String {
    // 24 alphanumeric chars. Stored only in the container env, from which
    // other processes (and `dotenv`) recover the value.
    Alphanumeric.sample_string(&mut rand::rng(), 24)
}

async fn stop(name: Option<String>, version: Option<&str>, json: bool) -> Result<()> {
    let Some(PgSelection {
        selection,
        target,
        metadata_lock,
        ..
    }) = lock_and_select_pg_target(name, version, "clickhousectl local postgres stop <name>")?
    else {
        // As for ClickHouse: with no instance there is nothing to stop.
        let project = crate::init::project_root()?;
        let project_state_missing = !project.is_marked();
        let out = output::ServerStopNoopOutput {
            engine: Engine::Postgres,
            stopped: false,
            selection: output::ServerSelection::Implicit,
            reason: "no_postgres_servers",
            project_scope: project_state_missing.then(|| output::server_project_scope(project)),
            guidance: if project_state_missing {
                output::project_list_guidance(project.kind, "clickhousectl local postgres list")
            } else {
                Vec::new()
            },
        };
        output::print_output(&out, json);
        return Ok(());
    };
    // `stop` is idempotent, as for ClickHouse: an instance that exists but
    // is not running is already in the desired end state.
    let already_stopped = !server::is_server_running_locked(&target.name, &metadata_lock)?;
    if !already_stopped {
        if !json {
            let display = format!("{} ({})", user_name_from_key(&target.name), target.version);
            println!("Stopping Postgres {}...", display);
        }
        server::kill_server_locked(&target.name, &metadata_lock)?;
    }
    let out = output::ServerStopOutput {
        name: user_name_from_key(&target.name).to_string(),
        already_stopped,
        selection: Some(selection),
    };
    output::print_output(&out, json);
    Ok(())
}

async fn stop_all(json: bool) -> Result<()> {
    let metadata_lock = server::lock_recovered_engine_metadata(Some(Engine::Postgres))?;
    let servers = match &metadata_lock {
        Some(lock) => server::list_running_engine_servers_locked(lock, Engine::Postgres)?,
        None => Vec::new(),
    };
    if !json && servers.is_empty() {
        println!("No running Postgres servers");
        return Ok(());
    }

    let out = super::stop_servers(&servers, json, |name| match &metadata_lock {
        Some(lock) => server::kill_server_locked(name, lock),
        None => Err(Error::ServerNotRunning(name.to_string())),
    });
    if json {
        output::print_json(&out);
    } else {
        println!("Done");
    }
    Ok(())
}

fn remove(name: &str, version: Option<&str>, json: bool) -> Result<()> {
    server::validate_server_name(name)?;
    let (target, metadata_lock) = lock_and_resolve_pg_target(name, version)?;
    let key = target.name.clone();
    if target.pg_runtime() == Some(PgRuntime::Native) {
        // Everything a native instance owns is in its directory, so stopping
        // the postmaster and deleting the directory leaves nothing behind.
        if server::is_server_running_locked(&key, &metadata_lock)? {
            server::kill_server_locked(&key, &metadata_lock)?;
        }
        match std::fs::remove_dir_all(server::servers_dir_join(&key)?) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        server::try_remove_server_info_locked(&key, &metadata_lock)?;
        let out = output::ServerRemoveOutput {
            name: name.to_string(),
            selection: None,
        };
        output::print_output(&out, json);
        return Ok(());
    }
    if server::is_server_running_locked(&key, &metadata_lock)? {
        let tag = target
            .version
            .strip_prefix("postgres:")
            .unwrap_or(&target.version);
        let major = pg_major_from_tag(tag);
        return Err(Error::ServerRunningCannotRemove {
            name: name.to_string(),
            command: format!("clickhousectl local postgres stop {name} --version {major}"),
        });
    }

    if let Some(cid) = target.container_id.as_deref() {
        let _ = docker::stop_and_remove_blocking(cid);
    }

    // Postgres data dir lives at .clickhouse/servers/<key>/data/. Remove the
    // <key>/ wrapper so the (name, version) pair leaves no on-disk state.
    // On Linux the bind-mounted PGDATA contains files owned by uid 999, so
    // a plain rm fails — `remove_host_dir_blocking` falls back to a
    // privileged container in that case.
    let pg_dir = server::servers_dir_join(&key)?;
    docker::remove_host_dir_blocking(&pg_dir)?;
    server::try_remove_server_info_locked(&key, &metadata_lock)?;
    let out = output::ServerRemoveOutput {
        name: name.to_string(),
        selection: None,
    };
    output::print_output(&out, json);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn client(
    name: Option<String>,
    version: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    query: Option<String>,
    queries_file: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    if host.is_some() || port.is_some() {
        // Direct connect — no server lookup; require host psql. Probing before
        // the handoff (the same probe the managed path already uses to choose
        // between host psql and `docker exec`) keeps a missing `psql` an
        // ordinary error event instead of a censored `exec_attempt` (#471).
        // `PostgresUsage`, not `Postgres`: this text is composed here, so it
        // renders verbatim in `--json` and the repair hint reaches agents.
        if !host_has_psql() {
            return Err(Error::PostgresUsage(
                "could not execute psql: not found on PATH (install the PostgreSQL client tools)"
                    .to_string(),
            ));
        }
        let h = host.unwrap_or_else(|| "127.0.0.1".to_string());
        let p = port.unwrap_or(DEFAULT_PG_PORT);
        return exec_psql(
            Path::new("psql"),
            &h,
            p,
            DEFAULT_USER,
            None,
            DEFAULT_DATABASE,
            query,
            queries_file,
            extra_args,
        );
    }

    let PgSelection {
        name: server_name,
        target: info,
        metadata_lock,
        ..
    } = lock_and_require_pg_target(
        name,
        version.as_deref(),
        "clickhousectl local postgres client <name>",
    )?;
    if !server::is_server_running_locked(&info.name, &metadata_lock)? {
        return Err(Error::ServerNotRunning(server_name));
    }
    drop(metadata_lock);

    if info.pg_runtime() == Some(PgRuntime::Native) {
        let state = native_state(&info)?;
        let bundle = installed_bundle_for(&state)?;
        return exec_psql(
            &bundle.bin("psql"),
            "127.0.0.1",
            info.tcp_port,
            &state.user,
            Some(&state.password),
            &state.database,
            query,
            queries_file,
            extra_args,
        );
    }

    let docker = docker::connect().await?;
    let container_id = info
        .container_id
        .as_deref()
        .ok_or_else(|| Error::DockerError("missing container_id".into()))?;
    let (user, password, database) = read_pg_env(&docker, container_id).await;

    // Prefer host psql; fall back to docker exec.
    if host_has_psql() {
        return exec_psql(
            Path::new("psql"),
            "127.0.0.1",
            info.tcp_port,
            &user,
            Some(&password),
            &database,
            query,
            queries_file,
            extra_args,
        );
    }

    let explicit_input = query.is_some() || queries_file.is_some();
    let interactive =
        !explicit_input && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let mut psql_args: Vec<String> = vec!["-U".into(), user, "-d".into(), database];
    if let Some(q) = query {
        psql_args.push("-c".into());
        psql_args.push(q);
    }
    // Host paths do not exist inside the container. Stream the selected file
    // through psql's explicit stdin file argument, after any -c command.
    let input: Option<Box<dyn std::io::Read + Send>> = match queries_file {
        Some(file) => {
            let reader: Box<dyn std::io::Read + Send> = if file == "-" {
                Box::new(std::io::stdin())
            } else {
                Box::new(
                    std::fs::File::open(&file).map_err(|error| Error::SqlInputOpen {
                        path: file.into(),
                        source: error,
                    })?,
                )
            };
            psql_args.extend(["-f".into(), "-".into()]);
            Some(reader)
        }
        // Match host psql: without an explicit wrapper input, a non-terminal
        // stdin is still SQL input. Docker must attach it and receive EOF.
        None if !explicit_input && !interactive => Some(Box::new(std::io::stdin())),
        None => None,
    };
    if input.is_some() {
        psql_args.extend(ON_ERROR_STOP.map(String::from));
    }
    psql_args.extend(extra_args);

    if !interactive {
        // Non-interactive: no TTY, no raw mode, output goes to stdout/stderr
        // so the caller can pipe / capture / redirect.
        docker::exec_psql_one_shot(&docker, container_id, &psql_args, input).await
    } else {
        docker::exec_psql_in_container(&docker, container_id, &psql_args).await
    }
}

/// Read POSTGRES_USER/PASSWORD/DB from the container's effective env so we
/// don't lose track of user-provided values across recoveries.
async fn read_pg_env(docker: &bollard::Docker, id: &str) -> (String, String, String) {
    let inspect = docker.inspect_container(id, None).await.ok();
    let env: Vec<String> = inspect
        .and_then(|c| c.config)
        .and_then(|c| c.env)
        .unwrap_or_default();
    let get = |k: &str| -> Option<String> {
        env.iter()
            .find_map(|e| e.strip_prefix(&format!("{k}=")).map(|s| s.to_string()))
    };
    (
        get("POSTGRES_USER").unwrap_or_else(|| DEFAULT_USER.into()),
        get("POSTGRES_PASSWORD").unwrap_or_default(),
        get("POSTGRES_DB").unwrap_or_else(|| DEFAULT_DATABASE.into()),
    )
}

/// psql runs a script past a failed statement and exits 0 unless
/// `ON_ERROR_STOP` is set. Script input (a file or non-terminal stdin) gets it
/// ahead of native arguments, so `-- -v ON_ERROR_STOP=0` still overrides it.
const ON_ERROR_STOP: [&str; 2] = ["-v", "ON_ERROR_STOP=1"];

fn host_has_psql() -> bool {
    Command::new("psql")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The stored credentials of a native instance.
fn native_state(info: &ServerInfo) -> Result<NativeState> {
    let name = user_name_from_key(&info.name);
    pg_native::load_state(&server::servers_dir_join(&info.name)?)?.ok_or_else(|| {
        Error::PostgresUsage(format!(
            "server '{name}' ({}) has no stored connection details. Run `clickhousectl local \
             postgres remove {name}` to clear it and start fresh.",
            info.version
        ))
    })
}

/// The installed bundle whose client tools match a native instance's major.
fn installed_bundle_for(state: &NativeState) -> Result<InstalledBundle> {
    let major = &state.postgres_major;
    pg_bundle::newest_installed(&PgVersionRequest::major(major))?.ok_or_else(|| {
        Error::PostgresBundleUnavailable(format!(
            "no PostgreSQL {major} bundle is installed; run `clickhousectl local install \
             postgres@{major}`"
        ))
    })
}

/// Hand this process over to `program`, a `psql`: the host's, or a bundle's.
#[allow(clippy::too_many_arguments)]
fn exec_psql(
    program: &Path,
    host: &str,
    port: u16,
    user: &str,
    password: Option<&str>,
    database: &str,
    query: Option<String>,
    queries_file: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    use std::os::unix::process::CommandExt;
    // psql reads a non-terminal stdin as a script unless -c supplies input.
    let script_input =
        queries_file.is_some() || (query.is_none() && !std::io::stdin().is_terminal());
    let mut cmd = Command::new(program);
    cmd.arg("-h")
        .arg(host)
        .arg("-p")
        .arg(port.to_string())
        .arg("-U")
        .arg(user)
        .arg("-d")
        .arg(database);
    if let Some(p) = password {
        cmd.env("PGPASSWORD", p);
    }
    if let Some(q) = query {
        cmd.arg("-c").arg(q);
    }
    if let Some(f) = queries_file {
        cmd.arg("-f").arg(f);
    }
    if script_input {
        cmd.args(ON_ERROR_STOP);
    }
    cmd.args(&extra_args);
    // `exec()` replaces the process image on success, so `main`'s telemetry
    // tail never runs for this invocation; record the censored handoff attempt
    // now (#320, #471). Both callers probe for `psql` first, so only a race
    // (or a `PATH` entry that is not launchable) can fail below this line;
    // `psql` itself then inherits this process's stdio, process group, session
    // and controlling TTY unchanged, which is why the handoff stays an
    // `exec()`.
    #[cfg(feature = "telemetry")]
    crate::telemetry::finalize_before_exec();
    let err = cmd.exec();
    Err(Error::Postgres(format!("could not execute psql: {err}")))
}

fn dotenv(name: Option<&str>, version: Option<&str>, use_local: bool, json: bool) -> Result<()> {
    let PgSelection {
        name: server_name,
        target: info,
        metadata_lock,
        ..
    } = lock_and_require_pg_target(
        name.map(str::to_string),
        version,
        "clickhousectl local postgres dotenv <name>",
    )?;
    if !server::is_server_running_locked(&info.name, &metadata_lock)? {
        return Err(Error::ServerNotRunning(server_name));
    }
    drop(metadata_lock);

    let (user, password, database) = match info.pg_runtime() {
        Some(PgRuntime::Native) => {
            let state = native_state(&info)?;
            (state.user, state.password, state.database)
        }
        // Read user/password/db from the container env so we always emit
        // accurate creds.
        _ => docker::block_on(read_pg_env_for_dotenv(
            info.container_id.as_deref().unwrap_or_default(),
        )),
    };

    let vars: Vec<(&str, String)> = vec![
        ("POSTGRES_HOST", "127.0.0.1".to_string()),
        ("POSTGRES_PORT", info.tcp_port.to_string()),
        ("POSTGRES_USER", user),
        ("POSTGRES_PASSWORD", password),
        ("POSTGRES_DATABASE", database),
    ];

    let filename = if use_local { ".env.local" } else { ".env" };
    let path = std::path::Path::new(filename);

    crate::dotenv::write_vars(path, "POSTGRES_", &vars)?;

    let out = output::DotenvOutput {
        file: filename.to_string(),
        server: server_name.to_string(),
        vars: vars
            .into_iter()
            .map(|(k, v)| output::DotenvVar {
                key: k.to_string(),
                value: v,
            })
            .collect(),
        engine: server::Engine::Postgres,
    };
    output::print_output(&out, json);
    Ok(())
}

async fn read_pg_env_for_dotenv(container_id: &str) -> (String, String, String) {
    if container_id.is_empty() {
        return (DEFAULT_USER.into(), String::new(), DEFAULT_DATABASE.into());
    }
    match docker::connect().await {
        Ok(d) => read_pg_env(&d, container_id).await,
        Err(_) => (DEFAULT_USER.into(), String::new(), DEFAULT_DATABASE.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::VecDeque;

    struct FakeReadinessProbe {
        states: VecDeque<ContainerReadinessState>,
        ready: VecDeque<Result<bool>>,
        readiness_checks: usize,
    }

    impl FakeReadinessProbe {
        fn new(states: Vec<ContainerReadinessState>, ready: Vec<Result<bool>>) -> Self {
            Self {
                states: states.into(),
                ready: ready.into(),
                readiness_checks: 0,
            }
        }
    }

    impl ReadinessProbe for FakeReadinessProbe {
        async fn container_state(&mut self) -> Result<ContainerReadinessState> {
            Ok(self
                .states
                .pop_front()
                .expect("fake container state exhausted"))
        }

        async fn postgres_is_ready(&mut self) -> Result<bool> {
            self.readiness_checks += 1;
            self.ready
                .pop_front()
                .expect("fake pg_isready result exhausted")
        }
    }

    #[test]
    fn docker_only_features_are_env_and_image_tags() {
        for version in [None, Some("17"), Some("18"), Some("18.6")] {
            assert_eq!(DockerOnlyFeature::of(version, false), None, "{version:?}");
        }
        assert_eq!(
            DockerOnlyFeature::of(Some("18-bookworm"), false),
            Some(DockerOnlyFeature::ImageTag("18-bookworm".into()))
        );
        assert_eq!(
            DockerOnlyFeature::of(Some("17.2-alpine"), false),
            Some(DockerOnlyFeature::ImageTag("17.2-alpine".into()))
        );
        assert_eq!(
            DockerOnlyFeature::of(Some("18"), true),
            Some(DockerOnlyFeature::Env)
        );
    }

    #[test]
    fn start_runtime_is_native_unless_docker_is_asked_for_or_already_in_use() {
        let select = |prior, docker, feature: Option<DockerOnlyFeature>| {
            select_start_runtime(prior, docker, feature.as_ref(), "dev", "18")
        };
        let env = || Some(DockerOnlyFeature::Env);
        let tag = || Some(DockerOnlyFeature::ImageTag("18-bookworm".into()));

        // New instances.
        assert_eq!(select(None, false, None).unwrap(), PgRuntime::Native);
        assert_eq!(select(None, true, None).unwrap(), PgRuntime::Docker);
        assert_eq!(select(None, true, env()).unwrap(), PgRuntime::Docker);
        assert_eq!(select(None, true, tag()).unwrap(), PgRuntime::Docker);
        // An instance keeps its runtime: Docker ones from older versions
        // resume without the flag, with or without Docker-only options.
        for docker in [false, true] {
            for feature in [None, env(), tag()] {
                assert_eq!(
                    select(Some(PgRuntime::Docker), docker, feature).unwrap(),
                    PgRuntime::Docker
                );
            }
        }
        assert_eq!(
            select(Some(PgRuntime::Native), false, None).unwrap(),
            PgRuntime::Native
        );

        // Usage errors (exit 2) that name the flag and the instance.
        for (prior, docker, feature) in [
            (None, false, env()),
            (None, false, tag()),
            (Some(PgRuntime::Native), true, None),
            (Some(PgRuntime::Native), true, env()),
            (Some(PgRuntime::Native), false, env()),
            (Some(PgRuntime::Native), false, tag()),
        ] {
            let error = select(prior, docker, feature.clone()).unwrap_err();
            assert_eq!(error.exit_code(), 2, "{prior:?} {docker} {feature:?}");
            let message = error.to_string();
            assert!(message.contains("--docker"), "{message}");
            if prior.is_some() {
                assert!(
                    message.contains("clickhousectl local postgres remove dev --version 18"),
                    "{message}"
                );
            }
        }
    }

    #[test]
    fn native_passwords_are_one_non_empty_line() {
        assert!(validate_native_password("example-password").is_ok());
        for password in ["", "two\nlines", "carriage\rreturn"] {
            let error = validate_native_password(password).unwrap_err();
            assert_eq!(error.exit_code(), 2, "{password:?}");
        }
    }

    #[test]
    fn runtime_follows_the_recorded_container() {
        let mut info = ServerInfo {
            name: server::pg_instance_key("dev", "18"),
            pid: 0,
            version: "postgres:18.7".into(),
            last_version: None,
            http_port: 0,
            tcp_port: 5432,
            started_at: "1700000000".into(),
            cwd: "/tmp/project".into(),
            engine: Engine::Postgres,
            container_id: None,
        };
        assert_eq!(info.pg_runtime(), Some(PgRuntime::Native));
        // What every Docker start has written, before and since.
        info.container_id = Some("abc123".into());
        assert_eq!(info.pg_runtime(), Some(PgRuntime::Docker));
        info.engine = Engine::Clickhouse;
        assert_eq!(info.pg_runtime(), None);
    }

    #[test]
    fn unnamed_start_after_running_default_selects_fresh_name() {
        let directory = tempfile::tempdir().unwrap();
        let lock = server::MetadataLock::acquire_at(directory.path()).unwrap();
        assert_eq!(
            default_pg_name_locked_with(&lock, |_| Ok(true)).unwrap(),
            "default"
        );

        let info = ServerInfo {
            name: server::pg_instance_key("default", "18"),
            pid: 0,
            version: "postgres:18".into(),
            last_version: None,
            http_port: 0,
            tcp_port: 5432,
            started_at: "1700000000".into(),
            cwd: "/tmp/project".into(),
            engine: Engine::Postgres,
            container_id: Some("running-default".into()),
        };
        server::save_server_info_locked(&info, &lock).unwrap();

        assert_eq!(
            default_pg_name_locked_with(&lock, |_| Ok(false)).unwrap(),
            "default"
        );

        let unavailable = default_pg_name_locked_with(&lock, |_| {
            Err(Error::DockerNotAvailable("test daemon unavailable".into()))
        });
        assert!(matches!(unavailable, Err(Error::DockerNotAvailable(_))));
        let failed_inspection = default_pg_name_locked_with(&lock, |_| {
            Err(Error::DockerError("test inspection failed".into()))
        });
        assert!(matches!(failed_inspection, Err(Error::DockerError(_))));

        let selected =
            default_pg_name_locked_with(&lock, |id| Ok(id == "running-default")).unwrap();

        assert_ne!(selected, "default");
        assert!(
            server::find_pg_instances_locked(&selected, &lock)
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn running_container_is_not_postgres_readiness() {
        let mut probe =
            FakeReadinessProbe::new(vec![ContainerReadinessState::Running], vec![Ok(false)]);
        let mut last_probe_error = None;

        let result = poll_postgres_readiness(
            &mut probe,
            1,
            || std::future::ready(()),
            &mut last_probe_error,
        )
        .await;

        assert!(matches!(
            result,
            Err(ReadinessFailure::TimedOut {
                last_probe_error: None
            })
        ));
        assert_eq!(probe.readiness_checks, 1);
    }

    #[tokio::test]
    async fn delayed_postgres_readiness_succeeds_after_retries() {
        let mut probe = FakeReadinessProbe::new(
            vec![
                ContainerReadinessState::Running,
                ContainerReadinessState::Running,
                ContainerReadinessState::Running,
            ],
            vec![Ok(false), Ok(false), Ok(true)],
        );
        let sleeps = Cell::new(0);
        let mut last_probe_error = None;

        let result = poll_postgres_readiness(
            &mut probe,
            3,
            || {
                sleeps.set(sleeps.get() + 1);
                std::future::ready(())
            },
            &mut last_probe_error,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(probe.readiness_checks, 3);
        assert_eq!(sleeps.get(), 2);
    }

    #[tokio::test]
    async fn immediate_container_exit_stops_readiness_checks() {
        let mut probe = FakeReadinessProbe::new(
            vec![ContainerReadinessState::Exited {
                status: "exited".to_string(),
                exit_code: Some(1),
                oom_killed: false,
            }],
            vec![],
        );
        let sleeps = Cell::new(0);
        let mut last_probe_error = None;

        let result = poll_postgres_readiness(
            &mut probe,
            3,
            || {
                sleeps.set(sleeps.get() + 1);
                std::future::ready(())
            },
            &mut last_probe_error,
        )
        .await;

        assert!(matches!(
            result,
            Err(ReadinessFailure::Exited {
                status,
                exit_code: Some(1),
                oom_killed: false,
            }) if status == "exited"
        ));
        assert_eq!(probe.readiness_checks, 0);
        assert_eq!(sleeps.get(), 0);
    }

    #[tokio::test]
    async fn transient_readiness_probe_error_is_retried() {
        let mut probe = FakeReadinessProbe::new(
            vec![
                ContainerReadinessState::Running,
                ContainerReadinessState::Running,
                ContainerReadinessState::Running,
            ],
            vec![
                Err(Error::DockerError("temporary exec failure".to_string())),
                Ok(true),
            ],
        );
        let mut last_probe_error = None;

        let result = poll_postgres_readiness(
            &mut probe,
            2,
            || std::future::ready(()),
            &mut last_probe_error,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(probe.readiness_checks, 2);
        assert_eq!(
            last_probe_error,
            Some(Error::DockerError("temporary exec failure".to_string()).to_string())
        );
    }

    #[tokio::test]
    async fn polling_limit_reports_timeout() {
        let mut probe = FakeReadinessProbe::new(
            vec![ContainerReadinessState::Running; 4],
            vec![Ok(false), Ok(false), Ok(false), Ok(false)],
        );
        let sleeps = Cell::new(0);
        let mut last_probe_error = None;

        let result = poll_postgres_readiness(
            &mut probe,
            4,
            || {
                sleeps.set(sleeps.get() + 1);
                std::future::ready(())
            },
            &mut last_probe_error,
        )
        .await;

        assert!(matches!(
            result,
            Err(ReadinessFailure::TimedOut {
                last_probe_error: None
            })
        ));
        assert_eq!(probe.readiness_checks, 4);
        assert_eq!(sleeps.get(), 3);

        let error = format_postgres_readiness_error(
            "test",
            Duration::from_secs(12),
            ReadinessFailure::TimedOut {
                last_probe_error: None,
            },
            "FATAL: database system is not ready",
        );
        assert!(
            matches!(&error, Error::StartupTimeout { kind: StartupKind::Postgres, name, seconds: 12, .. }
                if name == "test"),
            "{error}"
        );
        let error = error.to_string();
        assert!(error.contains(&READINESS_LOG_LINES.to_string()), "{error}");
        // The container's own log text is carried through.
        assert!(error.contains("FATAL: database system is not ready"));
    }

    #[tokio::test]
    async fn stalled_log_collection_returns_actionable_fallback() {
        let logs = collect_postgres_readiness_logs(
            "test-container",
            Duration::from_millis(10),
            std::future::pending::<Result<String>>(),
        )
        .await;
        let empty = collect_postgres_readiness_logs(
            "test-container",
            Duration::from_millis(10),
            std::future::ready(Ok(String::new())),
        )
        .await;
        let failed = collect_postgres_readiness_logs(
            "test-container",
            Duration::from_millis(10),
            std::future::ready(Err(Error::DockerError("logs unavailable".into()))),
        )
        .await;

        // Each fallback names the command that fetches the logs, and the
        // stall is told apart from an empty or failed read.
        for fallback in [&logs, &empty, &failed] {
            assert!(
                fallback.contains("`docker logs test-container`"),
                "{fallback}"
            );
        }
        assert_ne!(logs, empty);
        assert_ne!(logs, failed);
        assert!(failed.contains("logs unavailable"), "{failed}");
    }

    #[test]
    fn fresh_data_cleanup_ownership_is_conservative() {
        let tempdir = tempfile::tempdir().expect("create policy tempdir");
        let instance_dir = tempdir.path().join("policy-pg18");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        std::fs::create_dir(&instance_dir).expect("create empty instance dir");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        let data_dir = instance_dir.join("data");
        std::fs::create_dir(&data_dir).expect("create empty data dir");
        assert!(fresh_instance_dir_is_disposable(&instance_dir));

        std::fs::write(data_dir.join("PG_VERSION"), "existing").expect("write existing PGDATA");
        assert!(!fresh_instance_dir_is_disposable(&instance_dir));
    }

    #[test]
    fn resolve_port_passes_through_explicit_value() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert_eq!(resolve_port(Some(port)).unwrap(), port);
    }

    #[test]
    fn resolve_port_rejects_bound_explicit_value() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let err = resolve_port(Some(port)).unwrap_err();
        assert!(
            matches!(err, Error::PortInUse { kind: PortKind::Postgres, port: error_port } if error_port == port)
        );
    }

    #[test]
    fn resolve_port_auto_selects_when_omitted_default_is_bound() {
        let default_listener = match std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PG_PORT)) {
            Ok(listener) => Some(listener),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => None,
            Err(error) => panic!("bind default Postgres port: {error}"),
        };

        let port = resolve_port(None).unwrap();

        assert_ne!(port, DEFAULT_PG_PORT);
        drop(default_listener);
    }

    #[test]
    fn validate_pg_tag_accepts_supported_majors() {
        for tag in [
            "17",
            "18",
            "17-alpine",
            "17.0",
            "18-bookworm",
            "18-alpine3.20",
            "18.1-alpine3.20",
            "18.01-Custom_variant-1.0",
        ] {
            assert!(
                validate_pg_tag(tag).is_ok(),
                "expected `{}` to be accepted",
                tag
            );
        }
    }

    #[test]
    fn validate_pg_tag_rejects_unsupported() {
        for tag in [
            "latest",
            "15",
            "16",
            "16-alpine",
            "19",
            "14-alpine",
            "alpine",
            "",
            "18garbage",
            "18.1garbage",
            "18.",
            "18..1",
            "18.1.2",
            "18-",
            "18-.alpine",
            "18_alpine",
            "18 alpine",
            "18/alpine",
            "18:alpine",
        ] {
            assert!(
                validate_pg_tag(tag).is_err(),
                "expected `{}` to be rejected",
                tag
            );
        }
    }

    #[test]
    fn validate_pg_tag_enforces_docker_tag_length() {
        let max_length = format!("18-{}", "a".repeat(125));
        let too_long = format!("18-{}", "a".repeat(126));

        assert_eq!(max_length.len(), 128);
        assert!(validate_pg_tag(&max_length).is_ok());
        assert_eq!(too_long.len(), 129);
        assert!(validate_pg_tag(&too_long).is_err());
    }

    #[test]
    fn generate_password_is_24_alphanumeric() {
        let p = generate_password();
        assert_eq!(p.len(), 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn start_env_accepts_unique_assignments_and_equals_in_values() {
        let preflight = validate_start_options(
            Some("dev"),
            Some("18.1-alpine3.20"),
            None,
            None,
            vec![
                "APP_MODE=test".into(),
                "DATABASE_URL=postgres://user:pass@host/db?sslmode=require".into(),
                "POSTGRES_PASSWORD=a=b".into(),
            ],
        )
        .unwrap();

        assert_eq!(
            preflight.extra_env,
            [
                "APP_MODE=test",
                "DATABASE_URL=postgres://user:pass@host/db?sslmode=require"
            ]
        );
        assert_eq!(preflight.password_from_env.as_deref(), Some("a=b"));
        assert_eq!(preflight.host_port, None);
    }

    #[test]
    fn start_env_rejects_malformed_assignments() {
        for assignment in ["NO_EQUALS", "=value", "1KEY=value", "BAD-KEY=value"] {
            let error = validate_start_options(
                Some("dev"),
                Some("18"),
                None,
                None,
                vec![assignment.into()],
            )
            .err()
            .expect("malformed environment variable should fail");
            assert!(
                matches!(error, Error::PostgresUsage(_)),
                "{assignment}: {error}"
            );
        }
    }

    #[test]
    fn start_env_rejects_duplicate_keys() {
        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            None,
            vec!["APP_MODE=dev".into(), "APP_MODE=test".into()],
        )
        .err()
        .expect("duplicate environment variable should fail");

        assert!(
            matches!(error, Error::PostgresUsage(msg) if msg.contains("'APP_MODE'") && msg.contains("--env"))
        );
    }

    #[test]
    fn start_env_rejects_generated_keys_except_password() {
        for assignment in [
            "POSTGRES_USER=admin",
            "POSTGRES_DB=app",
            "PGDATA=/tmp/postgres",
        ] {
            let error = validate_start_options(
                Some("dev"),
                Some("18"),
                None,
                None,
                vec![assignment.into()],
            )
            .err()
            .expect("reserved environment variable should fail");
            let key = assignment.split_once('=').unwrap().0;
            assert!(
                matches!(&error, Error::PostgresUsage(msg) if msg.starts_with(key) && msg.contains("--")),
                "{error}"
            );
        }
    }

    #[test]
    fn start_env_password_sources_are_unambiguous() {
        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            Some("from-flag"),
            vec!["POSTGRES_PASSWORD=from-env".into()],
        )
        .err()
        .expect("password sources should conflict");
        assert!(matches!(error, Error::PostgresUsage(msg)
                if msg.contains("POSTGRES_PASSWORD") && msg.contains("--password") && msg.contains("--env")));

        let error = validate_start_options(
            Some("dev"),
            Some("18"),
            None,
            None,
            vec![
                "POSTGRES_PASSWORD=first".into(),
                "POSTGRES_PASSWORD=second".into(),
            ],
        )
        .err()
        .expect("duplicate password should fail");
        assert!(
            matches!(error, Error::PostgresUsage(msg) if msg.contains("'POSTGRES_PASSWORD'") && !msg.contains("--password"))
        );
    }
}
