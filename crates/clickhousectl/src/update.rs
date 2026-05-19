use crate::error::{Error, Result};
use crate::paths;
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs;
use std::io::{self, Cursor};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;

const GITHUB_REPO: &str = "ClickHouse/clickhousectl";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// The asset name suffix for this platform's binary in GitHub Releases.
fn asset_name() -> Result<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "x86_64") => Ok("clickhousectl-x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("clickhousectl-aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("clickhousectl-x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("clickhousectl-aarch64-unknown-linux-musl"),
        _ => Err(Error::UnsupportedPlatform {
            os: os.to_string(),
            arch: arch.to_string(),
        }),
    }
}

/// Parse a version tag like "v0.1.17" into a comparable tuple.
fn parse_version(tag: &str) -> Option<(u32, u32, u32)> {
    let v = tag.strip_prefix('v').unwrap_or(tag);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() == 3 {
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    } else {
        None
    }
}

/// Returns true if `latest` is newer than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Fetch the latest release info from GitHub with configurable timeout.
async fn fetch_latest_release(timeout: std::time::Duration) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .timeout(timeout)
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("GitHub API request failed: {}", e)))?;

    let release: GitHubRelease = response.json().await?;
    Ok(release)
}

/// Extract the `clickhousectl` binary from a `.tar.gz` release archive.
///
/// The release workflow packages the binary at
/// `clickhousectl-<target>-v<version>/clickhousectl` inside the tarball, so we
/// match on the entry's file name rather than the full path.
fn extract_binary_from_archive(archive_bytes: &[u8]) -> Result<Vec<u8>> {
    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| Error::Download(format!("Failed to read release archive: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| Error::Download(format!("Failed to read archive entry: {}", e)))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| Error::Download(format!("Failed to read archive entry path: {}", e)))?;
        if path.file_name().and_then(|n| n.to_str()) == Some("clickhousectl") {
            let mut buf = Vec::new();
            io::copy(&mut entry, &mut buf).map_err(|e| {
                Error::Download(format!("Failed to extract binary from archive: {}", e))
            })?;
            return Ok(buf);
        }
    }

    Err(Error::Download(
        "Release archive did not contain a clickhousectl binary".into(),
    ))
}

/// Timeout for explicit user-initiated commands (update, update --check).
const EXPLICIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// Timeout for the implicit background cache refresh.
const BACKGROUND_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(400);

/// Check for updates. Returns Some((current, latest)) if an update is available.
/// Uses the explicit (longer) timeout since this is called from user-initiated commands.
pub async fn check_for_update() -> Result<Option<(String, String)>> {
    let current = env!("CARGO_PKG_VERSION");
    let release = fetch_latest_release(EXPLICIT_TIMEOUT).await?;
    let latest = &release.tag_name;

    if is_newer(current, latest) {
        let display = latest.strip_prefix('v').unwrap_or(latest);
        Ok(Some((current.to_string(), display.to_string())))
    } else {
        Ok(None)
    }
}

/// Download the latest release and replace the current binary.
pub async fn perform_update() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let release = fetch_latest_release(EXPLICIT_TIMEOUT).await?;
    let latest = &release.tag_name;

    if !is_newer(current, latest) {
        let display = latest.strip_prefix('v').unwrap_or(latest);
        println!("Already up to date (v{}).", display);
        return Ok(());
    }

    let asset_prefix = asset_name()?;
    let expected_asset = format!("{}-{}.tar.gz", asset_prefix, latest);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == expected_asset)
        .ok_or_else(|| {
            Error::Download(format!(
                "No compatible binary found for this platform (expected {})",
                expected_asset
            ))
        })?;

    let display = latest.strip_prefix('v').unwrap_or(latest);
    println!("Downloading clickhousectl v{}...", display);

    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("Download failed: {}", e)))?;

    let archive_bytes = response.bytes().await?;
    let binary_bytes = extract_binary_from_archive(&archive_bytes)?;

    // Get the path to the currently running binary
    let current_exe = std::env::current_exe().map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("Could not determine current executable path: {}", e),
        ))
    })?;

    // Resolve symlinks to get the actual binary path
    let actual_path = fs::canonicalize(&current_exe).unwrap_or(current_exe);

    // Write to a temporary file next to the binary, then atomic-rename
    let tmp_path = actual_path.with_extension("tmp-update");
    fs::write(&tmp_path, &binary_bytes).map_err(|e| {
        Error::Download(format!(
            "Failed to write update to {}: {}. Check file permissions.",
            tmp_path.display(),
            e
        ))
    })?;

    // Make it executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755)).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            Error::Download(format!("Failed to set executable permissions: {}", e))
        })?;
    }

    // Atomic rename
    fs::rename(&tmp_path, &actual_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        Error::Download(format!(
            "Failed to replace binary at {}: {}. Check file permissions.",
            actual_path.display(),
            e
        ))
    })?;

    println!("Updated clickhousectl: v{} → v{}", current, display);
    // Save the check cache so we don't nag right after updating
    let _ = save_update_check(display);
    Ok(())
}

// --- Background update check with caching ---

fn update_check_path() -> Result<PathBuf> {
    Ok(paths::base_dir()?.join("last_update_check"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Save the update check result (timestamp + latest version).
fn save_update_check(latest_version: &str) -> Result<()> {
    let path = update_check_path()?;
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = format!("{}\n{}", now_secs(), latest_version);
    fs::write(&path, content)?;
    Ok(())
}

/// Read the cached update check. Returns Some((timestamp, latest_version)) if valid.
fn read_update_check() -> Option<(u64, String)> {
    let path = update_check_path().ok()?;
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let ts: u64 = lines.next()?.parse().ok()?;
    let version = lines.next()?.to_string();
    Some((ts, version))
}

/// Print an update notice from cached data only. No network, no async.
/// Called synchronously before the command runs so output never interleaves.
pub fn print_cached_update_notice() {
    if let Some((_, cached_version)) = read_update_check() {
        let current = env!("CARGO_PKG_VERSION");
        if is_newer(current, &cached_version) {
            eprintln!(
                "\nA new version of clickhousectl is available: v{} (current: v{})",
                cached_version, current
            );
            eprintln!("Run `clickhousectl update` to upgrade.\n");
        }
    }
}

/// Refresh the update cache in the background if stale. Never prints.
/// On any failure (timeout, network error, etc.), writes the current version
/// to the cache so we don't retry for another 24 hours.
pub async fn refresh_update_cache() {
    // Only hit the network if cache is stale or missing
    let needs_refresh = match read_update_check() {
        Some((ts, _)) => now_secs().saturating_sub(ts) >= CHECK_INTERVAL_SECS,
        None => true,
    };
    if !needs_refresh {
        return;
    }

    let current = env!("CARGO_PKG_VERSION");
    let release = fetch_latest_release(BACKGROUND_TIMEOUT).await;
    match release {
        Ok(r) => {
            let latest = r.tag_name;
            let display = latest.strip_prefix('v').unwrap_or(&latest);
            let _ = save_update_check(display);
        }
        Err(_) => {
            // Failed or timed out — write current version so we back off for 24h
            let _ = save_update_check(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v0.1.17"), Some((0, 1, 17)));
        assert_eq!(parse_version("0.1.17"), Some((0, 1, 17)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("v1.2"), None);
        assert_eq!(parse_version("garbage"), None);
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.1.17", "v0.2.0"));
        assert!(is_newer("0.1.17", "0.1.18"));
        assert!(is_newer("0.1.17", "1.0.0"));
        assert!(!is_newer("0.1.17", "0.1.17"));
        assert!(!is_newer("0.1.17", "0.1.16"));
        assert!(!is_newer("0.2.0", "0.1.99"));
    }

    #[test]
    fn test_asset_name() {
        // Should return something valid on macOS/Linux test hosts
        let name = asset_name().unwrap();
        assert!(name.starts_with("clickhousectl-"));
    }

    fn build_release_archive(inner_dir: &str, binary_bytes: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::Builder;

        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = Builder::new(encoder);

        let mut header = tar::Header::new_gnu();
        header
            .set_path(format!("{}/clickhousectl", inner_dir))
            .unwrap();
        header.set_size(binary_bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, binary_bytes).unwrap();

        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn extracts_clickhousectl_binary_from_release_archive() {
        let payload = b"\x7fELF fake binary contents".as_slice();
        let archive =
            build_release_archive("clickhousectl-aarch64-apple-darwin-v0.0.1", payload);

        let extracted = extract_binary_from_archive(&archive).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn extract_fails_when_archive_has_no_clickhousectl_entry() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::Builder;
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let builder = Builder::new(encoder);
        let empty = builder.into_inner().unwrap().finish().unwrap();

        let err = extract_binary_from_archive(&empty).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("did not contain"), "got: {}", msg);
    }
}
