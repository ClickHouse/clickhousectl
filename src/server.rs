use crate::error::{Error, Result};
use crate::init;
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

/// Metadata saved for each server instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub pid: u32,
    pub version: String,
    pub http_port: u16,
    pub tcp_port: u16,
    pub started_at: String,
    pub cwd: String,
}

/// A server entry shown in list output — may or may not be running.
pub struct ServerEntry {
    pub name: String,
    pub running: bool,
    pub info: Option<ServerInfo>,
}

/// Directory where server tracking files and data live: .clickhousectl/servers/
fn servers_dir() -> PathBuf {
    init::local_dir().join("servers")
}

fn server_meta_path(name: &str) -> PathBuf {
    servers_dir().join(format!("{}.json", name))
}

/// Data directory for a named server: .clickhousectl/servers/<name>/data/
pub fn server_data_dir(name: &str) -> PathBuf {
    servers_dir().join(name).join("data")
}

/// Ensure the data directory for a named server exists.
pub fn ensure_server_data_dir(name: &str) -> Result<()> {
    let dir = servers_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        // Ensure parent .clickhousectl has gitignore
        let local = init::local_dir();
        let gitignore = local.join(".gitignore");
        if !gitignore.exists() {
            let _ = std::fs::write(gitignore, "*\n");
        }
    }
    let data = server_data_dir(name);
    std::fs::create_dir_all(&data)?;
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

/// Load server metadata if it exists and the process is alive.
fn load_running_info(name: &str) -> Option<ServerInfo> {
    let path = server_meta_path(name);
    let content = std::fs::read_to_string(&path).ok()?;
    let info: ServerInfo = serde_json::from_str(&content).ok()?;
    if is_process_alive(info.pid) {
        Some(info)
    } else {
        // Stale — clean up
        let _ = std::fs::remove_file(&path);
        None
    }
}

/// List all known servers (both running and stopped).
/// Discovers servers by scanning data directories in .clickhousectl/servers/.
pub fn list_all_servers() -> Vec<ServerEntry> {
    let dir = servers_dir();
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return entries,
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        // Only look at directories (server data dirs), skip .json files
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Check if this server has a data subdir (sanity check)
        if !server_data_dir(&name).exists() {
            continue;
        }

        let info = load_running_info(&name);
        let running = info.is_some();

        entries.push(ServerEntry {
            name,
            running,
            info,
        });
    }

    // Sort: running first, then alphabetical
    entries.sort_by(|a, b| b.running.cmp(&a.running).then(a.name.cmp(&b.name)));
    entries
}

/// List only running servers.
pub fn list_running_servers() -> Vec<ServerInfo> {
    list_all_servers()
        .into_iter()
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

/// Kill a server by name.
pub fn kill_server(name: &str) -> Result<()> {
    let info = load_running_info(name).ok_or_else(|| Error::ServerNotRunning(name.to_string()))?;

    unsafe {
        libc::kill(info.pid as i32, libc::SIGTERM);
    }

    // Wait briefly for graceful shutdown
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_process_alive(info.pid) {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if is_process_alive(info.pid) {
            unsafe {
                libc::kill(info.pid as i32, libc::SIGKILL);
            }
        }
    }

    remove_server_info(name);
    Ok(())
}

/// Resolve the server name: use provided name, "default" if none and no default running,
/// or generate a random name if "default" is already running.
pub fn resolve_name(name: Option<&str>) -> String {
    match name {
        Some(n) => n.to_string(),
        None => {
            if is_server_running("default") {
                generate_random_name()
            } else {
                "default".to_string()
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
