//! Exact help snapshots for local Postgres commands (issue #465).

use std::path::PathBuf;
use std::process::Command;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn help(args: &[&str]) -> String {
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .args(args)
        .output()
        .expect("run clickhousectl help");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).expect("help is UTF-8")
}

#[test]
fn postgres_help_matches_snapshot() {
    assert_eq!(
        help(&["local", "postgres", "--help"]),
        include_str!("snapshots/local_postgres_help.txt")
    );
}

#[test]
fn postgres_start_help_matches_snapshot() {
    assert_eq!(
        help(&["local", "postgres", "start", "--help"]),
        include_str!("snapshots/local_postgres_start_help.txt")
    );
}
