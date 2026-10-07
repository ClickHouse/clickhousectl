//! Read a connection from a clickhouse-client config file.
//!
//! Parsing is delegated to ClickHouse's own config processor through
//! `clickhouse extract-from-config`, so includes, `config.d/` overlays,
//! `from_env` substitution and YAML behave exactly as they do for
//! clickhouse-client. This module only applies clickhouse-client's rules for
//! combining top-level settings with a `connections_credentials` entry.

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A connection resolved the way clickhouse-client resolves it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClientConnection {
    pub host: String,
    pub port: Option<u16>,
    pub secure: bool,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub accept_invalid_certificate: bool,
}

/// clickhouse-client's config search order, first existing file wins.
pub fn default_config_candidates(
    cwd: &Path,
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut bases = vec![cwd.join("clickhouse-client")];
    match (xdg_config_home, home) {
        (Some(xdg), _) => bases.push(xdg.join("clickhouse").join("config")),
        (None, Some(home)) => bases.push(home.join(".config").join("clickhouse").join("config")),
        (None, None) => {}
    }
    if let Some(home) = home {
        bases.push(home.join(".clickhouse-client").join("config"));
    }
    bases.push(PathBuf::from("/etc/clickhouse-client/config"));
    bases
        .into_iter()
        .flat_map(|base| {
            ["xml", "yaml", "yml"].map(|ext| {
                let mut path = base.clone().into_os_string();
                path.push(".");
                path.push(ext);
                PathBuf::from(path)
            })
        })
        .collect()
}

pub fn find_default_config() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let home = dirs::home_dir();
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let candidates = default_config_candidates(&cwd, home.as_deref(), xdg.as_deref());
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| {
            Error::Mcp(format!(
                "No clickhouse-client config found. Pass --client-config <PATH>, or create one of: {}",
                candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

/// Look up one key with `clickhouse extract-from-config`. `Ok(None)` means the
/// key is absent; a present but empty element is `Some("")`.
pub fn extract_key(binary: &Path, config: &Path, key: &str) -> Result<Option<String>> {
    let output = Command::new(binary)
        .arg("extract-from-config")
        .arg("--config-file")
        .arg(config)
        .arg("--key")
        .arg(key)
        .output()
        .map_err(|e| Error::Exec(e.to_string()))?;
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout);
        return Ok(Some(value.strip_suffix('\n').unwrap_or(&value).to_string()));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains(&format!("Not found: {key}")) {
        return Ok(None);
    }
    Err(Error::Mcp(format!(
        "Could not read {} with `clickhouse extract-from-config`: {}",
        config.display(),
        stderr.trim()
    )))
}

/// Resolve the connection clickhouse-client would use for `--connection`
/// (or for none). `lookup` returns a config key's value, `None` when absent.
pub fn resolve(
    lookup: &mut dyn FnMut(&str) -> Result<Option<String>>,
    connection: Option<&str>,
) -> Result<ClientConnection> {
    for unsupported in ["jwt", "ssh-key-file"] {
        if lookup(unsupported)?.is_some() {
            return Err(Error::Mcp(format!(
                "The client config uses `{unsupported}` authentication, which the MCP server cannot use. \
                 Use a user and password instead."
            )));
        }
    }

    let mut resolved = ClientConnection {
        host: lookup("host")?.unwrap_or_else(|| "localhost".into()),
        port: lookup("port")?
            .map(|v| parse_port(&v, "port"))
            .transpose()?,
        secure: lookup("secure")?
            .map(|v| parse_bool(&v, "secure"))
            .transpose()?
            .unwrap_or(false),
        user: lookup("user")?,
        password: lookup("password")?,
        database: lookup("database")?,
        accept_invalid_certificate: lookup("accept-invalid-certificate")?
            .map(|v| parse_bool(&v, "accept-invalid-certificate"))
            .transpose()?
            .unwrap_or(false),
    };

    // Like clickhouse-client: without --connection, an entry named after the
    // default host still applies. Every matching entry applies in order.
    let target = connection
        .map(str::to_string)
        .unwrap_or_else(|| resolved.host.clone());
    let mut names = Vec::new();
    let mut found = false;
    for index in 0.. {
        let prefix = format!("connections_credentials.connection[{index}]");
        let Some(name) = lookup(&format!("{prefix}.name"))? else {
            break;
        };
        let is_target = name == target;
        names.push(name);
        if !is_target {
            continue;
        }
        found = true;
        let field = |key: &str| format!("{prefix}.{key}");
        resolved.host = lookup(&field("hostname"))?.unwrap_or_else(|| target.clone());
        if let Some(port) = lookup(&field("port"))? {
            resolved.port = Some(parse_port(&port, &field("port"))?);
        }
        if let Some(secure) = lookup(&field("secure"))? {
            resolved.secure = parse_bool(&secure, &field("secure"))?;
        }
        if let Some(user) = lookup(&field("user"))? {
            resolved.user = Some(user);
        }
        if let Some(password) = lookup(&field("password"))? {
            resolved.password = Some(password);
        }
        if let Some(database) = lookup(&field("database"))? {
            resolved.database = Some(database);
        }
        if let Some(accept) = lookup(&field("accept-invalid-certificate"))? {
            resolved.accept_invalid_certificate =
                parse_bool(&accept, &field("accept-invalid-certificate"))?;
        }
    }

    if let Some(connection) = connection
        && !found
    {
        let available = if names.is_empty() {
            "the config defines no connections".to_string()
        } else {
            format!("available: {}", names.join(", "))
        };
        return Err(Error::Mcp(format!(
            "No connection '{connection}' in connections_credentials ({available})"
        )));
    }

    if resolved.host.contains(',') || resolved.host.split_whitespace().count() > 1 {
        return Err(Error::Mcp(format!(
            "Host '{}' names multiple hosts; the MCP server connects to one",
            resolved.host
        )));
    }

    Ok(resolved)
}

/// The HTTP port matching a native-protocol connection.
pub fn http_port(connection: &ClientConnection, http_port: Option<u16>) -> Result<u16> {
    if let Some(port) = http_port {
        return Ok(port);
    }
    match (connection.port, connection.secure) {
        (None, false) | (Some(9000), false) => Ok(8123),
        (None, true) | (Some(9440), true) => Ok(8443),
        (Some(port), secure) => Err(Error::Mcp(format!(
            "Native port {port}{} has no standard HTTP port. Pass --http-port <PORT>.",
            if secure { " (secure)" } else { "" }
        ))),
    }
}

/// Values follow mcp-clickhouse's meanings, which differ from
/// `local server dotenv`: `CLICKHOUSE_PORT` is the HTTP port, and
/// `CLICKHOUSE_SECURE` and `CLICKHOUSE_PASSWORD` must always be present.
pub fn mcp_env(connection: &ClientConnection, http_port: u16) -> Vec<(&'static str, String)> {
    let mut env = vec![
        ("CLICKHOUSE_HOST", connection.host.clone()),
        ("CLICKHOUSE_PORT", http_port.to_string()),
        ("CLICKHOUSE_SECURE", connection.secure.to_string()),
        (
            "CLICKHOUSE_VERIFY",
            (!connection.accept_invalid_certificate).to_string(),
        ),
        (
            "CLICKHOUSE_USER",
            connection.user.clone().unwrap_or_else(|| "default".into()),
        ),
        (
            "CLICKHOUSE_PASSWORD",
            connection.password.clone().unwrap_or_default(),
        ),
    ];
    if let Some(database) = &connection.database {
        env.push(("CLICKHOUSE_DATABASE", database.clone()));
    }
    env
}

/// Poco's `getBool` spellings.
fn parse_bool(value: &str, key: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(Error::Mcp(format!(
            "Invalid boolean '{value}' for `{key}` in the client config"
        ))),
    }
}

fn parse_port(value: &str, key: &str) -> Result<u16> {
    value.trim().parse().map_err(|_| {
        Error::Mcp(format!(
            "Invalid port '{value}' for `{key}` in the client config"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn resolve_from(pairs: &[(&str, &str)], connection: Option<&str>) -> Result<ClientConnection> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        resolve(&mut |key| Ok(map.get(key).cloned()), connection)
    }

    const TWO_CONNECTIONS: &[(&str, &str)] = &[
        ("user", "topuser"),
        ("password", "toppw"),
        ("connections_credentials.connection[0].name", "prod"),
        (
            "connections_credentials.connection[0].hostname",
            "prod.example.com",
        ),
        ("connections_credentials.connection[0].port", "9440"),
        ("connections_credentials.connection[0].secure", "1"),
        ("connections_credentials.connection[0].user", "admin"),
        ("connections_credentials.connection[0].password", "s3cret"),
        (
            "connections_credentials.connection[0].database",
            "analytics",
        ),
        ("connections_credentials.connection[1].name", "staging"),
    ];

    #[test]
    fn named_connection_overrides_top_level_settings() {
        let resolved = resolve_from(TWO_CONNECTIONS, Some("prod")).unwrap();
        assert_eq!(
            resolved,
            ClientConnection {
                host: "prod.example.com".into(),
                port: Some(9440),
                secure: true,
                user: Some("admin".into()),
                password: Some("s3cret".into()),
                database: Some("analytics".into()),
                accept_invalid_certificate: false,
            }
        );
    }

    #[test]
    fn connection_without_hostname_uses_its_name_and_inherits_top_level() {
        let resolved = resolve_from(TWO_CONNECTIONS, Some("staging")).unwrap();
        assert_eq!(resolved.host, "staging");
        assert_eq!(resolved.user.as_deref(), Some("topuser"));
        assert_eq!(resolved.password.as_deref(), Some("toppw"));
        assert!(!resolved.secure);
    }

    #[test]
    fn no_connection_uses_top_level_settings() {
        let resolved = resolve_from(TWO_CONNECTIONS, None).unwrap();
        assert_eq!(resolved.host, "localhost");
        assert_eq!(resolved.user.as_deref(), Some("topuser"));
        assert_eq!(resolved.port, None);
    }

    #[test]
    fn no_connection_still_applies_an_entry_named_after_the_host() {
        let mut pairs = TWO_CONNECTIONS.to_vec();
        pairs.push(("host", "staging"));
        pairs.push(("connections_credentials.connection[1].user", "stager"));
        let resolved = resolve_from(&pairs, None).unwrap();
        assert_eq!(resolved.host, "staging");
        assert_eq!(resolved.user.as_deref(), Some("stager"));
    }

    #[test]
    fn unknown_connection_lists_available_names() {
        let message = resolve_from(TWO_CONNECTIONS, Some("dev"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("'dev'"), "{message}");
        assert!(message.contains("prod, staging"), "{message}");
    }

    #[test]
    fn unsupported_auth_and_multiple_hosts_are_rejected() {
        assert!(resolve_from(&[("jwt", "token")], None).is_err());
        assert!(resolve_from(&[("ssh-key-file", "~/.ssh/id")], None).is_err());
        assert!(resolve_from(&[("host", "a,b")], None).is_err());
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(resolve_from(&[("port", "http")], None).is_err());
        assert!(resolve_from(&[("secure", "maybe")], None).is_err());
        for spelling in ["1", "true", "Yes", "on"] {
            assert!(
                resolve_from(&[("secure", spelling)], None).unwrap().secure,
                "{spelling}"
            );
        }
    }

    #[test]
    fn http_port_follows_the_standard_native_pairs() {
        let at = |port, secure| ClientConnection {
            port,
            secure,
            ..Default::default()
        };
        assert_eq!(http_port(&at(None, false), None).unwrap(), 8123);
        assert_eq!(http_port(&at(Some(9000), false), None).unwrap(), 8123);
        assert_eq!(http_port(&at(None, true), None).unwrap(), 8443);
        assert_eq!(http_port(&at(Some(9440), true), None).unwrap(), 8443);
        assert!(http_port(&at(Some(9001), false), None).is_err());
        assert!(http_port(&at(Some(9000), true), None).is_err());
        assert_eq!(http_port(&at(Some(9001), false), Some(8124)).unwrap(), 8124);
    }

    #[test]
    fn mcp_env_always_sets_secure_and_password() {
        let env = mcp_env(&ClientConnection::default(), 8123);
        let get = |key| env.iter().find(|(k, _)| *k == key).map(|(_, v)| v.as_str());
        assert_eq!(get("CLICKHOUSE_HOST"), Some(""));
        assert_eq!(get("CLICKHOUSE_PORT"), Some("8123"));
        assert_eq!(get("CLICKHOUSE_SECURE"), Some("false"));
        assert_eq!(get("CLICKHOUSE_VERIFY"), Some("true"));
        assert_eq!(get("CLICKHOUSE_USER"), Some("default"));
        assert_eq!(get("CLICKHOUSE_PASSWORD"), Some(""));
        assert_eq!(get("CLICKHOUSE_DATABASE"), None);

        let env = mcp_env(
            &ClientConnection {
                host: "h".into(),
                secure: true,
                accept_invalid_certificate: true,
                database: Some("db".into()),
                ..Default::default()
            },
            8443,
        );
        let get = |key| env.iter().find(|(k, _)| *k == key).map(|(_, v)| v.as_str());
        assert_eq!(get("CLICKHOUSE_SECURE"), Some("true"));
        assert_eq!(get("CLICKHOUSE_VERIFY"), Some("false"));
        assert_eq!(get("CLICKHOUSE_DATABASE"), Some("db"));
    }

    #[test]
    fn default_config_candidates_follow_clickhouse_client_order() {
        let candidates =
            default_config_candidates(Path::new("/proj"), Some(Path::new("/home/u")), None);
        let shown: Vec<_> = candidates.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(
            shown,
            [
                "/proj/clickhouse-client.xml",
                "/proj/clickhouse-client.yaml",
                "/proj/clickhouse-client.yml",
                "/home/u/.config/clickhouse/config.xml",
                "/home/u/.config/clickhouse/config.yaml",
                "/home/u/.config/clickhouse/config.yml",
                "/home/u/.clickhouse-client/config.xml",
                "/home/u/.clickhouse-client/config.yaml",
                "/home/u/.clickhouse-client/config.yml",
                "/etc/clickhouse-client/config.xml",
                "/etc/clickhouse-client/config.yaml",
                "/etc/clickhouse-client/config.yml",
            ]
        );
        let xdg = default_config_candidates(
            Path::new("/proj"),
            Some(Path::new("/home/u")),
            Some(Path::new("/xdg")),
        );
        assert_eq!(xdg[3], Path::new("/xdg/clickhouse/config.xml"));
    }
}
