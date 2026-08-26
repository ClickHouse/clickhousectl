use crate::error::{Error, Result};
use crate::init;
use crate::local::discovery;
use crate::local::docker;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_HTTP_PORT: u16 = 8123;
const DEFAULT_TCP_PORT: u16 = 9000;
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SPAWN_HEALTH_DELAY: Duration = Duration::from_millis(300);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const STARTUP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const METADATA_LOCK_FILE: &str = ".metadata.lock";
const METADATA_TEMP_PREFIX: &str = ".metadata-";

const ADJECTIVES: &[&str] = &[
    "bold", "calm", "dark", "fast", "gold", "keen", "loud", "neat", "pale", "red", "slim", "tall",
    "warm", "blue", "cool", "deep", "flat", "gray", "iron", "wild",
];

const NOUNS: &[&str] = &[
    "bear", "bird", "bolt", "crab", "crow", "dart", "fawn", "fish", "frog", "gull", "hare", "hawk",
    "lynx", "moth", "newt", "orca", "puma", "seal", "swan", "wolf",
];

/// Engine driving a server instance. ClickHouse is a managed binary process;
/// Postgres is a managed Docker container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Clickhouse,
    Postgres,
}

impl Engine {
    pub fn as_str(&self) -> &'static str {
        match self {
            Engine::Clickhouse => "clickhouse",
            Engine::Postgres => "postgres",
        }
    }
}

fn default_engine() -> Engine {
    Engine::Clickhouse
}

/// Metadata saved for each server instance.
///
/// `engine` and `container_id` are post-Postgres-support additions and default
/// to ClickHouse + None so existing `.clickhouse/servers/*.json` files keep
/// deserializing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    /// Active ClickHouse process PID; 0 when stopped or for Postgres.
    pub pid: u32,
    /// Running ClickHouse version like "25.12.5.44", empty when stopped, or
    /// "postgres:<tag>" for Postgres.
    pub version: String,
    /// Running ClickHouse HTTP port; 0 when stopped or for Postgres.
    pub http_port: u16,
    /// Running ClickHouse TCP port, 0 when stopped, or mapped host port for Postgres.
    pub tcp_port: u16,
    pub started_at: String,
    pub cwd: String,
    #[serde(default = "default_engine")]
    pub engine: Engine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

/// A server entry shown in list output — may or may not be running.
pub struct ServerEntry {
    pub name: String,
    pub running: bool,
    pub info: Option<ServerInfo>,
}

/// Validate that a server name is safe for use in path operations.
/// Rejects names containing path separators, `..` components, or null bytes.
pub fn validate_server_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
        || name.contains("../")
        || name.contains("..\\")
    {
        return Err(Error::InvalidServerName(name.to_string()));
    }
    Ok(())
}

/// Directory where server tracking files and data live: .clickhouse/servers/
fn servers_dir() -> PathBuf {
    init::local_dir().join("servers")
}

/// The one project-wide metadata lock. Lifecycle operations hold this lock
/// from their final state read through their state-determining write. No code
/// holding it may acquire an install lock, and metadata helpers with a
/// `_locked` suffix never acquire it again.
pub(crate) struct MetadataLock {
    _file: File,
    dir: PathBuf,
}

impl MetadataLock {
    fn acquire_at(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(METADATA_LOCK_FILE))?;
        file.lock()?;
        Ok(Self {
            _file: file,
            dir: dir.to_path_buf(),
        })
    }
}

pub(crate) fn lock_metadata() -> Result<MetadataLock> {
    MetadataLock::acquire_at(&servers_dir())
}

/// Data directory for a ClickHouse server: .clickhouse/servers/<name>/data/.
pub fn server_data_dir(name: &str) -> PathBuf {
    servers_dir().join(name).join("data")
}

/// Combined stdout/stderr log for a background ClickHouse server.
pub fn server_log_path(name: &str) -> PathBuf {
    servers_dir().join(name).join("server.log")
}

/// Disk identifier for a Postgres instance: `<name>-pg<major>`. Used in the
/// metadata filename, the data dir name, and the container name so that
/// distinct (name, major) pairs never share state.
pub fn pg_instance_key(name: &str, major: &str) -> String {
    format!("{}-pg{}", name, major)
}

/// Join a child name onto the servers directory. Exposed so handlers can
/// remove a whole `<key>/` wrapper without poking at internals.
pub fn servers_dir_join(child: &str) -> PathBuf {
    servers_dir().join(child)
}

/// Data directory for a Postgres instance.
pub fn pg_data_dir(name: &str, major: &str) -> PathBuf {
    servers_dir()
        .join(pg_instance_key(name, major))
        .join("data")
}

/// Ensure project-local servers dir + .gitignore exist. Idempotent.
fn ensure_servers_dir() -> Result<()> {
    let dir = servers_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        let gitignore = init::local_dir().join(".gitignore");
        if !gitignore.exists() {
            let _ = std::fs::write(gitignore, "*\n");
        }
    }
    Ok(())
}

/// Ensure the data directory for a ClickHouse server exists.
pub fn ensure_server_data_dir(name: &str) -> Result<()> {
    ensure_servers_dir()?;
    std::fs::create_dir_all(server_data_dir(name))?;
    Ok(())
}

/// Ensure the data directory for a Postgres instance exists. Returns whether
/// this call created the instance directory, for transactional startup cleanup.
pub fn ensure_pg_data_dir(name: &str, major: &str) -> Result<bool> {
    ensure_servers_dir()?;
    let instance_dir = servers_dir().join(pg_instance_key(name, major));
    let created = match std::fs::create_dir(&instance_dir) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error.into()),
    };
    if let Err(error) = std::fs::create_dir_all(instance_dir.join("data")) {
        if created {
            let _ = std::fs::remove_dir(&instance_dir);
        }
        return Err(error.into());
    }
    Ok(created)
}

fn metadata_write_error(path: &Path, source: std::io::Error) -> Error {
    Error::ServerMetadataWrite {
        path: path.to_path_buf(),
        source,
    }
}

fn sync_directory(dir: &Path, metadata_path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| metadata_write_error(metadata_path, error))?;
    Ok(())
}

fn save_server_info_at(dir: &Path, info: &ServerInfo) -> Result<()> {
    let path = dir.join(format!("{}.json", info.name));
    let json = serde_json::to_vec_pretty(info)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(METADATA_TEMP_PREFIX)
        .tempfile_in(dir)
        .map_err(|error| metadata_write_error(&path, error))?;
    temporary
        .write_all(&json)
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| metadata_write_error(&path, error))?;
    temporary
        .persist(&path)
        .map_err(|error| metadata_write_error(&path, error.error))?;
    sync_directory(dir, &path)
}

pub(crate) fn save_server_info_locked(info: &ServerInfo, lock: &MetadataLock) -> Result<()> {
    validate_server_name(&info.name)?;
    save_server_info_at(&lock.dir, info)
}

pub(crate) fn try_remove_server_info_locked(name: &str, lock: &MetadataLock) -> Result<()> {
    let path = lock.dir.join(format!("{name}.json"));
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(metadata_write_error(&path, error)),
    }
    sync_directory(&lock.dir, &path)
}

/// Mark a ClickHouse server as stopped without discarding its metadata.
///
/// The PID match avoids overwriting metadata when a newer process was already
/// recorded before this transition began.
pub fn mark_server_stopped(name: &str, pid: u32) -> Result<()> {
    let lock = lock_metadata()?;
    mark_server_stopped_locked(name, pid, &lock)
}

pub(crate) fn mark_server_stopped_locked(name: &str, pid: u32, lock: &MetadataLock) -> Result<()> {
    let Some(mut info) = load_info_locked(name, lock)? else {
        return Ok(());
    };
    if info.engine == Engine::Clickhouse && info.pid == pid {
        info.pid = 0;
        info.version.clear();
        info.http_port = 0;
        info.tcp_port = 0;
        save_server_info_locked(&info, lock)?;
    }
    Ok(())
}

/// Engine-aware liveness check.
fn is_alive(info: &ServerInfo) -> bool {
    match info.engine {
        Engine::Clickhouse => is_process_alive(info.pid),
        Engine::Postgres => match info.container_id.as_deref() {
            Some(id) => docker::is_container_running_blocking(id),
            None => false,
        },
    }
}

fn load_info_at(path: &Path) -> Result<Option<ServerInfo>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::ServerMetadataRead {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let content = String::from_utf8(bytes).map_err(|source| Error::ServerMetadataUtf8 {
        path: path.to_path_buf(),
        source,
    })?;
    let info = serde_json::from_str(&content).map_err(|source| Error::ServerMetadataParse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some(info))
}

pub(crate) fn load_info_locked(name: &str, lock: &MetadataLock) -> Result<Option<ServerInfo>> {
    validate_server_name(name)?;
    load_info_at(&lock.dir.join(format!("{name}.json")))
}

/// Find every Postgres instance whose user-facing name is `name`. Returns
/// one entry per major version that has a metadata file on disk.
pub(crate) fn find_pg_instances_locked(name: &str, lock: &MetadataLock) -> Result<Vec<ServerInfo>> {
    let prefix = format!("{}-pg", name);
    let dir = match std::fs::read_dir(&lock.dir) {
        Ok(d) => d,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut out = Vec::new();
    for entry in dir {
        let entry = entry?;
        let fname = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let stem = match fname.strip_suffix(".json") {
            Some(s) => s,
            None => continue,
        };
        if !stem.starts_with(&prefix) {
            continue;
        }
        // Major must be all digits to match — guards against e.g. `dev-pg-foo`
        // matching when `name = "dev"`.
        let major = &stem[prefix.len()..];
        if major.is_empty() || !major.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Some(info) = load_info_locked(stem, lock)?
            && info.engine == Engine::Postgres
        {
            out.push(info);
        }
    }
    Ok(out)
}

/// Load server metadata only if the underlying process/container is alive.
/// Does not update stale metadata. `list_all_servers` is the single place that
/// marks ClickHouse entries stopped when their PID is gone, so callers like
/// `is_server_running` and `resolve_name` can read metadata without side effects.
fn load_running_info_locked(name: &str, lock: &MetadataLock) -> Result<Option<ServerInfo>> {
    let Some(info) = load_info_locked(name, lock)? else {
        return Ok(None);
    };
    if is_alive(&info) {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

/// List all known servers (both running and stopped).
///
/// Scans `.clickhouse/servers/*.json` for metadata. Each metadata file is one
/// entry — for ClickHouse the disk id is the user-facing name; for Postgres
/// it's `<name>-pg<major>`. Also runs process/container discovery so
/// orphaned instances reappear.
pub fn list_all_servers() -> Result<Vec<ServerEntry>> {
    let lock = lock_metadata()?;
    recover_current_project_servers_locked(&lock)?;
    list_all_servers_locked(&lock)
}

pub(crate) fn list_all_servers_locked(lock: &MetadataLock) -> Result<Vec<ServerEntry>> {
    let dir = &lock.dir;
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(error) => return Err(error.into()),
    };

    for entry in dir_entries {
        let entry = entry?;
        let fname = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let stem = match fname.strip_suffix(".json") {
            Some(s) => s,
            None => continue,
        };
        let Some(entry) = server_entry_locked(stem, lock)? else {
            // The file was removed after read_dir; absence is not corruption
            // and must not produce a phantom stopped entry.
            continue;
        };
        entries.push(entry);
    }

    entries.sort_by(|a, b| b.running.cmp(&a.running).then(a.name.cmp(&b.name)));
    Ok(entries)
}

pub(crate) fn server_entry_locked(name: &str, lock: &MetadataLock) -> Result<Option<ServerEntry>> {
    server_entry_locked_with(name, lock, || {})
}

fn server_entry_locked_with(
    name: &str,
    lock: &MetadataLock,
    before_stale_write: impl FnOnce(),
) -> Result<Option<ServerEntry>> {
    let Some(mut info) = load_info_locked(name, lock)? else {
        return Ok(None);
    };
    let mut running = is_alive(&info);

    // Keep the lock across liveness, comparison, and replacement. A restart
    // either commits before this read or waits and commits after normalization.
    if !running && info.engine == Engine::Clickhouse && info.pid != 0 {
        before_stale_write();
        mark_server_stopped_locked(name, info.pid, lock)?;
        info =
            load_info_locked(name, lock)?.ok_or_else(|| Error::ServerNotFound(name.to_string()))?;
        running = is_alive(&info);
    }

    Ok(Some(ServerEntry {
        name: name.to_string(),
        running,
        info: Some(info),
    }))
}

/// List only currently running servers.
pub fn list_running_servers() -> Result<Vec<ServerInfo>> {
    Ok(list_all_servers()?
        .into_iter()
        .filter(|e| e.running)
        .filter_map(|e| e.info)
        .collect())
}

pub(crate) fn list_running_servers_locked(lock: &MetadataLock) -> Result<Vec<ServerInfo>> {
    Ok(list_all_servers_locked(lock)?
        .into_iter()
        .filter(|entry| entry.running)
        .filter_map(|entry| entry.info)
        .collect())
}

/// Check if a named server is currently running.
pub fn is_server_running(name: &str) -> Result<bool> {
    let lock = lock_metadata()?;
    is_server_running_locked(name, &lock)
}

pub(crate) fn is_server_running_locked(name: &str, lock: &MetadataLock) -> Result<bool> {
    Ok(load_running_info_locked(name, lock)?.is_some())
}

fn is_process_alive(pid: u32) -> bool {
    pid != 0 && i32::try_from(pid).is_ok_and(|pid| unsafe { libc::kill(pid, 0) == 0 })
}

/// Send a signal to a process and return an error if the signal could not be delivered
/// (e.g. EPERM from a process owned by another user).
fn send_signal(pid: u32, signal: i32) -> Result<()> {
    let ret = unsafe { libc::kill(pid as i32, signal) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        Err(Error::Exec(format!(
            "Failed to send signal to PID {}: {}",
            pid, err
        )))
    } else {
        Ok(())
    }
}

/// Attempt to terminate a process: SIGTERM, wait, SIGKILL if needed, then verify exit.
fn kill_process(pid: u32) -> Result<()> {
    send_signal(pid, libc::SIGTERM)?;

    // Wait briefly for graceful shutdown
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_process_alive(pid) {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if is_process_alive(pid) {
            send_signal(pid, libc::SIGKILL)?;
            // Give the kernel a moment to reap the process
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    if is_process_alive(pid) {
        return Err(Error::Exec(format!(
            "Process {} did not exit after SIGKILL",
            pid
        )));
    }

    Ok(())
}

/// Stop a running server by name.
///
/// * ClickHouse: SIGTERM (then SIGKILL on timeout); metadata is retained with
///   PID 0 so the stopped instance remains discoverable.
/// * Postgres: stops the container only — does **not** remove it, and keeps
///   the metadata file so a subsequent `start` resumes the same container
///   (preserving the password and any other PGDATA-encoded settings).
pub fn kill_server(name: &str) -> Result<()> {
    let lock = lock_metadata()?;
    kill_server_locked(name, &lock)
}

pub(crate) fn kill_server_locked(name: &str, lock: &MetadataLock) -> Result<()> {
    let info = load_running_info_locked(name, lock)?
        .ok_or_else(|| Error::ServerNotRunning(name.to_string()))?;

    match info.engine {
        Engine::Clickhouse => {
            kill_process(info.pid)?;
            mark_server_stopped_locked(name, info.pid, lock)?;
        }
        Engine::Postgres => {
            let id = info.container_id.as_deref().ok_or_else(|| {
                Error::DockerError(format!(
                    "Postgres server '{}' has no container_id in metadata",
                    name
                ))
            })?;
            docker::stop_blocking(id)?;
            // Metadata + container preserved so `start` can resume.
        }
    }
    Ok(())
}

/// Resolve the server name: use provided name, "default" if none and no default running,
/// or generate a random name if "default" is already running.
/// Returns an error if the provided name contains path traversal characters.
pub fn resolve_name(name: Option<&str>) -> Result<String> {
    let lock = lock_metadata()?;
    resolve_name_locked(name, &lock)
}

pub(crate) fn resolve_name_locked(name: Option<&str>, lock: &MetadataLock) -> Result<String> {
    match name {
        Some(n) => {
            validate_server_name(n)?;
            Ok(n.to_string())
        }
        None => {
            if is_server_running_locked("default", lock)? {
                generate_random_name_locked(lock)
            } else {
                Ok("default".to_string())
            }
        }
    }
}

fn generate_random_name_locked(lock: &MetadataLock) -> Result<String> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mixed = seed ^ (std::process::id() as u128);
    let adj = ADJECTIVES[(mixed % ADJECTIVES.len() as u128) as usize];
    let noun = NOUNS[((mixed / ADJECTIVES.len() as u128) % NOUNS.len() as u128) as usize];
    let tag = format!("{}-{}", adj, noun);

    if is_server_running_locked(&tag, lock)? {
        for i in 2..100 {
            let candidate = format!("{}-{}", tag, i);
            if !is_server_running_locked(&candidate, lock)? {
                return Ok(candidate);
            }
        }
    }
    Ok(tag)
}

/// Wait a moment after spawn and check if the child exited immediately.
pub async fn check_spawn_health(
    child: &mut std::process::Child,
    name: &str,
    log_path: &Path,
) -> Result<()> {
    tokio::time::sleep(SPAWN_HEALTH_DELAY).await;
    if let Some(status) = child.try_wait().map_err(|e| Error::Exec(e.to_string()))? {
        let message = format!(
            "Server '{}' exited immediately after starting ({}). See server log: {}",
            name,
            status,
            log_path.display()
        );
        return match mark_server_stopped(name, child.id()) {
            Ok(()) => Err(Error::Exec(message)),
            Err(metadata_error) => Err(Error::Exec(format!(
                "{message}; additionally failed to record the stopped server: {metadata_error}"
            ))),
        };
    }
    Ok(())
}

async fn stop_starting_child(
    child: &mut std::process::Child,
    name: &str,
) -> std::result::Result<(), String> {
    let pid = child.id();
    if child.try_wait().map_err(|e| e.to_string())?.is_none()
        && let Err(signal_error) = send_signal(pid, libc::SIGTERM)
        && child.try_wait().map_err(|e| e.to_string())?.is_none()
    {
        return Err(signal_error.to_string());
    }

    let deadline = tokio::time::Instant::now() + STARTUP_SHUTDOWN_TIMEOUT;
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return mark_server_stopped(name, pid).map_err(|e| e.to_string());
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }

    child.kill().map_err(|e| e.to_string())?;
    child.wait().map_err(|e| e.to_string())?;
    mark_server_stopped(name, pid).map_err(|e| e.to_string())
}

/// Wait until ClickHouse responds to HTTP health checks and accepts TCP connections.
pub async fn wait_for_server_ready(
    child: &mut std::process::Child,
    name: &str,
    http_port: u16,
    tcp_port: u16,
    log_path: &Path,
    timeout: Duration,
) -> Result<()> {
    let started = tokio::time::Instant::now();
    check_spawn_health(child, name, log_path).await?;
    let health_client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(CONNECT_TIMEOUT)
        .build()?;
    let health_url = format!("http://localhost:{http_port}/ping");

    loop {
        if let Some(status) = child.try_wait().map_err(|e| Error::Exec(e.to_string()))? {
            let message = format!(
                "Server '{}' exited before becoming ready on HTTP port {} and TCP port {} ({}). \
                 See server log: {}",
                name,
                http_port,
                tcp_port,
                status,
                log_path.display()
            );
            return match mark_server_stopped(name, child.id()) {
                Ok(()) => Err(Error::Exec(message)),
                Err(metadata_error) => Err(Error::Exec(format!(
                    "{message}; additionally failed to record the stopped server: {metadata_error}"
                ))),
            };
        }

        let tcp_ready = matches!(
            tokio::time::timeout(
                CONNECT_TIMEOUT,
                tokio::net::TcpStream::connect(("localhost", tcp_port))
            )
            .await,
            Ok(Ok(_))
        );
        let http_ready = if tcp_ready {
            match health_client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    response.text().await.is_ok_and(|body| body.trim() == "Ok.")
                }
                _ => false,
            }
        } else {
            false
        };
        if tcp_ready && http_ready {
            return Ok(());
        }

        if started.elapsed() >= timeout {
            let pid = child.id();
            let cleanup = match stop_starting_child(child, name).await {
                Ok(()) => " and was stopped".to_string(),
                Err(error) => format!("; failed to stop PID {}: {}", pid, error),
            };
            return Err(Error::Exec(format!(
                "Server '{}' did not become ready on HTTP port {} and TCP port {} within {} seconds{}. \
                 See server log: {}",
                name,
                http_port,
                tcp_port,
                timeout.as_secs(),
                cleanup,
                log_path.display()
            )));
        }

        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }
}

/// Check if a TCP port is available by attempting to bind to it.
fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find a free port starting from `start`, incrementing by 1.
fn find_free_port(start: u16) -> Option<u16> {
    (start..=start.saturating_add(100)).find(|&p| is_port_available(p))
}

/// Resolve the HTTP and TCP ports to use.
/// If explicit ports are given, use them as-is.
/// Otherwise, try defaults (8123/9000) and auto-assign free ports if they're taken.
/// Returns (http_port, tcp_port, auto_assigned) where auto_assigned is true if
/// we picked non-default ports.
pub fn resolve_ports(http_port: Option<u16>, tcp_port: Option<u16>) -> Result<(u16, u16, bool)> {
    let http = match http_port {
        Some(0) => {
            return Err(Error::Exec(
                "--http-port 0 is not allowed; pick a specific port or omit the flag".into(),
            ));
        }
        Some(p) if is_port_available(p) => p,
        Some(p) => return Err(Error::Exec(format!("HTTP port {} is already in use", p))),
        None => {
            if is_port_available(DEFAULT_HTTP_PORT) {
                DEFAULT_HTTP_PORT
            } else {
                find_free_port(DEFAULT_HTTP_PORT + 1)
                    .ok_or_else(|| Error::Exec("Could not find a free HTTP port".into()))?
            }
        }
    };

    let tcp = match tcp_port {
        Some(0) => {
            return Err(Error::Exec(
                "--tcp-port 0 is not allowed; pick a specific port or omit the flag".into(),
            ));
        }
        Some(p) if is_port_available(p) => p,
        Some(p) => return Err(Error::Exec(format!("TCP port {} is already in use", p))),
        None => {
            if is_port_available(DEFAULT_TCP_PORT) {
                DEFAULT_TCP_PORT
            } else {
                find_free_port(DEFAULT_TCP_PORT + 1)
                    .ok_or_else(|| Error::Exec("Could not find a free TCP port".into()))?
            }
        }
    };

    let auto_assigned = http_port.is_none() && http != DEFAULT_HTTP_PORT
        || tcp_port.is_none() && tcp != DEFAULT_TCP_PORT;

    Ok((http, tcp, auto_assigned))
}

/// Build ClickHouse server port flags.
pub fn port_flags(http_port: u16, tcp_port: u16) -> Vec<String> {
    vec![
        format!("--http_port={}", http_port),
        format!("--tcp_port={}", tcp_port),
    ]
}

/// Format a timestamp for now.
pub fn now_timestamp() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

/// Recover orphaned servers for the current project via process discovery.
///
/// Scans for running ClickHouse processes whose cwd matches this project's
/// `.clickhouse/servers/<name>/data/` path. If a process is found that has no
/// metadata file, a new `ServerInfo` is saved so it appears in `server list`
/// and can be managed normally.
pub fn recover_current_project_servers() -> Result<()> {
    let lock = lock_metadata()?;
    recover_current_project_servers_locked(&lock)
}

pub(crate) fn recover_current_project_servers_locked(lock: &MetadataLock) -> Result<()> {
    let current_dir = std::env::current_dir()?
        .canonicalize()?
        .display()
        .to_string();

    let processes = discovery::discover_clickhouse_processes();
    for proc in processes {
        // Canonicalize the discovered project root for comparison
        let discovered_root = match std::path::Path::new(&proc.project_root).canonicalize() {
            Ok(p) => p.display().to_string(),
            Err(_) => proc.project_root.clone(),
        };

        if discovered_root != current_dir {
            continue;
        }

        validate_server_name(&proc.server_name)?;
        let info = ServerInfo {
            name: proc.server_name,
            pid: proc.pid,
            version: proc.version.unwrap_or_else(|| "unknown".to_string()),
            http_port: proc.http_port.unwrap_or(0),
            tcp_port: proc.tcp_port.unwrap_or(0),
            started_at: "recovered".to_string(),
            cwd: current_dir.clone(),
            engine: Engine::Clickhouse,
            container_id: None,
        };
        recover_clickhouse_info_locked(&info, lock)?;
    }

    // Also recover orphaned Postgres containers belonging to this project.
    docker::recover_project_postgres_blocking(&current_dir, lock)
}

fn recover_clickhouse_info_locked(info: &ServerInfo, lock: &MetadataLock) -> Result<()> {
    // A corrupt existing file is an error, not an absent entry that recovery
    // is allowed to overwrite.
    if load_running_info_locked(&info.name, lock)?.is_none() {
        save_server_info_locked(info, lock)?;
    }
    Ok(())
}

/// A server entry for global listing — always running (discovered via process inspection).
pub struct GlobalServerEntry {
    pub name: String,
    pub pid: u32,
    pub project: String,
    pub http_port: Option<u16>,
    pub tcp_port: Option<u16>,
    pub version: Option<String>,
    pub engine: Engine,
    pub container_id: Option<String>,
}

/// List all running ClickHouse servers across all projects via process discovery.
/// (Postgres containers are not currently merged in — a future change will add
/// `docker ps` based discovery here as well.)
pub fn list_all_servers_global() -> Vec<GlobalServerEntry> {
    let processes = discovery::discover_clickhouse_processes();
    processes
        .into_iter()
        .map(|p| GlobalServerEntry {
            name: p.server_name,
            pid: p.pid,
            project: p.project_root,
            http_port: p.http_port,
            tcp_port: p.tcp_port,
            version: p.version,
            engine: Engine::Clickhouse,
            container_id: None,
        })
        .collect()
}

/// Kill a server found via global process discovery.
/// Takes a PID directly and kills it, without requiring local metadata.
pub fn kill_server_by_pid(pid: u32) -> Result<()> {
    if !is_process_alive(pid) {
        return Err(Error::ServerNotRunning(format!("PID {}", pid)));
    }

    kill_process(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_info(pid: u32, version: &str) -> ServerInfo {
        ServerInfo {
            name: "default".into(),
            pid,
            version: version.into(),
            http_port: 8123,
            tcp_port: 9000,
            started_at: "1700000000".into(),
            cwd: "/tmp/project".into(),
            engine: Engine::Clickhouse,
            container_id: None,
        }
    }

    #[test]
    fn engine_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Engine::Clickhouse).unwrap(),
            "\"clickhouse\""
        );
        assert_eq!(
            serde_json::to_string(&Engine::Postgres).unwrap(),
            "\"postgres\""
        );
    }

    #[test]
    fn server_info_legacy_json_deserializes_as_clickhouse() {
        // Legacy JSON written before the engine field existed.
        let legacy = r#"{
            "name": "default",
            "pid": 12345,
            "version": "25.12.5.44",
            "http_port": 8123,
            "tcp_port": 9000,
            "started_at": "1700000000",
            "cwd": "/tmp/proj"
        }"#;
        let info: ServerInfo = serde_json::from_str(legacy).expect("legacy JSON should parse");
        assert_eq!(info.engine, Engine::Clickhouse);
        assert!(info.container_id.is_none());
    }

    #[test]
    fn server_info_postgres_round_trip() {
        let info = ServerInfo {
            name: "dev".into(),
            pid: 0,
            version: "postgres:17".into(),
            http_port: 0,
            tcp_port: 5432,
            started_at: "1700000000".into(),
            cwd: "/tmp/proj".into(),
            engine: Engine::Postgres,
            container_id: Some("abc123".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.engine, Engine::Postgres);
        assert_eq!(parsed.container_id.as_deref(), Some("abc123"));
        assert!(json.contains("\"engine\":\"postgres\""));
    }

    #[test]
    fn selected_metadata_reports_partial_json() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("default.json");
        std::fs::write(&path, br#"{"name":"default","pid":12"#).unwrap();

        let error = load_info_at(&path).unwrap_err();
        assert!(matches!(error, Error::ServerMetadataParse { .. }));
        assert!(error.to_string().contains("not valid JSON"));
        assert!(error.to_string().contains("default.json"));
    }

    #[test]
    fn selected_metadata_reports_invalid_utf8() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("default.json");
        std::fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();

        let error = load_info_at(&path).unwrap_err();
        assert!(matches!(error, Error::ServerMetadataUtf8 { .. }));
        assert!(error.to_string().contains("not valid UTF-8"));
    }

    #[test]
    fn selected_metadata_reports_read_io_errors() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("default.json");
        std::fs::create_dir(&path).unwrap();

        let error = load_info_at(&path).unwrap_err();
        assert!(matches!(error, Error::ServerMetadataRead { .. }));
        assert!(error.to_string().contains("Failed to read server metadata"));
    }

    #[cfg(unix)]
    #[test]
    fn selected_metadata_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("default.json");
        std::fs::write(&path, b"{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let error = load_info_at(&path).unwrap_err();
        let Error::ServerMetadataRead { source, .. } = error else {
            panic!("expected metadata read error");
        };
        assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn atomic_save_ignores_interrupted_sibling_temp_files() {
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        let stale_temp = directory.path().join(".metadata-interrupted-write");
        std::fs::write(&stale_temp, br#"{"name":"default""#).unwrap();

        save_server_info_locked(&test_info(0, "25.12.1.1"), &lock).unwrap();
        let entries = list_all_servers_locked(&lock).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "default");
        assert_eq!(entries[0].info.as_ref().unwrap().version, "25.12.1.1");
        assert_eq!(
            std::fs::read_to_string(stale_temp).unwrap(),
            r#"{"name":"default""#
        );
    }

    #[test]
    fn concurrent_unlocked_readers_never_observe_partial_json() {
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        save_server_info_locked(&test_info(0, "initial"), &lock).unwrap();
        drop(lock);
        let writer_dir = directory.path().to_path_buf();
        let metadata_path = directory.path().join("default.json");

        let writer = std::thread::spawn(move || {
            for generation in 0..200 {
                let lock = MetadataLock::acquire_at(&writer_dir).unwrap();
                save_server_info_locked(&test_info(0, &format!("generation-{generation}")), &lock)
                    .unwrap();
            }
        });
        for _ in 0..1_000 {
            let bytes = std::fs::read(&metadata_path).unwrap();
            serde_json::from_slice::<ServerInfo>(&bytes).unwrap();
        }
        writer.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn metadata_write_permission_failure_is_not_discarded() {
        use std::os::unix::fs::PermissionsExt;

        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        save_server_info_locked(&test_info(0, "preserved"), &lock).unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        let error = save_server_info_locked(&test_info(0, "blocked"), &lock).unwrap_err();
        assert!(matches!(error, Error::ServerMetadataWrite { .. }));

        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            load_info_locked("default", &lock).unwrap().unwrap().version,
            "preserved"
        );
    }

    #[test]
    fn stale_pid_normalization_is_durable() {
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        save_server_info_locked(&test_info(u32::MAX, "25.12.1.1"), &lock).unwrap();

        let entry = server_entry_locked("default", &lock).unwrap().unwrap();
        let normalized = entry.info.unwrap();
        assert!(!entry.running);
        assert_eq!(normalized.pid, 0);
        assert!(normalized.version.is_empty());
        assert_eq!(load_info_locked("default", &lock).unwrap().unwrap().pid, 0);
    }

    #[test]
    fn recovery_does_not_overwrite_corrupt_live_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        let path = directory.path().join("default.json");
        let partial = br#"{"name":"default""#;
        std::fs::write(&path, partial).unwrap();

        let error =
            recover_clickhouse_info_locked(&test_info(std::process::id(), "recovered"), &lock)
                .unwrap_err();

        assert!(matches!(error, Error::ServerMetadataParse { .. }));
        assert_eq!(std::fs::read(path).unwrap(), partial);
    }

    #[test]
    fn restart_waiting_during_normalization_commits_last() {
        let directory = tempfile::tempdir().unwrap();
        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        save_server_info_locked(&test_info(u32::MAX, "stale"), &lock).unwrap();
        let restart_dir = directory.path().to_path_buf();
        let mut restart = None;

        let normalized = server_entry_locked_with("default", &lock, || {
            restart = Some(std::thread::spawn(move || {
                let restart_lock = MetadataLock::acquire_at(&restart_dir).unwrap();
                save_server_info_locked(&test_info(std::process::id(), "restarted"), &restart_lock)
                    .unwrap();
            }));
        })
        .unwrap()
        .unwrap();
        assert_eq!(normalized.info.unwrap().pid, 0);
        drop(lock);
        restart.unwrap().join().unwrap();

        let lock = MetadataLock::acquire_at(directory.path()).unwrap();
        let final_info = load_info_locked("default", &lock).unwrap().unwrap();
        assert_eq!(final_info.pid, std::process::id());
        assert_eq!(final_info.version, "restarted");
    }

    #[test]
    fn stopped_and_out_of_range_pids_are_never_alive() {
        assert!(!is_process_alive(0));
        assert!(!is_process_alive(u32::MAX));
    }

    #[tokio::test]
    async fn readiness_timeout_stops_child() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "exec sleep 10"])
            .spawn()
            .unwrap();
        let pid = child.id();

        let error = wait_for_server_ready(
            &mut child,
            "readiness-timeout-test",
            0,
            0,
            Path::new("server.log"),
            Duration::from_secs(1),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("did not become ready"));
        assert!(error.contains("server.log"));
        assert!(!is_process_alive(pid));
    }

    #[test]
    fn explicit_ports_must_be_available() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let http_error = resolve_ports(Some(port), None).unwrap_err();
        assert!(http_error.to_string().contains("HTTP port"));

        let tcp_error = resolve_ports(None, Some(port)).unwrap_err();
        assert!(tcp_error.to_string().contains("TCP port"));
    }

    #[test]
    fn explicit_ports_reject_zero() {
        let http_error = resolve_ports(Some(0), None).unwrap_err();
        assert!(matches!(http_error, Error::Exec(msg) if msg.contains("--http-port 0")));

        let tcp_error = resolve_ports(None, Some(0)).unwrap_err();
        assert!(matches!(tcp_error, Error::Exec(msg) if msg.contains("--tcp-port 0")));
    }
}
