pub mod cli;
pub mod output;
pub mod server;

use cli::{LocalCommands, ServerCommands};

use crate::error::{Error, Result};
use crate::{init, paths, version_manager};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub async fn run(cmd: LocalCommands, json: bool) -> Result<()> {
    match cmd {
        LocalCommands::Install { version, force } => install(&version, force, json).await,
        LocalCommands::List { remote } => {
            if remote {
                list_available(json).await
            } else {
                list_installed(json)
            }
        }
        LocalCommands::Use { version } => use_version(&version, json).await,
        LocalCommands::Remove { version } => remove(&version, json),
        LocalCommands::Which => which(json),
        LocalCommands::Init => {
            init::init()?;
            let out = output::InitOutput {
                path: ".clickhouse/".to_string(),
            };
            output::print_output(&out, json);
            Ok(())
        }
        LocalCommands::Client {
            name,
            host,
            port,
            query,
            queries_file,
            args,
        } => run_client(name, host, port, query, queries_file, args),
        LocalCommands::Server { command } => run_server_commands(command, json).await,
    }
}

async fn install(version_spec: &str, force: bool, json: bool) -> Result<()> {
    let spec = version_manager::parse_version_spec(version_spec)?;
    let platform = version_manager::platform::Platform::detect()?;

    eprintln!("Resolving {}...", spec);
    let resolved = version_manager::resolve::resolve(&spec, &platform).await?;

    let version = version_manager::install::install_resolved(&resolved, &platform, force).await?;

    // If this is the first installed version, set it as default
    let set_as_default = version_manager::get_default_version().is_err();
    if set_as_default {
        version_manager::set_default_version(&version)?;
        if !json {
            eprintln!("Set as default version");
        }
    }

    let out = output::InstallOutput {
        version,
        set_as_default,
    };
    output::print_output(&out, json);

    Ok(())
}

fn list_installed(json: bool) -> Result<()> {
    let versions = version_manager::list_installed_versions()?;
    let default = version_manager::get_default_version().ok();

    let out = output::ListInstalledOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let is_default = Some(&v) == default.as_ref();
                output::InstalledVersion {
                    version: v,
                    default: is_default,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn list_available(json: bool) -> Result<()> {
    eprintln!("Checking available versions on builds.clickhouse.com...");
    let versions = version_manager::list_available_versions_from_builds().await?;

    let installed = version_manager::list_installed_versions().unwrap_or_default();

    let out = output::ListAvailableOutput {
        versions: versions
            .into_iter()
            .map(|v| {
                let prefix = format!("{}.", v);
                let is_installed = installed
                    .iter()
                    .any(|iv| iv.starts_with(&prefix) || iv == &v);
                output::AvailableVersion {
                    version: v,
                    installed: is_installed,
                }
            })
            .collect(),
    };
    output::print_output(&out, json);

    Ok(())
}

async fn use_version(version_spec: &str, json: bool) -> Result<()> {
    let spec = version_manager::parse_version_spec(version_spec)?;
    let platform = version_manager::platform::Platform::detect()?;

    eprintln!("Resolving {}...", spec);
    let resolved = version_manager::resolve::resolve(&spec, &platform).await?;

    // If exact version is known, check if already installed
    let version = if let Some(ref v) = resolved.exact_version {
        let installed = version_manager::list_installed_versions()?;
        if installed.contains(v) {
            v.clone()
        } else {
            eprintln!("Version {} not installed, installing...", v);
            version_manager::install::install_resolved(&resolved, &platform, false).await?
        }
    } else {
        // Version not known upfront (builds source) — install will detect it
        version_manager::install::install_resolved(&resolved, &platform, false).await?
    };

    version_manager::set_default_version(&version)?;
    let out = output::UseOutput { version };
    output::print_output(&out, json);
    Ok(())
}

fn remove(version: &str, json: bool) -> Result<()> {
    let version_dir = paths::version_dir(version)?;

    if !version_dir.exists() {
        return Err(Error::VersionNotFound(version.to_string()));
    }

    // Check if this is the default version
    if let Ok(default) = version_manager::get_default_version()
        && default == version
    {
        let default_file = paths::default_file()?;
        let _ = std::fs::remove_file(default_file);
    }

    std::fs::remove_dir_all(&version_dir)?;
    let out = output::RemoveOutput {
        version: version.to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn which(json: bool) -> Result<()> {
    let version = version_manager::get_default_version()?;
    let binary = paths::binary_path(&version)?;
    let out = output::WhichOutput {
        version,
        binary_path: binary.display().to_string(),
    };
    output::print_output(&out, json);
    Ok(())
}

fn run_client(
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    query: Option<String>,
    queries_file: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    // If --host or --port is set, connect directly (bypass local server lookup).
    // Otherwise, look up the named server for port and version.
    let (resolved_host, tcp_port, version) = if host.is_some() || port.is_some() {
        let h = host.unwrap_or_else(|| "localhost".to_string());
        let p = port.unwrap_or(9000);
        let v = version_manager::get_default_version()?;
        (h, p, v)
    } else {
        let server_name = name.as_deref().unwrap_or("default");
        let entries = server::list_all_servers();
        let entry = entries
            .iter()
            .find(|e| e.name == server_name)
            .ok_or_else(|| Error::ServerNotFound(server_name.to_string()))?;
        let info = entry
            .info
            .as_ref()
            .ok_or_else(|| Error::ServerNotRunning(server_name.to_string()))?;
        ("localhost".to_string(), info.tcp_port, info.version.clone())
    };

    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    let mut cmd = Command::new(&binary);
    cmd.arg("client")
        .arg("--host")
        .arg(&resolved_host)
        .arg("--port")
        .arg(tcp_port.to_string());

    if let Some(q) = &query {
        cmd.arg("--query").arg(q);
    }

    if let Some(f) = &queries_file {
        cmd.arg("--queries-file").arg(f);
    }

    cmd.args(&args);
    let err = cmd.exec();
    Err(Error::Exec(err.to_string()))
}

async fn start_server(
    name: Option<String>,
    version_spec: Option<String>,
    http_port: Option<u16>,
    tcp_port: Option<u16>,
    foreground: bool,
    args: Vec<String>,
    json: bool,
) -> Result<()> {
    if json && foreground {
        return Err(Error::JsonForegroundConflict);
    }

    // Resolve server name and check for collisions before any downloads
    let server_name = server::resolve_name(name.as_deref());

    if name.is_some() && server::is_server_running(&server_name) {
        return Err(Error::ServerAlreadyRunning(server_name));
    }

    let version = if let Some(spec_str) = &version_spec {
        let spec = version_manager::parse_version_spec(spec_str)?;
        let platform = version_manager::platform::Platform::detect()?;
        eprintln!("Resolving {}...", spec);
        let resolved = version_manager::resolve::resolve(&spec, &platform).await?;
        version_manager::install::ensure_installed(&resolved, &platform).await?
    } else {
        version_manager::get_default_version()?
    };
    let binary = paths::binary_path(&version)?;

    if !binary.exists() {
        return Err(Error::VersionNotFound(version));
    }

    // Show running server count
    let running = server::running_server_count();
    if running > 0 {
        eprintln!(
            "Note: {} server{} already running (use `clickhousectl local server list` to see them)",
            running,
            if running == 1 { "" } else { "s" }
        );
    }

    let (http_port, tcp_port, auto_assigned) = server::resolve_ports(http_port, tcp_port)?;
    if auto_assigned {
        eprintln!(
            "Note: default ports in use, auto-assigned HTTP:{} TCP:{}",
            http_port, tcp_port
        );
    }
    // Check if the user passed their own config in the trailing args (after --).
    // If not, we inject our managed data directory and --path flag.
    // If they did, we leave ClickHouse to use their config as-is.
    let has_config = args
        .iter()
        .any(|a| a.starts_with("--config-file") || a.starts_with("-C"));

    let mut cmd = Command::new(&binary);
    cmd.arg("server");

    if !has_config {
        server::ensure_server_data_dir(&server_name)?;
        cmd.current_dir(server::server_data_dir(&server_name));
        cmd.args(init::server_flags());
    }

    cmd.args(server::port_flags(http_port, tcp_port));
    cmd.args(&args);

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if !foreground {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        let child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
        };
        server::save_server_info(&info)?;

        // Check that it actually started
        server::check_spawn_health(pid, &server_name)?;

        let out = output::ServerStartOutput {
            name: server_name,
            pid,
            http_port,
            tcp_port,
            version,
        };
        output::print_output(&out, json);
        Ok(())
    } else {
        let mut child = cmd.spawn().map_err(|e| Error::Exec(e.to_string()))?;
        let pid = child.id();

        let info = server::ServerInfo {
            name: server_name.clone(),
            pid,
            version: version.clone(),
            http_port,
            tcp_port,
            started_at: server::now_timestamp(),
            cwd,
        };
        server::save_server_info(&info)?;

        eprintln!(
            "Server '{}' running (PID: {}, HTTP: {}, TCP: {})",
            server_name, pid, http_port, tcp_port
        );

        let status = child.wait().map_err(|e| Error::Exec(e.to_string()))?;
        server::remove_server_info(&server_name);

        if !status.success()
            && let Some(code) = status.code()
        {
            std::process::exit(code);
        }
        Ok(())
    }
}

fn dotenv_server(
    name: Option<&str>,
    use_local: bool,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
    json: bool,
) -> Result<()> {
    let server_name = name.unwrap_or("default");
    let entries = server::list_all_servers();
    let entry = entries
        .iter()
        .find(|e| e.name == server_name)
        .ok_or_else(|| Error::ServerNotFound(server_name.to_string()))?;
    let info = entry
        .info
        .as_ref()
        .ok_or_else(|| Error::ServerNotRunning(server_name.to_string()))?;

    // Only write vars we actually know from server metadata.
    // User, password, and database are only included when explicitly provided.
    let mut vars: Vec<(&str, String)> = vec![
        ("CLICKHOUSE_HOST", "localhost".to_string()),
        ("CLICKHOUSE_PORT", info.tcp_port.to_string()),
        ("CLICKHOUSE_HTTP_PORT", info.http_port.to_string()),
    ];
    if let Some(u) = user {
        vars.push(("CLICKHOUSE_USER", u));
    }
    if let Some(p) = password {
        vars.push(("CLICKHOUSE_PASSWORD", p));
    }
    if let Some(d) = database {
        vars.push(("CLICKHOUSE_DATABASE", d));
    }

    let filename = if use_local { ".env.local" } else { ".env" };
    let path = std::path::Path::new(filename);

    let content = if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        update_dotenv(&existing, &vars)
    } else {
        vars.iter()
            .map(|(k, v)| format_dotenv_line("", k, v))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    std::fs::write(path, &content)?;

    let out = output::ServerDotenvOutput {
        file: filename.to_string(),
        server: server_name.to_string(),
        vars: vars
            .into_iter()
            .map(|(k, v)| output::DotenvVar {
                key: k.to_string(),
                value: v,
            })
            .collect(),
    };
    output::print_output(&out, json);
    Ok(())
}

/// Format a dotenv line. Values that are plain alphanumeric tokens are written
/// bare; anything containing spaces, `#`, quotes, backslashes, or newlines is
/// double-quoted with inner `"`, `\`, and newlines escaped.
fn format_dotenv_line(prefix: &str, key: &str, val: &str) -> String {
    let needs_quoting = val.is_empty()
        || val
            .bytes()
            .any(|b| b == b' ' || b == b'#' || b == b'"' || b == b'\'' || b == b'\\' || b == b'\n');

    if needs_quoting {
        let escaped = val
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        format!("{}{}=\"{}\"", prefix, key, escaped)
    } else {
        format!("{}{}={}", prefix, key, val)
    }
}

/// Extract a CLICKHOUSE_* key from a dotenv line, handling optional `export`
/// prefix and whitespace around `=`.
/// Returns the bare key (e.g. "CLICKHOUSE_HOST") or None if the line isn't
/// a CLICKHOUSE_* assignment.
fn extract_dotenv_key(line: &str) -> Option<&str> {
    let s = line.trim();
    let s = s
        .strip_prefix("export")
        .map(|rest| rest.trim_start())
        .unwrap_or(s);
    let eq_pos = s.find('=')?;
    let key = s[..eq_pos].trim_end();
    if key.starts_with("CLICKHOUSE_") && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        Some(key)
    } else {
        None
    }
}

/// Update an existing .env file: replace CLICKHOUSE_* vars in-place, append any missing ones.
fn update_dotenv(existing: &str, vars: &[(&str, String)]) -> String {
    let mut result = String::new();
    let mut written: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for line in existing.lines() {
        if let Some(key) = extract_dotenv_key(line) {
            if let Some((_, val)) = vars.iter().find(|(k, _)| *k == key) {
                let prefix = if line.trim_start().starts_with("export") {
                    "export "
                } else {
                    ""
                };
                result.push_str(&format_dotenv_line(prefix, key, val));
                written.insert(key);
            } else {
                // A CLICKHOUSE_* var we don't manage — keep as-is
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Append any vars that weren't already in the file
    for (key, val) in vars {
        if !written.contains(key) {
            result.push_str(&format_dotenv_line("", key, val));
            result.push('\n');
        }
    }

    result
}

async fn run_server_commands(command: ServerCommands, json: bool) -> Result<()> {
    match command {
        ServerCommands::Start {
            name,
            version,
            http_port,
            tcp_port,
            foreground,
            args,
        } => start_server(name, version, http_port, tcp_port, foreground, args, json).await,
        ServerCommands::List => {
            let entries = server::list_all_servers();
            let running_count = entries.iter().filter(|e| e.running).count();
            let total = entries.len();

            let out = output::ServerListOutput {
                servers: entries
                    .into_iter()
                    .map(|e| {
                        let (pid, version, http_port, tcp_port) = match e.info {
                            Some(info) => (
                                Some(info.pid),
                                Some(info.version),
                                Some(info.http_port),
                                Some(info.tcp_port),
                            ),
                            None => (None, None, None, None),
                        };
                        output::ServerListEntry {
                            name: e.name,
                            running: e.running,
                            pid,
                            version,
                            http_port,
                            tcp_port,
                        }
                    })
                    .collect(),
                total_servers: total,
                total_running_servers: running_count,
            };
            output::print_output(&out, json);
            Ok(())
        }
        ServerCommands::Stop { name } => {
            if !json {
                println!("Stopping server '{}'...", name);
            }
            server::kill_server(&name)?;
            let out = output::ServerStopOutput { name };
            output::print_output(&out, json);
            Ok(())
        }
        ServerCommands::StopAll => {
            let servers = server::list_running_servers();
            let mut stop_entries = Vec::new();
            for s in &servers {
                if !json {
                    print!("Stopping '{}'...", s.name);
                }
                match server::kill_server(&s.name) {
                    Ok(()) => {
                        if !json {
                            println!(" stopped");
                        }
                        stop_entries.push(output::ServerStopEntry {
                            name: s.name.clone(),
                            stopped: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        if !json {
                            println!(" error: {}", e);
                        }
                        stop_entries.push(output::ServerStopEntry {
                            name: s.name.clone(),
                            stopped: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            if json {
                let out = output::ServerStopAllOutput {
                    servers: stop_entries,
                };
                output::print_output(&out, json);
            } else if servers.is_empty() {
                println!("No running servers");
            } else {
                println!("Done");
            }
            Ok(())
        }
        ServerCommands::Dotenv {
            name,
            local,
            user,
            password,
            database,
        } => dotenv_server(name.as_deref(), local, user, password, database, json),
        ServerCommands::Remove { name } => {
            if server::is_server_running(&name) {
                return Err(Error::ServerAlreadyRunning(name));
            }
            let data_dir = server::server_data_dir(&name);
            if !data_dir.exists() {
                return Err(Error::ServerNotFound(name));
            }
            // Remove the whole server directory (parent of data/)
            let server_dir = data_dir.parent().unwrap();
            std::fs::remove_dir_all(server_dir)?;
            server::remove_server_info(&name);
            let out = output::ServerRemoveOutput { name };
            output::print_output(&out, json);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_start_rejects_json_with_foreground() {
        let result = start_server(None, None, None, None, true, vec![], true).await;
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::JsonForegroundConflict),
            "expected JsonForegroundConflict, got: {err}"
        );
    }

    #[test]
    fn update_dotenv_creates_fresh_content() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv("", &vars);
        assert_eq!(result, "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\n");
    }

    #[test]
    fn update_dotenv_replaces_existing_vars() {
        let existing = "CLICKHOUSE_HOST=oldhost\nDATABASE_URL=postgres://...\nCLICKHOUSE_PORT=1234\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
        assert!(result.contains("DATABASE_URL=postgres://..."));
        assert!(!result.contains("oldhost"));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_preserves_non_clickhouse_vars() {
        let existing = "FOO=bar\nBAZ=qux\n";
        let vars = vec![("CLICKHOUSE_HOST", "localhost".to_string())];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("FOO=bar"));
        assert!(result.contains("BAZ=qux"));
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
    }

    #[test]
    fn update_dotenv_appends_missing_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
    }

    #[test]
    fn update_dotenv_handles_export_prefix() {
        let existing = "export CLICKHOUSE_HOST=oldhost\nexport CLICKHOUSE_PORT=1234\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("export CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(!result.contains("oldhost"));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_handles_spaces_around_equals() {
        let existing = "CLICKHOUSE_HOST = oldhost\n";
        let vars = vec![("CLICKHOUSE_HOST", "localhost".to_string())];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(!result.contains("oldhost"));
    }

    #[test]
    fn update_dotenv_handles_export_with_spaces() {
        let existing = "export CLICKHOUSE_PORT = 1234\nDATABASE_URL=postgres://...\n";
        let vars = vec![("CLICKHOUSE_PORT", "9000".to_string())];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(result.contains("DATABASE_URL=postgres://..."));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_preserves_unmanaged_clickhouse_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PASSWORD=secret\n";
        // Only updating HOST — PASSWORD should be left alone
        let vars = vec![("CLICKHOUSE_HOST", "newhost".to_string())];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains("CLICKHOUSE_HOST=newhost"));
        assert!(result.contains("CLICKHOUSE_PASSWORD=secret"));
    }

    #[test]
    fn extract_dotenv_key_simple() {
        assert_eq!(extract_dotenv_key("CLICKHOUSE_HOST=localhost"), Some("CLICKHOUSE_HOST"));
    }

    #[test]
    fn extract_dotenv_key_with_export() {
        assert_eq!(extract_dotenv_key("export CLICKHOUSE_HOST=localhost"), Some("CLICKHOUSE_HOST"));
    }

    #[test]
    fn extract_dotenv_key_with_spaces() {
        assert_eq!(extract_dotenv_key("CLICKHOUSE_HOST = localhost"), Some("CLICKHOUSE_HOST"));
        assert_eq!(
            extract_dotenv_key("export CLICKHOUSE_HOST = localhost"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_non_clickhouse() {
        assert_eq!(extract_dotenv_key("DATABASE_URL=postgres://..."), None);
        assert_eq!(extract_dotenv_key("export FOO=bar"), None);
    }

    #[test]
    fn extract_dotenv_key_comment_and_blank() {
        assert_eq!(extract_dotenv_key("# CLICKHOUSE_HOST=localhost"), None);
        assert_eq!(extract_dotenv_key(""), None);
    }

    #[test]
    fn format_dotenv_line_plain_value() {
        assert_eq!(format_dotenv_line("", "KEY", "value"), "KEY=value");
    }

    #[test]
    fn format_dotenv_line_with_prefix() {
        assert_eq!(format_dotenv_line("export ", "KEY", "value"), "export KEY=value");
    }

    #[test]
    fn format_dotenv_line_quotes_spaces() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "my secret"),
            r#"CLICKHOUSE_PASSWORD="my secret""#
        );
    }

    #[test]
    fn format_dotenv_line_quotes_hash() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "pass#123"),
            r#"CLICKHOUSE_PASSWORD="pass#123""#
        );
    }

    #[test]
    fn format_dotenv_line_escapes_quotes_and_backslashes() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", r#"a"b\c"#),
            r#"CLICKHOUSE_PASSWORD="a\"b\\c""#
        );
    }

    #[test]
    fn format_dotenv_line_escapes_newlines() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", "line1\nline2"),
            r#"CLICKHOUSE_PASSWORD="line1\nline2""#
        );
    }

    #[test]
    fn format_dotenv_line_quotes_empty_value() {
        assert_eq!(
            format_dotenv_line("", "CLICKHOUSE_PASSWORD", ""),
            r#"CLICKHOUSE_PASSWORD="""#
        );
    }

    #[test]
    fn update_dotenv_quotes_special_values() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PASSWORD", "my secret#123".to_string()),
        ];
        let result = update_dotenv("", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="my secret#123""#));
    }

    #[test]
    fn update_dotenv_quotes_when_replacing_in_place() {
        let existing = "CLICKHOUSE_PASSWORD=old\n";
        let vars = vec![("CLICKHOUSE_PASSWORD", "new pass".to_string())];
        let result = update_dotenv(existing, &vars);
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="new pass""#));
    }
}
