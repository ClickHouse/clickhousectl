use crate::error::{Error, Result};
use crate::failure::FailureKind;
use crate::paths;
use flate2::read::GzDecoder;
use serde::Serialize;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
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
/// Where each release's `SHA256SUMS` is read: the GitHub release, not
/// builds.clickhouse.com, so the server that serves an archive cannot also
/// vouch for it (#1307).
const RELEASE_DOWNLOAD_BASE_URL: &str =
    "https://github.com/ClickHouse/clickhousectl/releases/download";
const CHECKSUMS_FILE: &str = "SHA256SUMS";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours

/// Set to `npm` by the npm package's launcher (`npm/bin/cli.js`) on the
/// binary it spawns, so `update` knows npm owns this install (#1305).
const INSTALL_CHANNEL_ENV: &str = "CLICKHOUSECTL_INSTALL_CHANNEL";

/// How this binary was installed, which decides whether `update` may replace
/// it. Only the tarball channels (install script, manual download,
/// cargo-binstall, `cargo install`) are self-updatable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallChannel {
    Standalone,
    Npm,
    /// The PyPI wheel, installed by pip, uv or pipx.
    Pip(PipTool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipTool {
    Pip,
    Uv,
    Pipx,
}

impl InstallChannel {
    /// The channel of the running binary: wheels are built with the
    /// `channel-pip` feature (`pypi/pyproject.toml`); npm marks the
    /// environment of the binary it spawns.
    fn current() -> Self {
        let exe = std::env::current_exe()
            .ok()
            .map(|exe| fs::canonicalize(&exe).unwrap_or(exe));
        Self::detect(
            cfg!(feature = "channel-pip"),
            std::env::var_os(INSTALL_CHANNEL_ENV).as_deref(),
            exe.as_deref(),
        )
    }

    fn detect(pip_build: bool, marker: Option<&std::ffi::OsStr>, exe: Option<&Path>) -> Self {
        if pip_build {
            return Self::Pip(PipTool::from_exe(exe));
        }
        match marker.and_then(|marker| marker.to_str()) {
            Some("npm") => Self::Npm,
            _ => Self::Standalone,
        }
    }

    /// The package manager that owns the install, if any.
    fn manager(self) -> Option<&'static str> {
        match self {
            Self::Standalone => None,
            Self::Npm => Some("npm"),
            Self::Pip(PipTool::Pip) => Some("pip"),
            Self::Pip(PipTool::Uv) => Some("uv"),
            Self::Pip(PipTool::Pipx) => Some("pipx"),
        }
    }

    /// The command that upgrades this install.
    fn upgrade_command(self) -> &'static str {
        match self {
            Self::Standalone => "clickhousectl update",
            Self::Npm => "npm install -g clickhousectl@latest",
            Self::Pip(PipTool::Pip) => "pip install --upgrade clickhousectl",
            Self::Pip(PipTool::Uv) => "uv tool upgrade clickhousectl",
            Self::Pip(PipTool::Pipx) => "pipx upgrade clickhousectl",
        }
    }

    /// `Ok` when `update` may replace the binary, else the refusal that names
    /// the package manager's upgrade command.
    fn ensure_self_updatable(self) -> Result<()> {
        match self.manager() {
            None => Ok(()),
            Some(manager) => Err(Error::PackageManagedInstall {
                manager,
                command: self.upgrade_command(),
            }),
        }
    }
}

impl PipTool {
    /// uv and pipx keep each tool in its own environment under a directory
    /// that names them; anything else was installed by pip.
    fn from_exe(exe: Option<&Path>) -> Self {
        let Some(exe) = exe else {
            return Self::Pip;
        };
        let components: Vec<_> = exe
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect();
        let under = |parent: &str, child: &str| {
            components
                .windows(2)
                .any(|pair| pair[0] == parent && pair[1] == child)
        };
        if under("uv", "tools") {
            Self::Uv
        } else if under("pipx", "venvs") {
            Self::Pipx
        } else {
            Self::Pip
        }
    }
}

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
    #[serde(skip)]
    upgrade_command: &'static str,
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
            upgrade_command: InstallChannel::Standalone.upgrade_command(),
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
                    writeln!(output, "Run `{}` to upgrade.", self.upgrade_command)?;
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

/// Why one source of the latest release tag failed: the text for the human
/// message, and its class for the `--json` code (#1312).
#[derive(Debug)]
struct TagSourceError {
    message: String,
    failure: FailureKind,
}

impl TagSourceError {
    /// A response that arrived but held no release tag.
    fn no_tag(message: String) -> Self {
        Self {
            message,
            failure: FailureKind::Other,
        }
    }

    /// A response whose status says why it held no release tag.
    fn status(message: String, status: reqwest::StatusCode) -> Self {
        let failure = if status.is_client_error() || status.is_server_error() {
            crate::failure::status_kind(status.as_u16())
        } else {
            FailureKind::Other
        };
        Self { message, failure }
    }
}

impl From<reqwest::Error> for TagSourceError {
    /// Classified from the error's own kind, never its text.
    fn from(error: reqwest::Error) -> Self {
        let failure = if error.is_timeout() {
            FailureKind::Timeout
        } else if let Some(status) = error.status() {
            crate::failure::status_kind(status.as_u16())
        } else {
            FailureKind::Transport
        };
        Self {
            message: error.to_string(),
            failure,
        }
    }
}

/// Read `latest.txt`. builds.clickhouse.com caches every path for days, so a
/// unique query string makes the CDN fetch it from the bucket.
async fn fetch_manifest_tag(
    client: &reqwest::Client,
    url: &str,
) -> std::result::Result<String, TagSourceError> {
    let url = format!("{}?t={}", url, now_secs());
    let response = client
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())?;
    let body = response.text().await?;
    parse_release_tag(&body)
        .ok_or_else(|| TagSourceError::no_tag(format!("{} did not contain a release tag", url)))
}

/// Read the tag from the `releases/latest` redirect without following it.
async fn fetch_release_redirect_tag(
    client: &reqwest::Client,
    url: &str,
) -> std::result::Result<String, TagSourceError> {
    let response = client.get(url).send().await?;
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|location| location.to_str().ok())
        .filter(|_| response.status().is_redirection())
        .and_then(tag_from_release_location)
        .ok_or_else(|| {
            TagSourceError::status(
                format!(
                    "{} returned {} without a release tag redirect",
                    url,
                    response.status()
                ),
                response.status(),
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
        .map_err(|redirect_err| Error::UpdateCheck {
            message: format!(
                "Could not determine the latest clickhousectl version: {}; {}",
                manifest_err.message, redirect_err.message
            ),
            // The fallback ran last; the manifest's class stands in only when
            // the fallback's failure has none.
            failure: match redirect_err.failure {
                FailureKind::Other => manifest_err.failure,
                failure => failure,
            },
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
    .map_err(|_| Error::UpdateCheck {
        message: "Timed out determining the latest clickhousectl version".into(),
        failure: FailureKind::Timeout,
    })?
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

    let mut result = UpdateResult::checked(current, latest);
    result.upgrade_command = InstallChannel::current().upgrade_command();
    Ok(result)
}

/// Download the latest release and replace the current binary. A binary a
/// package manager installed is refused before any network access, since
/// replacing it would leave the manager's records wrong (#1305).
pub async fn perform_update(json: bool) -> Result<UpdateResult> {
    InstallChannel::current().ensure_self_updatable()?;

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

    let sums_url = format!(
        "{}/{}/{}",
        RELEASE_DOWNLOAD_BASE_URL, latest, CHECKSUMS_FILE
    );
    let client = crate::http::client_builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let archive_bytes =
        download_verified_archive(&client, &download_url, &sums_url, &archive_name).await?;
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
    replace_binary(&actual_path, &binary_bytes, PROBE_TIMEOUT).await?;

    // Clear the check cache so the update notice disappears immediately.
    let _ = clear_update_check();
    result.action = UpdateAction::Updated;
    Ok(result)
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Download(format!("Download failed: {}", e)))?;
    Ok(response.bytes().await?.to_vec())
}

/// Download the release archive and check it against the release's
/// `SHA256SUMS`. Missing sums, a missing entry or a different digest is an
/// error, so nothing unverified reaches the binary swap (#1307).
async fn download_verified_archive(
    client: &reqwest::Client,
    archive_url: &str,
    sums_url: &str,
    archive_name: &str,
) -> Result<Vec<u8>> {
    let sums = download(client, sums_url).await?;
    let expected =
        expected_sha256(&String::from_utf8_lossy(&sums), archive_name).ok_or_else(|| {
            Error::ArchiveVerification(format!("{} has no SHA-256 for {}", sums_url, archive_name))
        })?;
    let archive = download(client, archive_url).await?;
    let actual = sha256_hex(&archive);
    if actual != expected {
        return Err(Error::ArchiveVerification(format!(
            "{} does not match {}: expected SHA-256 {}, got {}",
            archive_name, sums_url, expected, actual
        )));
    }
    Ok(archive)
}

/// The lowercase hex SHA-256 that `sums`, in `sha256sum` output form
/// (`<digest>  <name>` or `<digest> *<name>` per line), lists for `file`.
fn expected_sha256(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (digest, name) = line.trim_end_matches('\r').split_once(' ')?;
        let name = name.strip_prefix([' ', '*'])?;
        (name == file && digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| digest.to_ascii_lowercase())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// How long the downloaded binary has to answer `--version`.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Replace `target` with `binary`: write it to a uniquely named file in the
/// same directory, check that it runs, then rename it over `target`. A failure
/// at any step leaves `target` untouched and removes the new file.
async fn replace_binary(
    target: &Path,
    binary: &[u8],
    probe_timeout: std::time::Duration,
) -> Result<()> {
    let dir = target.parent().ok_or_else(|| {
        Error::Download(format!(
            "Cannot replace {}: it has no parent directory",
            target.display()
        ))
    })?;
    let write_error = |e: io::Error| {
        Error::Download(format!(
            "Failed to write update to {}: {}. Check file permissions.",
            dir.display(),
            e
        ))
    };
    let mut file = tempfile::Builder::new()
        .prefix(".clickhousectl-update-")
        .tempfile_in(dir)
        .map_err(write_error)?;
    file.write_all(binary).map_err(write_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|e| Error::Download(format!("Failed to set executable permissions: {}", e)))?;
    }
    // Close the file before running it: Linux refuses to execute a file that
    // is open for writing. The path is still removed on drop.
    let candidate = file.into_temp_path();

    probe_binary(&candidate, probe_timeout).await?;

    candidate.persist(target).map_err(|e| {
        Error::Download(format!(
            "Failed to replace binary at {}: {}. Check file permissions.",
            target.display(),
            e.error
        ))
    })?;
    Ok(())
}

/// Run `<binary> --version` and require it to succeed, so a download that
/// cannot run on this machine never replaces a working binary. The probe makes
/// no network request and sends no telemetry.
async fn probe_binary(binary: &Path, timeout: std::time::Duration) -> Result<()> {
    let failed = |reason: String| {
        Error::Download(format!(
            "The downloaded clickhousectl did not run ({reason}); the installed binary was left unchanged"
        ))
    };
    let mut command = tokio::process::Command::new(binary);
    command
        .arg("--version")
        .env(crate::timing::UPDATE_CHECK_TIMEOUT_ENV, "0")
        .env("DO_NOT_TRACK", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = spawn_retrying_busy(&mut command).map_err(|e| failed(e.to_string()))?;
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(failed(format!("`--version` exited with {status}"))),
        Ok(Err(e)) => Err(failed(e.to_string())),
        Err(_) => Err(failed(format!(
            "`--version` did not finish within {}s",
            timeout.as_secs()
        ))),
    }
}

/// Spawn `command`, retrying briefly while the file is busy: another thread's
/// fork can still hold the just-closed write handle open (`ETXTBSY`).
fn spawn_retrying_busy(command: &mut tokio::process::Command) -> io::Result<tokio::process::Child> {
    let mut attempts = 0;
    loop {
        match command.spawn() {
            #[cfg(unix)]
            Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && attempts < 20 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            result => return result,
        }
    }
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
            eprintln!(
                "\nThere is a new version of clickhousectl. Update with `{}`.",
                InstallChannel::current().upgrade_command()
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
    let timeout = crate::timing::duration_override(
        crate::timing::UPDATE_CHECK_TIMEOUT_ENV,
        BACKGROUND_TIMEOUT,
    );
    // No time allowed means no request: subprocess tests turn the background
    // check off. The `update` command's explicit check is unaffected.
    if timeout.is_zero() {
        return;
    }
    do_refresh_update_cache(timeout).await;
}

/// Force a network check and refresh the cache + timer regardless of staleness.
/// Used by `--version`, which should always reflect the freshest state. Uses the
/// longer explicit timeout, which the update-check override also bounds so
/// subprocess tests can turn the request off with `0`. Never prints.
pub async fn force_refresh_update_cache() {
    let timeout =
        crate::timing::duration_override(crate::timing::UPDATE_CHECK_TIMEOUT_ENV, EXPLICIT_TIMEOUT);
    if timeout.is_zero() {
        return;
    }
    do_refresh_update_cache(timeout).await;
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
    fn install_channel_comes_from_the_wheel_feature_or_the_npm_marker() {
        use std::ffi::OsStr;
        let plain = Path::new("/home/u/.local/bin/clickhousectl");
        let detect = InstallChannel::detect;
        assert_eq!(detect(false, None, Some(plain)), InstallChannel::Standalone);
        assert_eq!(
            detect(false, Some(OsStr::new("npm")), Some(plain)),
            InstallChannel::Npm
        );
        // Only the exact marker counts.
        for marker in ["", "NPM", "pip", "tarball"] {
            assert_eq!(
                detect(false, Some(OsStr::new(marker)), Some(plain)),
                InstallChannel::Standalone,
                "{marker:?}"
            );
        }
        // A wheel build is pip-owned whatever the environment says; the
        // executable's location tells uv and pipx apart from pip.
        for (exe, tool) in [
            (
                "/usr/lib/python3/site-packages/../bin/clickhousectl",
                PipTool::Pip,
            ),
            (
                "/home/u/.local/share/uv/tools/clickhousectl/bin/clickhousectl",
                PipTool::Uv,
            ),
            (
                "/home/u/.local/share/pipx/venvs/clickhousectl/bin/clickhousectl",
                PipTool::Pipx,
            ),
        ] {
            assert_eq!(
                detect(true, Some(OsStr::new("npm")), Some(Path::new(exe))),
                InstallChannel::Pip(tool),
                "{exe}"
            );
        }
        assert_eq!(detect(true, None, None), InstallChannel::Pip(PipTool::Pip));
    }

    #[test]
    fn package_managed_channels_refuse_self_update_with_their_upgrade_command() {
        assert!(InstallChannel::Standalone.ensure_self_updatable().is_ok());
        for (channel, manager, command) in [
            (
                InstallChannel::Npm,
                "npm",
                "npm install -g clickhousectl@latest",
            ),
            (
                InstallChannel::Pip(PipTool::Pip),
                "pip",
                "pip install --upgrade clickhousectl",
            ),
            (
                InstallChannel::Pip(PipTool::Uv),
                "uv",
                "uv tool upgrade clickhousectl",
            ),
            (
                InstallChannel::Pip(PipTool::Pipx),
                "pipx",
                "pipx upgrade clickhousectl",
            ),
        ] {
            let err = channel.ensure_self_updatable().unwrap_err();
            assert!(
                matches!(
                    &err,
                    Error::PackageManagedInstall { manager: m, command: c }
                        if *m == manager && *c == command
                ),
                "got: {err:?}"
            );
            assert_eq!(err.exit_code(), 1);
            assert_eq!(channel.upgrade_command(), command);
        }
    }

    #[test]
    fn update_available_names_the_channel_upgrade_command() {
        let mut result = UpdateResult::checked("0.4.2", "v0.5.0");
        result.upgrade_command = InstallChannel::Npm.upgrade_command();
        let mut output = Vec::new();
        result.write(&mut output, false).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(
            text.contains("npm install -g clickhousectl@latest"),
            "got: {text}"
        );
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
        // The not-found branch, which names the binary it looked for.
        assert!(matches!(&err, Error::Extract(_)), "got: {err:?}");
        assert!(err.to_string().contains("clickhousectl"), "got: {err}");
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

    #[cfg(unix)]
    mod replace {
        use super::super::*;

        const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

        fn installed(dir: &Path) -> PathBuf {
            let target = dir.join("clickhousectl");
            fs::write(&target, b"old binary").unwrap();
            target
        }

        /// Every file in `dir`, so a leftover temporary file shows up.
        fn entries(dir: &Path) -> Vec<String> {
            let mut names: Vec<_> = fs::read_dir(dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }

        #[tokio::test]
        async fn a_binary_that_runs_replaces_the_target() {
            let dir = tempfile::tempdir().unwrap();
            let target = installed(dir.path());
            let new = b"#!/bin/sh\n[ \"$1\" = --version ] || exit 9\n";

            replace_binary(&target, new, TIMEOUT).await.unwrap();

            assert_eq!(fs::read(&target).unwrap(), new);
            assert_eq!(entries(dir.path()), ["clickhousectl"]);
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }

        #[tokio::test]
        async fn a_binary_that_fails_its_probe_leaves_the_target_alone() {
            let dir = tempfile::tempdir().unwrap();
            let target = installed(dir.path());

            let err = replace_binary(&target, b"#!/bin/sh\nexit 3\n", TIMEOUT)
                .await
                .unwrap_err();

            assert!(matches!(&err, Error::Download(_)), "got: {err:?}");
            assert_eq!(fs::read(&target).unwrap(), b"old binary");
            assert_eq!(entries(dir.path()), ["clickhousectl"]);
        }

        #[tokio::test]
        async fn a_binary_that_hangs_is_abandoned_at_the_timeout() {
            let dir = tempfile::tempdir().unwrap();
            let target = installed(dir.path());

            let err = replace_binary(
                &target,
                b"#!/bin/sh\nexec sleep 30\n",
                std::time::Duration::from_millis(200),
            )
            .await
            .unwrap_err();

            assert!(matches!(&err, Error::Download(_)), "got: {err:?}");
            assert_eq!(fs::read(&target).unwrap(), b"old binary");
            assert_eq!(entries(dir.path()), ["clickhousectl"]);
        }
    }

    #[test]
    fn sha256sums_lines_are_matched_by_exact_file_name() {
        let digest = "ab".repeat(32);
        let sums = format!(
            "{d}  other.tar.gz\n{D} *app.tar.gz\r\nnot-hex  app.tar.gz\n",
            d = "cd".repeat(32),
            D = digest.to_uppercase()
        );
        assert_eq!(expected_sha256(&sums, "app.tar.gz"), Some(digest));
        assert_eq!(expected_sha256(&sums, "app.tar"), None);
        assert_eq!(expected_sha256("ab  app.tar.gz\n", "app.tar.gz"), None);
        // `sha256sum` prints lowercase hex; this is the digest of no bytes.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// `update` checks the archive against the release's `SHA256SUMS` before
    /// anything is extracted or swapped, and fails closed (#1307).
    mod verify {
        use super::super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        const ARCHIVE: &str = "clickhousectl-x86_64-apple-darwin-v9.9.9.tar.gz";

        async fn serve(server: &MockServer, archive: &[u8], sums: Option<String>, hits: u64) {
            Mock::given(method("GET"))
                .and(path(format!("/{ARCHIVE}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(archive.to_vec()))
                .expect(hits)
                .mount(server)
                .await;
            let sums = match sums {
                Some(body) => ResponseTemplate::new(200).set_body_string(body),
                None => ResponseTemplate::new(404),
            };
            Mock::given(method("GET"))
                .and(path("/SHA256SUMS"))
                .respond_with(sums)
                .mount(server)
                .await;
        }

        async fn fetch(server: &MockServer) -> Result<Vec<u8>> {
            download_verified_archive(
                &reqwest::Client::new(),
                &format!("{}/{ARCHIVE}", server.uri()),
                &format!("{}/SHA256SUMS", server.uri()),
                ARCHIVE,
            )
            .await
        }

        #[tokio::test]
        async fn an_archive_matching_its_listed_digest_is_returned() {
            let server = MockServer::start().await;
            let sums = format!("{}  {ARCHIVE}\n", sha256_hex(b"release"));
            serve(&server, b"release", Some(sums), 1).await;
            assert_eq!(fetch(&server).await.unwrap(), b"release");
        }

        #[tokio::test]
        async fn a_different_digest_is_rejected() {
            let server = MockServer::start().await;
            let sums = format!("{}  {ARCHIVE}\n", sha256_hex(b"release"));
            serve(&server, b"tampered", Some(sums), 1).await;
            let err = fetch(&server).await.unwrap_err();
            assert!(matches!(err, Error::ArchiveVerification(_)), "got: {err:?}");
            assert!(err.to_string().contains(ARCHIVE), "got: {err}");
        }

        #[tokio::test]
        async fn missing_sums_or_entry_fails_before_the_archive_download() {
            let server = MockServer::start().await;
            serve(&server, b"release", None, 0).await;
            let err = fetch(&server).await.unwrap_err();
            assert!(matches!(err, Error::Download(_)), "got: {err:?}");

            let server = MockServer::start().await;
            let sums = format!("{}  other.tar.gz\n", sha256_hex(b"release"));
            serve(&server, b"release", Some(sums), 0).await;
            let err = fetch(&server).await.unwrap_err();
            assert!(matches!(err, Error::ArchiveVerification(_)), "got: {err:?}");
        }
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

            let err = resolve(&server).await.unwrap_err();
            // The fallback's class wins: it is the last request made.
            assert!(
                matches!(
                    &err,
                    Error::UpdateCheck {
                        failure: FailureKind::RateLimited,
                        ..
                    }
                ),
                "got: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("403"), "got: {}", msg);
            assert!(msg.contains("429"), "got: {}", msg);
        }

        /// Each source's failure is classified from the request, so `update
        /// --check --json` reports `transport`/`timeout` (#1312).
        #[tokio::test]
        async fn source_failures_keep_their_class() {
            let failure = |result: Result<String>| match result.unwrap_err() {
                Error::UpdateCheck { failure, .. } => failure,
                other => panic!("got: {other:?}"),
            };

            // Answers without a tag fall back to the manifest's class.
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_string("<html>"))
                .mount(&server)
                .await;
            assert_eq!(failure(resolve(&server).await), FailureKind::Other);
            Mock::given(method("GET"))
                .and(path("/latest.txt"))
                .respond_with(ResponseTemplate::new(503))
                .with_priority(1)
                .mount(&server)
                .await;
            assert_eq!(failure(resolve(&server).await), FailureKind::Http5xx);

            let closed = {
                let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
                format!("http://{}", listener.local_addr().unwrap())
            };
            let unreachable = resolve_latest_tag(
                &client(),
                &format!("{closed}/latest.txt"),
                &format!("{closed}/releases/latest"),
            )
            .await;
            assert_eq!(failure(unreachable), FailureKind::Transport);

            let slow = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(5)),
                )
                .mount(&slow)
                .await;
            let impatient = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(50))
                .build()
                .unwrap();
            let timed_out = resolve_latest_tag(
                &impatient,
                &format!("{}/latest.txt", slow.uri()),
                &format!("{}/releases/latest", slow.uri()),
            )
            .await;
            assert_eq!(failure(timed_out), FailureKind::Timeout);
        }
    }
}
