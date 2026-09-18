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
}
