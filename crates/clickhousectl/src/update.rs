use crate::error::{Error, Result};
use crate::paths;
use flate2::read::GzDecoder;
use serde::Serialize;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;

const BUILDS_BASE_URL: &str = "https://builds.clickhouse.com/clickhousectl";
/// Holds the latest release tag (`vX.Y.Z`). The release workflow's
/// `publish-latest` job writes it after each release.
const LATEST_MANIFEST_URL: &str = "https://builds.clickhouse.com/clickhousectl/latest.txt";
/// Fallback when `latest.txt` is unavailable: GitHub's web redirect to the
/// latest release, which is not subject to the REST API's 60 requests/hour
/// unauthenticated limit.
const LATEST_RELEASE_PAGE_URL: &str = "https://github.com/ClickHouse/clickhousectl/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum UpdateAction {
    UpToDate,
    UpdateAvailable,
    Updated,
}

#[derive(Serialize)]
pub struct UpdateResult {
    current_version: String,
    latest_version: String,
    action: UpdateAction,
}

impl UpdateResult {
    fn checked(current: &str, latest: &str) -> Self {
        Self {
            current_version: current.to_owned(),
            latest_version: latest.strip_prefix('v').unwrap_or(latest).to_owned(),
            action: if is_newer(current, latest) {
                UpdateAction::UpdateAvailable
            } else {
                UpdateAction::UpToDate
            },
        }
    }

    pub fn write(&self, output: &mut dyn Write, json: bool) -> Result<()> {
        if json {
            writeln!(output, "{}", serde_json::to_string_pretty(self)?)?;
        } else {
            match self.action {
                UpdateAction::UpToDate => {
                    writeln!(output, "Already up to date (v{}).", self.current_version)?;
                }
                UpdateAction::UpdateAvailable => {
                    writeln!(
                        output,
                        "Update available: v{} → v{}",
                        self.current_version, self.latest_version
                    )?;
                    writeln!(output, "Run `clickhousectl update` to upgrade.")?;
                }
                UpdateAction::Updated => {
                    writeln!(
                        output,
                        "Updated clickhousectl: v{} → v{}",
                        self.current_version, self.latest_version
                    )?;
                }
            }
        }
        Ok(())
    }
}

/// The platform target triple used in release artifact names.
fn target_triple() -> Result<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
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

/// Parse a release tag such as `v0.5.0`, as written to `latest.txt`.
fn parse_release_tag(text: &str) -> Option<String> {
    let tag = text.trim();
    (tag.starts_with('v') && parse_version(tag).is_some()).then(|| tag.to_owned())
}

/// Take the tag from the `Location` of GitHub's `releases/latest` redirect,
/// e.g. `https://github.com/ClickHouse/clickhousectl/releases/tag/v0.5.0`.
fn tag_from_release_location(location: &str) -> Option<String> {
    let (_, tag) = location
        .trim_end_matches('/')
        .rsplit_once("/releases/tag/")?;
    parse_release_tag(tag)
}

/// Read `latest.txt`. builds.clickhouse.com caches every path for days, so a
/// unique query string makes the CDN fetch it from the bucket.
async fn fetch_manifest_tag(
    client: &reqwest::Client,
    url: &str,
) -> std::result::Result<String, String> {
    let url = format!("{}?t={}", url, now_secs());
    let response = client
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let body = response.text().await.map_err(|e| e.to_string())?;
    parse_release_tag(&body).ok_or_else(|| format!("{} did not contain a release tag", url))
}

/// Read the tag from the `releases/latest` redirect without following it.
async fn fetch_release_redirect_tag(
    client: &reqwest::Client,
    url: &str,
) -> std::result::Result<String, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|location| location.to_str().ok())
        .filter(|_| response.status().is_redirection())
        .and_then(tag_from_release_location)
        .ok_or_else(|| {
            format!(
                "{} returned {} without a release tag redirect",
                url,
                response.status()
            )
        })
}

/// Resolve the latest release tag from `latest.txt`, falling back to the
/// GitHub release redirect when the manifest is missing or invalid.
async fn resolve_latest_tag(
    client: &reqwest::Client,
    manifest_url: &str,
    release_page_url: &str,
) -> Result<String> {
    let manifest_err = match fetch_manifest_tag(client, manifest_url).await {
        Ok(tag) => return Ok(tag),
        Err(e) => e,
    };
    fetch_release_redirect_tag(client, release_page_url)
        .await
        .map_err(|redirect_err| {
            Error::Download(format!(
                "Could not determine the latest clickhousectl version: {}; {}",
                manifest_err, redirect_err
            ))
        })
}

/// Fetch the latest release tag, bounding the whole lookup (manifest plus any
/// fallback) by `timeout`.
async fn fetch_latest_tag(timeout: std::time::Duration) -> Result<String> {
    let client = crate::http::client_builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    tokio::time::timeout(
        timeout,
        resolve_latest_tag(&client, LATEST_MANIFEST_URL, LATEST_RELEASE_PAGE_URL),
    )
    .await
    .map_err(|_| Error::Download("Timed out determining the latest clickhousectl version".into()))?
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
        .map_err(|e| Error::Extract(format!("Failed to read release archive: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| Error::Extract(format!("Failed to read archive entry: {}", e)))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| Error::Extract(format!("Failed to read archive entry path: {}", e)))?;
        if path.file_name().and_then(|n| n.to_str()) == Some("clickhousectl") {
            let mut buf = Vec::new();
            io::copy(&mut entry, &mut buf).map_err(|e| {
                Error::Extract(format!("Failed to extract binary from archive: {}", e))
            })?;
            return Ok(buf);
        }
    }

    Err(Error::Extract(
        "Release archive did not contain a clickhousectl binary".into(),
    ))
}

/// Timeout for explicit user-initiated commands (update, update --check).
const EXPLICIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// Timeout for the implicit background cache refresh.
const BACKGROUND_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(400);

/// Check for updates and retain both versions even when no update is available.
/// Uses the explicit (longer) timeout since this is called from user-initiated commands.
pub async fn check_for_update() -> Result<UpdateResult> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = &fetch_latest_tag(EXPLICIT_TIMEOUT).await?;
    let display = latest.strip_prefix('v').unwrap_or(latest);

    // An explicit check always refreshes the cache and resets the staleness
    // timer, so subsequent commands reflect what we just learned.
    let _ = save_update_check(display);

    Ok(UpdateResult::checked(current, latest))
}

/// Download the latest release and replace the current binary.
pub async fn perform_update(json: bool) -> Result<UpdateResult> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = &fetch_latest_tag(EXPLICIT_TIMEOUT).await?;
    let mut result = UpdateResult::checked(current, latest);

    if matches!(result.action, UpdateAction::UpToDate) {
        let display = latest.strip_prefix('v').unwrap_or(latest);
        // Refresh the cache with the network truth so a stale "update available"
        // notice can't keep nagging after the user explicitly checked.
        let _ = save_update_check(display);
        return Ok(result);
    }

    let target = target_triple()?;
    let archive_name = format!("clickhousectl-{}-{}.tar.gz", target, latest);
    let download_url = format!("{}/{}", BUILDS_BASE_URL, archive_name);

    let display = latest.strip_prefix('v').unwrap_or(latest);
    if !json {
        println!("Downloading clickhousectl v{}...", display);
    }

    let client = crate::http::client_builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client
        .get(&download_url)
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

    // Clear the check cache so the update notice disappears immediately.
    let _ = clear_update_check();
    result.action = UpdateAction::Updated;
    Ok(result)
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

/// Remove the cached update check. Used after a successful self-update so the
/// notice disappears immediately. Missing file is not an error.
fn clear_update_check() -> Result<()> {
    let path = update_check_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Io(e)),
    }
}

/// Whether the cache is stale enough to warrant a network refresh. A missing
/// cache is always stale; a present one is stale once it is older than the
/// check interval.
fn cache_is_stale(cache: Option<(u64, String)>, now: u64) -> bool {
    match cache {
        Some((ts, _)) => now.saturating_sub(ts) >= CHECK_INTERVAL_SECS,
        None => true,
    }
}

/// Print an update notice from cached data only. No network, no async.
/// Called synchronously before the command runs so output never interleaves.
pub fn print_cached_update_notice() {
    if let Some((_, cached_version)) = read_update_check() {
        let current = env!("CARGO_PKG_VERSION");
        if is_newer(current, &cached_version) {
            use std::io::Write;
            // Not `eprintln!`, which panics on a closed stderr — see
            // `telemetry::print_first_run_notice`.
            let _ = writeln!(
                std::io::stderr(),
                "\nThere is a new version of clickhousectl. Update with `clickhousectl update`."
            );
        }
    }
}

/// Hit the network, refresh the cache, and reset the staleness timer. Never
/// prints. On any failure (timeout, network error, etc.) the timestamp is still
/// reset so we back off for another 24h, but a previously-cached "update
/// available" version is preserved so we don't hide a known update.
async fn do_refresh_update_cache(timeout: std::time::Duration) {
    let current = env!("CARGO_PKG_VERSION");
    match fetch_latest_tag(timeout).await {
        Ok(latest) => {
            let display = latest.strip_prefix('v').unwrap_or(&latest);
            let _ = save_update_check(display);
        }
        Err(_) => {
            // Preserve any previously-cached latest version; fall back to the
            // current version when there is nothing cached yet.
            let version = read_update_check()
                .map(|(_, v)| v)
                .unwrap_or_else(|| current.to_string());
            let _ = save_update_check(&version);
        }
    }
}

/// Refresh the update cache in the background if stale. Never prints. Skips the
/// network entirely when the cache is still fresh (within 24h).
pub async fn refresh_update_cache() {
    if !cache_is_stale(read_update_check(), now_secs()) {
        return;
    }
    do_refresh_update_cache(BACKGROUND_TIMEOUT).await;
}

/// Force a network check and refresh the cache + timer regardless of staleness.
/// Used by explicit user actions (e.g. `--version`) that should always reflect
/// the freshest state. Uses the longer explicit timeout. Never prints.
pub async fn force_refresh_update_cache() {
    do_refresh_update_cache(EXPLICIT_TIMEOUT).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_update_check_preserves_the_current_and_actual_latest_versions() {
        for (latest, action) in [
            ("v0.5.0", "update_available"),
            ("v0.4.2", "up_to_date"),
            ("v0.4.1", "up_to_date"),
        ] {
            let result = UpdateResult::checked("0.4.2", latest);
            let mut output = Vec::new();
            result.write(&mut output, true).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(value["current_version"], "0.4.2");
            assert_eq!(value["latest_version"], latest.trim_start_matches('v'));
            assert_eq!(value["action"], action);
        }
    }

    #[test]
    fn json_update_completion_reports_the_upgrade() {
        let mut result = UpdateResult::checked("0.4.2", "v0.5.0");
        result.action = UpdateAction::Updated;
        let mut output = Vec::new();
        result.write(&mut output, true).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["action"], "updated");
        assert_eq!(value["current_version"], "0.4.2");
        assert_eq!(value["latest_version"], "0.5.0");
    }

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
    fn test_cache_is_stale() {
        let now = 1_000_000;
        // Missing cache is always stale.
        assert!(cache_is_stale(None, now));
        // Fresh cache (just written) is not stale.
        assert!(!cache_is_stale(Some((now, "0.2.0".into())), now));
        // Cache one second short of the interval is not stale.
        assert!(!cache_is_stale(
            Some((now - (CHECK_INTERVAL_SECS - 1), "0.2.0".into())),
            now
        ));
        // Cache exactly at the interval is stale.
        assert!(cache_is_stale(
            Some((now - CHECK_INTERVAL_SECS, "0.2.0".into())),
            now
        ));
        // Older cache is stale.
        assert!(cache_is_stale(
            Some((now - 2 * CHECK_INTERVAL_SECS, "0.2.0".into())),
            now
        ));
    }

    #[test]
    fn test_target_triple() {
        // Should return something valid on macOS/Linux test hosts
        let target = target_triple().unwrap();
        assert!(target.contains('-'));
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
        let archive = build_release_archive("clickhousectl-aarch64-apple-darwin-v0.0.1", payload);

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

    #[test]
    fn release_tags_need_a_v_prefix_and_three_numeric_parts() {
        assert_eq!(parse_release_tag("v0.5.0\n").as_deref(), Some("v0.5.0"));
        assert_eq!(parse_release_tag("  v1.2.3  ").as_deref(), Some("v1.2.3"));
        for bad in [
            "",
            "0.5.0",
            "v0.5",
            "v0.5.0-rc1",
            "<html>",
            "v0.5.0\nv0.6.0",
        ] {
            assert_eq!(parse_release_tag(bad), None, "{:?}", bad);
        }
    }

    #[test]
    fn release_location_yields_its_tag() {
        assert_eq!(
            tag_from_release_location(
                "https://github.com/ClickHouse/clickhousectl/releases/tag/v0.5.0"
            )
            .as_deref(),
            Some("v0.5.0")
        );
        // No release yet: GitHub redirects to the releases list.
        assert_eq!(
            tag_from_release_location("https://github.com/ClickHouse/clickhousectl/releases"),
            None
        );
    }

    mod resolve {
        use super::super::*;
        use wiremock::matchers::{method, path, query_param_is_missing};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        fn client() -> reqwest::Client {
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap()
        }

        async fn resolve(server: &MockServer) -> Result<String> {
            resolve_latest_tag(
                &client(),
                &format!("{}/latest.txt", server.uri()),
                &format!("{}/releases/latest", server.uri()),
            )
            .await
        }

        async fn mount_redirect(server: &MockServer, location: &str) {
            Mock::given(method("GET"))
                .and(path("/releases/latest"))
                .respond_with(ResponseTemplate::new(302).insert_header("location", location))
                .mount(server)
                .await;
        }

        #[tokio::test]
        async fn manifest_is_read_with_a_cache_busting_query() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/latest.txt"))
                .and(query_param_is_missing("t"))
                .respond_with(ResponseTemplate::new(200).set_body_string("v0.0.1\n"))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/latest.txt"))
                .respond_with(ResponseTemplate::new(200).set_body_string("v9.8.7\n"))
                .mount(&server)
                .await;
            mount_redirect(&server, "/releases/tag/v0.0.2").await;

            assert_eq!(resolve(&server).await.unwrap(), "v9.8.7");
        }

        #[tokio::test]
        async fn missing_or_invalid_manifest_falls_back_to_the_release_redirect() {
            for manifest in [
                ResponseTemplate::new(403),
                ResponseTemplate::new(200).set_body_string("<html>"),
            ] {
                let server = MockServer::start().await;
                Mock::given(method("GET"))
                    .and(path("/latest.txt"))
                    .respond_with(manifest)
                    .mount(&server)
                    .await;
                mount_redirect(
                    &server,
                    "https://github.com/ClickHouse/clickhousectl/releases/tag/v0.6.1",
                )
                .await;

                assert_eq!(resolve(&server).await.unwrap(), "v0.6.1");
            }
        }

        #[tokio::test]
        async fn both_sources_failing_reports_each_failure() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/latest.txt"))
                .respond_with(ResponseTemplate::new(403))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/releases/latest"))
                .respond_with(ResponseTemplate::new(429))
                .mount(&server)
                .await;

            let msg = resolve(&server).await.unwrap_err().to_string();
            assert!(msg.contains("latest clickhousectl version"), "got: {}", msg);
            assert!(msg.contains("403"), "got: {}", msg);
            assert!(msg.contains("429"), "got: {}", msg);
        }
    }
}
