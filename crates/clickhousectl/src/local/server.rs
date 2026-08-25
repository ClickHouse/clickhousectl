use crate::error::{Error, Result};
use crate::init;
use crate::local::discovery;
use crate::local::docker;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_HTTP_PORT: u16 = 8123;
const DEFAULT_TCP_PORT: u16 = 9000;
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SPAWN_HEALTH_DELAY: Duration = Duration::from_millis(300);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const STARTUP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

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

fn server_meta_path(name: &str) -> PathBuf {
    servers_dir().join(format!("{}.json", name))
}

/// Optimistic existence check for work that is safe to perform without the
/// lifecycle lock. Callers must re-read metadata after acquiring the lock.
pub(crate) fn server_metadata_exists(name: &str) -> Result<bool> {
    validate_server_name(name)?;
    server_meta_path(name)
        .try_exists()
        .map_err(|source| metadata_access_error(name, source))
}

fn server_lock_path(name: &str) -> PathBuf {
    servers_dir().join(".locks").join(format!("{}.lock", name))
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

/// Ensure the data directory for a Postgres instance exists.
pub fn ensure_pg_data_dir(name: &str, major: &str) -> Result<()> {
    ensure_servers_dir()?;
    std::fs::create_dir_all(pg_data_dir(name, major))?;
    Ok(())
}

struct FileLock {
    file: File,
}

impl FileLock {
    fn acquire(path: &Path, name: &str) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| metadata_write_error(name, source))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)
            .map_err(|source| metadata_access_error(name, source))?;

        loop {
            // SAFETY: `file` owns this descriptor for the lifetime of the lock.
            let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
            if result == 0 {
                return Ok(Self { file });
            }
            let source = std::io::Error::last_os_error();
            if source.kind() != std::io::ErrorKind::Interrupted {
                return Err(metadata_access_error(name, source));
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: `self.file` remains open until after `drop` returns.
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

struct TemporaryMetadata {
    path: PathBuf,
    committed: bool,
}

impl Drop for TemporaryMetadata {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn metadata_access_error(name: &str, source: std::io::Error) -> Error {
    if source.kind() == std::io::ErrorKind::PermissionDenied {
        Error::ServerMetadataPermission {
            name: name.to_string(),
            source,
        }
    } else {
        Error::ServerMetadataRead {
            name: name.to_string(),
            source,
        }
    }
}

fn metadata_write_error(name: &str, source: std::io::Error) -> Error {
    if source.kind() == std::io::ErrorKind::PermissionDenied {
        Error::ServerMetadataPermission {
            name: name.to_string(),
            source,
        }
    } else {
        Error::ServerMetadataWrite {
            name: name.to_string(),
            source,
        }
    }
}

fn read_server_info(name: &str) -> Result<Option<ServerInfo>> {
    let content = match std::fs::read(server_meta_path(name)) {
        Ok(content) => content,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(metadata_access_error(name, source)),
    };
    serde_json::from_slice(&content)
        .map(Some)
        .map_err(|source| Error::ServerMetadataParse {
            name: name.to_string(),
            source,
        })
}

fn write_server_info(name: &str, info: &ServerInfo) -> Result<()> {
    let path = server_meta_path(name);
    let file_name = path
        .file_name()
        .expect("server metadata path has a file name")
        .to_string_lossy();
    let temp_path = path.with_file_name(format!(
        ".{file_name}.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let mut temporary = TemporaryMetadata {
        path: temp_path.clone(),
        committed: false,
    };
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .map_err(|source| metadata_write_error(name, source))?;
    let json = serde_json::to_vec_pretty(info)?;
    file.write_all(&json)
        .map_err(|source| metadata_write_error(name, source))?;
    file.flush()
        .map_err(|source| metadata_write_error(name, source))?;
    file.sync_all()
        .map_err(|source| metadata_write_error(name, source))?;
    pause_before_metadata_rename_for_test();
    std::fs::rename(&temp_path, &path).map_err(|source| metadata_write_error(name, source))?;
    temporary.committed = true;
    Ok(())
}

/// The per-server cross-process lifecycle lock. Callers that perform an
/// external lifecycle action keep this guard through the corresponding
/// metadata update so another invocation cannot act on an obsolete snapshot.
pub struct ServerLock {
    name: String,
    _file: FileLock,
}

impl ServerLock {
    pub fn acquire(name: &str) -> Result<Self> {
        validate_server_name(name)?;
        ensure_servers_dir()?;
        Ok(Self {
            name: name.to_string(),
            _file: FileLock::acquire(&server_lock_path(name), name)?,
        })
    }

    pub fn load_info(&self) -> Result<Option<ServerInfo>> {
        read_server_info(&self.name)
    }

    pub fn metadata_path(&self) -> PathBuf {
        server_meta_path(&self.name)
    }

    pub fn load_running_info(&self) -> Result<Option<ServerInfo>> {
        Ok(self.load_info()?.filter(is_alive))
    }

    pub fn is_running(&self) -> Result<bool> {
        Ok(self.load_running_info()?.is_some())
    }

    pub fn save_info(&self, info: &ServerInfo) -> Result<()> {
        write_server_info(&self.name, info)
    }

    pub fn remove_info(&self) -> Result<()> {
        match std::fs::remove_file(server_meta_path(&self.name)) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(metadata_write_error(&self.name, source)),
        }
    }

    pub fn mark_stopped(&self, pid: u32) -> Result<()> {
        let Some(mut info) = self.load_info()? else {
            return Ok(());
        };
        if info.engine == Engine::Clickhouse && info.pid == pid {
            set_stopped(&mut info);
            self.save_info(&info)?;
        }
        Ok(())
    }

    pub fn kill(&self) -> Result<()> {
        let info = self
            .load_running_info()?
            .ok_or_else(|| Error::ServerNotRunning(self.name.clone()))?;
        match info.engine {
            Engine::Clickhouse => {
                kill_process(info.pid)?;
                self.mark_stopped(info.pid)?;
            }
            Engine::Postgres => {
                let id = info.container_id.as_deref().ok_or_else(|| {
                    Error::DockerError(format!(
                        "Postgres server '{}' has no container_id in metadata",
                        self.name
                    ))
                })?;
                docker::stop_blocking(id)?;
            }
        }
        Ok(())
    }
}

fn set_stopped(info: &mut ServerInfo) {
    info.pid = 0;
    info.version.clear();
    info.http_port = 0;
    info.tcp_port = 0;
}

/// Save server info with an atomic replacement under the lifecycle lock.
#[cfg(test)]
pub fn save_server_info(info: &ServerInfo) -> Result<()> {
    ServerLock::acquire(&info.name)?.save_info(info)
}

/// Mark a ClickHouse server as stopped if the recorded PID still matches.
pub fn mark_server_stopped(name: &str, pid: u32) -> Result<()> {
    ServerLock::acquire(name)?.mark_stopped(pid)
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

/// Load server metadata regardless of liveness. A missing file is `None`;
/// access and parse failures remain actionable errors.
pub fn load_info(name: &str) -> Result<Option<ServerInfo>> {
    ServerLock::acquire(name)?.load_info()
}

/// Find every Postgres instance whose user-facing name is `name`. Returns
/// one entry per major version that has a metadata file on disk.
pub fn find_pg_instances(name: &str) -> Result<Vec<ServerInfo>> {
    let prefix = format!("{}-pg", name);
    let dir = match std::fs::read_dir(servers_dir()) {
        Ok(d) => d,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(source.into()),
    };
    let mut out = Vec::new();
    for entry in dir.flatten() {
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
        if let Some(info) = load_info(stem)?
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
pub fn load_running_info(name: &str) -> Result<Option<ServerInfo>> {
    ServerLock::acquire(name)?.load_running_info()
}

/// List all known servers (both running and stopped).
///
/// Scans `.clickhouse/servers/*.json` for metadata. Each metadata file is one
/// entry — for ClickHouse the disk id is the user-facing name; for Postgres
/// it's `<name>-pg<major>`. Also runs process/container discovery so
/// orphaned instances reappear.
pub fn list_all_servers() -> Result<Vec<ServerEntry>> {
    list_all_servers_inner(false)
}

fn list_all_servers_inner(skip_entry_errors: bool) -> Result<Vec<ServerEntry>> {
    recover_current_project_servers()?;

    let dir = servers_dir();
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(source) => return Err(source.into()),
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
        if !entry.path().is_file() {
            continue;
        }
        let normalized = match normalize_server_info(stem) {
            Ok(normalized) => normalized,
            Err(_) if skip_entry_errors => continue,
            Err(error) => return Err(error),
        };
        let Some((info, running)) = normalized else {
            continue;
        };

        entries.push(ServerEntry {
            name: stem.to_string(),
            running,
            info: Some(info),
        });
    }

    entries.sort_by(|a, b| b.running.cmp(&a.running).then(a.name.cmp(&b.name)));
    Ok(entries)
}

fn normalize_server_info(name: &str) -> Result<Option<(ServerInfo, bool)>> {
    let lock = ServerLock::acquire(name)?;
    let Some(mut info) = lock.load_info()? else {
        return Ok(None);
    };
    let running = is_alive(&info);
    if !running && info.engine == Engine::Clickhouse && info.pid != 0 {
        pause_during_stale_normalization_for_test();
        set_stopped(&mut info);
        lock.save_info(&info)?;
    }
    Ok(Some((info, running)))
}

/// List only currently running servers.
pub fn list_running_servers() -> Result<Vec<ServerInfo>> {
    Ok(list_all_servers()?
        .into_iter()
        .filter(|e| e.running)
        .filter_map(|e| e.info)
        .collect())
}

/// List known ClickHouse server identities, including stopped data directories
/// retained from older versions that may not have metadata.
pub fn list_clickhouse_server_names() -> Result<Vec<String>> {
    let entries = list_all_servers()?;
    let mut clickhouse_names = BTreeSet::new();
    let mut postgres_keys = BTreeSet::new();

    for entry in entries {
        match entry.info.as_ref().map(|info| info.engine) {
            Some(Engine::Postgres) => {
                postgres_keys.insert(entry.name);
            }
            Some(Engine::Clickhouse) | None => {
                clickhouse_names.insert(entry.name);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(servers_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || !path.join("data").is_dir() {
                continue;
            }
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if !postgres_keys.contains(&name) {
                clickhouse_names.insert(name);
            }
        }
    }

    Ok(clickhouse_names.into_iter().collect())
}

/// Check if a named server is currently running.
pub fn is_server_running(name: &str) -> Result<bool> {
    Ok(load_running_info(name)?.is_some())
}

/// Best-effort count used only for the informational notice during start.
pub fn advisory_running_server_count() -> usize {
    list_all_servers_inner(true)
        .map(|entries| entries.iter().filter(|entry| entry.running).count())
        .unwrap_or_default()
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
    ServerLock::acquire(name)?.kill()
}

/// Resolve the server name: use provided name, "default" if none and no default running,
/// or generate a random name if "default" is already running.
/// Returns an error if the provided name contains path traversal characters.
pub fn resolve_name(name: Option<&str>) -> Result<String> {
    match name {
        Some(n) => {
            validate_server_name(n)?;
            Ok(n.to_string())
        }
        None => {
            if is_server_running("default")? {
                generate_random_name()
            } else {
                Ok("default".to_string())
            }
        }
    }
}

fn generate_random_name() -> Result<String> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mixed = seed ^ (std::process::id() as u128);
    let adj = ADJECTIVES[(mixed % ADJECTIVES.len() as u128) as usize];
    let noun = NOUNS[((mixed / ADJECTIVES.len() as u128) % NOUNS.len() as u128) as usize];
    let tag = format!("{}-{}", adj, noun);

    if is_server_running(&tag)? {
        for i in 2..100 {
            let candidate = format!("{}-{}", tag, i);
            if !is_server_running(&candidate)? {
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
        mark_server_stopped(name, child.id())?;
        return Err(Error::StartupExit(format!(
            "Server '{}' exited immediately after starting ({}). See server log: {}",
            name,
            status,
            log_path.display()
        )));
    }
    Ok(())
}

async fn stop_starting_child(child: &mut std::process::Child, name: &str) -> Result<()> {
    let pid = child.id();
    if child
        .try_wait()
        .map_err(|error| Error::Exec(error.to_string()))?
        .is_none()
        && let Err(signal_error) = send_signal(pid, libc::SIGTERM)
        && child
            .try_wait()
            .map_err(|error| Error::Exec(error.to_string()))?
            .is_none()
    {
        return Err(signal_error);
    }

    let deadline = tokio::time::Instant::now() + STARTUP_SHUTDOWN_TIMEOUT;
    loop {
        if child
            .try_wait()
            .map_err(|error| Error::Exec(error.to_string()))?
            .is_some()
        {
            return mark_server_stopped(name, pid);
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }

    child
        .kill()
        .map_err(|error| Error::Exec(error.to_string()))?;
    child
        .wait()
        .map_err(|error| Error::Exec(error.to_string()))?;
    mark_server_stopped(name, pid)
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
            mark_server_stopped(name, child.id())?;
            return Err(Error::StartupExit(format!(
                "Server '{}' exited before becoming ready on HTTP port {} and TCP port {} ({}). \
                 See server log: {}",
                name,
                http_port,
                tcp_port,
                status,
                log_path.display()
            )));
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
                Err(error)
                    if matches!(
                        &error,
                        Error::ServerMetadataRead { .. }
                            | Error::ServerMetadataPermission { .. }
                            | Error::ServerMetadataParse { .. }
                            | Error::ServerMetadataWrite { .. }
                    ) =>
                {
                    return Err(error);
                }
                Err(error) => format!("; failed to stop PID {}: {}", pid, error),
            };
            return Err(Error::StartupTimeout(format!(
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
        Some(p) => {
            return Err(Error::PortInUse(format!(
                "HTTP port {} is already in use",
                p
            )));
        }
        None => {
            if is_port_available(DEFAULT_HTTP_PORT) {
                DEFAULT_HTTP_PORT
            } else {
                find_free_port(DEFAULT_HTTP_PORT + 1)
                    .ok_or_else(|| Error::PortInUse("Could not find a free HTTP port".into()))?
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
        Some(p) => {
            return Err(Error::PortInUse(format!(
                "TCP port {} is already in use",
                p
            )));
        }
        None => {
            if is_port_available(DEFAULT_TCP_PORT) {
                DEFAULT_TCP_PORT
            } else {
                find_free_port(DEFAULT_TCP_PORT + 1)
                    .ok_or_else(|| Error::PortInUse("Could not find a free TCP port".into()))?
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
    let current_dir = std::env::current_dir()
        .and_then(|path| path.canonicalize())?
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
        save_recovered_server_info(&info, true)?;
    }

    // Also recover orphaned Postgres containers belonging to this project.
    docker::recover_project_postgres_blocking(&current_dir)?;
    Ok(())
}

/// Install recovered ClickHouse metadata without racing a normal lifecycle write.
/// Discovery may replace a stale stopped record.
pub fn save_recovered_server_info(info: &ServerInfo, replace_stale: bool) -> Result<()> {
    let lock = ServerLock::acquire(&info.name)?;
    if let Some(existing) = lock.load_info()?
        && (!replace_stale || existing.engine != Engine::Clickhouse || is_alive(&existing))
    {
        return Ok(());
    }
    lock.save_info(info)
}

/// Install recovered Postgres metadata only when no lifecycle record exists.
/// Keep the existence check, data-directory creation, and write under one lock.
pub fn save_recovered_postgres_server_info(
    info: &ServerInfo,
    user_name: &str,
    major: &str,
) -> Result<()> {
    let lock = ServerLock::acquire(&info.name)?;
    if lock.load_info()?.is_some() {
        return Ok(());
    }
    ensure_pg_data_dir(user_name, major)?;
    lock.save_info(info)
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
fn wait_for_test_release(marker_var: &str, release_var: &str) {
    let (Ok(marker), Ok(release)) = (std::env::var(marker_var), std::env::var(release_var)) else {
        return;
    };
    std::fs::write(marker, b"ready").expect("write metadata test marker");
    let release = PathBuf::from(release);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for metadata test release"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
fn pause_before_metadata_rename_for_test() {
    wait_for_test_release(
        "CHCTL_TEST_METADATA_RENAME_READY",
        "CHCTL_TEST_METADATA_RENAME_RELEASE",
    );
}

#[cfg(not(test))]
fn pause_before_metadata_rename_for_test() {}

#[cfg(test)]
fn pause_during_stale_normalization_for_test() {
    wait_for_test_release(
        "CHCTL_TEST_STALE_NORMALIZE_READY",
        "CHCTL_TEST_STALE_NORMALIZE_RELEASE",
    );
}

#[cfg(not(test))]
fn pause_during_stale_normalization_for_test() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::Instant;

    const WRITE_HELPER: &str = "local::server::tests::metadata_write_subprocess";
    const NORMALIZE_HELPER: &str = "local::server::tests::metadata_normalize_subprocess";
    const CONCURRENCY_HELPER: &str = "local::server::tests::metadata_concurrency_subprocess";
    const LIST_HELPER: &str = "local::server::tests::list_all_servers_ignores_json_directories";
    const ADVISORY_COUNT_HELPER: &str =
        "local::server::tests::advisory_count_ignores_unrelated_corrupt_metadata";
    const STRICT_LIST_HELPER: &str = "local::server::tests::strict_list_reports_corrupt_metadata";
    const POSTGRES_RECOVERY_HELPER: &str = "local::server::tests::postgres_recovery_subprocess";
    const CLICKHOUSE_RECOVERY_HELPER: &str = "local::server::tests::clickhouse_recovery_subprocess";

    fn test_info(name: &str, pid: u32, version: &str) -> ServerInfo {
        ServerInfo {
            name: name.to_string(),
            pid,
            version: version.to_string(),
            http_port: 8123,
            tcp_port: 9000,
            started_at: "1700000000".to_string(),
            cwd: "/tmp/project".to_string(),
            engine: Engine::Clickhouse,
            container_id: None,
        }
    }

    fn test_postgres_info(version: &str) -> ServerInfo {
        ServerInfo {
            name: pg_instance_key("default", "17"),
            pid: 0,
            version: version.to_string(),
            http_port: 0,
            tcp_port: 5432,
            started_at: "recovered".to_string(),
            cwd: "/tmp/project".to_string(),
            engine: Engine::Postgres,
            container_id: Some("container-17".to_string()),
        }
    }

    fn metadata_path(project: &Path) -> PathBuf {
        metadata_path_for(project, "default")
    }

    fn metadata_path_for(project: &Path, name: &str) -> PathBuf {
        project.join(format!(".clickhouse/servers/{name}.json"))
    }

    fn write_initial_metadata(project: &Path, info: &ServerInfo) {
        let path = metadata_path_for(project, &info.name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_vec_pretty(info).unwrap()).unwrap();
    }

    fn helper(test_name: &str, project: &Path) -> Command {
        let mut command = Command::new(std::env::current_exe().expect("locate test binary"));
        command
            .args(["--exact", test_name, "--nocapture"])
            .env("CHCTL_TEST_METADATA_PROJECT", project)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    fn wait_for_path(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !path.exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {}",
                path.display()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_success(child: &mut Child) {
        let status = child.wait().expect("wait for metadata helper");
        assert!(status.success(), "metadata helper failed with {status}");
    }

    fn enter_helper_project() -> Option<PathBuf> {
        let project = std::env::var_os("CHCTL_TEST_METADATA_PROJECT")?;
        let project = PathBuf::from(project);
        std::env::set_current_dir(&project).unwrap();
        Some(project)
    }

    #[test]
    fn metadata_write_subprocess() {
        if enter_helper_project().is_none() {
            return;
        }
        let pid = std::env::var("CHCTL_TEST_METADATA_PID")
            .unwrap()
            .parse()
            .unwrap();
        let version = std::env::var("CHCTL_TEST_METADATA_VERSION").unwrap();
        if let Some(attempt) = std::env::var_os("CHCTL_TEST_METADATA_ATTEMPT") {
            std::fs::write(attempt, b"attempt").unwrap();
        }
        save_server_info(&test_info("default", pid, &version)).unwrap();
    }

    #[test]
    fn metadata_normalize_subprocess() {
        if enter_helper_project().is_none() {
            return;
        }
        normalize_server_info("default").unwrap();
    }

    #[test]
    fn metadata_concurrency_subprocess() {
        let Some(_) = enter_helper_project() else {
            return;
        };
        let pid = std::env::var("CHCTL_TEST_METADATA_PID")
            .unwrap()
            .parse()
            .unwrap();
        match std::env::var("CHCTL_TEST_METADATA_OPERATION")
            .unwrap()
            .as_str()
        {
            "lifecycle" => {
                let lock = ServerLock::acquire("default").unwrap();
                wait_for_test_release("CHCTL_TEST_LIFECYCLE_READY", "CHCTL_TEST_LIFECYCLE_RELEASE");
                lock.save_info(&test_info("default", pid, "26.8.1.1"))
                    .unwrap();
            }
            "client" => {
                std::fs::write(
                    std::env::var_os("CHCTL_TEST_METADATA_ATTEMPT").unwrap(),
                    b"attempt",
                )
                .unwrap();
                let state = if load_running_info("default").unwrap().is_some() {
                    b"running".as_slice()
                } else {
                    b"stopped".as_slice()
                };
                std::fs::write(std::env::var_os("CHCTL_TEST_CLIENT_RESULT").unwrap(), state)
                    .unwrap();
            }
            "stop" => {
                std::fs::write(
                    std::env::var_os("CHCTL_TEST_METADATA_ATTEMPT").unwrap(),
                    b"attempt",
                )
                .unwrap();
                kill_server("default").unwrap();
            }
            operation => panic!("unknown metadata test operation {operation}"),
        }
    }

    #[test]
    fn postgres_recovery_subprocess() {
        if enter_helper_project().is_none() {
            return;
        }
        save_recovered_postgres_server_info(
            &test_postgres_info("postgres:17-recovered"),
            "default",
            "17",
        )
        .unwrap();
    }

    #[test]
    fn clickhouse_recovery_subprocess() {
        if enter_helper_project().is_none() {
            return;
        }
        let name = std::env::var("CHCTL_TEST_METADATA_NAME").unwrap();
        save_recovered_server_info(
            &test_info(&name, std::process::id(), "clickhouse-recovered"),
            true,
        )
        .unwrap();
    }

    #[test]
    fn clickhouse_recovery_preserves_postgres_collision_and_replaces_stale_clickhouse() {
        let project = tempfile::tempdir().unwrap();
        let postgres = test_postgres_info("postgres:17-existing");
        let stale_clickhouse = test_info("stale-clickhouse", u32::MAX, "clickhouse-stale");
        write_initial_metadata(project.path(), &postgres);
        write_initial_metadata(project.path(), &stale_clickhouse);

        for name in [&postgres.name, &stale_clickhouse.name] {
            let status = helper(CLICKHOUSE_RECOVERY_HELPER, project.path())
                .env("CHCTL_TEST_METADATA_NAME", name)
                .status()
                .unwrap();
            assert!(
                status.success(),
                "ClickHouse recovery helper failed with {status}"
            );
        }

        let live_postgres: ServerInfo = serde_json::from_slice(
            &std::fs::read(metadata_path_for(project.path(), &postgres.name)).unwrap(),
        )
        .unwrap();
        assert_eq!(live_postgres.version, "postgres:17-existing");
        assert_eq!(live_postgres.engine, Engine::Postgres);
        assert_eq!(live_postgres.container_id.as_deref(), Some("container-17"));

        let live_clickhouse: ServerInfo = serde_json::from_slice(
            &std::fs::read(metadata_path_for(project.path(), &stale_clickhouse.name)).unwrap(),
        )
        .unwrap();
        assert_eq!(live_clickhouse.version, "clickhouse-recovered");
        assert_eq!(live_clickhouse.engine, Engine::Clickhouse);
        assert!(live_clickhouse.container_id.is_none());
    }

    #[test]
    fn postgres_recovery_skips_failing_data_path_when_metadata_exists() {
        let project = tempfile::tempdir().unwrap();
        let servers = project.path().join(".clickhouse/servers");
        let key = pg_instance_key("default", "17");
        let metadata = servers.join(format!("{key}.json"));
        std::fs::create_dir_all(&servers).unwrap();
        std::fs::write(
            &metadata,
            serde_json::to_vec_pretty(&test_postgres_info("postgres:17-existing")).unwrap(),
        )
        .unwrap();
        std::fs::write(servers.join(&key), b"blocks data directory creation").unwrap();

        let status = helper(POSTGRES_RECOVERY_HELPER, project.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Postgres recovery helper failed with {status}"
        );

        let live: ServerInfo = serde_json::from_slice(&std::fs::read(metadata).unwrap()).unwrap();
        assert_eq!(live.version, "postgres:17-existing");
    }

    #[test]
    fn postgres_recovery_creates_data_dir_and_metadata_when_absent() {
        let project = tempfile::tempdir().unwrap();

        let status = helper(POSTGRES_RECOVERY_HELPER, project.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Postgres recovery helper failed with {status}"
        );

        assert!(
            project
                .path()
                .join(".clickhouse/servers/default-pg17/data")
                .is_dir()
        );
        let live: ServerInfo = serde_json::from_slice(
            &std::fs::read(project.path().join(".clickhouse/servers/default-pg17.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(live.version, "postgres:17-recovered");
        assert_eq!(live.engine, Engine::Postgres);
        assert_eq!(live.container_id.as_deref(), Some("container-17"));
    }

    #[test]
    fn list_all_servers_ignores_json_directories() {
        if enter_helper_project().is_some() {
            assert!(list_all_servers().unwrap().is_empty());
            return;
        }

        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join(".clickhouse/servers/foo.json")).unwrap();

        let status = helper(LIST_HELPER, project.path()).status().unwrap();
        assert!(
            status.success(),
            "metadata list helper failed with {status}"
        );
    }

    #[test]
    fn advisory_count_ignores_unrelated_corrupt_metadata() {
        if enter_helper_project().is_some() {
            assert_eq!(advisory_running_server_count(), 1);
            return;
        }

        let project = tempfile::tempdir().unwrap();
        let servers = project.path().join(".clickhouse/servers");
        std::fs::create_dir_all(&servers).unwrap();
        std::fs::write(
            servers.join("healthy.json"),
            serde_json::to_vec_pretty(&test_info("healthy", std::process::id(), "26.8.1.1"))
                .unwrap(),
        )
        .unwrap();
        std::fs::write(servers.join("corrupt.json"), b"not json").unwrap();

        let status = helper(ADVISORY_COUNT_HELPER, project.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "advisory count helper failed with {status}"
        );
    }

    #[test]
    fn strict_list_reports_corrupt_metadata() {
        if enter_helper_project().is_some() {
            assert!(matches!(
                list_all_servers(),
                Err(Error::ServerMetadataParse { name, .. }) if name == "corrupt"
            ));
            return;
        }

        let project = tempfile::tempdir().unwrap();
        let servers = project.path().join(".clickhouse/servers");
        std::fs::create_dir_all(&servers).unwrap();
        std::fs::write(servers.join("corrupt.json"), b"not json").unwrap();

        let status = helper(STRICT_LIST_HELPER, project.path()).status().unwrap();
        assert!(status.success(), "strict list helper failed with {status}");
    }

    #[test]
    fn interrupted_atomic_write_keeps_the_previous_document() {
        let project = tempfile::tempdir().unwrap();
        write_initial_metadata(project.path(), &test_info("default", 0, "old"));
        let ready = project.path().join("rename-ready");
        let release = project.path().join("never-release");
        let mut writer = helper(WRITE_HELPER, project.path())
            .env("CHCTL_TEST_METADATA_PID", "12345")
            .env("CHCTL_TEST_METADATA_VERSION", "new")
            .env("CHCTL_TEST_METADATA_RENAME_READY", &ready)
            .env("CHCTL_TEST_METADATA_RENAME_RELEASE", &release)
            .spawn()
            .unwrap();

        wait_for_path(&ready);
        let live: ServerInfo =
            serde_json::from_slice(&std::fs::read(metadata_path(project.path())).unwrap()).unwrap();
        assert_eq!(live.version, "old");
        writer.kill().unwrap();
        writer.wait().unwrap();

        let live: ServerInfo =
            serde_json::from_slice(&std::fs::read(metadata_path(project.path())).unwrap()).unwrap();
        assert_eq!(live.version, "old");
        let abandoned = std::fs::read_dir(project.path().join(".clickhouse/servers"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(abandoned, 1);
    }

    #[test]
    fn stale_normalizer_cannot_overwrite_a_concurrent_restart() {
        let project = tempfile::tempdir().unwrap();
        write_initial_metadata(project.path(), &test_info("default", u32::MAX, "stale"));
        let ready = project.path().join("normalizer-ready");
        let release = project.path().join("normalizer-release");
        let mut normalizer = helper(NORMALIZE_HELPER, project.path())
            .env("CHCTL_TEST_STALE_NORMALIZE_READY", &ready)
            .env("CHCTL_TEST_STALE_NORMALIZE_RELEASE", &release)
            .spawn()
            .unwrap();
        wait_for_path(&ready);

        let restart_pid = std::process::id();
        let restart_attempt = project.path().join("restart-attempt");
        let mut restart = helper(WRITE_HELPER, project.path())
            .env("CHCTL_TEST_METADATA_PID", restart_pid.to_string())
            .env("CHCTL_TEST_METADATA_VERSION", "restarted")
            .env("CHCTL_TEST_METADATA_ATTEMPT", &restart_attempt)
            .spawn()
            .unwrap();
        wait_for_path(&restart_attempt);
        assert!(restart.try_wait().unwrap().is_none());

        std::fs::write(&release, b"release").unwrap();
        wait_success(&mut normalizer);
        wait_success(&mut restart);

        let live: ServerInfo =
            serde_json::from_slice(&std::fs::read(metadata_path(project.path())).unwrap()).unwrap();
        assert_eq!(live.pid, restart_pid);
        assert_eq!(live.version, "restarted");
    }

    #[test]
    fn concurrent_lifecycle_client_and_stop_observe_complete_metadata() {
        let project = tempfile::tempdir().unwrap();
        write_initial_metadata(project.path(), &test_info("default", 0, ""));
        let mut process = Command::new("sh")
            .args(["-c", "exec sleep 30"])
            .spawn()
            .unwrap();
        let pid = process.id();
        let reaper = std::thread::spawn(move || process.wait().unwrap());
        let ready = project.path().join("lifecycle-ready");
        let release = project.path().join("lifecycle-release");
        let client_result = project.path().join("client-result");
        let client_attempt = project.path().join("client-attempt");
        let stop_attempt = project.path().join("stop-attempt");

        let mut lifecycle = helper(CONCURRENCY_HELPER, project.path())
            .env("CHCTL_TEST_METADATA_OPERATION", "lifecycle")
            .env("CHCTL_TEST_METADATA_PID", pid.to_string())
            .env("CHCTL_TEST_LIFECYCLE_READY", &ready)
            .env("CHCTL_TEST_LIFECYCLE_RELEASE", &release)
            .spawn()
            .unwrap();
        wait_for_path(&ready);
        let mut client = helper(CONCURRENCY_HELPER, project.path())
            .env("CHCTL_TEST_METADATA_OPERATION", "client")
            .env("CHCTL_TEST_METADATA_PID", pid.to_string())
            .env("CHCTL_TEST_CLIENT_RESULT", &client_result)
            .env("CHCTL_TEST_METADATA_ATTEMPT", &client_attempt)
            .spawn()
            .unwrap();
        let mut stop = helper(CONCURRENCY_HELPER, project.path())
            .env("CHCTL_TEST_METADATA_OPERATION", "stop")
            .env("CHCTL_TEST_METADATA_PID", pid.to_string())
            .env("CHCTL_TEST_METADATA_ATTEMPT", &stop_attempt)
            .spawn()
            .unwrap();
        wait_for_path(&client_attempt);
        wait_for_path(&stop_attempt);
        assert!(client.try_wait().unwrap().is_none());
        assert!(stop.try_wait().unwrap().is_none());

        std::fs::write(&release, b"release").unwrap();
        wait_success(&mut lifecycle);
        wait_success(&mut client);
        wait_success(&mut stop);

        let client_state = std::fs::read_to_string(client_result).unwrap();
        assert!(matches!(client_state.as_str(), "running" | "stopped"));
        let live: ServerInfo =
            serde_json::from_slice(&std::fs::read(metadata_path(project.path())).unwrap()).unwrap();
        assert_eq!(live.pid, 0);
        assert!(!reaper.join().unwrap().success());
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
        .unwrap_err();

        assert!(matches!(&error, Error::StartupTimeout(_)));
        let message = error.to_string();
        assert!(message.contains("did not become ready"));
        assert!(message.contains("server.log"));
        assert!(!is_process_alive(pid));
    }

    #[test]
    fn explicit_ports_must_be_available() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let http_error = resolve_ports(Some(port), None).unwrap_err();
        assert!(matches!(&http_error, Error::PortInUse(_)));
        assert!(http_error.to_string().contains("HTTP port"));

        let tcp_error = resolve_ports(None, Some(port)).unwrap_err();
        assert!(matches!(&tcp_error, Error::PortInUse(_)));
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
