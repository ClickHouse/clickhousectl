use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// In-memory snapshot of `.env` values that share the `CLICKHOUSE_` prefix.
///
/// We deliberately do NOT mutate `std::env` to avoid the `unsafe set_var`
/// requirement in edition 2024 — `set_var` races with `getenv` across
/// threads, and tokio has worker threads spun up before our code runs.
/// Callers fold this into the credential resolver as a fallback after the
/// real environment.
#[derive(Debug, Default, Clone)]
pub struct DotenvVars {
    source_path: Option<PathBuf>,
    vars: HashMap<String, String>,
}

impl DotenvVars {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    /// Path of the `.env` file that was actually parsed, if any.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Construct directly from a map. Reserved for unit tests in other
    /// modules that need to feed a synthetic snapshot into the resolver.
    #[cfg(test)]
    pub(crate) fn from_map_for_tests(
        vars: HashMap<String, String>,
        source_path: Option<PathBuf>,
    ) -> Self {
        Self { source_path, vars }
    }
}

/// Parse `dir/.env` if it exists. Only keys starting with `CLICKHOUSE_`
/// are retained.
///
/// Errors are swallowed — missing files, unreadable files, and malformed
/// lines all yield an empty result rather than failing the command. `.env`
/// is strictly opportunistic. We deliberately do NOT walk up to parent
/// directories: an ancestor `.env` could be in a directory the user can't
/// read (or in someone else's home), and surprise credential pickup is
/// worse than the marginal convenience of cross-directory discovery.
pub fn load_from(dir: &Path) -> DotenvVars {
    let path = dir.join(".env");
    if !path.is_file() {
        return DotenvVars::empty();
    }

    let iter = match dotenvy::from_path_iter(&path) {
        Ok(iter) => iter,
        Err(_) => return DotenvVars::empty(),
    };

    let mut vars = HashMap::new();
    for entry in iter {
        let Ok((key, value)) = entry else { continue };
        if key.starts_with("CLICKHOUSE_") {
            vars.insert(key, value);
        }
    }

    DotenvVars {
        source_path: Some(path),
        vars,
    }
}

/// Convenience wrapper that reads `.env` from `std::env::current_dir()`.
pub fn load() -> DotenvVars {
    match std::env::current_dir() {
        Ok(cwd) => load_from(&cwd),
        Err(_) => DotenvVars::empty(),
    }
}

static DOTENV: OnceLock<DotenvVars> = OnceLock::new();

/// Initialize the process-level `.env` snapshot. Idempotent — safe to call
/// more than once; the first call wins.
pub fn init() {
    let _ = DOTENV.set(load());
}

/// Get the process-level `.env` snapshot, parsing on first access if
/// `init()` was never called.
pub fn get() -> &'static DotenvVars {
    DOTENV.get_or_init(load)
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

/// Extract a `<prefix>*` key from a dotenv line, handling optional `export`
/// prefix and whitespace around `=`. Returns the bare key (e.g. "CLICKHOUSE_HOST"
/// for prefix "CLICKHOUSE_") or None if the line isn't a matching assignment.
fn extract_dotenv_key<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let s = line.trim();
    let s = s
        .strip_prefix("export")
        .map(|rest| rest.trim_start())
        .unwrap_or(s);
    let eq_pos = s.find('=')?;
    let key = s[..eq_pos].trim_end();
    if key.starts_with(prefix) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Some(key)
    } else {
        None
    }
}

/// Update an existing .env file: replace `<prefix>*` vars in-place, append any
/// missing ones. Lines for the same prefix that aren't in `vars` are preserved
/// (e.g. a manually-set CLICKHOUSE_PASSWORD survives a host/port-only update).
fn update_dotenv(existing: &str, prefix: &str, vars: &[(&str, String)]) -> String {
    let mut result = String::new();
    let mut written: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for line in existing.lines() {
        if let Some(key) = extract_dotenv_key(line, prefix) {
            if let Some((_, val)) = vars.iter().find(|(k, _)| *k == key) {
                let line_prefix = if line.trim_start().starts_with("export") {
                    "export "
                } else {
                    ""
                };
                result.push_str(&format_dotenv_line(line_prefix, key, val));
                written.insert(key);
            } else {
                // A matching-prefix var we don't manage — keep as-is
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    for (key, val) in vars {
        if !written.contains(key) {
            result.push_str(&format_dotenv_line("", key, val));
            result.push('\n');
        }
    }

    result
}

/// Write `vars` to the dotenv file at `path`. An existing file is edited in
/// place with [`update_dotenv`]; otherwise a new file holds just these vars.
pub(crate) fn write_vars(
    path: &Path,
    prefix: &str,
    vars: &[(&str, String)],
) -> std::io::Result<()> {
    let content = if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        update_dotenv(&existing, prefix, vars)
    } else {
        vars.iter()
            .map(|(k, v)| format_dotenv_line("", k, v))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    std::fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write_env(dir: &Path, contents: &str) {
        let path = dir.join(".env");
        let mut f = fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn finds_env_in_cwd() {
        let dir = tempfile::tempdir().unwrap();
        write_env(dir.path(), "CLICKHOUSE_CLOUD_API_KEY=k\n");
        let loaded = load_from(dir.path());
        assert_eq!(loaded.get("CLICKHOUSE_CLOUD_API_KEY"), Some("k"));
        assert_eq!(
            loaded.source_path(),
            Some(dir.path().join(".env").as_path())
        );
    }

    #[test]
    fn does_not_read_parent_env() {
        // A `.env` in an ancestor directory must NOT be picked up — we look
        // only in the directory passed to `load_from`.
        let parent = tempfile::tempdir().unwrap();
        write_env(parent.path(), "CLICKHOUSE_CLOUD_API_KEY=from_parent\n");
        let child = parent.path().join("child");
        fs::create_dir(&child).unwrap();
        let loaded = load_from(&child);
        assert!(loaded.get("CLICKHOUSE_CLOUD_API_KEY").is_none());
        assert!(loaded.source_path().is_none());
    }

    #[test]
    fn missing_env_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        // No `.env` written.
        let loaded = load_from(dir.path());
        assert!(loaded.get("CLICKHOUSE_CLOUD_API_KEY").is_none());
        assert!(loaded.source_path().is_none());
    }

    #[test]
    fn filters_non_clickhouse_keys() {
        let dir = tempfile::tempdir().unwrap();
        write_env(dir.path(), "OPENAI_KEY=foo\nCLICKHOUSE_CLOUD_API_KEY=bar\n");
        let loaded = load_from(dir.path());
        assert_eq!(loaded.get("CLICKHOUSE_CLOUD_API_KEY"), Some("bar"));
        assert!(loaded.get("OPENAI_KEY").is_none());
    }

    #[test]
    fn malformed_file_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        // No '=' on the first line — dotenvy treats it as an error for that
        // entry but the iterator yields subsequent valid entries. Either way
        // we must not panic.
        write_env(
            dir.path(),
            "this is not a valid env line\nCLICKHOUSE_CLOUD_API_KEY=ok\n",
        );
        let loaded = load_from(dir.path());
        // The well-formed line should still come through.
        assert_eq!(loaded.get("CLICKHOUSE_CLOUD_API_KEY"), Some("ok"));
    }

    #[test]
    fn comments_and_quotes_handled() {
        let dir = tempfile::tempdir().unwrap();
        write_env(
            dir.path(),
            "# a comment\nCLICKHOUSE_CLOUD_API_KEY=\"value with spaces\"\n",
        );
        let loaded = load_from(dir.path());
        assert_eq!(
            loaded.get("CLICKHOUSE_CLOUD_API_KEY"),
            Some("value with spaces")
        );
    }

    #[test]
    fn write_vars_creates_a_file_or_edits_one_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        write_vars(&path, "CLICKHOUSE_", &vars).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\n"
        );

        fs::write(
            &path,
            "APP=1\nCLICKHOUSE_PASSWORD=kept\nCLICKHOUSE_HOST=old\n",
        )
        .unwrap();
        write_vars(&path, "CLICKHOUSE_", &vars).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "APP=1\nCLICKHOUSE_PASSWORD=kept\nCLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\n"
        );
    }

    #[test]
    fn update_dotenv_postgres_prefix_isolates_clickhouse_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\nDATABASE_URL=x\n";
        let vars = vec![
            ("POSTGRES_HOST", "localhost".to_string()),
            ("POSTGRES_PORT", "5432".to_string()),
        ];
        let result = update_dotenv(existing, "POSTGRES_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("CLICKHOUSE_PORT=9000"));
        assert!(result.contains("POSTGRES_HOST=localhost"));
        assert!(result.contains("POSTGRES_PORT=5432"));
    }

    #[test]
    fn extract_dotenv_key_postgres_prefix() {
        assert_eq!(
            extract_dotenv_key("POSTGRES_USER=postgres", "POSTGRES_"),
            Some("POSTGRES_USER")
        );
        assert_eq!(extract_dotenv_key("CLICKHOUSE_HOST=x", "POSTGRES_"), None);
    }

    #[test]
    fn update_dotenv_creates_fresh_content() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv("", "CLICKHOUSE_", &vars);
        assert_eq!(result, "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\n");
    }

    #[test]
    fn update_dotenv_replaces_existing_vars() {
        let existing =
            "CLICKHOUSE_HOST=oldhost\nDATABASE_URL=postgres://...\nCLICKHOUSE_PORT=1234\n";
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
        ];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
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
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
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
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
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
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("export CLICKHOUSE_HOST=localhost"));
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(!result.contains("oldhost"));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_handles_spaces_around_equals() {
        let existing = "CLICKHOUSE_HOST = oldhost\n";
        let vars = vec![("CLICKHOUSE_HOST", "localhost".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(!result.contains("oldhost"));
    }

    #[test]
    fn update_dotenv_handles_export_with_spaces() {
        let existing = "export CLICKHOUSE_PORT = 1234\nDATABASE_URL=postgres://...\n";
        let vars = vec![("CLICKHOUSE_PORT", "9000".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("export CLICKHOUSE_PORT=9000"));
        assert!(result.contains("DATABASE_URL=postgres://..."));
        assert!(!result.contains("1234"));
    }

    #[test]
    fn update_dotenv_preserves_unmanaged_clickhouse_vars() {
        let existing = "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PASSWORD=secret\n";
        // Only updating HOST — PASSWORD should be left alone
        let vars = vec![("CLICKHOUSE_HOST", "newhost".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=newhost"));
        assert!(result.contains("CLICKHOUSE_PASSWORD=secret"));
    }

    #[test]
    fn extract_dotenv_key_simple() {
        assert_eq!(
            extract_dotenv_key("CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_with_export() {
        assert_eq!(
            extract_dotenv_key("export CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_with_spaces() {
        assert_eq!(
            extract_dotenv_key("CLICKHOUSE_HOST = localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
        assert_eq!(
            extract_dotenv_key("export CLICKHOUSE_HOST = localhost", "CLICKHOUSE_"),
            Some("CLICKHOUSE_HOST")
        );
    }

    #[test]
    fn extract_dotenv_key_non_clickhouse() {
        assert_eq!(
            extract_dotenv_key("DATABASE_URL=postgres://...", "CLICKHOUSE_"),
            None
        );
        assert_eq!(extract_dotenv_key("export FOO=bar", "CLICKHOUSE_"), None);
    }

    #[test]
    fn extract_dotenv_key_comment_and_blank() {
        assert_eq!(
            extract_dotenv_key("# CLICKHOUSE_HOST=localhost", "CLICKHOUSE_"),
            None
        );
        assert_eq!(extract_dotenv_key("", "CLICKHOUSE_"), None);
    }

    #[test]
    fn format_dotenv_line_plain_value() {
        assert_eq!(format_dotenv_line("", "KEY", "value"), "KEY=value");
    }

    #[test]
    fn format_dotenv_line_with_prefix() {
        assert_eq!(
            format_dotenv_line("export ", "KEY", "value"),
            "export KEY=value"
        );
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
        let result = update_dotenv("", "CLICKHOUSE_", &vars);
        assert!(result.contains("CLICKHOUSE_HOST=localhost"));
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="my secret#123""#));
    }

    #[test]
    fn update_dotenv_quotes_when_replacing_in_place() {
        let existing = "CLICKHOUSE_PASSWORD=old\n";
        let vars = vec![("CLICKHOUSE_PASSWORD", "new pass".to_string())];
        let result = update_dotenv(existing, "CLICKHOUSE_", &vars);
        assert!(result.contains(r#"CLICKHOUSE_PASSWORD="new pass""#));
    }
}
