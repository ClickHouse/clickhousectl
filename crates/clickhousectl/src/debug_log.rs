//! Internal invocation log for agent-experience experiments.
//!
//! Compiled only with the non-default `debug-log` feature, so no released
//! binary contains it. When [`LOG_PATH_ENV`] names a file, every invocation
//! appends one JSON line recording exactly what the CLI saw: raw argv, working
//! directory, relevant environment, `.env` values, the detected agent, the
//! output mode, and how it ended. Nothing is redacted; credentials passed as
//! flags or environment variables are written as-is.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::{Value, json};

/// File to append to. Unset or empty: nothing is recorded.
pub const LOG_PATH_ENV: &str = "CLICKHOUSECTL_DEBUG_LOG";

/// Environment captured in full: every variable with one of these prefixes.
const ENV_PREFIXES: &[&str] = &["CLICKHOUSE_", "CLICKHOUSECTL_", "CHCTL_"];
/// Other variables that change CLI behaviour.
const ENV_EXACT: &[&str] = &["CI", "DO_NOT_TRACK"];

struct Session {
    path: PathBuf,
    started: Instant,
    entry: serde_json::Map<String, Value>,
}

static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
static WRITTEN: AtomicBool = AtomicBool::new(false);

/// Snapshot the invocation. Call once, before parsing; a no-op unless
/// [`LOG_PATH_ENV`] is set.
pub fn start(argv: &[std::ffi::OsString]) {
    let Some(path) = std::env::var_os(LOG_PATH_ENV).filter(|p| !p.is_empty()) else {
        return;
    };
    let cwd = std::env::current_dir().ok();
    let path = match &cwd {
        Some(cwd) => cwd.join(path),
        None => PathBuf::from(path),
    };
    let dotenv = crate::dotenv::get();
    let entry = initial_entry(
        argv,
        cwd.as_deref(),
        std::env::vars_os(),
        dotenv.source_path(),
        dotenv.iter(),
        is_ai_agent::detect(),
    );
    let _ = SESSION.set(Mutex::new(Session {
        path,
        started: Instant::now(),
        entry,
    }));
}

/// Record the effective output mode once the command is parsed. `None` for
/// commands without a JSON mode.
pub fn set_json_output(json: Option<bool>) {
    with_session(|session| {
        session
            .entry
            .insert("json_output".into(), json.map_or(Value::Null, Value::Bool));
    });
}

/// Append the entry with the exit code the process is about to exit with.
pub fn finish(exit_code: i32) {
    write_once("exit", Some(exit_code));
}

/// Append the entry before an `exec()` handoff replaces the process image,
/// which makes `main`'s exit unreachable. The exit code is the handed-over
/// program's, so it is not known here.
pub fn finish_before_exec() {
    write_once("exec", None);
}

fn with_session(f: impl FnOnce(&mut Session)) {
    if let Some(session) = SESSION.get() {
        let mut session = session.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut session);
    }
}

fn write_once(outcome: &str, exit_code: Option<i32>) {
    with_session(|session| {
        if WRITTEN.swap(true, Ordering::SeqCst) {
            return;
        }
        let mut entry = session.entry.clone();
        entry.insert("outcome".into(), json!(outcome));
        entry.insert("exit_code".into(), json!(exit_code));
        entry.insert(
            "duration_ms".into(),
            json!(session.started.elapsed().as_millis() as u64),
        );
        if let Err(error) = append_line(&session.path, &Value::Object(entry)) {
            eprintln!(
                "Warning: could not write {LOG_PATH_ENV} file {}: {error}",
                session.path.display()
            );
        }
    });
}

fn initial_entry<'a>(
    argv: &[std::ffi::OsString],
    cwd: Option<&Path>,
    env: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    dotenv_path: Option<&Path>,
    dotenv_vars: impl Iterator<Item = (&'a str, &'a str)>,
    agent: Option<is_ai_agent::Agent>,
) -> serde_json::Map<String, Value> {
    let lossy = |s: &std::ffi::OsStr| s.to_string_lossy().into_owned();
    let env: BTreeMap<String, String> = env
        .map(|(k, v)| (lossy(&k), lossy(&v)))
        .filter(|(k, _)| is_relevant_env(k))
        .collect();
    let dotenv_vars: BTreeMap<&str, &str> = dotenv_vars.collect();
    let mut entry = serde_json::Map::new();
    entry.insert(
        "timestamp".into(),
        json!(format!("{:.3}", jiff::Timestamp::now())),
    );
    entry.insert("pid".into(), json!(std::process::id()));
    entry.insert("version".into(), json!(crate::build_info::VERSION));
    entry.insert(
        "argv".into(),
        json!(argv.iter().map(|a| lossy(a)).collect::<Vec<_>>()),
    );
    entry.insert("cwd".into(), json!(cwd.map(|p| lossy(p.as_os_str()))));
    entry.insert("env".into(), json!(env));
    entry.insert(
        "dotenv".into(),
        json!({
            "path": dotenv_path.map(|p| lossy(p.as_os_str())),
            "vars": dotenv_vars,
        }),
    );
    entry.insert(
        "agent".into(),
        match agent {
            Some(agent) => json!({
                "id": agent.id.as_str(),
                "name": agent.name,
                "session_id": agent.session_id,
            }),
            None => Value::Null,
        },
    );
    entry.insert("json_output".into(), Value::Null);
    entry
}

fn is_relevant_env(key: &str) -> bool {
    ENV_EXACT.contains(&key) || ENV_PREFIXES.iter().any(|p| key.starts_with(p))
}

/// One `write` of the whole line to an `O_APPEND` file, so concurrent
/// invocations do not interleave within a line.
fn append_line(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    // The log is unredacted (argv, environment, `.env` values), so a newly
    // created file is readable by its owner only, not the umask default.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn os(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs
            .iter()
            .map(|(k, v)| (OsString::from(k), OsString::from(v)))
            .collect()
    }

    #[test]
    fn entry_keeps_raw_argv_and_only_relevant_env() {
        let argv: Vec<OsString> = ["clickhousectl", "cloud", "--api-secret", "s3cr3t"]
            .iter()
            .map(OsString::from)
            .collect();
        let env = os(&[
            ("CLICKHOUSE_CLOUD_API_KEY", "key"),
            ("CLICKHOUSECTL_AUTO_JSON", "0"),
            ("CHCTL_TERM_TIMEOUT_MS", "5"),
            ("DO_NOT_TRACK", "1"),
            ("PATH", "/usr/bin"),
            ("HOME", "/home/x"),
        ]);
        let entry = initial_entry(
            &argv,
            Some(Path::new("/work")),
            env.into_iter(),
            Some(Path::new("/work/.env")),
            [("CLICKHOUSE_CLOUD_API_SECRET", "dotenv-secret")].into_iter(),
            None,
        );

        assert_eq!(
            entry["argv"],
            json!(["clickhousectl", "cloud", "--api-secret", "s3cr3t"])
        );
        assert_eq!(entry["cwd"], json!("/work"));
        assert_eq!(
            entry["env"],
            json!({
                "CHCTL_TERM_TIMEOUT_MS": "5",
                "CLICKHOUSECTL_AUTO_JSON": "0",
                "CLICKHOUSE_CLOUD_API_KEY": "key",
                "DO_NOT_TRACK": "1",
            })
        );
        assert_eq!(entry["dotenv"]["path"], json!("/work/.env"));
        assert_eq!(
            entry["dotenv"]["vars"]["CLICKHOUSE_CLOUD_API_SECRET"],
            json!("dotenv-secret")
        );
        assert_eq!(entry["agent"], Value::Null);
        assert_eq!(entry["pid"], json!(std::process::id()));
    }

    #[cfg(unix)]
    #[test]
    fn append_line_creates_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debug.jsonl");
        append_line(&path, &json!({"n": 1})).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn append_line_adds_one_line_per_call() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debug.jsonl");
        append_line(&path, &json!({"n": 1})).unwrap();
        append_line(&path, &json!({"n": 2})).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines, vec![json!({"n": 1}), json!({"n": 2})]);
        assert!(text.ends_with('\n'));
    }
}
