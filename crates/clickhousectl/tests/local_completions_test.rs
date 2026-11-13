//! Shell completion through the real binary (#1155). A Tab press runs the
//! binary with `CHCTL_COMPLETE` set; it must answer from the command tree and
//! local state alone, and leave nothing behind: no telemetry notice, marker or
//! event, no update cache, no project state.
//!
//!     cargo test -p clickhousectl --test local_completions_test

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Output;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Ask for bash candidates for the last of `words`, as the bash hook does.
fn tab(home: &Path, cwd: &Path, words: &[&str]) -> Output {
    std::process::Command::new(clickhousectl_binary())
        .arg("--")
        .args(words)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("CHCTL_COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", (words.len() - 1).to_string())
        // Were telemetry to run, it would print its payload instead of sending.
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .env("CHCTL_TELEMETRY_URL", "http://127.0.0.1:9/v1/telemetry")
        .output()
        .expect("failed to spawn binary")
}

fn candidates(output: &Output) -> Vec<String> {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .split('\n')
        .map(str::to_string)
        .collect()
}

/// Every path under `root`, so a test can assert nothing was created.
fn tree(root: &Path) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path.clone());
            }
            paths.insert(path);
        }
    }
    paths
}

#[test]
fn tab_completes_subcommands_under_both_names_and_leaves_no_state() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    for name in ["clickhousectl", "chctl"] {
        let output = tab(home.path(), cwd.path(), &[name, "cloud", "ser"]);
        assert_eq!(candidates(&output), ["service"], "{name}");
    }
    assert!(tree(home.path()).is_empty(), "{:?}", tree(home.path()));
    assert!(tree(cwd.path()).is_empty(), "{:?}", tree(cwd.path()));
}

#[test]
fn tab_completes_local_names_and_versions_without_writing() {
    let home = tempfile::tempdir().unwrap();
    let version = home.path().join(".clickhouse/versions/26.8.1.922");
    std::fs::create_dir_all(&version).unwrap();
    std::fs::write(version.join("clickhouse"), b"").unwrap();

    let project = tempfile::tempdir().unwrap();
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).unwrap();
    for (key, name, engine) in [
        ("dev", "dev", "clickhouse"),
        ("pgdb-pg17", "pgdb", "postgres"),
    ] {
        std::fs::write(
            servers.join(format!("{key}.json")),
            serde_json::json!({
                "name": name, "pid": 0, "version": "", "http_port": 0, "tcp_port": 0,
                "started_at": "1700000000", "cwd": project.path(), "engine": engine,
            })
            .to_string(),
        )
        .unwrap();
    }
    // From a subdirectory, so the project root is discovered as for any command.
    let cwd = project.path().join("src");
    std::fs::create_dir(&cwd).unwrap();
    let before = (tree(home.path()), tree(project.path()));

    // Each name argument offers only its own engine's servers.
    for (words, expected, other) in [
        (
            &["chctl", "local", "clickhouse", "stop", ""][..],
            "dev",
            "pgdb",
        ),
        (&["chctl", "local", "postgres", "stop", ""], "pgdb", "dev"),
        (&["chctl", "local", "remove", "26."], "26.8.1.922", "dev"),
    ] {
        let candidates = candidates(&tab(home.path(), &cwd, words));
        assert!(
            candidates.iter().any(|c| c == expected),
            "{words:?}: {candidates:?}"
        );
        assert!(
            !candidates.iter().any(|c| c == other),
            "{words:?}: {candidates:?}"
        );
    }
    assert_eq!((tree(home.path()), tree(project.path())), before);
}
