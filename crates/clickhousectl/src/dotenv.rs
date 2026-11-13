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
/// is strictly opportunistic.
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

/// The `.env` of `cwd` when it has one, else that of `project_root`. One file
/// wins; the two are never merged.
///
/// Arbitrary ancestors are never searched: surprise credential pickup from a
/// directory the user does not own, or from above their home, is worse than
/// the convenience. The project root is the exception because discovery
/// already vouches for it: the nearest `.clickhouse/` owned by this user, never
/// `~/.clickhouse` and never above home (`init::discover_project_root`).
fn load_for(cwd: &Path, project_root: Option<&Path>) -> DotenvVars {
    if cwd.join(".env").is_file() {
        return load_from(cwd);
    }
    match project_root {
        Some(root) => load_from(root),
        None => DotenvVars::empty(),
    }
}

/// Read `.env` for this invocation: the current directory's, else the project
/// root's. Silent on every failure, since it runs before every command,
/// `--help` included.
pub fn load() -> DotenvVars {
    let Ok(cwd) = std::env::current_dir() else {
        return DotenvVars::empty();
    };
    let root = crate::init::project_root()
        .ok()
        .map(|root| root.dir.as_path());
    load_for(&cwd, root)
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

/// Format a dotenv line, quoting the value by the least powerful form that
/// keeps it literal:
///
/// - A non-empty token of `[A-Za-z0-9_./:@%+-]` is written bare.
/// - Anything else without `'`, `\`, `\n` or `\r` is single-quoted, which
///   dotenvy, python-dotenv, Node dotenv, Docker `--env-file` and POSIX shells
///   all read literally (no `$` substitution, no escapes).
/// - Otherwise it is double-quoted, escaping `\`, `"`, `$` and `\n`. dotenvy
///   has no `\r` escape (`\r` is a parse error), so a carriage return is
///   written raw; inside quotes its line splitter keeps it.
fn format_dotenv_line(prefix: &str, key: &str, val: &str) -> String {
    let is_token = !val.is_empty()
        && val.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(b, b'_' | b'.' | b'/' | b':' | b'@' | b'%' | b'+' | b'-')
        });

    if is_token {
        format!("{}{}={}", prefix, key, val)
    } else if !val
        .bytes()
        .any(|b| matches!(b, b'\'' | b'\\' | b'\n' | b'\r'))
    {
        format!("{}{}='{}'", prefix, key, val)
    } else {
        let escaped = val
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('\n', "\\n");
        format!("{}{}=\"{}\"", prefix, key, escaped)
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

    /// One case per load path: a file here, none, one only in a parent, a
    /// foreign key, a malformed line, and comments with a quoted value.
    #[test]
    fn load_from_reads_only_clickhouse_keys_from_this_directory() {
        for (contents, in_parent, expected) in [
            (Some("CLICKHOUSE_CLOUD_API_KEY=k\n"), false, Some("k")),
            (None, false, None),
            (Some("CLICKHOUSE_CLOUD_API_KEY=from_parent\n"), true, None),
            (
                Some("OPENAI_KEY=foo\nCLICKHOUSE_CLOUD_API_KEY=bar\n"),
                false,
                Some("bar"),
            ),
            (
                Some("this is not a valid env line\nCLICKHOUSE_CLOUD_API_KEY=ok\n"),
                false,
                Some("ok"),
            ),
            (
                Some("# a comment\nCLICKHOUSE_CLOUD_API_KEY=\"value with spaces\"\n"),
                false,
                Some("value with spaces"),
            ),
        ] {
            let parent = tempfile::tempdir().unwrap();
            let dir = parent.path().join("project");
            fs::create_dir(&dir).unwrap();
            let file_dir = if in_parent { parent.path() } else { &dir };
            if let Some(contents) = contents {
                fs::write(file_dir.join(".env"), contents).unwrap();
            }
            let loaded = load_from(&dir);
            assert_eq!(
                loaded.get("CLICKHOUSE_CLOUD_API_KEY"),
                expected,
                "{contents:?}"
            );
            assert!(loaded.get("OPENAI_KEY").is_none());
            let read_a_file = contents.is_some() && !in_parent;
            assert_eq!(
                loaded.source_path(),
                read_a_file.then(|| dir.join(".env")).as_deref(),
                "{contents:?}"
            );
        }
    }

    /// The current directory's file wins; without one the project root's is
    /// read; with neither nothing is.
    #[test]
    fn load_for_prefers_the_current_directory_then_the_project_root() {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("sub");
        fs::create_dir(&cwd).unwrap();
        let key = |vars: &DotenvVars| vars.get("CLICKHOUSE_CLOUD_API_KEY").map(str::to_owned);

        let none = load_for(&cwd, Some(root.path()));
        assert_eq!((key(&none), none.source_path()), (None, None));

        fs::write(root.path().join(".env"), "CLICKHOUSE_CLOUD_API_KEY=root\n").unwrap();
        let from_root = load_for(&cwd, Some(root.path()));
        assert_eq!(key(&from_root).as_deref(), Some("root"));
        assert_eq!(
            from_root.source_path(),
            Some(root.path().join(".env").as_path())
        );
        assert!(load_for(&cwd, None).source_path().is_none());

        // No merging: the root's key is not read once the current directory
        // has a file of its own.
        fs::write(cwd.join(".env"), "CLICKHOUSE_CLOUD_API_SECRET=here\n").unwrap();
        let here = load_for(&cwd, Some(root.path()));
        assert_eq!(key(&here), None);
        assert_eq!(here.get("CLICKHOUSE_CLOUD_API_SECRET"), Some("here"));
        assert_eq!(here.source_path(), Some(cwd.join(".env").as_path()));
    }

    #[test]
    fn dotenv_lines_are_recognised_by_prefix_and_quoted_only_when_needed() {
        for (line, prefix, expected) in [
            (
                "CLICKHOUSE_HOST=localhost",
                "CLICKHOUSE_",
                Some("CLICKHOUSE_HOST"),
            ),
            (
                "export CLICKHOUSE_HOST=localhost",
                "CLICKHOUSE_",
                Some("CLICKHOUSE_HOST"),
            ),
            (
                "CLICKHOUSE_HOST = localhost",
                "CLICKHOUSE_",
                Some("CLICKHOUSE_HOST"),
            ),
            (
                "export CLICKHOUSE_HOST = localhost",
                "CLICKHOUSE_",
                Some("CLICKHOUSE_HOST"),
            ),
            ("POSTGRES_USER=postgres", "POSTGRES_", Some("POSTGRES_USER")),
            ("CLICKHOUSE_HOST=x", "POSTGRES_", None),
            ("DATABASE_URL=postgres://...", "CLICKHOUSE_", None),
            ("export FOO=bar", "CLICKHOUSE_", None),
            ("CLICKHOUSE_A.B=x", "CLICKHOUSE_", None),
            ("# CLICKHOUSE_HOST=localhost", "CLICKHOUSE_", None),
            ("CLICKHOUSE_HOST", "CLICKHOUSE_", None),
            ("", "CLICKHOUSE_", None),
        ] {
            assert_eq!(extract_dotenv_key(line, prefix), expected, "{line:?}");
        }

        for (prefix, value, expected) in [
            ("", "value", "KEY=value"),
            ("export ", "value", "export KEY=value"),
            // Plain ASCII tokens stay bare.
            (
                "",
                "abc.us-east-1.aws.clickhouse.cloud",
                "KEY=abc.us-east-1.aws.clickhouse.cloud",
            ),
            ("", "9440", "KEY=9440"),
            ("", "https://h:8443/a_b", "KEY=https://h:8443/a_b"),
            ("", "user@example.com", "KEY=user@example.com"),
            ("", "a%2Bb+c-d", "KEY=a%2Bb+c-d"),
            // Other values without ' \ \n \r are single-quoted.
            ("", "", "KEY=''"),
            ("", "my secret", "KEY='my secret'"),
            ("", "pass#123", "KEY='pass#123'"),
            ("", r#"a"b"#, r#"KEY='a"b'"#),
            ("", "pa$$word", "KEY='pa$$word'"),
            ("", "a\tb", "KEY='a\tb'"),
            ("", "a;b&c|d", "KEY='a;b&c|d'"),
            ("", "`cmd`", "KEY='`cmd`'"),
            ("", "(x)<y>", "KEY='(x)<y>'"),
            ("", "!~*?=", "KEY='!~*?='"),
            ("", "café", "KEY='café'"),
            // Values with ' \ \n or \r are double-quoted with escapes.
            ("", "it's", r#"KEY="it's""#),
            ("", r#"a"b\c"#, r#"KEY="a\"b\\c""#),
            ("", "line1\nline2", r#"KEY="line1\nline2""#),
            ("", "a\rb", "KEY=\"a\rb\""),
            ("", "it's $HOME", r#"KEY="it's \$HOME""#),
        ] {
            assert_eq!(
                format_dotenv_line(prefix, "KEY", value),
                expected,
                "{value:?}"
            );
        }
    }

    /// Every quoting tier survives a write and a re-read through dotenvy,
    /// which substitutes `$NAME` in bare and double-quoted values, fails on a
    /// bare tab, and has no `\r` escape.
    #[test]
    fn values_round_trip_through_dotenvy() {
        for value in [
            // Bare tokens.
            "localhost",
            "https://user@h:8443/db_1.x",
            "a%2B+b-c",
            // Single-quoted.
            "",
            " lead and trail ",
            "a\tb",
            "a;b",
            "`whoami`",
            "pa$word",
            "${HOME}x",
            "$",
            "a $b c",
            r#"say "hi""#,
            "#hash",
            "&|()<>!~*?=",
            "café",
            // Double-quoted.
            "it's",
            r"back\slash",
            "line1\nline2",
            "a\rb",
            "trailing\r",
            "\r\n",
            "it's $HOME",
            "$a\nb",
            r#"$"\"#,
            "'\t;`\"\\$\r\n",
        ] {
            let line = format_dotenv_line("", "KEY", value);
            let parsed: Vec<(String, String)> = dotenvy::from_read_iter(line.as_bytes())
                .collect::<Result<_, _>>()
                .unwrap_or_else(|e| panic!("{line:?}: {e}"));
            assert_eq!(parsed, [("KEY".to_string(), value.to_string())], "{line:?}");

            let file = format!("BEFORE=1\n{line}\nAFTER=2\n");
            let parsed: Vec<(String, String)> = dotenvy::from_read_iter(file.as_bytes())
                .collect::<Result<_, _>>()
                .unwrap_or_else(|e| panic!("{file:?}: {e}"));
            assert_eq!(
                parsed,
                [
                    ("BEFORE".to_string(), "1".to_string()),
                    ("KEY".to_string(), value.to_string()),
                    ("AFTER".to_string(), "2".to_string()),
                ],
                "{file:?}"
            );
        }
    }

    /// Managed keys are rewritten in place (keeping `export`, dropping spaces
    /// around `=`, quoting values that need it), every other line survives
    /// verbatim, and missing keys are appended in order.
    #[test]
    fn update_dotenv_rewrites_managed_keys_in_place_and_appends_the_rest() {
        let vars = vec![
            ("CLICKHOUSE_HOST", "localhost".to_string()),
            ("CLICKHOUSE_PORT", "9000".to_string()),
            ("CLICKHOUSE_PASSWORD", "my secret#123".to_string()),
        ];
        for (existing, expected) in [
            (
                "",
                "CLICKHOUSE_HOST=localhost\nCLICKHOUSE_PORT=9000\nCLICKHOUSE_PASSWORD='my secret#123'\n",
            ),
            (
                "# keep\nexport CLICKHOUSE_HOST = oldhost\nDATABASE_URL=postgres://...\nCLICKHOUSE_USER=kept\nCLICKHOUSE_PASSWORD=old\n",
                "# keep\nexport CLICKHOUSE_HOST=localhost\nDATABASE_URL=postgres://...\nCLICKHOUSE_USER=kept\nCLICKHOUSE_PASSWORD='my secret#123'\nCLICKHOUSE_PORT=9000\n",
            ),
        ] {
            assert_eq!(update_dotenv(existing, "CLICKHOUSE_", &vars), expected);
        }

        let postgres = [("POSTGRES_HOST", "localhost".to_string())];
        assert_eq!(
            update_dotenv("CLICKHOUSE_HOST=localhost\n", "POSTGRES_", &postgres),
            "CLICKHOUSE_HOST=localhost\nPOSTGRES_HOST=localhost\n"
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
}
