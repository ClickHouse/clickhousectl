use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::download::download_from_source;
use crate::version_manager::list::list_installed_versions;
use crate::version_manager::master;
use crate::version_manager::platform::{DownloadSource, Platform};
use crate::version_manager::resolve::{ResolvedVersion, resolve, try_resolve_local};
use crate::version_manager::spec::VersionSpec;
use std::os::unix::fs::PermissionsExt;

/// Install a version spec, trying installed versions first before any remote call.
/// Matches the UX of `install_resolved`'s post-resolve local check, but avoids the
/// network round-trip when a local match exists.
pub async fn install_local_first(
    spec: &VersionSpec,
    platform: &Platform,
    force: bool,
) -> Result<String> {
    if !force && let Some(local) = try_resolve_local(spec) {
        if matches!(spec, VersionSpec::Exact(_)) {
            return Err(Error::VersionAlreadyInstalled(local));
        }
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
        let version_dir = paths::version_dir(version)?;
        if version_dir.exists() && !force {
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

    // Download to a temp directory first
    let temp_dir = paths::versions_dir()?.join(".download-temp");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }
    std::fs::create_dir_all(&temp_dir)?;

    let binary_path = temp_dir.join("clickhouse");

    eprintln!("Downloading ClickHouse {}...", resolved.display_version);

    if resolved.source.is_tarball(platform) {
        let tarball_path = temp_dir.join("clickhouse.tgz");
        download_from_source(&resolved.source, platform, &tarball_path).await?;
        eprintln!("Extracting...");
        extract_tarball_auto(&tarball_path, &temp_dir)?;
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

    // Check if this exact version is already installed (post-detection check for builds source).
    // Skipped for master: a master build's version string is shared across commits, so an
    // existing dir doesn't mean the content matches — we got here because the etag changed
    // (or there was no record), so overwrite the existing binary in place and adopt the dir
    // as the master install (the record write below re-points the master record at it).
    let version_dir = paths::version_dir(&exact_version)?;
    if version_dir.exists() && !force && !is_master {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(Error::VersionAlreadyInstalled(exact_version));
    }

    // Move to final location
    let replaced_existing = version_dir.exists();
    if replaced_existing {
        std::fs::remove_dir_all(&version_dir)?;
    }
    std::fs::create_dir_all(&version_dir)?;
    std::fs::rename(&binary_path, version_dir.join("clickhouse"))?;

    // Replacing a build on disk never affects already-running servers (they keep
    // executing the old binary) — just say so, so the swap isn't silent.
    if is_master && replaced_existing && version_in_use_by_running_server(&exact_version) {
        eprintln!(
            "Note: running servers keep using the previous {} build until restarted",
            exact_version
        );
    }

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Record the master etag so a later `latest` resolve can skip the download
    // when master is unchanged. Best-effort: a sidecar write failure must not
    // fail an otherwise-successful install.
    if is_master {
        if let Some(head) = &master_head {
            let _ = master::record(platform, head, &exact_version);
        }
    } else {
        // A non-master install just wrote versions/<exact_version>/. If the
        // sidecar recorded that dir as the installed master build, the record
        // is now stale and would make a later `latest` resolve reuse this
        // binary as if it were master. Best-effort, like `record`.
        let _ = master::clear_record_for_version(platform, &exact_version);
    }

    let channel_suffix = match resolved.channel {
        Some(ch) => format!(" ({})", ch),
        None => String::new(),
    };
    eprintln!("Installed ClickHouse {}{}", exact_version, channel_suffix);

    Ok(exact_version)
}

/// Like `install_resolved`, but returns the existing version instead of erroring
/// when already installed. Intended for cases like `server start --version` where
/// the goal is "make sure this version is available" rather than "install this".
pub async fn ensure_installed(resolved: &ResolvedVersion, platform: &Platform) -> Result<String> {
    // If we know the exact version upfront, return it if already installed
    if let Some(ref version) = resolved.exact_version {
        let version_dir = paths::version_dir(version)?;
        if version_dir.exists() {
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
