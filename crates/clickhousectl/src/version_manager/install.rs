use crate::error::{Error, Result};
use crate::paths;
use crate::version_manager::download::download_from_source;
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
use tar::Archive;

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

    // The floating `latest`/master build has no version upfront and a stable
    // URL whose content moves. Use the HTTP etag to skip the ~153MB download
    // when master hasn't changed since the installed build.
    let is_master = matches!(
        resolved.source,
        DownloadSource::Builds { ref version_path } if version_path == "master"
    );
    let mut master_head = None;
    if is_master {
        match master::head_info(platform).await {
            Ok(head) => master_head = head,
            Err(error) => eprintln!("Master freshness check skipped: {error}"),
        }
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
    paths::ensure_dir(&temp_dir)?;

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
    paths::ensure_dir(&version_dir)?;
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
    use flate2::{Compression, write::GzEncoder};
    use std::fs;
    use tar::{Builder, EntryType, Header};

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

        assert_eq!(
            error.to_string(),
            format!(
                "Extraction failed: Archive '{}' does not contain a ClickHouse binary at 'clickhouse' or '*/usr/bin/clickhouse'",
                archive_path.display()
            )
        );
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
        assert!(message.contains("Not a directory"), "{message}");
    }

    #[test]
    fn refuses_link_for_clickhouse_binary() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("linked.tgz");
        write_symlink_archive(&archive_path, "package/usr/bin/clickhouse");

        let error = extract_tarball_auto(&archive_path, temp.path()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("is not a regular file; symbolic and hard links are not followed")
        );
        assert!(!temp.path().join("clickhouse").exists());
    }

    #[test]
    fn refuses_archive_path_traversal() {
        let archive_path = Path::new("/tmp/clickhouse.tgz");
        let error =
            validate_archive_entry_path(archive_path, Path::new("../package/usr/bin/clickhouse"))
                .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Extraction failed: Archive '/tmp/clickhouse.tgz' contains unsafe entry path '../package/usr/bin/clickhouse'"
        );
    }
}
