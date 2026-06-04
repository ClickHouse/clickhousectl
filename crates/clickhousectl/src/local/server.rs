use crate::error::{Error, Result};
use crate::init;
use crate::local::discovery;
use crate::local::docker;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_HTTP_PORT: u16 = 8123;
const DEFAULT_TCP_PORT: u16 = 9000;

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
    /// Process PID for ClickHouse; 0 for Postgres (use `container_id` instead).
    pub pid: u32,
    /// ClickHouse version like "25.12.5.44", or "postgres:<tag>" for Postgres.
    pub version: String,
    /// Unused for Postgres (set to 0).
    pub http_port: u16,
    /// TCP port for ClickHouse; mapped host port for Postgres.
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

/// Public alias used by `docker.rs` during orphan recovery — keeps the path
/// computation in one place.
pub fn server_meta_path_for_recovery(name: &str) -> PathBuf {
    server_meta_path(name)
}

/// Data directory for a ClickHouse server: .clickhouse/servers/<name>/data/.
pub fn server_data_dir(name: &str) -> PathBuf {
    servers_dir().join(name).join("data")
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
    servers_dir().join(pg_instance_key(name, major)).join("data")
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

/// Save server info to a metadata file.
pub fn save_server_info(info: &ServerInfo) -> Result<()> {
    let dir = servers_dir();
    std::fs::create_dir_all(&dir)?;
    let path = server_meta_path(&info.name);
    let json = serde_json::to_string_pretty(info)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Remove a server's metadata file.
pub fn remove_server_info(name: &str) {
    let _ = std::fs::remove_file(server_meta_path(name));
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

/// Load server metadata regardless of liveness. Returns None if no metadata
/// file exists or it can't be parsed. `name` is the disk identifier — for
/// ClickHouse this is just the user-facing name, for Postgres it's
/// `<name>-pg<major>` (use `pg_instance_key`).
pub fn load_info(name: &str) -> Option<ServerInfo> {
    let content = std::fs::read_to_string(server_meta_path(name)).ok()?;
    serde_json::from_str(&content).ok()
}

/// Find every Postgres instance whose user-facing name is `name`. Returns
/// one entry per major version that has a metadata file on disk.
pub fn find_pg_instances(name: &str) -> Vec<ServerInfo> {
    let prefix = format!("{}-pg", name);
    let dir = match std::fs::read_dir(servers_dir()) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
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
        if let Some(info) = load_info(stem)
            && info.engine == Engine::Postgres
        {
            out.push(info);
        }
    }
    out
}

/// Load server metadata only if the underlying process/container is alive.
/// Does **not** auto-delete stale metadata — `list_all_servers` is the single
/// place that GCs ClickHouse entries whose PID is gone, so callers like
/// `is_server_running` and `resolve_name` can read the metadata without side
/// effects.
fn load_running_info(name: &str) -> Option<ServerInfo> {
    let info = load_info(name)?;
    if is_alive(&info) { Some(info) } else { None }
}

/// List all known servers (both running and stopped).
///
/// Scans `.clickhouse/servers/*.json` for metadata. Each metadata file is one
/// entry — for ClickHouse the disk id is the user-facing name; for Postgres
/// it's `<name>-pg<major>`. Also runs process/container discovery so
/// orphaned instances reappear.
pub fn list_all_servers() -> Vec<ServerEntry> {
    recover_current_project_servers();

    let dir = servers_dir();
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return entries,
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        let fname = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let stem = match fname.strip_suffix(".json") {
            Some(s) => s,
            None => continue,
        };
        if !path.is_file() {
            continue;
        }

        let info = load_info(stem);
        let running = match &info {
            Some(i) => is_alive(i),
            None => false,
        };

        // GC stale ClickHouse metadata: process is gone for good. Postgres
        // entries stay so `start` can resume the existing container.
        if let Some(i) = &info
            && !running
            && i.engine == Engine::Clickhouse
        {
            let _ = std::fs::remove_file(server_meta_path(stem));
            entries.push(ServerEntry {
                name: stem.to_string(),
                running: false,
                info: None,
            });
            continue;
        }

        entries.push(ServerEntry {
            name: stem.to_string(),
            running,
            info,
        });
    }

    entries.sort_by(|a, b| b.running.cmp(&a.running).then(a.name.cmp(&b.name)));
    entries
}

/// List only currently running servers.
pub fn list_running_servers() -> Vec<ServerInfo> {
    list_all_servers()
        .into_iter()
        .filter(|e| e.running)
        .filter_map(|e| e.info)
        .collect()
}

/// Check if a named server is currently running.
pub fn is_server_running(name: &str) -> bool {
    load_running_info(name).is_some()
}

/// Count running servers.
pub fn running_server_count() -> usize {
    list_running_servers().len()
}

fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
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
/// * ClickHouse: SIGTERM (then SIGKILL on timeout); metadata is removed
///   because the process is gone for good.
/// * Postgres: stops the container only — does **not** remove it, and keeps
///   the metadata file so a subsequent `start` resumes the same container
///   (preserving the password and any other PGDATA-encoded settings).
pub fn kill_server(name: &str) -> Result<()> {
    let info = load_running_info(name).ok_or_else(|| Error::ServerNotRunning(name.to_string()))?;

    match info.engine {
        Engine::Clickhouse => {
            kill_process(info.pid)?;
            remove_server_info(name);
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
    match name {
        Some(n) => {
            validate_server_name(n)?;
            Ok(n.to_string())
        }
        None => {
            if is_server_running("default") {
                Ok(generate_random_name())
            } else {
                Ok("default".to_string())
            }
        }
    }
}

fn generate_random_name() -> String {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mixed = seed ^ (std::process::id() as u128);
    let adj = ADJECTIVES[(mixed % ADJECTIVES.len() as u128) as usize];
    let noun = NOUNS[((mixed / ADJECTIVES.len() as u128) % NOUNS.len() as u128) as usize];
    let tag = format!("{}-{}", adj, noun);

    if is_server_running(&tag) {
        for i in 2..100 {
            let candidate = format!("{}-{}", tag, i);
            if !is_server_running(&candidate) {
                return candidate;
            }
        }
    }
    tag
}

/// Wait a moment after spawn and check if the process is still alive.
/// Returns Ok if alive, Err with details if it died immediately.
pub fn check_spawn_health(pid: u32, name: &str) -> Result<()> {
    // Give it a moment to start (or fail)
    std::thread::sleep(std::time::Duration::from_millis(300));

    if !is_process_alive(pid) {
        remove_server_info(name);
        return Err(Error::Exec(format!(
            "Server '{}' exited immediately after starting. \
             Check if another server is using the same ports, \
             or run in foreground to see the error output.",
            name
        )));
    }
    Ok(())
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
        Some(p) => p,
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
        Some(p) => p,
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
pub fn recover_current_project_servers() {
    let current_dir = match std::env::current_dir().and_then(|p| p.canonicalize()) {
        Ok(p) => p.display().to_string(),
        Err(_) => return,
    };

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

        // Only recover if we don't already have metadata for this server
        if load_running_info(&proc.server_name).is_some() {
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
        let _ = save_server_info(&info);
    }

    // Also recover orphaned Postgres containers belonging to this project.
    docker::recover_project_postgres_blocking(&current_dir);
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

    #[test]
    fn engine_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Engine::Clickhouse).unwrap(), "\"clickhouse\"");
        assert_eq!(serde_json::to_string(&Engine::Postgres).unwrap(), "\"postgres\"");
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
}
