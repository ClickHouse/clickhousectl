use crate::error::{Error, Result};
use crate::paths;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Fetch the latest release info from GitHub.
async fn fetch_latest_release() -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let client = reqwest::Client::builder()
        .user_agent(crate::user_agent::user_agent())
        .timeout(std::time::Duration::from_secs(10))
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

/// Check for updates. Returns Some((current, latest)) if an update is available.
pub async fn check_for_update() -> Result<Option<(String, String)>> {
    let current = env!("CARGO_PKG_VERSION");
    let release = fetch_latest_release().await?;
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
    let release = fetch_latest_release().await?;
    let latest = &release.tag_name;

    if !is_newer(current, latest) {
        let display = latest.strip_prefix('v').unwrap_or(latest);
        println!("Already up to date (v{}).", display);
        return Ok(());
    }

    let expected_asset = asset_name()?;
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

    let bytes = response.bytes().await?;

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
    fs::write(&tmp_path, &bytes).map_err(|e| {
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

/// Refresh the update cache in the background if stale. Fire-and-forget —
/// never prints, silently ignores errors. The *next* invocation will see
/// the refreshed cache and print the notice synchronously.
pub async fn refresh_update_cache() {
    // Only hit the network if cache is stale or missing
    let needs_refresh = match read_update_check() {
        Some((ts, _)) => now_secs().saturating_sub(ts) >= CHECK_INTERVAL_SECS,
        None => true,
    };
    if !needs_refresh {
        return;
    }

    let Ok(result) = check_for_update().await else {
        return;
    };
    match result {
        Some((_current, latest)) => {
            let _ = save_update_check(&latest);
        }
        None => {
            let current = env!("CARGO_PKG_VERSION");
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
}
