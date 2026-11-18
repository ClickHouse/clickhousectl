//! The internal `debug-log` invocation log (`src/debug_log.rs`). The default
//! build must ignore `CLICKHOUSECTL_DEBUG_LOG`; a `--features debug-log` build
//! appends one JSON line per invocation:
//!
//!     cargo test -p clickhousectl --features debug-log --test debug_log_test
use std::path::Path;
use std::process::Command;

fn command(home: &Path, log: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
    command
        .env_clear()
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("CLICKHOUSECTL_AUTO_JSON", "0")
        .env("CLICKHOUSECTL_DEBUG_LOG", log)
        .current_dir(home)
        .args(args);
    command
}

#[cfg(not(feature = "debug-log"))]
#[test]
fn debug_log_default_build_ignores_the_env() {
    let home = tempfile::tempdir().unwrap();
    let log = home.path().join("debug.jsonl");
    let output = command(home.path(), &log, &["local", "list"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!log.exists());
}

#[cfg(feature = "debug-log")]
#[test]
fn debug_log_appends_one_line_per_invocation() {
    let home = tempfile::tempdir().unwrap();
    let log = home.path().join("debug.jsonl");
    let runs: [(&[&str], i32); 3] = [
        (&["local", "list", "--json"], 0),
        (&["--version"], 0),
        (&["no-such-command", "--secret=v"], 2),
    ];
    for (args, code) in runs {
        let output = command(home.path(), &log, args).output().unwrap();
        assert_eq!(output.status.code(), Some(code), "{args:?}: {output:?}");
    }

    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), runs.len());
    for (line, (args, code)) in lines.iter().zip(runs) {
        let argv: Vec<&str> = line["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        assert_eq!(&argv[1..], args);
        assert_eq!(line["exit_code"], code);
        assert_eq!(line["outcome"], "exit");
        assert_eq!(line["env"]["CLICKHOUSECTL_AUTO_JSON"], "0");
        assert!(line["env"].get("HOME").is_none());
        assert!(line["duration_ms"].is_u64());
    }
    assert_eq!(lines[0]["json_output"], true);
    assert_eq!(lines[1]["json_output"], serde_json::Value::Null);
}
