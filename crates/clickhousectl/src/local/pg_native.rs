//! Native (no-Docker) local Postgres, run from a portable bundle.
//!
//! Prototype, opt-in with `CLICKHOUSECTL_PG_RUNTIME=native`. Docker stays the
//! default. A bundle is a `pg-<pg>-pgch-<pgch>-<target>.tar.gz` tarball with a
//! single top-level directory holding `bin/`, `lib/`, `share/` and
//! `BUNDLE.json`. It is chosen by `CLICKHOUSECTL_PG_BUNDLE` (an `https://` or
//! `file://` URL), or else the latest `sdairs/pg-bundles` release asset for the
//! host target. Bundles are cached in `~/.clickhouse/postgres/<bundle-name>/`.
//!
//! Instances share the Docker backend's naming and metadata: the
//! `<name>-pg<major>` key, `.clickhouse/servers/<key>.json` (with
//! `container_id: None` and `pid` set to the postmaster) and
//! `.clickhouse/servers/<key>/data/`. Native-only state (bundle location and
//! credentials, which Docker keeps in the container env) lives in
//! `.clickhouse/servers/<key>/native.json`.

use crate::error::{Error, Result, StartupKind};
use crate::local::output;
use crate::local::postgres::{
    DEFAULT_DATABASE, DEFAULT_USER, generate_password, pg_major_from_tag, resolve_port,
    user_name_from_key, validate_pg_tag,
};
use crate::local::server::{self, Engine, ServerInfo};
use crate::paths;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const RUNTIME_ENV: &str = "CLICKHOUSECTL_PG_RUNTIME";
pub const BUNDLE_ENV: &str = "CLICKHOUSECTL_PG_BUNDLE";
const RELEASES_API: &str = "https://api.github.com/repos/sdairs/pg-bundles/releases/latest";
const STATE_FILE: &str = "native.json";
const LOG_FILE: &str = "postgres.log";
const READY_POLL_INTERVAL: Duration = Duration::from_millis(100);
const LOG_TAIL_LINES: usize = 50;
/// macOS `sun_path` is 104 bytes including the NUL terminator.
const MAX_SOCKET_PATH: usize = 103;

/// Whether `CLICKHOUSECTL_PG_RUNTIME` selects the native backend.
pub fn native_runtime_selected() -> Result<bool> {
    match std::env::var(RUNTIME_ENV) {
        Ok(value) => parse_runtime(&value),
        Err(_) => Ok(false),
    }
}

fn parse_runtime(value: &str) -> Result<bool> {
    match value.trim() {
        "" | "docker" => Ok(false),
        "native" => Ok(true),
        other => Err(Error::PostgresUsage(format!(
            "invalid {RUNTIME_ENV} '{other}': expected 'docker' or 'native'"
        ))),
    }
}

/// Whether `start` should use the native backend: selected by the env var,
/// or because the named instance (default "default") already exists as a
/// native one, so resuming never needs the env var again.
pub fn should_start_native(name: Option<&str>) -> Result<bool> {
    if native_runtime_selected()? {
        return Ok(true);
    }
    let name = name.unwrap_or("default");
    if server::validate_server_name(name).is_err() {
        return Ok(false);
    }
    // Read-only probe: taking the metadata lock would create project state
    // before Docker start validation runs.
    let Ok(entries) = std::fs::read_dir(server::servers_dir_join("")) else {
        return Ok(false);
    };
    let prefix = format!("{name}-pg");
    Ok(entries.flatten().any(|entry| {
        entry.file_name().to_str().is_some_and(|key| {
            key.strip_prefix(&prefix)
                .is_some_and(|major| !major.is_empty() && major.chars().all(|c| c.is_ascii_digit()))
                && entry.path().join(STATE_FILE).is_file()
        })
    }))
}

// ── instance state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeState {
    /// Bundle name, e.g. `pg-18.6-pgch-0.11.0-aarch64-apple-darwin`.
    pub bundle: String,
    /// Absolute path of the bundle's `bin/` directory.
    pub bin_dir: PathBuf,
    pub user: String,
    pub password: String,
    pub database: String,
}

fn state_path(key: &str) -> PathBuf {
    server::servers_dir_join(key).join(STATE_FILE)
}

pub fn load_state(key: &str) -> Result<Option<NativeState>> {
    let path = state_path(key);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).map(Some).map_err(|error| {
            Error::Postgres(format!(
                "invalid native state '{}': {error}",
                path.display()
            ))
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn save_state(key: &str, state: &NativeState) -> Result<()> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|error| Error::Postgres(format!("could not encode native state: {error}")))?;
    write_private_file(&state_path(key), json.as_bytes())
}

/// Write a file readable only by the current user (it holds a password).
fn write_private_file(path: &Path, content: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content)?;
    Ok(())
}

/// A Postgres instance is native when it has no container and its instance
/// directory holds native state.
pub fn is_native(info: &ServerInfo) -> bool {
    info.engine == Engine::Postgres
        && info.container_id.is_none()
        && state_path(&info.name).is_file()
}

/// Credentials for a native instance: `(user, password, database)`.
pub fn credentials(info: &ServerInfo) -> Result<(String, String, String)> {
    let state = load_state(&info.name)?.ok_or_else(|| missing_state(&info.name))?;
    Ok((state.user, state.password, state.database))
}

pub fn psql_path(info: &ServerInfo) -> Result<PathBuf> {
    let state = load_state(&info.name)?.ok_or_else(|| missing_state(&info.name))?;
    Ok(state.bin_dir.join("psql"))
}

fn missing_state(key: &str) -> Error {
    Error::PostgresUsage(format!(
        "native Postgres '{}' has no state file at '{}'",
        user_name_from_key(key),
        state_path(key).display()
    ))
}

// ── bundle source and naming ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum BundleSource {
    File(PathBuf),
    Https(String),
}

fn parse_bundle_source(value: &str) -> Result<BundleSource> {
    let value = value.trim();
    if let Some(path) = value.strip_prefix("file://") {
        let path = urlencoding::decode(path)
            .map_err(|_| Error::PostgresUsage(format!("invalid {BUNDLE_ENV} '{value}'")))?;
        if !path.starts_with('/') {
            return Err(Error::PostgresUsage(format!(
                "invalid {BUNDLE_ENV} '{value}': file:// URLs need an absolute path"
            )));
        }
        return Ok(BundleSource::File(PathBuf::from(path.into_owned())));
    }
    if value.starts_with("https://") {
        url::Url::parse(value)
            .map_err(|error| Error::PostgresUsage(format!("invalid {BUNDLE_ENV}: {error}")))?;
        return Ok(BundleSource::Https(value.to_string()));
    }
    Err(Error::PostgresUsage(format!(
        "invalid {BUNDLE_ENV} '{value}': expected an https:// or file:// URL to a .tar.gz bundle"
    )))
}

/// Bundle name from the archive file name: the last path segment minus
/// `.tar.gz`. Restricted to a safe character set since it names a directory.
fn bundle_name(source: &BundleSource) -> Result<String> {
    let last = match source {
        BundleSource::File(path) => path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        BundleSource::Https(url) => url
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string(),
    };
    let name = last.strip_suffix(".tar.gz").unwrap_or_default();
    let valid = name.starts_with("pg-")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !name.contains("..")
        && bundle_pg_version(name).is_some();
    if !valid {
        return Err(Error::PostgresUsage(format!(
            "unrecognised bundle file name '{last}': expected pg-<version>-pgch-<version>-<target>.tar.gz"
        )));
    }
    Ok(name.to_string())
}

/// `pg-18.6-pgch-0.11.0-aarch64-apple-darwin` → `18.6`.
fn bundle_pg_version(name: &str) -> Option<&str> {
    let version = name.strip_prefix("pg-")?.split('-').next()?;
    let valid = !version.is_empty()
        && version.chars().next().is_some_and(|c| c.is_ascii_digit())
        && version.chars().all(|c| c.is_ascii_digit() || c == '.');
    valid.then_some(version)
}

/// Bundle target triple for this host, matching the pg-bundles release names.
fn host_target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        _ => None,
    }
}

/// Pick the bundle asset for `target` from a GitHub "release" API response.
fn select_release_asset(release: &serde_json::Value, target: &str) -> Option<String> {
    let suffix = format!("-{target}.tar.gz");
    release["assets"]
        .as_array()?
        .iter()
        .filter(|asset| {
            asset["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("pg-") && name.ends_with(&suffix))
        })
        .filter_map(|asset| asset["browser_download_url"].as_str())
        .max()
        .map(str::to_string)
}

fn http_client() -> Result<reqwest::Client> {
    crate::http::client_builder()
        .build()
        .map_err(|error| Error::Postgres(format!("could not build HTTP client: {error}")))
}

fn with_github_token(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match std::env::var("GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => request.bearer_auth(token),
        _ => request,
    }
}

async fn resolve_bundle_source() -> Result<BundleSource> {
    if let Ok(value) = std::env::var(BUNDLE_ENV)
        && !value.trim().is_empty()
    {
        return parse_bundle_source(&value);
    }
    let target = host_target().ok_or_else(|| {
        Error::PostgresUsage(format!(
            "no native Postgres bundle for this platform; set {BUNDLE_ENV}"
        ))
    })?;
    let unavailable = |detail: String| {
        Error::PostgresUsage(format!(
            "could not find the latest Postgres bundle ({detail}); set {BUNDLE_ENV} to an \
             https:// or file:// bundle URL"
        ))
    };
    let response = with_github_token(http_client()?.get(RELEASES_API))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| unavailable(error.to_string()))?;
    if !response.status().is_success() {
        return Err(unavailable(format!(
            "GitHub API returned {}",
            response.status()
        )));
    }
    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|error| unavailable(error.to_string()))?;
    let url = select_release_asset(&release, target)
        .ok_or_else(|| unavailable(format!("no asset for {target} in the latest release")))?;
    Ok(BundleSource::Https(url))
}

// ── install ─────────────────────────────────────────────────────────────────

struct Bundle {
    name: String,
    root: PathBuf,
}

impl Bundle {
    fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    fn pg_version(&self) -> &str {
        bundle_pg_version(&self.name).unwrap_or_default()
    }
}

fn bundles_dir() -> Result<PathBuf> {
    Ok(paths::base_dir()?.join("postgres"))
}

/// Resolve the bundle and make sure it is extracted into the cache.
async fn ensure_bundle(json: bool) -> Result<Bundle> {
    let source = resolve_bundle_source().await?;
    let name = bundle_name(&source)?;
    let dir = bundles_dir()?;
    let root = dir.join(&name);
    if root.join("bin").join("postgres").is_file() {
        return Ok(Bundle { name, root });
    }

    paths::ensure_dir(&dir)?;
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(&dir)?;
    let (archive, expected_sha) = match &source {
        BundleSource::File(path) => {
            if !path.is_file() {
                return Err(Error::PostgresUsage(format!(
                    "bundle '{}' does not exist",
                    path.display()
                )));
            }
            let sidecar = PathBuf::from(format!("{}.sha256", path.display()));
            let expected = match std::fs::read_to_string(&sidecar) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            (path.clone(), expected)
        }
        BundleSource::Https(url) => {
            let archive = staging.path().join("bundle.tar.gz");
            if !json {
                eprintln!("Downloading {name}...");
            }
            download(url, &archive).await?;
            (
                archive,
                fetch_optional_text(&format!("{url}.sha256")).await?,
            )
        }
    };

    match expected_sha.as_deref().and_then(parse_sha256_file) {
        Some(expected) => {
            let actual = sha256_file(&archive)?;
            if actual != expected {
                return Err(Error::PostgresUsage(format!(
                    "sha256 mismatch for bundle {name}: expected {expected}, got {actual}"
                )));
            }
        }
        None if !json => eprintln!("Note: no .sha256 found for {name}; skipping verification"),
        None => {}
    }

    if !json {
        eprintln!("Extracting {name} to {}...", root.display());
    }
    let extract_dir = staging.path().join("extract");
    std::fs::create_dir(&extract_dir)?;
    let archive_path = archive.clone();
    let extract_target = extract_dir.clone();
    tokio::task::spawn_blocking(move || extract_tarball(&archive_path, &extract_target))
        .await
        .map_err(|error| Error::Postgres(format!("bundle extraction failed: {error}")))??;

    let extracted = extract_dir.join(&name);
    if !extracted.join("bin").join("postgres").is_file() {
        return Err(Error::PostgresUsage(format!(
            "bundle archive does not contain {name}/bin/postgres"
        )));
    }
    // Atomic publish; a concurrent installer winning the race is fine.
    if let Err(error) = std::fs::rename(&extracted, &root)
        && !root.join("bin").join("postgres").is_file()
    {
        return Err(error.into());
    }
    Ok(Bundle { name, root })
}

fn extract_tarball(archive: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_permissions(true);
    // `unpack` refuses entries that escape `dest`.
    tar.unpack(dest)
        .map_err(|error| Error::Postgres(format!("could not extract bundle: {error}")))
}

async fn download(url: &str, dest: &Path) -> Result<()> {
    use futures_util::StreamExt;
    let response = with_github_token(http_client()?.get(url))
        .send()
        .await
        .map_err(|error| Error::Postgres(format!("could not download {url}: {error}")))?;
    if !response.status().is_success() {
        return Err(Error::Postgres(format!(
            "could not download {url}: HTTP {}",
            response.status()
        )));
    }
    let mut file = std::fs::File::create(dest)?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| Error::Postgres(format!("download of {url} failed: {error}")))?;
        file.write_all(&chunk)?;
    }
    file.sync_all()?;
    Ok(())
}

/// GET a small text resource; `None` when it does not exist.
async fn fetch_optional_text(url: &str) -> Result<Option<String>> {
    let response = with_github_token(http_client()?.get(url))
        .send()
        .await
        .map_err(|error| Error::Postgres(format!("could not fetch {url}: {error}")))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(Error::Postgres(format!(
            "could not fetch {url}: HTTP {}",
            response.status()
        )));
    }
    response
        .text()
        .await
        .map(Some)
        .map_err(|error| Error::Postgres(format!("could not fetch {url}: {error}")))
}

/// First token of a `sha256sum`-style file, if it is a hex digest.
fn parse_sha256_file(content: &str) -> Option<String> {
    let digest = content.split_whitespace().next()?.to_ascii_lowercase();
    (digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit())).then_some(digest)
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

// ── start ───────────────────────────────────────────────────────────────────

pub struct StartArgs {
    pub name: Option<String>,
    pub version: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub extra_env: Vec<String>,
    pub wait_timeout: Duration,
}

pub async fn start(args: StartArgs, json: bool) -> Result<()> {
    if unsafe { libc::geteuid() } == 0 {
        return Err(Error::PostgresUsage(
            "native Postgres cannot run as root; run clickhousectl as an unprivileged user".into(),
        ));
    }
    if let Some(name) = args.name.as_deref() {
        server::validate_server_name(name)?;
    }
    if let Some(version) = args.version.as_deref() {
        validate_pg_tag(version)?;
    }
    if !args.extra_env.is_empty() {
        return Err(Error::PostgresUsage(format!(
            "--env is not supported with {RUNTIME_ENV}=native"
        )));
    }
    let explicit_port = args.port.map(|port| resolve_port(Some(port))).transpose()?;
    let wanted_major = args.version.as_deref().map(pg_major_from_tag);
    let project_cwd = std::env::current_dir()?
        .canonicalize()?
        .display()
        .to_string();

    let metadata_lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&metadata_lock)?;
    let user_name = match args.name.as_deref() {
        Some(name) => name.to_string(),
        None => default_name_locked(&metadata_lock)?,
    };
    let mut existing = server::find_pg_instances_locked(&user_name, &metadata_lock)?;
    if let Some(major) = wanted_major.as_deref() {
        existing.retain(|info| info.name == server::pg_instance_key(&user_name, major));
    }
    match existing.len() {
        0 => {}
        1 => {
            let prior = existing.remove(0);
            return resume(prior, &args, json, metadata_lock).await;
        }
        _ => {
            return Err(Error::PostgresUsage(format!(
                "multiple postgres instances named '{user_name}'; pass --version to select one"
            )));
        }
    }
    drop(metadata_lock);

    // Fresh instance. Install the bundle outside the metadata lock.
    let started = Instant::now();
    let bundle = ensure_bundle(json).await?;
    let major = pg_major_from_tag(bundle.pg_version());
    if let Some(wanted) = wanted_major.as_deref()
        && wanted != major
    {
        return Err(Error::PostgresUsage(format!(
            "--version {wanted} does not match the bundle's Postgres {} ({})",
            bundle.pg_version(),
            bundle.name
        )));
    }

    let metadata_lock = server::lock_metadata()?;
    let key = server::pg_instance_key(&user_name, &major);
    if server::load_info_locked(&key, &metadata_lock)?.is_some() {
        return Err(Error::PostgresUsage(format!(
            "postgres '{user_name}' was created concurrently; run the command again"
        )));
    }
    crate::init::ensure_runtime_gitignore()?;
    let port = match explicit_port {
        Some(port) => port,
        None => resolve_port(None)?,
    };
    let created = server::ensure_pg_data_dir(&user_name, &major)?;
    let instance_dir = server::servers_dir_join(&key);
    let data_dir = server::pg_data_dir(&user_name, &major);

    let state = match load_state(&key)? {
        Some(state) => state,
        None => NativeState {
            bundle: bundle.name.clone(),
            bin_dir: bundle.bin_dir(),
            user: args.user.clone().unwrap_or_else(|| DEFAULT_USER.into()),
            password: args.password.clone().unwrap_or_else(generate_password),
            database: args
                .database
                .clone()
                .unwrap_or_else(|| DEFAULT_DATABASE.into()),
        },
    };

    let fresh_data = dir_is_empty(&data_dir)?;
    if fresh_data {
        if let Err(error) = run_initdb(&state, &instance_dir, &data_dir) {
            if created {
                let _ = std::fs::remove_dir_all(&instance_dir);
            }
            return Err(error);
        }
    } else if !state_path(&key).is_file() {
        return Err(Error::PostgresUsage(format!(
            "'{}' already contains data but no native state; run \
             `clickhousectl local postgres remove {user_name}` to start over",
            data_dir.display()
        )));
    }
    save_state(&key, &state)?;

    let mut child = spawn_postmaster(&state.bin_dir, &instance_dir, &data_dir, port)?;
    let info = ServerInfo {
        name: key.clone(),
        pid: child.id(),
        version: format!("postgres:{}", bundle.pg_version()),
        http_port: 0,
        tcp_port: port,
        started_at: server::now_timestamp(),
        cwd: project_cwd,
        engine: Engine::Postgres,
        container_id: None,
    };
    if let Err(error) = server::save_server_info_locked(&info, &metadata_lock) {
        let _ = server::stop_native_postmaster(child.id());
        return Err(error);
    }
    drop(metadata_lock);

    wait_until_ready(
        &mut child,
        &state,
        &instance_dir,
        &user_name,
        port,
        args.wait_timeout,
    )?;
    if fresh_data && state.database != DEFAULT_DATABASE {
        create_database(&state, port)?;
    }
    if !json {
        eprintln!(
            "Postgres ready in {:.2}s (native, {})",
            started.elapsed().as_secs_f64(),
            bundle.name
        );
    }
    print_started(&user_name, &info, &state, json);
    Ok(())
}

async fn resume(
    prior: ServerInfo,
    args: &StartArgs,
    json: bool,
    metadata_lock: server::MetadataLock,
) -> Result<()> {
    let user_name = user_name_from_key(&prior.name).to_string();
    if !is_native(&prior) {
        return Err(Error::PostgresUsage(format!(
            "postgres '{user_name}' is a Docker instance; unset {RUNTIME_ENV} to manage it, or \
             run `clickhousectl local postgres remove {user_name}` first"
        )));
    }
    if server::is_server_running_locked(&prior.name, &metadata_lock)? {
        return Err(Error::ServerAlreadyRunning(user_name));
    }
    if !json
        && (args.port.is_some()
            || args.user.is_some()
            || args.password.is_some()
            || args.database.is_some())
    {
        eprintln!(
            "Note: postgres '{user_name}' already exists; resuming with stored settings. \
             Run `local postgres remove {user_name}` to start over."
        );
    }
    let state = load_state(&prior.name)?.ok_or_else(|| missing_state(&prior.name))?;
    if !state.bin_dir.join("postgres").is_file() {
        return Err(Error::PostgresUsage(format!(
            "bundle '{}' is missing from '{}'; reinstall it or remove this instance",
            state.bundle,
            state.bin_dir.display()
        )));
    }
    let started = Instant::now();
    let port = resolve_port(Some(prior.tcp_port))?;
    let instance_dir = server::servers_dir_join(&prior.name);
    let data_dir = instance_dir.join("data");
    let mut child = spawn_postmaster(&state.bin_dir, &instance_dir, &data_dir, port)?;
    let info = ServerInfo {
        pid: child.id(),
        started_at: server::now_timestamp(),
        ..prior
    };
    if let Err(error) = server::save_server_info_locked(&info, &metadata_lock) {
        let _ = server::stop_native_postmaster(child.id());
        return Err(error);
    }
    drop(metadata_lock);

    if let Err(error) = wait_until_ready(
        &mut child,
        &state,
        &instance_dir,
        &user_name,
        port,
        args.wait_timeout,
    ) {
        if let Ok(lock) = server::lock_metadata() {
            let _ = server::save_server_info_locked(&ServerInfo { pid: 0, ..info }, &lock);
        }
        return Err(error);
    }
    if !json {
        eprintln!(
            "Postgres ready in {:.2}s (native, {})",
            started.elapsed().as_secs_f64(),
            state.bundle
        );
    }
    print_started(&user_name, &info, &state, json);
    Ok(())
}

fn print_started(user_name: &str, info: &ServerInfo, state: &NativeState, json: bool) {
    let out = output::PostgresStartOutput {
        name: user_name.to_string(),
        container_id: String::new(),
        pid: Some(info.pid),
        image: state.bundle.clone(),
        port: info.tcp_port,
        user: state.user.clone(),
        password: state.password.clone(),
        database: state.database.clone(),
    };
    output::print_output(&out, json);
}

/// `"default"` unless a Postgres "default" is already running.
fn default_name_locked(metadata_lock: &server::MetadataLock) -> Result<String> {
    for info in server::find_pg_instances_locked("default", metadata_lock)? {
        if server::is_server_running_locked(&info.name, metadata_lock)? {
            return server::generate_random_name_locked(metadata_lock);
        }
    }
    Ok("default".into())
}

fn dir_is_empty(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn run_initdb(state: &NativeState, instance_dir: &Path, data_dir: &Path) -> Result<()> {
    // The password goes through a private file, never argv.
    let pwfile = instance_dir.join(".pwfile");
    write_private_file(&pwfile, state.password.as_bytes())?;
    let result = Command::new(state.bin_dir.join("initdb"))
        .arg("-D")
        .arg(data_dir)
        .arg("-U")
        .arg(&state.user)
        .arg("--auth=scram-sha-256")
        .arg(format!("--pwfile={}", pwfile.display()))
        .args(["-E", "UTF8", "--locale=C"])
        .stdin(Stdio::null())
        .output();
    let _ = std::fs::remove_file(&pwfile);
    let output =
        result.map_err(|error| Error::Postgres(format!("could not run initdb: {error}")))?;
    if !output.status.success() {
        return Err(Error::Postgres(format!(
            "initdb failed ({}):\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn spawn_postmaster(
    bin_dir: &Path,
    instance_dir: &Path,
    data_dir: &Path,
    port: u16,
) -> Result<Child> {
    use std::os::unix::process::CommandExt;
    let socket_dir = instance_dir.join("sock");
    std::fs::create_dir_all(&socket_dir)?;
    // Unix sockets are a convenience; clients use TCP. Disable them when the
    // project path is too long for `sun_path` rather than fail to start.
    let socket_path_len = socket_dir.as_os_str().len() + "/.s.PGSQL.65535".len();
    let socket_dirs = if socket_path_len <= MAX_SOCKET_PATH {
        socket_dir.display().to_string()
    } else {
        String::new()
    };
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(instance_dir.join(LOG_FILE))?;
    Command::new(bin_dir.join("postgres"))
        .arg("-D")
        .arg(data_dir)
        .arg("-p")
        .arg(port.to_string())
        .arg("-c")
        .arg("listen_addresses=127.0.0.1")
        .arg("-c")
        .arg(format!("unix_socket_directories={socket_dirs}"))
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        // Own process group: a terminal Ctrl-C aimed at clickhousectl must
        // not reach the server.
        .process_group(0)
        .spawn()
        .map_err(|error| Error::Exec(format!("could not start postgres: {error}")))
}

fn wait_until_ready(
    child: &mut Child,
    state: &NativeState,
    instance_dir: &Path,
    user_name: &str,
    port: u16,
    timeout: Duration,
) -> Result<()> {
    let log_path = instance_dir.join(LOG_FILE);
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(Error::StartupExit {
                kind: StartupKind::Postgres,
                name: user_name.to_string(),
                details: format!(
                    "Postgres '{user_name}' exited before it became ready ({status}).\n\
                     --- last {LOG_TAIL_LINES} lines of {} ---\n{}",
                    log_path.display(),
                    log_tail(&log_path)
                ),
            });
        }
        let ready = Command::new(state.bin_dir.join("pg_isready"))
            .args(["-q", "-h", "127.0.0.1", "-p", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if ready {
            return Ok(());
        }
        if Instant::now() >= deadline {
            let _ = server::stop_native_postmaster(child.id());
            return Err(Error::StartupTimeout {
                kind: StartupKind::Postgres,
                name: user_name.to_string(),
                seconds: timeout.as_secs(),
                details: format!(
                    "Postgres '{user_name}' did not become ready within {} seconds.\n\
                     --- last {LOG_TAIL_LINES} lines of {} ---\n{}",
                    timeout.as_secs(),
                    log_path.display(),
                    log_tail(&log_path)
                ),
            });
        }
        std::thread::sleep(READY_POLL_INTERVAL);
    }
}

fn log_tail(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            lines[lines.len().saturating_sub(LOG_TAIL_LINES)..].join("\n")
        }
        Err(error) => format!("(could not read log: {error})"),
    }
}

fn create_database(state: &NativeState, port: u16) -> Result<()> {
    let output = Command::new(state.bin_dir.join("createdb"))
        .args([
            "-h",
            "127.0.0.1",
            "-p",
            &port.to_string(),
            "-U",
            &state.user,
        ])
        .arg(&state.database)
        .env("PGPASSWORD", &state.password)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| Error::Postgres(format!("could not run createdb: {error}")))?;
    if !output.status.success() {
        return Err(Error::Postgres(format!(
            "createdb {} failed: {}",
            state.database,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_values() {
        assert!(!parse_runtime("").unwrap());
        assert!(!parse_runtime("docker").unwrap());
        assert!(parse_runtime("native").unwrap());
        assert!(matches!(
            parse_runtime("podman"),
            Err(Error::PostgresUsage(_))
        ));
    }

    #[test]
    fn bundle_sources_and_names() {
        let file =
            parse_bundle_source("file:///tmp/dist/pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz")
                .unwrap();
        assert_eq!(
            file,
            BundleSource::File(PathBuf::from(
                "/tmp/dist/pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz"
            ))
        );
        assert_eq!(
            bundle_name(&file).unwrap(),
            "pg-18.6-pgch-0.11.0-aarch64-apple-darwin"
        );

        let https = parse_bundle_source(
            "https://github.com/sdairs/pg-bundles/releases/download/v0.1.0/pg-17.11-pgch-0.11.0-x86_64-unknown-linux-gnu.tar.gz?x=1",
        )
        .unwrap();
        let name = bundle_name(&https).unwrap();
        assert_eq!(name, "pg-17.11-pgch-0.11.0-x86_64-unknown-linux-gnu");
        assert_eq!(bundle_pg_version(&name), Some("17.11"));
        assert_eq!(pg_major_from_tag(bundle_pg_version(&name).unwrap()), "17");

        for bad in [
            "http://example.com/pg-18.6.tar.gz",
            "/tmp/pg.tar.gz",
            "file://relative/x",
        ] {
            assert!(parse_bundle_source(bad).is_err(), "{bad}");
        }
        for bad in [
            "file:///tmp/postgres.tar.gz",
            "file:///tmp/pg-18.6-x.zip",
            "file:///tmp/pg-x-pgch-1-t.tar.gz",
        ] {
            assert!(
                bundle_name(&parse_bundle_source(bad).unwrap()).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn release_asset_selection() {
        let release = serde_json::json!({
            "assets": [
                {"name": "pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz.sha256",
                 "browser_download_url": "https://x/a.sha256"},
                {"name": "pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz",
                 "browser_download_url": "https://x/pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz"},
                {"name": "pg-18.6-pgch-0.11.0-x86_64-unknown-linux-gnu.tar.gz",
                 "browser_download_url": "https://x/linux.tar.gz"}
            ]
        });
        assert_eq!(
            select_release_asset(&release, "aarch64-apple-darwin").as_deref(),
            Some("https://x/pg-18.6-pgch-0.11.0-aarch64-apple-darwin.tar.gz")
        );
        assert_eq!(select_release_asset(&release, "x86_64-apple-darwin"), None);
    }

    #[test]
    fn sha256_sidecar_parsing() {
        let digest = "A".repeat(64);
        assert_eq!(
            parse_sha256_file(&format!("{digest}  pg.tar.gz\n")),
            Some("a".repeat(64))
        );
        assert_eq!(parse_sha256_file("not-a-digest pg.tar.gz"), None);
        assert_eq!(parse_sha256_file(""), None);
    }
}
