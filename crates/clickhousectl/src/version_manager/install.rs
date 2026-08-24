use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::download::download_from_source;
use crate::version_manager::list::list_installed_versions;
use crate::version_manager::lock::FileLock;
use crate::version_manager::master;
use crate::version_manager::platform::{DownloadSource, Platform};
use crate::version_manager::resolve::{ResolvedVersion, resolve, try_resolve_local};
use crate::version_manager::spec::VersionSpec;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

struct StagingDir {
    path: PathBuf,
}

impl StagingDir {
    fn create(versions_dir: &Path) -> Result<Self> {
        let staging_root = versions_dir.join(".staging");
        std::fs::create_dir_all(&staging_root)?;
        loop {
            let path = staging_root.join(uuid::Uuid::new_v4().simple().to_string());
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Install a version spec, trying installed versions first before any remote call.
/// An installed match is a successful no-op regardless of whether the spec is
/// exact or partial.
pub async fn install_local_first(
    spec: &VersionSpec,
    platform: &Platform,
    force: bool,
) -> Result<String> {
    if !force && let Some(local) = try_resolve_local(spec) {
        eprintln!("ClickHouse {} is already installed as {}", spec, local);
        eprintln!("Use --force to re-download the latest build");
        return Ok(local);
    }

    eprintln!("Resolving {}...", spec);
    let resolved = resolve(spec, platform).await?;
    install_resolved(&resolved, platform, force).await
}

/// Like `install_local_first`, but returns an existing local version silently
/// (matching `ensure_installed`'s semantics). For `server start -v <spec>`.
pub async fn ensure_installed_local_first(
    spec: &VersionSpec,
    platform: &Platform,
) -> Result<String> {
    if let Some(local) = try_resolve_local(spec) {
        return Ok(local);
    }

    eprintln!("Resolving {}...", spec);
    let resolved = resolve(spec, platform).await?;
    ensure_installed(&resolved, platform).await
}

/// Installs a ClickHouse version using the multi-source resolution system.
/// Returns the exact version string of the installed binary.
pub async fn install_resolved(
    resolved: &ResolvedVersion,
    platform: &Platform,
    force: bool,
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
    let mut master_head = None;
    if is_master {
        master_head = master::head_info(platform).await;
        if !force && let Some(version) = master::reuse_if_unchanged(platform, master_head.as_ref())
        {
            eprintln!(
                "latest is up to date (master build unchanged); using {}",
                version
            );
            return Ok(version);
        }
    }

    // If we know the exact version upfront, check if already installed
    if let Some(ref version) = resolved.exact_version {
        let binary = paths::binary_path(version)?;
        if binary.exists() && !force {
            return Err(Error::VersionAlreadyInstalled(version.to_string()));
        }
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
            eprintln!(
                "ClickHouse {} is already installed as {}",
                version_path, existing
            );
            eprintln!("Use --force to re-download the latest build");
            return Ok(existing.clone());
        }
    }

    // Keep every download isolated. A killed process may leave its own staging
    // directory behind, but no later invocation reuses or removes it.
    let staging = StagingDir::create(&versions_dir)?;
    let temp_dir = staging.path();

    let binary_path = temp_dir.join("clickhouse");

    eprintln!("Downloading ClickHouse {}...", resolved.display_version);

    if resolved.source.is_tarball(platform) {
        let tarball_path = temp_dir.join("clickhouse.tgz");
        download_from_source(&resolved.source, platform, &tarball_path).await?;
        eprintln!("Extracting...");
        extract_tarball_auto(&tarball_path, temp_dir)?;
    } else {
        download_from_source(&resolved.source, platform, &binary_path).await?;
    }

    // Make the binary executable
    let mut perms = std::fs::metadata(&binary_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary_path, perms)?;

    // Detect the exact version from the binary
    let exact_version = if resolved.exact_version_known {
        resolved.exact_version.clone().unwrap()
    } else {
        eprintln!("Detecting version...");
        detect_binary_version(&binary_path)?
    };

    let replaced_existing = commit_staged_binary(
        &versions_dir,
        &binary_path,
        &exact_version,
        force,
        is_master,
        platform,
        master_head.as_ref(),
    )?;

    // Replacing a build on disk never affects already-running servers (they keep
    // executing the old binary) — just say so, so the swap isn't silent.
    if is_master && replaced_existing && version_in_use_by_running_server(&exact_version) {
        eprintln!(
            "Note: running servers keep using the previous {} build until restarted",
            exact_version
        );
    }

    let channel_suffix = match resolved.channel {
        Some(ch) => format!(" ({})", ch),
        None => String::new(),
    };
    eprintln!("Installed ClickHouse {}{}", exact_version, channel_suffix);

    Ok(exact_version)
}

#[allow(clippy::too_many_arguments)]
fn commit_staged_binary(
    versions_dir: &Path,
    staged_binary: &Path,
    exact_version: &str,
    force: bool,
    is_master: bool,
    platform: &Platform,
    master_head: Option<&master::HeadInfo>,
) -> Result<bool> {
    let lock_path = versions_dir
        .join(".locks")
        .join(format!("version-{exact_version}.lock"));
    let _lock = FileLock::acquire(&lock_path)?;
    pause_after_target_lock_for_test();

    let version_dir = versions_dir.join(exact_version);
    let target_binary = version_dir.join("clickhouse");
    if target_binary.exists() && !force && !is_master {
        return Err(Error::VersionAlreadyInstalled(exact_version.to_string()));
    }

    let replaced_existing = target_binary.exists();
    let new_head = if is_master { master_head } else { None };
    master::commit_install_in(versions_dir, platform, exact_version, new_head, || {
        if version_dir.exists() {
            // Both paths are under versions_dir, so rename atomically replaces
            // an old complete binary without exposing staged contents.
            std::fs::rename(staged_binary, &target_binary)?;
        } else {
            // Rename a complete directory into place for a first install;
            // interruption cannot leave an empty target version behind.
            let commit_dir = staged_binary
                .parent()
                .expect("staged binary has a parent")
                .join("install");
            std::fs::create_dir(&commit_dir)?;
            std::fs::rename(staged_binary, commit_dir.join("clickhouse"))?;
            std::fs::rename(commit_dir, &version_dir)?;
        }
        Ok(())
    })?;

    Ok(replaced_existing)
}

#[cfg(test)]
fn pause_after_target_lock_for_test() {
    let (Ok(marker), Ok(release)) = (
        std::env::var("CHCTL_TEST_TARGET_LOCKED"),
        std::env::var("CHCTL_TEST_TARGET_RELEASE"),
    ) else {
        return;
    };
    std::fs::write(marker, b"locked").expect("write target lock marker");
    let release = PathBuf::from(release);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release target lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(not(test))]
fn pause_after_target_lock_for_test() {}

/// Like `install_resolved`, but returns the existing version instead of erroring
/// when already installed. Intended for cases like `server start --version` where
/// the goal is "make sure this version is available" rather than "install this".
pub async fn ensure_installed(resolved: &ResolvedVersion, platform: &Platform) -> Result<String> {
    // If we know the exact version upfront, return it if already installed
    if let Some(ref version) = resolved.exact_version {
        let binary = paths::binary_path(version)?;
        if binary.exists() {
            return Ok(version.clone());
        }
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
    match install_resolved(resolved, platform, false).await {
        Err(Error::VersionAlreadyInstalled(version)) => Ok(version),
        other => other,
    }
}

/// Whether a running managed server (in the current project) was started from
/// this version. Recovers orphans first (like `local remove`) so a server that
/// lost its metadata file is still counted.
fn version_in_use_by_running_server(version: &str) -> bool {
    crate::local::server::recover_current_project_servers();
    crate::local::server::list_running_servers()
        .iter()
        .any(|s| s.version == version)
}

/// Detect the version of a clickhouse binary by running `./clickhouse --version`
fn detect_binary_version(binary_path: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new(binary_path)
        .arg("--version")
        .output()
        .map_err(|e| Error::Exec(format!("Failed to run clickhouse --version: {}", e)))?;

    if !output.status.success() {
        return Err(Error::Exec(
            "clickhouse --version returned non-zero exit code".to_string(),
        ));
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
    let status = std::process::Command::new("tar")
        .args(["xzf", &tarball_path.to_string_lossy()])
        .current_dir(dest_dir)
        .status()
        .map_err(|e| Error::Extract(format!("Failed to run tar: {}", e)))?;

    if !status.success() {
        let _ = std::fs::remove_file(tarball_path);
        return Err(Error::Extract("tar extraction failed".to_string()));
    }

    let final_binary = dest_dir.join("clickhouse");

    // Search extracted directories for the binary
    for entry in std::fs::read_dir(dest_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join("usr/bin/clickhouse");
            if candidate.exists() {
                std::fs::rename(&candidate, &final_binary)?;
                let _ = std::fs::remove_file(tarball_path);
                let _ = std::fs::remove_dir_all(&path);
                return Ok(());
            }
        }
    }

    // Binary might already be at top level
    if final_binary.exists() {
        let _ = std::fs::remove_file(tarball_path);
        return Ok(());
    }

    let _ = std::fs::remove_file(tarball_path);
    Err(Error::Extract(
        "Could not find clickhouse binary in extracted tarball".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version_manager::platform::{Arch, Os};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    const COMMIT_HELPER: &str = "version_manager::install::tests::commit_install_subprocess";

    struct ChildInstall<'a> {
        versions_dir: &'a Path,
        version: &'a str,
        contents: &'a str,
        etag: &'a str,
        platform: &'a str,
        ready: &'a Path,
        target_pause: Option<(&'a Path, &'a Path)>,
        sidecar_pause: Option<(&'a Path, &'a Path)>,
        binary_pause: Option<(&'a Path, &'a Path)>,
    }

    fn spawn_install(config: ChildInstall<'_>) -> Child {
        let mut command = Command::new(std::env::current_exe().expect("locate test binary"));
        command
            .args(["--exact", COMMIT_HELPER, "--nocapture"])
            .env("CHCTL_TEST_COMMIT_ROOT", config.versions_dir)
            .env("CHCTL_TEST_COMMIT_VERSION", config.version)
            .env("CHCTL_TEST_COMMIT_CONTENTS", config.contents)
            .env("CHCTL_TEST_COMMIT_ETAG", config.etag)
            .env("CHCTL_TEST_COMMIT_PLATFORM", config.platform)
            .env("CHCTL_TEST_COMMIT_READY", config.ready)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some((marker, release)) = config.target_pause {
            command
                .env("CHCTL_TEST_TARGET_LOCKED", marker)
                .env("CHCTL_TEST_TARGET_RELEASE", release);
        }
        if let Some((marker, release)) = config.sidecar_pause {
            command
                .env("CHCTL_TEST_SIDECAR_LOCKED", marker)
                .env("CHCTL_TEST_SIDECAR_RELEASE", release);
        }
        if let Some((marker, release)) = config.binary_pause {
            command
                .env("CHCTL_TEST_BINARY_COMMIT_PAUSED", marker)
                .env("CHCTL_TEST_BINARY_COMMIT_RELEASE", release);
        }
        command.spawn().expect("spawn install helper")
    }

    fn wait_for_path(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !path.exists() {
            assert!(Instant::now() < deadline, "timed out waiting for {path:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_success(child: &mut Child) {
        let status = child.wait().expect("wait for install helper");
        assert!(status.success(), "install helper failed with {status}");
    }

    fn test_platform(name: &str) -> Platform {
        match name {
            "amd64" => Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            "macos-aarch64" => Platform {
                os: Os::MacOS,
                arch: Arch::Aarch64,
            },
            other => panic!("unknown test platform {other}"),
        }
    }

    fn sidecar(versions_dir: &Path) -> serde_json::Value {
        let bytes =
            std::fs::read(versions_dir.join(".master-builds.json")).expect("read master sidecar");
        serde_json::from_slice(&bytes).expect("parse master sidecar")
    }

    #[test]
    fn commit_install_subprocess() {
        let Ok(versions_dir) = std::env::var("CHCTL_TEST_COMMIT_ROOT") else {
            return;
        };
        let versions_dir = PathBuf::from(versions_dir);
        std::fs::create_dir_all(&versions_dir).unwrap();
        let version = std::env::var("CHCTL_TEST_COMMIT_VERSION").unwrap();
        let contents = std::env::var("CHCTL_TEST_COMMIT_CONTENTS").unwrap();
        let etag = std::env::var("CHCTL_TEST_COMMIT_ETAG").unwrap();
        let platform = test_platform(&std::env::var("CHCTL_TEST_COMMIT_PLATFORM").unwrap());
        let staging = StagingDir::create(&versions_dir).unwrap();
        let binary = staging.path().join("clickhouse");
        std::fs::write(&binary, contents).unwrap();
        std::fs::write(
            std::env::var("CHCTL_TEST_COMMIT_READY").unwrap(),
            staging.path().to_string_lossy().as_bytes(),
        )
        .unwrap();
        commit_staged_binary(
            &versions_dir,
            &binary,
            &version,
            true,
            true,
            &platform,
            Some(&master::HeadInfo {
                etag,
                last_modified: None,
            }),
        )
        .unwrap();
    }

    #[test]
    fn same_version_processes_commit_binary_and_sidecar_together() {
        let temp = tempfile::tempdir().unwrap();
        let versions = temp.path().join("versions");
        std::fs::create_dir(&versions).unwrap();
        let ready_a = temp.path().join("ready-a");
        let ready_b = temp.path().join("ready-b");
        let locked_a = temp.path().join("locked-a");
        let release_a = temp.path().join("release-a");
        let mut first = spawn_install(ChildInstall {
            versions_dir: &versions,
            version: "26.8.1.1",
            contents: "first-complete-binary",
            etag: "first-etag",
            platform: "amd64",
            ready: &ready_a,
            target_pause: Some((&locked_a, &release_a)),
            sidecar_pause: None,
            binary_pause: None,
        });
        wait_for_path(&locked_a);

        let mut second = spawn_install(ChildInstall {
            versions_dir: &versions,
            version: "26.8.1.1",
            contents: "second-complete-binary",
            etag: "second-etag",
            platform: "amd64",
            ready: &ready_b,
            target_pause: None,
            sidecar_pause: None,
            binary_pause: None,
        });
        wait_for_path(&ready_b);
        std::thread::sleep(Duration::from_millis(50));
        assert!(second.try_wait().unwrap().is_none());
        assert_ne!(
            std::fs::read_to_string(&ready_a).unwrap(),
            std::fs::read_to_string(&ready_b).unwrap()
        );

        std::fs::write(&release_a, b"release").unwrap();
        wait_success(&mut first);
        wait_success(&mut second);

        assert_eq!(
            std::fs::read_to_string(versions.join("26.8.1.1/clickhouse")).unwrap(),
            "second-complete-binary"
        );
        let sidecar = sidecar(&versions);
        assert_eq!(sidecar["builds"]["amd64"]["etag"], "second-etag");
        assert_eq!(sidecar["builds"]["amd64"]["version"], "26.8.1.1");
    }

    #[test]
    fn different_version_processes_preserve_binaries_and_sidecar_records() {
        let temp = tempfile::tempdir().unwrap();
        let versions = temp.path().join("versions");
        std::fs::create_dir(&versions).unwrap();
        let ready_a = temp.path().join("ready-a");
        let ready_b = temp.path().join("ready-b");
        let locked_a = temp.path().join("sidecar-locked-a");
        let release_a = temp.path().join("sidecar-release-a");
        let mut first = spawn_install(ChildInstall {
            versions_dir: &versions,
            version: "26.8.1.1",
            contents: "linux-binary",
            etag: "linux-etag",
            platform: "amd64",
            ready: &ready_a,
            target_pause: None,
            sidecar_pause: Some((&locked_a, &release_a)),
            binary_pause: None,
        });
        wait_for_path(&locked_a);

        let mut second = spawn_install(ChildInstall {
            versions_dir: &versions,
            version: "26.8.2.2",
            contents: "macos-binary",
            etag: "macos-etag",
            platform: "macos-aarch64",
            ready: &ready_b,
            target_pause: None,
            sidecar_pause: None,
            binary_pause: None,
        });
        wait_for_path(&ready_b);
        std::thread::sleep(Duration::from_millis(50));
        assert!(second.try_wait().unwrap().is_none());

        std::fs::write(&release_a, b"release").unwrap();
        wait_success(&mut first);
        wait_success(&mut second);

        assert_eq!(
            std::fs::read_to_string(versions.join("26.8.1.1/clickhouse")).unwrap(),
            "linux-binary"
        );
        assert_eq!(
            std::fs::read_to_string(versions.join("26.8.2.2/clickhouse")).unwrap(),
            "macos-binary"
        );
        let sidecar = sidecar(&versions);
        assert_eq!(sidecar["builds"]["amd64"]["etag"], "linux-etag");
        assert_eq!(sidecar["builds"]["macos-aarch64"]["etag"], "macos-etag");
    }

    #[test]
    fn interrupted_install_keeps_valid_binary_and_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let versions = temp.path().join("versions");
        let version = "26.8.1.1";
        std::fs::create_dir_all(versions.join(version)).unwrap();
        std::fs::write(versions.join(version).join("clickhouse"), b"valid-binary").unwrap();
        std::fs::write(
            versions.join(".master-builds.json"),
            format!(r#"{{"builds":{{"amd64":{{"etag":"valid-etag","version":"{version}"}}}}}}"#),
        )
        .unwrap();

        let ready = temp.path().join("ready");
        let locked = temp.path().join("binary-commit-paused");
        let release = temp.path().join("never-release");
        let mut child = spawn_install(ChildInstall {
            versions_dir: &versions,
            version,
            contents: "partial-replacement",
            etag: "partial-etag",
            platform: "amd64",
            ready: &ready,
            target_pause: None,
            sidecar_pause: None,
            binary_pause: Some((&locked, &release)),
        });
        wait_for_path(&locked);
        child.kill().unwrap();
        child.wait().unwrap();

        let abandoned_staging = PathBuf::from(std::fs::read_to_string(&ready).unwrap());
        assert_eq!(
            std::fs::read_to_string(versions.join(version).join("clickhouse")).unwrap(),
            "valid-binary"
        );
        assert_eq!(
            std::fs::read_to_string(abandoned_staging.join("clickhouse")).unwrap(),
            "partial-replacement"
        );
        let sidecar = sidecar(&versions);
        assert!(sidecar["builds"].get("amd64").is_none());
    }

    #[test]
    fn stale_staging_is_ignored_during_atomic_commit() {
        let temp = tempfile::tempdir().unwrap();
        let versions = temp.path().join("versions");
        let stale = versions.join(".staging/stale-install");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("clickhouse"), b"stale-binary").unwrap();
        let version = "26.8.1.1";
        std::fs::create_dir(versions.join(version)).unwrap();
        std::fs::write(versions.join(version).join("clickhouse"), b"old-binary").unwrap();

        let staging = StagingDir::create(&versions).unwrap();
        let staged_binary = staging.path().join("clickhouse");
        std::fs::write(&staged_binary, b"new-complete-binary").unwrap();
        let platform = test_platform("amd64");
        commit_staged_binary(
            &versions,
            &staged_binary,
            version,
            true,
            true,
            &platform,
            Some(&master::HeadInfo {
                etag: "new-etag".to_string(),
                last_modified: None,
            }),
        )
        .unwrap();
        drop(staging);

        assert_eq!(
            std::fs::read_to_string(stale.join("clickhouse")).unwrap(),
            "stale-binary"
        );
        assert_eq!(
            std::fs::read_to_string(versions.join(version).join("clickhouse")).unwrap(),
            "new-complete-binary"
        );
        let sidecar = sidecar(&versions);
        assert_eq!(sidecar["builds"]["amd64"]["etag"], "new-etag");
        assert_eq!(sidecar["builds"]["amd64"]["version"], version);
    }

    #[test]
    fn test_parse_version_output_client() {
        let output = "ClickHouse client version 25.12.9.61 (official build).";
        assert_eq!(parse_version_output(output).unwrap(), "25.12.9.61");
    }

    #[test]
    fn test_parse_version_output_server() {
        let output = "ClickHouse server version 26.3.1.100 (official build).";
        assert_eq!(parse_version_output(output).unwrap(), "26.3.1.100");
    }

    #[test]
    fn test_parse_version_output_multiline() {
        let output = "ClickHouse client version 25.5.2.1 (official build).\nSome other info.";
        assert_eq!(parse_version_output(output).unwrap(), "25.5.2.1");
    }

    #[test]
    fn test_parse_version_output_no_version() {
        let output = "Some random output without a version.";
        assert!(parse_version_output(output).is_err());
    }

    #[test]
    fn test_parse_version_output_empty() {
        assert!(parse_version_output("").is_err());
    }
}
