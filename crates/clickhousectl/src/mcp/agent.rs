//! Register an MCP server entry in a coding agent's config.

use super::cli::Source;
use crate::error::{Error, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::path::Path;

/// ClickHouse Cloud's hosted MCP server (OAuth, read-only).
pub const CLOUD_MCP_URL: &str = "https://mcp.clickhouse.cloud/mcp";

/// Claude Code's project-scoped MCP config.
pub const CLAUDE_PROJECT_FILE: &str = ".mcp.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteStatus {
    Created,
    Updated,
    Unchanged,
}

/// The `mcpServers` entry for a target. `None` source registers the hosted
/// Cloud server; any other source launches `clickhousectl mcp run` so the
/// agent config never holds connection settings or credentials.
pub fn server_entry(source: Option<&Source>) -> Value {
    let Some(source) = source else {
        return json!({ "type": "http", "url": CLOUD_MCP_URL });
    };
    let mut args = vec!["mcp".to_string(), "run".to_string()];
    match source {
        Source::Local(name) => {
            args.push("--local".into());
            args.push(name.clone());
        }
        Source::ClientConfig {
            path,
            connection,
            http_port,
        } => {
            args.push("--client-config".into());
            if let Some(path) = path {
                args.push(path.display().to_string());
            }
            if let Some(connection) = connection {
                args.push("--connection".into());
                args.push(connection.clone());
            }
            if let Some(port) = http_port {
                args.push("--http-port".into());
                args.push(port.to_string());
            }
        }
    }
    json!({ "command": "clickhousectl", "args": args })
}

/// Merge `entry` into `mcpServers.<name>` of the JSON document `existing`
/// (absent file: `None`), keeping every other key and server.
pub fn merge_entry(
    existing: Option<&str>,
    name: &str,
    entry: Value,
) -> Result<(String, WriteStatus)> {
    let mut document = match existing {
        None => Value::Object(Map::new()),
        Some(text) if text.trim().is_empty() => Value::Object(Map::new()),
        Some(text) => serde_json::from_str(text)
            .map_err(|e| Error::Mcp(format!("{CLAUDE_PROJECT_FILE} is not valid JSON: {e}")))?,
    };
    let Value::Object(root) = &mut document else {
        return Err(Error::Mcp(format!(
            "{CLAUDE_PROJECT_FILE} must contain a JSON object"
        )));
    };
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(servers) = servers else {
        return Err(Error::Mcp(format!(
            "`mcpServers` in {CLAUDE_PROJECT_FILE} must be a JSON object"
        )));
    };
    let status = match servers.get(name) {
        None => WriteStatus::Created,
        Some(current) if *current == entry => WriteStatus::Unchanged,
        Some(_) => WriteStatus::Updated,
    };
    servers.insert(name.to_string(), entry);
    Ok((serde_json::to_string_pretty(&document)? + "\n", status))
}

pub fn write_claude_project(dir: &Path, name: &str, entry: Value) -> Result<WriteStatus> {
    let path = dir.join(CLAUDE_PROJECT_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let (content, status) = merge_entry(existing.as_deref(), name, entry)?;
    if status != WriteStatus::Unchanged {
        std::fs::write(&path, content)?;
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_launch_mcp_run_with_the_source_flags() {
        assert_eq!(
            server_entry(Some(&Source::Local("dev".into()))),
            json!({ "command": "clickhousectl", "args": ["mcp", "run", "--local", "dev"] })
        );
        assert_eq!(
            server_entry(Some(&Source::ClientConfig {
                path: Some("/c/config.xml".into()),
                connection: Some("prod".into()),
                http_port: Some(8124),
            })),
            json!({ "command": "clickhousectl", "args": [
                "mcp", "run", "--client-config", "/c/config.xml",
                "--connection", "prod", "--http-port", "8124"
            ] })
        );
        assert_eq!(
            server_entry(Some(&Source::ClientConfig {
                path: None,
                connection: None,
                http_port: None,
            })),
            json!({ "command": "clickhousectl", "args": ["mcp", "run", "--client-config"] })
        );
        assert_eq!(
            server_entry(None),
            json!({ "type": "http", "url": CLOUD_MCP_URL })
        );
    }

    #[test]
    fn merge_keeps_other_servers_and_keys() {
        let existing = r#"{"other": 1, "mcpServers": {"github": {"command": "gh"}}}"#;
        let (content, status) =
            merge_entry(Some(existing), "clickhouse", server_entry(None)).unwrap();
        assert_eq!(status, WriteStatus::Created);
        let value: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(value["other"], 1);
        assert_eq!(value["mcpServers"]["github"]["command"], "gh");
        assert_eq!(value["mcpServers"]["clickhouse"]["url"], CLOUD_MCP_URL);
        // Existing order is preserved; the new server is appended.
        let keys: Vec<_> = value["mcpServers"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["github", "clickhouse"]);

        let (_, status) = merge_entry(Some(&content), "clickhouse", server_entry(None)).unwrap();
        assert_eq!(status, WriteStatus::Unchanged);
        let local = server_entry(Some(&Source::Local("default".into())));
        let (_, status) = merge_entry(Some(&content), "clickhouse", local).unwrap();
        assert_eq!(status, WriteStatus::Updated);
    }

    #[test]
    fn merge_creates_a_document_from_nothing() {
        for existing in [None, Some(""), Some("  \n")] {
            let (content, status) = merge_entry(existing, "ch", server_entry(None)).unwrap();
            assert_eq!(status, WriteStatus::Created);
            assert!(content.ends_with('\n'));
            let value: Value = serde_json::from_str(&content).unwrap();
            assert!(value["mcpServers"]["ch"].is_object());
        }
    }

    #[test]
    fn merge_rejects_documents_it_cannot_safely_edit() {
        for existing in ["not json", "[]", r#"{"mcpServers": []}"#] {
            assert!(
                merge_entry(Some(existing), "ch", server_entry(None)).is_err(),
                "{existing}"
            );
        }
    }

    #[test]
    fn write_only_touches_the_file_when_the_entry_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CLAUDE_PROJECT_FILE);
        assert_eq!(
            write_claude_project(dir.path(), "ch", server_entry(None)).unwrap(),
            WriteStatus::Created
        );
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            write_claude_project(dir.path(), "ch", server_entry(None)).unwrap(),
            WriteStatus::Unchanged
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), written);
    }
}
