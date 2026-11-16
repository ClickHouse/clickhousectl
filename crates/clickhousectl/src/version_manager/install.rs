use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::atomic::{CommitLock, InstallStaging, sync_directory};
use crate::version_manager::download::{DownloadOutcome, download_from_source};
use crate::version_manager::list::list_installed_versions;
use crate::version_manager::master;
use crate::version_manager::platform::{DownloadSource, Platform};
use crate::version_manager::resolve::{ResolvedVersion, resolve, try_resolve_local};
use crate::version_manager::spec::VersionSpec;
use flate2::read::GzDecoder;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path};
use std::process::Stdio;
use std::time::Duration;
use tar::Archive;

/// Install a version spec, trying installed versions first before any remote call.
/// An installed match is a successful no-op regardless of whether the spec is
/// exact or partial.
pub async fn install_local_first(
    spec: &VersionSpec,
    platform: &Platform,
    force: bool,
    structured_output: bool,
) -> Result<String> {
    if !force && let Some(local) = try_resolve_local(spec)? {
        if !structured_output {
            eprintln!("ClickHouse {} is already installed as {}", spec, local);
            eprintln!("Use --force to re-download the latest build");
        }
        return Ok(local);
    }

    if !structured_output {
        eprintln!("Resolving {}...", spec);
    }
    let resolved = resolve(spec, platform).await?;
    install_resolved(&resolved, platform, force, structured_output).await
}

/// Like `install_local_first`, but returns an existing local version silently
/// (matching `ensure_installed`'s semantics). For `clickhouse start -v <spec>`.
pub async fn ensure_installed_local_first(
    spec: &VersionSpec,
    platform: &Platform,
    structured_output: bool,
) -> Result<String> {
    if let Some(local) = try_resolve_local(spec)? {
        return Ok(local);
    }

    if !structured_output {
        eprintln!("Resolving {}...", spec);
    }
    let resolved = resolve(spec, platform).await?;
    ensure_installed(&resolved, platform, structured_output).await
}

/// Installs a ClickHouse version using the multi-source resolution system.
/// Returns the exact version string of the installed binary.
pub async fn install_resolved(
    resolved: &ResolvedVersion,
    platform: &Platform,
    force: bool,
    structured_output: bool,
) -> Result<String> {
    paths::ensure_dirs()?;
    let versions_dir = paths::versions_dir()?;

    // The floating `latest`/master build has no version upfront and a stable
    // URL whose content moves. Use the HTTP etag to skip the ~153MB download
    // when master hasn't changed since the installed build.
    let is_master = matches!(
        resolved.source,
        DownloadSource::Builds { ref version_path } if version_path == "master"
    );
    let cached_master = if is_master && !force {
        let lock = CommitLock::acquire(&versions_dir).await?;
        master::usable_record(&lock, platform)
    } else {
        None
    };

    // If we know the exact version upfront, check if already installed
    if let Some(ref version) = resolved.exact_version
        && is_installed(&paths::binary_path(version)?)
        && !force
    {
        return Err(Error::VersionAlreadyInstalled(version.to_string()));
    }

    // For builds source (minor versions like "25.12"), check if we already have
    // an installed version matching that minor — avoids re-downloading ~150MB
    if !force
        && let DownloadSource::Builds { ref version_path } = resolved.source
        && version_path != "master"
    {
        let prefix = format!("{}.", version_path);
        if let Ok(installed) = list_installed_versions()
            && let Some(existing) = installed.iter().find(|v| v.starts_with(&prefix))
        {
            if !structured_output {
                eprintln!(
                    "ClickHouse {} is already installed as {}",
                    version_path, existing
                );
                eprintln!("Use --force to re-download the latest build");
            }
            return Ok(existing.clone());
        }
    }

    // Downloads stay outside the commit lock in invocation-owned staging.
    let staging = InstallStaging::create(&versions_dir)?;
    let binary_path = staging.binary_path();

    if !structured_output {
        if cached_master.is_some() {
            eprintln!("Checking ClickHouse {}...", resolved.display_version);
        } else {
            eprintln!("Downloading ClickHouse {}...", resolved.display_version);
        }
    }

    let download_path = if resolved.source.is_tarball(platform) {
        staging.path().join("clickhouse.tgz")
    } else {
        binary_path.clone()
    };
    let outcome = download_from_source(
        &resolved.source,
        platform,
        &download_path,
        structured_output,
        cached_master.as_ref().map(|record| record.etag.as_str()),
    )
    .await?;
    let artifact_info = match outcome {
        DownloadOutcome::Downloaded(info) => info,
        DownloadOutcome::NotModified => {
            // Another installer can replace the same version while GET is in
            // flight. Recheck the record under the commit lock before reuse.
            let lock = CommitLock::acquire(&versions_dir).await?;
            let current = master::usable_record(&lock, platform);
            if let Some(record) = current.filter(|record| Some(record) == cached_master.as_ref()) {
                if !structured_output {
                    eprintln!(
                        "latest is up to date (master build unchanged); using {}",
                        record.version
                    );
                }
                return Ok(record.version);
            }
            drop(lock);
            // The validated local artifact disappeared or was replaced. Fetch
            // bytes unconditionally instead of reusing a different artifact.
            match download_from_source(
                &resolved.source,
                platform,
                &download_path,
                structured_output,
                None,
            )
            .await?
            {
                DownloadOutcome::Downloaded(info) => info,
                DownloadOutcome::NotModified => unreachable!("unconditional downloads reject 304"),
            }
        }
    };
    if resolved.source.is_tarball(platform) {
        if !structured_output {
            eprintln!("Extracting...");
        }
        extract_tarball_auto(&download_path, staging.payload())?;
    }

    // Make the binary executable
    let mut perms = std::fs::metadata(&binary_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary_path, perms)?;

    // Detect the exact version from the binary
    let exact_version = if resolved.exact_version_known {
        resolved.exact_version.clone().unwrap()
    } else {
        if !structured_output {
            eprintln!("Detecting version...");
        }
        detect_binary_version(&binary_path)?
    };

    let commit_lock = CommitLock::acquire(&versions_dir).await?;
    let (_, notify_running_servers) = commit_staged_install_locked(
        &commit_lock,
        &versions_dir,
        &staging,
        &exact_version,
        force,
        is_master,
        platform,
        artifact_info.as_ref(),
        |version| {
            if structured_output {
                Ok(false)
            } else {
                version_in_use_by_running_server(version)
            }
        },
        |_| Ok(()),
    )?;

    // Replacing a build on disk never affects already-running servers (they keep
    // executing the old binary) — just say so, so the swap isn't silent.
    drop(commit_lock);
    if notify_running_servers {
        eprintln!(
            "Note: running servers keep using the previous {} build until restarted",
            exact_version
        );
    }

    let channel_suffix = match resolved.channel {
        Some(ch) => format!(" ({})", ch),
        None => String::new(),
    };
    if !structured_output {
        eprintln!("Installed ClickHouse {}{}", exact_version, channel_suffix);
    }

    Ok(exact_version)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitCheckpoint {
    SidecarInvalidated,
    BinaryReplaced,
}

#[allow(clippy::too_many_arguments)]
fn commit_staged_install_locked(
    lock: &CommitLock,
    versions_dir: &Path,
    staging: &InstallStaging,
    exact_version: &str,
    force: bool,
    is_master: bool,
    platform: &Platform,
    artifact_info: Option<&master::ArtifactInfo>,
    version_in_use: impl FnOnce(&str) -> Result<bool>,
    mut checkpoint: impl FnMut(CommitCheckpoint) -> Result<()>,
) -> Result<(bool, bool)> {
    let version_dir = versions_dir.join(exact_version);
    let target_binary = version_dir.join("clickhouse");
    let target_metadata = match std::fs::symlink_metadata(&version_dir) {
        Ok(metadata) if metadata.file_type().is_dir() => Some(metadata),
        Ok(_) => {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "install target '{}' exists and is not a directory",
                    version_dir.display()
                ),
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let replaced_existing = target_binary.exists();
    if replaced_existing && !force && !is_master {
        return Err(Error::VersionAlreadyInstalled(exact_version.to_string()));
    }
    let notify_running_servers = is_master && replaced_existing && version_in_use(exact_version)?;

    File::open(staging.binary_path())?.sync_all()?;
    sync_directory(staging.payload())?;

    master::invalidate_version(lock, versions_dir, staging.path(), exact_version)?;
    checkpoint(CommitCheckpoint::SidecarInvalidated)?;

    if target_metadata.is_some() {
        std::fs::rename(staging.binary_path(), &target_binary)?;
        sync_directory(&version_dir)?;
    } else {
        std::fs::rename(staging.payload(), &version_dir)?;
    }
    sync_directory(versions_dir)?;
    checkpoint(CommitCheckpoint::BinaryReplaced)?;

    if is_master && let Some(info) = artifact_info {
        // The binary is already durably committed. A failed freshness record
        // must only cause a later re-download, not report the install as failed.
        let _ = master::record_install(
            lock,
            versions_dir,
            staging.path(),
            platform,
            info,
            exact_version,
        );
    }

    Ok((replaced_existing, notify_running_servers))
}

/// Like `install_resolved`, but returns the existing version instead of erroring
/// when already installed. Intended for cases like `clickhouse start --version` where
/// the goal is "make sure this version is available" rather than "install this".
pub async fn ensure_installed(
    resolved: &ResolvedVersion,
    platform: &Platform,
    structured_output: bool,
) -> Result<String> {
    // If we know the exact version upfront, return it if already installed
    if let Some(ref version) = resolved.exact_version
        && is_installed(&paths::binary_path(version)?)
    {
        return Ok(version.clone());
    }

    // For builds source (minor versions), check if a matching minor is installed
    if let DownloadSource::Builds { ref version_path } = resolved.source
        && version_path != "master"
    {
        let prefix = format!("{}.", version_path);
        if let Ok(installed) = list_installed_versions()
            && let Some(existing) = installed.iter().find(|v| v.starts_with(&prefix))
        {
            return Ok(existing.clone());
        }
    }

    // Not installed (or a master/`latest` build whose exact version we can only
    // learn after downloading) — delegate to install_resolved. For master builds
    // try_resolve_local always returns None and the exact version isn't known
    // upfront, so install_resolved downloads, detects the version, and may find it
    // already installed. That's a success for the "ensure" contract, not an error:
    // map VersionAlreadyInstalled back to the existing version.
    match install_resolved(resolved, platform, false, structured_output).await {
        Err(Error::VersionAlreadyInstalled(version)) => Ok(version),
        other => other,
    }
}

/// How long `local install --binary` gives the copied file to answer `--version`.
const BINARY_VERSION_TIMEOUT: Duration = Duration::from_secs(60);
/// The most of each output stream that probe keeps. A file that writes more
/// is not ClickHouse; its pipe is closed and it fails or times out.
const BINARY_VERSION_OUTPUT_LIMIT: u64 = 64 * 1024;

/// Install a local ClickHouse build (#129): copy `source` into staging, ask the
/// copy (never the original) for its version, and commit it under that
/// version. A file that does not report a ClickHouse version leaves nothing
/// installed, and an installed version is replaced only with `force`.
pub async fn install_from_binary(
    source: &Path,
    force: bool,
    structured_output: bool,
) -> Result<String> {
    let rejected = |reason: String| Error::NotAClickHouseBinary {
        path: source.display().to_string(),
        reason,
    };
    check_source_binary(source).map_err(rejected)?;
    let platform = Platform::detect()?;
    paths::ensure_dirs()?;
    let versions_dir = paths::versions_dir()?;

    let staging = InstallStaging::create(&versions_dir)?;
    let binary_path = staging.binary_path();
    if !structured_output {
        eprintln!("Copying {}...", source.display());
    }
    std::fs::copy(source, &binary_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "could not copy {} to {}: {error}",
                source.display(),
                binary_path.display()
            ),
        )
    })?;
    std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))?;

    if !structured_output {
        eprintln!("Detecting version...");
    }
    let mut probe = tokio::process::Command::new(&binary_path);
    probe.arg("--version");
    let timeout = crate::timing::duration_override(
        crate::timing::BINARY_VERSION_TIMEOUT_ENV,
        BINARY_VERSION_TIMEOUT,
    );
    let output = run_version_probe(probe, timeout).await.map_err(rejected)?;
    let version = parse_clickhouse_version_output(&output).ok_or_else(|| {
        rejected(format!(
            "`--version` reported no ClickHouse version: {}",
            excerpt(output.as_bytes())
        ))
    })?;
    // The version becomes a directory under ~/.clickhouse/versions/ (#1291).
    paths::validate_installed_version(&version)?;

    let commit_lock = CommitLock::acquire(&versions_dir).await?;
    commit_staged_install_locked(
        &commit_lock,
        &versions_dir,
        &staging,
        &version,
        force,
        false,
        &platform,
        None,
        |_| Ok(false),
        |_| Ok(()),
    )?;
    drop(commit_lock);

    if !structured_output {
        eprintln!("Installed ClickHouse {} from {}", version, source.display());
    }
    Ok(version)
}

/// The reason `source` cannot be installed with `--binary`, if any. Symlinks
/// are followed: `build/programs/clickhouse-server` links to the real binary.
fn check_source_binary(source: &Path) -> std::result::Result<(), String> {
    let metadata =
        std::fs::metadata(source).map_err(|error| format!("it cannot be read: {error}"))?;
    if !metadata.is_file() {
        return Err("it is not a regular file".to_string());
    }
    Ok(())
}

/// Run `command` with no stdin and return its stdout, or why it did not
/// succeed within `timeout`. The child is killed if the wait ends early.
async fn run_version_probe(
    mut command: tokio::process::Command,
    timeout: Duration,
) -> std::result::Result<String, String> {
    use tokio::io::AsyncReadExt;

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| format!("it could not be run: {error}"))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    // Each reader owns its pipe and closes it once it stops reading, at EOF or
    // at the limit, so a child still writing gets EPIPE instead of blocking
    // the other stream's EOF until the timeout.
    async fn read_bounded(pipe: impl tokio::io::AsyncRead + Unpin) -> (io::Result<usize>, Vec<u8>) {
        let mut buffer = Vec::new();
        let read = pipe
            .take(BINARY_VERSION_OUTPUT_LIMIT)
            .read_to_end(&mut buffer)
            .await;
        (read, buffer)
    }
    let run = async {
        let ((read_out, out), (read_err, err)) =
            tokio::join!(read_bounded(stdout), read_bounded(stderr));
        let status = child.wait().await;
        (read_out.and(read_err), status, out, err)
    };
    match tokio::time::timeout(timeout, run).await {
        Err(_) => Err(format!("`--version` did not finish within {timeout:?}")),
        Ok((_, Err(error), _, _)) | Ok((Err(error), _, _, _)) => {
            Err(format!("`--version` failed: {error}"))
        }
        Ok((Ok(_), Ok(status), out, err)) => {
            if status.success() {
                Ok(String::from_utf8_lossy(&out).into_owned())
            } else if err.iter().all(u8::is_ascii_whitespace) {
                Err(format!("`--version` exited with {status}"))
            } else {
                Err(format!(
                    "`--version` exited with {status}: {}",
                    excerpt(&err)
                ))
            }
        }
    }
}

/// The version on the first `ClickHouse ...` line, as in
/// "ClickHouse local version 25.12.9.61 (official build).", so another
/// program's four-part version is never taken for one.
fn parse_clickhouse_version_output(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("ClickHouse "))
        .find_map(|line| parse_version_output(line).ok())
}

/// The start of a foreign program's output, for an error message.
fn excerpt(bytes: &[u8]) -> String {
    const MAX_CHARS: usize = 200;
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    if text.is_empty() {
        return "(no output)".to_string();
    }
    if text.chars().count() <= MAX_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(MAX_CHARS).collect();
    format!("{cut}...")
}

fn is_installed(binary_path: &Path) -> bool {
    binary_path.exists()
}

/// Whether a running managed server (in the current project) was started from
/// this version. Recovers orphans first (like `local remove`) so a server that
/// lost its metadata file is still counted.
fn version_in_use_by_running_server(version: &str) -> Result<bool> {
    // One recovery pass, then a listing under the same lock, so the process
    // table is scanned once.
    let Some(lock) = crate::local::server::lock_recovered_metadata()? else {
        return Ok(false);
    };
    Ok(crate::local::server::list_running_servers_locked(&lock)?
        .iter()
        .any(|s| s.version == version))
}

/// Detect the version of a clickhouse binary by running `./clickhouse --version`
fn detect_binary_version(binary_path: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new(binary_path)
        .arg("--version")
        .output()
        .map_err(|e| Error::Exec(format!("Failed to run clickhouse --version: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        return Err(Error::Exec(format!(
            "clickhouse --version returned non-zero exit code. {}",
            if stderr.is_empty() {
                format!("status={}", output.status)
            } else {
                stderr.to_string()
            }
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version_output(&stdout)
}

/// Parse the version string from clickhouse --version output
/// Example outputs:
///   "ClickHouse client version 25.12.9.61 (official build)."
///   "ClickHouse server version 25.12.9.61 (official build)."
fn parse_version_output(output: &str) -> Result<String> {
    for word in output.split_whitespace() {
        let parts: Vec<&str> = word.trim_end_matches('.').split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u64>().is_ok()) {
            return Ok(parts.join("."));
        }
    }

    Err(Error::Exec(format!(
        "Could not parse version from output: {}",
        output.trim()
    )))
}

/// Extract a tarball, finding the clickhouse binary automatically.
/// Handles both packages.clickhouse.com layout (usr/bin/clickhouse inside subdir)
/// and GitHub releases layout (same structure).
fn extract_tarball_auto(tarball_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<()> {
    let final_binary = dest_dir.join("clickhouse");
    let partial_binary = dest_dir.join(".clickhouse.extracting");
    let extraction_error = |destination: &Path, source| Error::ExtractArchive {
        archive: tarball_path.to_path_buf(),
        destination: destination.to_path_buf(),
        source,
    };
    let archive_file =
        File::open(tarball_path).map_err(|source| extraction_error(&final_binary, source))?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = Archive::new(decoder);

    let extraction_result = (|| -> Result<()> {
        let entries = archive
            .entries()
            .map_err(|source| extraction_error(&final_binary, source))?;
        let mut found_binary = false;

        for entry in entries {
            let mut entry = entry.map_err(|source| extraction_error(&final_binary, source))?;
            let entry_path = entry
                .path()
                .map_err(|source| extraction_error(&final_binary, source))?
                .into_owned();

            validate_archive_entry_path(tarball_path, &entry_path)?;
            if !is_clickhouse_binary_path(&entry_path) {
                continue;
            }
            if found_binary {
                return Err(Error::Extract(format!(
                    "Archive '{}' contains more than one ClickHouse binary",
                    tarball_path.display()
                )));
            }
            if !entry.header().entry_type().is_file() {
                return Err(Error::Extract(format!(
                    "Archive '{}' entry '{}' is not a regular file; symbolic and hard links are not followed",
                    tarball_path.display(),
                    entry_path.display()
                )));
            }

            let mut output = File::create(&partial_binary)
                .map_err(|source| extraction_error(&partial_binary, source))?;
            io::copy(&mut entry, &mut output)
                .and_then(|_| output.flush())
                .map_err(|source| extraction_error(&partial_binary, source))?;
            found_binary = true;
        }

        if !found_binary {
            return Err(Error::Extract(format!(
                "Archive '{}' does not contain a ClickHouse binary at 'clickhouse' or '*/usr/bin/clickhouse'",
                tarball_path.display()
            )));
        }

        // tar stops at its end-of-archive blocks. Read the gzip stream itself
        // to EOF so GzDecoder validates the trailer CRC and uncompressed size.
        let mut decoder = archive.into_inner();
        io::copy(&mut decoder, &mut io::sink())
            .map_err(|source| extraction_error(&final_binary, source))?;

        Ok(())
    })();

    if let Err(error) = extraction_result {
        let _ = std::fs::remove_file(&partial_binary);
        return Err(error);
    }

    if let Err(source) = std::fs::rename(&partial_binary, &final_binary) {
        let _ = std::fs::remove_file(&partial_binary);
        return Err(extraction_error(&final_binary, source));
    }
    let _ = std::fs::remove_file(tarball_path);
    Ok(())
}

fn validate_archive_entry_path(archive_path: &Path, entry_path: &Path) -> Result<()> {
    let safe = !entry_path.as_os_str().is_empty()
        && entry_path
            .components()
            .all(|component| matches!(component, Component::CurDir | Component::Normal(_)));
    if safe {
        return Ok(());
    }

    Err(Error::Extract(format!(
        "Archive '{}' contains unsafe entry path '{}'",
        archive_path.display(),
        entry_path.display()
    )))
}

fn is_clickhouse_binary_path(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect::<Vec<_>>();

    (components.len() == 1 && components[0] == OsStr::new("clickhouse"))
        || (components.len() >= 3
            && components[components.len() - 3] == OsStr::new("usr")
            && components[components.len() - 2] == OsStr::new("bin")
            && components[components.len() - 1] == OsStr::new("clickhouse"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version_manager::atomic::cleanup_staging_before;
    use crate::version_manager::platform::{Arch, Os};
    use flate2::{Compression, write::GzEncoder};
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime};
    use tar::{Builder, EntryType, Header};

    #[tokio::test]
    #[ignore = "isolated subprocess helper for master download tests"]
    async fn master_download_process_helper() {
        let Ok(args) = std::env::var("CHCTL_TEST_MASTER_ARGS") else {
            return;
        };
        if args == "ensure" {
            ensure_installed_local_first(&VersionSpec::Latest, &Platform::detect().unwrap(), true)
                .await
                .unwrap();
            return;
        }
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(
            ["clickhousectl", "local"]
                .into_iter()
                .chain(args.split_whitespace()),
        )
        .unwrap();
        let crate::cli::Commands::Local(local) = cli.command else {
            panic!("expected local command");
        };
        crate::local::run(local.command, true).await.unwrap();
    }

    struct MasterFixture {
        home: tempfile::TempDir,
        server: wiremock::MockServer,
    }

    impl MasterFixture {
        async fn new() -> Self {
            Self {
                home: tempfile::tempdir().unwrap(),
                server: wiremock::MockServer::start().await,
            }
        }

        fn binary(&self) -> PathBuf {
            self.home
                .path()
                .join(".clickhouse/versions/26.9.1.1292/clickhouse")
        }

        fn sidecar(&self) -> PathBuf {
            self.home
                .path()
                .join(".clickhouse/versions/.master-builds.json")
        }

        fn record(&self) -> Option<serde_json::Value> {
            let bytes = fs::read(self.sidecar()).ok()?;
            let sidecar: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            sidecar["builds"]
                .get(Platform::detect().unwrap().builds_path())
                .cloned()
        }

        fn default(&self) -> String {
            fs::read_to_string(self.home.path().join(".clickhouse/default")).unwrap()
        }

        async fn run(&self, args: &str) -> std::process::Output {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "version_manager::install::tests::master_download_process_helper",
                    "--ignored",
                    "--nocapture",
                ])
                .env_clear()
                .env("HOME", self.home.path())
                .env("DO_NOT_TRACK", "1")
                .env(
                    "CHCTL_TEST_MASTER_URL",
                    format!("{}/master", self.server.uri()),
                )
                .env("CHCTL_TEST_MASTER_ARGS", args)
                .current_dir(self.home.path());
            tokio::task::spawn_blocking(move || command.output().unwrap())
                .await
                .unwrap()
        }

        async fn success(&self, args: &str) {
            let output = self.run(args).await;
            assert!(
                output.status.success(),
                "{args}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        async fn artifact(&self, etag: Option<&str>, marker: &str) {
            self.server.reset().await;
            let mut response = wiremock::ResponseTemplate::new(200)
                .set_body_string(format!(
                    "#!/bin/sh\necho detected >> \"$HOME/detections\"\necho 'ClickHouse client version 26.9.1.1292 (official build).'\n# {marker}\n"
                ))
                .insert_header("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT");
            if let Some(etag) = etag {
                response = response.insert_header("etag", etag);
            }
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .respond_with(response)
                .mount(&self.server)
                .await;
        }

        async fn unchanged(&self, etag: &str) {
            self.server.reset().await;
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::header("if-none-match", etag))
                .respond_with(wiremock::ResponseTemplate::new(304))
                .mount(&self.server)
                .await;
        }

        async fn validators(&self) -> Vec<Option<String>> {
            self.server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .map(|request| {
                    assert_eq!(request.method.as_str(), "GET");
                    request
                        .headers
                        .get("if-none-match")
                        .map(|value| value.to_str().unwrap().to_string())
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn master_install_use_and_ensure_transfer_one_body_without_head() {
        let fixture = MasterFixture::new().await;
        // Preserve an existing default on install, then change it on use.
        let previous_binary = fixture
            .home
            .path()
            .join(".clickhouse/versions/25.12.9.61/clickhouse");
        fs::create_dir_all(previous_binary.parent().unwrap()).unwrap();
        fs::write(previous_binary, "previous build").unwrap();
        fs::write(
            fixture.home.path().join(".clickhouse/default"),
            "25.12.9.61",
        )
        .unwrap();
        fixture.artifact(Some("\"get-etag\""), "original").await;
        // HEAD is deliberately unusable. The conditional-GET path must never
        // wait for it or use its different ETag to describe the GET bytes.
        wiremock::Mock::given(wiremock::matchers::method("HEAD"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .insert_header("etag", "\"head-etag\"")
                    .set_delay(Duration::from_secs(60)),
            )
            .expect(0)
            .mount(&fixture.server)
            .await;
        fixture.success("install latest").await;
        assert_eq!(fixture.validators().await, vec![None]);
        assert_eq!(fixture.default(), "25.12.9.61");
        let record = fixture.record().unwrap();
        assert_eq!(record["etag"], "\"get-etag\"");
        assert_eq!(record["last_modified"], "Wed, 21 Oct 2015 07:28:00 GMT");
        let bytes = fs::read(fixture.binary()).unwrap();

        fixture.unchanged("\"get-etag\"").await;
        fixture.success("install latest").await;
        assert_eq!(fixture.default(), "25.12.9.61");
        fixture.success("use latest --no-global").await;
        fixture.success("ensure").await;
        assert_eq!(fixture.default(), "26.9.1.1292");
        assert_eq!(
            fixture.validators().await,
            vec![Some("\"get-etag\"".to_string()); 3]
        );
        assert_eq!(fs::read(fixture.binary()).unwrap(), bytes);
        assert_eq!(
            fs::read_to_string(fixture.home.path().join("detections")).unwrap(),
            "detected\n"
        );
        assert_eq!(fixture.record().unwrap(), record);
    }

    #[tokio::test]
    async fn master_changed_etag_replaces_bytes_with_the_same_version() {
        let fixture = MasterFixture::new().await;
        fixture.artifact(Some("\"old\""), "old").await;
        fixture.success("install latest").await;
        let old_bytes = fs::read(fixture.binary()).unwrap();
        fixture.artifact(Some("\"new\""), "new").await;
        fixture.success("use latest --no-global").await;
        assert_eq!(
            fixture.validators().await,
            vec![Some("\"old\"".to_string())]
        );
        assert_ne!(fs::read(fixture.binary()).unwrap(), old_bytes);
        assert_eq!(fixture.record().unwrap()["etag"], "\"new\"");
        assert_eq!(fixture.record().unwrap()["version"], "26.9.1.1292");
    }

    #[tokio::test]
    async fn master_force_and_missing_binary_download_without_validator() {
        let fixture = MasterFixture::new().await;
        fixture.artifact(Some("\"same\""), "first").await;
        fixture.success("install latest").await;
        fixture.artifact(Some("\"same\""), "forced").await;
        fixture.success("install latest --force").await;
        assert_eq!(fixture.validators().await, vec![None]);
        assert!(
            fs::read_to_string(fixture.binary())
                .unwrap()
                .contains("# forced")
        );
        fs::remove_file(fixture.binary()).unwrap();
        fixture.artifact(Some("\"same\""), "restored").await;
        fixture.success("use latest --no-global").await;
        assert_eq!(fixture.validators().await, vec![None]);
        assert!(fixture.binary().is_file());
    }

    #[tokio::test]
    async fn master_missing_or_invalid_validator_never_reuses_blindly() {
        for etag in [None, Some("*"), Some("invalid")] {
            let fixture = MasterFixture::new().await;
            fixture.artifact(etag, "first").await;
            fixture.success("install latest").await;
            assert!(fixture.record().is_none());
            fixture.success("install latest").await;
            assert_eq!(fixture.validators().await, vec![None, None]);
        }
    }

    #[tokio::test]
    async fn master_download_without_etag_invalidates_old_record() {
        let fixture = MasterFixture::new().await;
        fixture.artifact(Some("\"old\""), "old").await;
        fixture.success("install latest").await;
        fixture.artifact(None, "new").await;
        fixture.success("install latest").await;
        assert!(fixture.record().is_none());
        fixture.artifact(Some("\"next\""), "next").await;
        fixture.success("install latest").await;
        assert_eq!(fixture.validators().await, vec![None]);
    }

    #[tokio::test]
    async fn master_failed_validation_or_version_detection_preserves_install() {
        let fixture = MasterFixture::new().await;
        fixture.artifact(Some("\"old\""), "old").await;
        fixture.success("install latest").await;
        let bytes = fs::read(fixture.binary()).unwrap();
        let record = fixture.record().unwrap();
        for response in [
            wiremock::ResponseTemplate::new(403),
            wiremock::ResponseTemplate::new(200)
                .insert_header("etag", "\"broken\"")
                .set_body_string("#!/bin/sh\nexit 1\n"),
        ] {
            fixture.server.reset().await;
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .respond_with(response)
                .mount(&fixture.server)
                .await;
            assert!(!fixture.run("use latest --no-global").await.status.success());
            assert_eq!(fs::read(fixture.binary()).unwrap(), bytes);
            assert_eq!(fixture.record().unwrap(), record);
            assert_eq!(
                fixture.validators().await,
                vec![Some("\"old\"".to_string())]
            );
        }
    }

    #[tokio::test]
    async fn master_changed_local_record_during_validation_forces_full_download() {
        for remove_binary in [false, true] {
            let fixture = MasterFixture::new().await;
            fixture.artifact(Some("\"old\""), "old").await;
            fixture.success("install latest").await;
            fixture.artifact(Some("\"current\""), "current").await;
            let binary = fixture.binary();
            let sidecar = fixture.sidecar();
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::header("if-none-match", "\"old\""))
                .respond_with(move |_: &wiremock::Request| {
                    if remove_binary {
                        fs::remove_file(&binary).unwrap();
                    } else {
                        fs::write(&sidecar, "{}").unwrap();
                    }
                    wiremock::ResponseTemplate::new(304)
                })
                .with_priority(1)
                .mount(&fixture.server)
                .await;
            fixture.success("use latest --no-global").await;
            assert_eq!(
                fixture.validators().await,
                vec![Some("\"old\"".to_string()), None]
            );
            assert_eq!(fixture.record().unwrap()["etag"], "\"current\"");
            assert!(
                fs::read_to_string(fixture.binary())
                    .unwrap()
                    .contains("# current")
            );
        }
    }

    fn write_archive(archive_path: &Path, entry_path: &str, contents: &[u8]) {
        let archive_file = File::create(archive_path).unwrap();
        let encoder = GzEncoder::new(archive_file, Compression::default());
        let mut archive = Builder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_path(entry_path).unwrap();
        header.set_entry_type(EntryType::Regular);
        header.set_mode(0o755);
        header.set_size(contents.len() as u64);
        header.set_cksum();
        archive.append(&header, contents).unwrap();
        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn write_symlink_archive(archive_path: &Path, entry_path: &str) {
        let archive_file = File::create(archive_path).unwrap();
        let encoder = GzEncoder::new(archive_file, Compression::default());
        let mut archive = Builder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_path(entry_path).unwrap();
        header.set_entry_type(EntryType::Symlink);
        header.set_link_name("../../outside").unwrap();
        header.set_mode(0o777);
        header.set_size(0);
        header.set_cksum();
        archive.append(&header, io::empty()).unwrap();
        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn assert_invalid_gzip_is_not_committed(mutate: impl FnOnce(&Path)) {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("clickhouse.tgz");
        write_archive(&archive_path, "clickhouse", b"clickhouse binary");
        mutate(&archive_path);

        let error = extract_tarball_auto(&archive_path, temp.path()).unwrap_err();

        assert!(matches!(error, Error::ExtractArchive { .. }), "{error}");
        assert!(!temp.path().join("clickhouse").exists());
        assert!(!temp.path().join(".clickhouse.extracting").exists());
    }

    fn test_platform() -> Platform {
        Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        }
    }

    fn signal(path: &Path, value: &str) {
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut file = File::create(&temporary).unwrap();
        file.write_all(value.as_bytes()).unwrap();
        file.sync_all().unwrap();
        drop(file);
        fs::rename(temporary, path).unwrap();
    }

    fn wait_for_file(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !path.exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {}",
                path.display()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_for_env_path(name: &str) {
        if let Some(path) = std::env::var_os(name).map(PathBuf::from) {
            wait_for_file(&path);
        }
    }

    fn signal_env_path(name: &str, value: &str) {
        if let Some(path) = std::env::var_os(name).map(PathBuf::from) {
            signal(&path, value);
        }
    }

    fn helper_command(versions_dir: &Path, version: &str, contents: &str, etag: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "version_manager::install::tests::atomic_install_process_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("CHCTL_ATOMIC_HELPER", "1")
            .env("CHCTL_ATOMIC_VERSIONS_DIR", versions_dir)
            .env("CHCTL_ATOMIC_VERSION", version)
            .env("CHCTL_ATOMIC_CONTENTS", contents)
            .env("CHCTL_ATOMIC_ETAG", etag)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn assert_child_success(child: Child) {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "helper failed with {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn seed_sidecar(versions_dir: &Path, etag: &str, version: &str) {
        let sidecar = serde_json::json!({
            "builds": {
                "amd64": {
                    "etag": etag,
                    "version": version
                }
            }
        });
        fs::write(
            versions_dir.join(".master-builds.json"),
            serde_json::to_vec_pretty(&sidecar).unwrap(),
        )
        .unwrap();
    }

    fn assert_master_record(versions_dir: &Path, expected: Option<(&str, &str)>) {
        let sidecar: serde_json::Value =
            serde_json::from_slice(&fs::read(versions_dir.join(".master-builds.json")).unwrap())
                .unwrap();
        let record = sidecar["builds"].get("amd64");
        match expected {
            Some((etag, version)) => {
                let record = record.expect("amd64 master record");
                assert_eq!(record["etag"], etag);
                assert_eq!(record["version"], version);
            }
            None => assert!(record.is_none(), "unexpected master record: {record:?}"),
        }
    }

    #[test]
    fn empty_version_directory_is_not_installed() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = temp.path().join("26.5.1.1");
        fs::create_dir(&version_dir).unwrap();
        let binary = version_dir.join("clickhouse");

        assert!(!is_installed(&binary));

        fs::write(&binary, b"clickhouse").unwrap();
        assert!(is_installed(&binary));
    }

    #[test]
    fn post_commit_sidecar_failure_does_not_fail_install() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        fs::create_dir(&versions_dir).unwrap();
        let staging = InstallStaging::create(&versions_dir).unwrap();
        fs::write(staging.binary_path(), b"complete-master-build").unwrap();
        fs::write(staging.path().join("master-builds.json.tmp"), b"occupied").unwrap();
        let lock = CommitLock::acquire_blocking(&versions_dir).unwrap();
        let head = master::ArtifactInfo {
            etag: "etag-new".to_string(),
            last_modified: None,
        };

        let replaced = commit_staged_install_locked(
            &lock,
            &versions_dir,
            &staging,
            "26.5.1.1",
            true,
            true,
            &test_platform(),
            Some(&head),
            |_| Ok(false),
            |_| Ok(()),
        )
        .unwrap();

        assert!(!replaced.0);
        assert_eq!(
            fs::read(versions_dir.join("26.5.1.1/clickhouse")).unwrap(),
            b"complete-master-build"
        );
        assert!(!versions_dir.join(".master-builds.json").exists());
    }

    #[test]
    fn metadata_failure_does_not_replace_existing_master_binary() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        let version_dir = versions_dir.join("26.5.1.1");
        fs::create_dir_all(&version_dir).unwrap();
        let installed_binary = version_dir.join("clickhouse");
        fs::write(&installed_binary, b"existing master build").unwrap();
        let staging = InstallStaging::create(&versions_dir).unwrap();
        fs::write(staging.binary_path(), b"replacement master build").unwrap();
        let lock = CommitLock::acquire_blocking(&versions_dir).unwrap();

        let error = commit_staged_install_locked(
            &lock,
            &versions_dir,
            &staging,
            "26.5.1.1",
            true,
            true,
            &test_platform(),
            None,
            |_| {
                Err(Error::ServerMetadataRead {
                    path: PathBuf::from("broken.json"),
                    source: std::io::Error::other("metadata unavailable"),
                })
            },
            |_| Ok(()),
        )
        .unwrap_err();

        assert!(matches!(error, Error::ServerMetadataRead { .. }));
        assert_eq!(
            fs::read(installed_binary).unwrap(),
            b"existing master build"
        );
        assert_eq!(
            fs::read(staging.binary_path()).unwrap(),
            b"replacement master build"
        );
    }

    #[test]
    #[ignore = "subprocess helper for atomic install tests"]
    fn atomic_install_process_helper() {
        if std::env::var_os("CHCTL_ATOMIC_HELPER").is_none() {
            return;
        }

        let versions_dir = PathBuf::from(std::env::var_os("CHCTL_ATOMIC_VERSIONS_DIR").unwrap());
        let version = std::env::var("CHCTL_ATOMIC_VERSION").unwrap();
        let contents = std::env::var("CHCTL_ATOMIC_CONTENTS").unwrap();
        let etag = std::env::var("CHCTL_ATOMIC_ETAG").unwrap();
        let staging = InstallStaging::create(&versions_dir).unwrap();
        fs::write(staging.binary_path(), contents).unwrap();
        let mut permissions = fs::metadata(staging.binary_path()).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(staging.binary_path(), permissions).unwrap();

        signal_env_path("CHCTL_ATOMIC_STAGED", &staging.path().to_string_lossy());
        wait_for_env_path("CHCTL_ATOMIC_BEFORE_LOCK_RELEASE");

        let lock = CommitLock::acquire_blocking(&versions_dir).unwrap();
        signal_env_path("CHCTL_ATOMIC_LOCKED", "locked");
        wait_for_env_path("CHCTL_ATOMIC_LOCK_RELEASE");

        let pause_at = std::env::var("CHCTL_ATOMIC_PAUSE_AT").ok();
        let head = master::ArtifactInfo {
            etag,
            last_modified: None,
        };
        commit_staged_install_locked(
            &lock,
            &versions_dir,
            &staging,
            &version,
            true,
            true,
            &test_platform(),
            Some(&head),
            |_| Ok(false),
            |checkpoint| {
                let checkpoint_name = match checkpoint {
                    CommitCheckpoint::SidecarInvalidated => "invalidated",
                    CommitCheckpoint::BinaryReplaced => "replaced",
                };
                if pause_at.as_deref() == Some(checkpoint_name) {
                    signal_env_path("CHCTL_ATOMIC_CHECKPOINT", checkpoint_name);
                    wait_for_env_path("CHCTL_ATOMIC_CHECKPOINT_RELEASE");
                }
                Ok(())
            },
        )
        .unwrap();
    }

    #[test]
    fn atomic_same_version_race_commits_binary_and_matching_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        fs::create_dir_all(&versions_dir).unwrap();
        let first_staged = temp.path().join("first-staged");
        let first_locked = temp.path().join("first-locked");
        let release_first = temp.path().join("release-first");
        let second_staged = temp.path().join("second-staged");
        let release_second = temp.path().join("release-second");

        let mut first = helper_command(&versions_dir, "26.5.1.1", "master-a", "etag-a");
        first
            .env("CHCTL_ATOMIC_STAGED", &first_staged)
            .env("CHCTL_ATOMIC_LOCKED", &first_locked)
            .env("CHCTL_ATOMIC_LOCK_RELEASE", &release_first);
        let first = first.spawn().unwrap();
        wait_for_file(&first_locked);

        let mut second = helper_command(&versions_dir, "26.5.1.1", "master-b", "etag-b");
        second
            .env("CHCTL_ATOMIC_STAGED", &second_staged)
            .env("CHCTL_ATOMIC_BEFORE_LOCK_RELEASE", &release_second);
        let second = second.spawn().unwrap();
        wait_for_file(&second_staged);
        let first_stage = PathBuf::from(fs::read_to_string(&first_staged).unwrap());
        let second_stage = PathBuf::from(fs::read_to_string(&second_staged).unwrap());
        assert_ne!(first_stage, second_stage);
        assert!(first_stage.exists());
        assert!(second_stage.exists());

        signal(&release_second, "release");
        signal(&release_first, "release");
        assert_child_success(first);
        assert_child_success(second);

        assert_eq!(
            fs::read(versions_dir.join("26.5.1.1/clickhouse")).unwrap(),
            b"master-b"
        );
        assert_master_record(&versions_dir, Some(("etag-b", "26.5.1.1")));
    }

    #[test]
    fn atomic_different_version_race_preserves_binaries_and_latest_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        fs::create_dir_all(&versions_dir).unwrap();
        let first_locked = temp.path().join("first-locked");
        let release_first = temp.path().join("release-first");
        let second_staged = temp.path().join("second-staged");

        let mut first = helper_command(&versions_dir, "26.5.1.1", "version-a", "etag-a");
        first
            .env("CHCTL_ATOMIC_LOCKED", &first_locked)
            .env("CHCTL_ATOMIC_LOCK_RELEASE", &release_first);
        let first = first.spawn().unwrap();
        wait_for_file(&first_locked);

        let mut second = helper_command(&versions_dir, "26.6.2.2", "version-b", "etag-b");
        second.env("CHCTL_ATOMIC_STAGED", &second_staged);
        let second = second.spawn().unwrap();
        wait_for_file(&second_staged);

        signal(&release_first, "release");
        assert_child_success(first);
        assert_child_success(second);

        assert_eq!(
            fs::read(versions_dir.join("26.5.1.1/clickhouse")).unwrap(),
            b"version-a"
        );
        assert_eq!(
            fs::read(versions_dir.join("26.6.2.2/clickhouse")).unwrap(),
            b"version-b"
        );
        assert_master_record(&versions_dir, Some(("etag-b", "26.6.2.2")));
    }

    #[test]
    fn interrupted_commit_keeps_valid_binary_and_invalidates_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        let version_dir = versions_dir.join("26.5.1.1");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("clickhouse"), b"known-good").unwrap();
        seed_sidecar(&versions_dir, "etag-old", "26.5.1.1");
        let checkpoint = temp.path().join("invalidated");
        let never_release = temp.path().join("never-release");

        let mut command = helper_command(&versions_dir, "26.5.1.1", "partial-new", "etag-new");
        command
            .env("CHCTL_ATOMIC_PAUSE_AT", "invalidated")
            .env("CHCTL_ATOMIC_CHECKPOINT", &checkpoint)
            .env("CHCTL_ATOMIC_CHECKPOINT_RELEASE", &never_release);
        let mut child = command.spawn().unwrap();
        wait_for_file(&checkpoint);
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());

        assert_eq!(
            fs::read(version_dir.join("clickhouse")).unwrap(),
            b"known-good"
        );
        assert_master_record(&versions_dir, None);
    }

    #[test]
    fn interrupted_after_replacement_leaves_complete_binary_and_safe_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        let version_dir = versions_dir.join("26.5.1.1");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("clickhouse"), b"known-good").unwrap();
        seed_sidecar(&versions_dir, "etag-old", "26.5.1.1");
        let checkpoint = temp.path().join("replaced");
        let never_release = temp.path().join("never-release");

        let mut command = helper_command(&versions_dir, "26.5.1.1", "complete-new", "etag-new");
        command
            .env("CHCTL_ATOMIC_PAUSE_AT", "replaced")
            .env("CHCTL_ATOMIC_CHECKPOINT", &checkpoint)
            .env("CHCTL_ATOMIC_CHECKPOINT_RELEASE", &never_release);
        let mut child = command.spawn().unwrap();
        wait_for_file(&checkpoint);
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());

        assert_eq!(
            fs::read(version_dir.join("clickhouse")).unwrap(),
            b"complete-new"
        );
        assert_master_record(&versions_dir, None);
    }

    #[test]
    fn stale_cleanup_removes_only_unowned_stages_during_live_install() {
        let temp = tempfile::tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        let staging_root = versions_dir.join(".staging");
        fs::create_dir_all(&staging_root).unwrap();
        let stale = staging_root.join(format!("install-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(stale.join("payload")).unwrap();
        fs::write(stale.join(".owner.lock"), b"").unwrap();
        fs::write(stale.join("payload/clickhouse"), b"abandoned").unwrap();
        let unknown = staging_root.join("install-not-owned-by-clickhousectl");
        fs::create_dir(&unknown).unwrap();

        let live_staged = temp.path().join("live-staged");
        let release_live = temp.path().join("release-live");
        let mut command = helper_command(&versions_dir, "26.7.3.3", "live-build", "etag-live");
        command
            .env("CHCTL_ATOMIC_STAGED", &live_staged)
            .env("CHCTL_ATOMIC_BEFORE_LOCK_RELEASE", &release_live);
        let child = command.spawn().unwrap();
        wait_for_file(&live_staged);
        let live_stage = PathBuf::from(fs::read_to_string(&live_staged).unwrap());

        cleanup_staging_before(&versions_dir, SystemTime::now() + Duration::from_secs(1)).unwrap();
        assert!(!stale.exists());
        assert!(unknown.exists());
        assert!(live_stage.exists());

        signal(&release_live, "release");
        assert_child_success(child);
        assert!(!live_stage.exists());
        assert_eq!(
            fs::read(versions_dir.join("26.7.3.3/clickhouse")).unwrap(),
            b"live-build"
        );
        assert_master_record(&versions_dir, Some(("etag-live", "26.7.3.3")));
    }

    #[test]
    fn parse_version_output_takes_the_first_four_part_numeric_word() {
        for (output, expected) in [
            (
                "ClickHouse client version 25.12.9.61 (official build).",
                "25.12.9.61",
            ),
            (
                "ClickHouse server version 26.3.1.100 (official build).",
                "26.3.1.100",
            ),
            (
                "ClickHouse client version 25.5.2.1 (official build).\nSome other info.",
                "25.5.2.1",
            ),
        ] {
            assert_eq!(
                parse_version_output(output).unwrap(),
                expected,
                "{output:?}"
            );
        }
        for output in ["Some random output without a version.", ""] {
            assert!(parse_version_output(output).is_err(), "{output:?}");
        }
    }

    #[test]
    fn a_binary_version_comes_only_from_a_clickhouse_line() {
        for (output, expected) in [
            (
                "ClickHouse local version 25.12.9.61 (official build).\n",
                "25.12.9.61",
            ),
            // A local build has no build tag and may print a warning first.
            (
                "warning: x\nClickHouse local version 26.1.1.1.\n",
                "26.1.1.1",
            ),
        ] {
            assert_eq!(
                parse_clickhouse_version_output(output).as_deref(),
                Some(expected),
                "{output:?}"
            );
        }
        for output in [
            "",
            "Python 3.12.1",
            "tool 1.2.3.4",
            "ClickHouse keeper, no version",
        ] {
            assert_eq!(parse_clickhouse_version_output(output), None, "{output:?}");
        }
    }

    #[test]
    fn binary_source_must_be_a_readable_regular_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("clickhouse");
        fs::write(&file, b"binary").unwrap();
        let link = temp.path().join("clickhouse-server");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        check_source_binary(&file).unwrap();
        check_source_binary(&link).unwrap();

        check_source_binary(temp.path()).unwrap_err();
        check_source_binary(&temp.path().join("missing")).unwrap_err();
    }

    fn shell_probe(script: &str) -> tokio::process::Command {
        let mut command = tokio::process::Command::new("/bin/sh");
        command.args(["-c", script]);
        command
    }

    #[tokio::test]
    async fn version_probe_returns_stdout_only_on_success() {
        let out = run_version_probe(
            shell_probe("echo 'ClickHouse local version 26.1.1.1.'"),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
        assert_eq!(
            parse_clickhouse_version_output(&out).as_deref(),
            Some("26.1.1.1")
        );

        let error = run_version_probe(
            shell_probe("echo stderr-token >&2; exit 3"),
            Duration::from_secs(20),
        )
        .await
        .unwrap_err();
        assert!(error.contains("stderr-token"), "{error}");
        assert!(error.contains('3'), "{error}");

        let missing = tokio::process::Command::new("/nonexistent/clickhouse");
        run_version_probe(missing, Duration::from_secs(20))
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn version_probe_kills_a_binary_that_never_answers() {
        // A shell builtin loop, so no child process outlives the kill.
        let started = Instant::now();
        let error = run_version_probe(
            shell_probe("while :; do :; done"),
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();
        assert!(error.contains("100ms"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn version_probe_stops_reading_a_flood_of_output() {
        let started = Instant::now();
        let result = run_version_probe(
            shell_probe("while echo flood-of-output; do :; done"),
            Duration::from_secs(20),
        )
        .await;
        // The pipe closes at the limit, so the loop ends rather than the wait.
        assert!(started.elapsed() < Duration::from_secs(20));
        if let Ok(out) = result {
            assert!(out.len() as u64 <= BINARY_VERSION_OUTPUT_LIMIT);
        }
    }

    #[test]
    fn extracts_package_binary_without_host_tar() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("clickhouse.tgz");
        write_archive(
            &archive_path,
            "clickhouse-common-static/usr/bin/clickhouse",
            b"clickhouse binary",
        );

        extract_tarball_auto(&archive_path, temp.path()).unwrap();

        assert_eq!(
            fs::read(temp.path().join("clickhouse")).unwrap(),
            b"clickhouse binary"
        );
        assert!(!archive_path.exists());
        assert!(!temp.path().join(".clickhouse.extracting").exists());
    }

    #[test]
    fn corrupted_gzip_crc_is_not_committed() {
        assert_invalid_gzip_is_not_committed(|archive_path| {
            let mut bytes = fs::read(archive_path).unwrap();
            let crc_offset = bytes.len() - 8;
            bytes[crc_offset] ^= 0xff;
            fs::write(archive_path, bytes).unwrap();
        });
    }

    #[test]
    fn truncated_gzip_trailer_is_not_committed() {
        assert_invalid_gzip_is_not_committed(|archive_path| {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(archive_path)
                .unwrap();
            file.set_len(file.metadata().unwrap().len() - 4).unwrap();
        });
    }

    #[test]
    fn malformed_archive_error_preserves_archive_destination_and_cause() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("broken.tgz");
        fs::write(&archive_path, "not a gzip archive").unwrap();
        let destination = temp.path().join("clickhouse");

        let error = extract_tarball_auto(&archive_path, temp.path()).unwrap_err();
        let Error::ExtractArchive {
            archive,
            destination: error_destination,
            source,
        } = &error
        else {
            panic!("expected contextual archive error: {error}");
        };

        assert_eq!(archive, &archive_path);
        assert_eq!(error_destination, &destination);
        assert!(error.to_string().contains(&source.to_string()));
        assert!(!temp.path().join("clickhouse").exists());
    }

    #[test]
    fn missing_binary_error_names_archive_and_expected_paths() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("missing.tgz");
        write_archive(&archive_path, "package/README.md", b"read me");

        let error = extract_tarball_auto(&archive_path, temp.path()).unwrap_err();

        assert!(matches!(&error, Error::Extract(_)), "{error:?}");
        let message = error.to_string();
        for required in [
            format!("'{}'", archive_path.display()),
            "'clickhouse'".to_string(),
            "'*/usr/bin/clickhouse'".to_string(),
        ] {
            assert!(message.contains(&required), "missing {required}: {message}");
        }
    }

    #[test]
    fn extraction_destination_error_includes_path_and_os_cause() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("clickhouse.tgz");
        write_archive(&archive_path, "package/usr/bin/clickhouse", b"binary");
        let destination = temp.path().join("not-a-directory");
        fs::write(&destination, "content").unwrap();

        let error = extract_tarball_auto(&archive_path, &destination).unwrap_err();
        let message = error.to_string();

        assert!(
            message.contains(
                &destination
                    .join(".clickhouse.extracting")
                    .display()
                    .to_string()
            ),
            "{message}"
        );
        let Error::ExtractArchive { source, .. } = &error else {
            panic!("expected contextual archive error: {error}");
        };
        assert_eq!(source.kind(), std::io::ErrorKind::NotADirectory);
        assert!(message.contains(&source.to_string()), "{message}");
    }

    #[test]
    fn refuses_link_for_clickhouse_binary() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("linked.tgz");
        write_symlink_archive(&archive_path, "package/usr/bin/clickhouse");

        let error = extract_tarball_auto(&archive_path, temp.path()).unwrap_err();

        // The link is rejected by name rather than followed or reported missing.
        assert!(matches!(&error, Error::Extract(_)), "{error:?}");
        let message = error.to_string();
        assert!(
            message.contains("'package/usr/bin/clickhouse'"),
            "{message}"
        );
        assert!(!message.contains("'*/usr/bin/clickhouse'"), "{message}");
        assert!(!temp.path().join("clickhouse").exists());
    }

    #[test]
    fn refuses_archive_path_traversal() {
        let archive_path = Path::new("/tmp/clickhouse.tgz");
        let error =
            validate_archive_entry_path(archive_path, Path::new("../package/usr/bin/clickhouse"))
                .unwrap_err();

        assert!(matches!(&error, Error::Extract(_)), "{error:?}");
        let message = error.to_string();
        for required in ["'/tmp/clickhouse.tgz'", "'../package/usr/bin/clickhouse'"] {
            assert!(message.contains(required), "missing {required}: {message}");
        }
    }
}
