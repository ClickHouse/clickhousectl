//! `cloud mcp add` drives the agent's own CLI. Fake agent binaries on PATH
//! record their arguments, so these tests check call order, exit statuses and
//! that stdout holds only clickhousectl's output. stdin is never a terminal
//! here, so Claude's sign-in is always skipped; unit tests cover the other case.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const URL: &str = "https://mcp.clickhouse.cloud/mcp";

/// A fake agent CLI: logs its arguments, prints to stdout, and exits with
/// `FAKE_<SUBCOMMAND>_EXIT` (default 0) for `mcp <subcommand>`.
fn install_fake(bin: &Path, program: &str) {
    let path = bin.join(program);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             echo \"{program} $*\" >> \"$CALL_LOG\"\n\
             echo \"fake {program} output\"\n\
             case \"$2\" in\n\
               get) exit \"${{FAKE_GET_EXIT:-0}}\" ;;\n\
               add) exit \"${{FAKE_ADD_EXIT:-0}}\" ;;\n\
               *) exit \"${{FAKE_LOGIN_EXIT:-0}}\" ;;\n\
             esac\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Run {
    output: Output,
    calls: Vec<String>,
}

fn run(fakes: &[&str], env: &[(&str, &str)], args: &[&str]) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    for program in fakes {
        install_fake(&bin, program);
    }
    let log = dir.path().join("calls.log");
    let output = Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
        .env_clear()
        .env("HOME", dir.path())
        .env("PATH", &bin)
        .env("DO_NOT_TRACK", "1")
        .env("CALL_LOG", &log)
        .envs(env.iter().copied())
        .current_dir(dir.path())
        .args(["cloud", "mcp", "add"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let calls = std::fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    Run { output, calls }
}

fn json(run: &Run) -> serde_json::Value {
    assert!(run.output.status.success(), "{:?}", run.output);
    serde_json::from_slice(&run.output.stdout).expect("stdout is only the JSON result")
}

#[test]
fn claude_without_a_terminal_registers_and_skips_sign_in() {
    let run = run(
        &["claude"],
        &[("FAKE_GET_EXIT", "1")],
        &["claude", "--json"],
    );
    assert_eq!(
        run.calls,
        [
            "claude mcp get clickhouse-cloud".to_owned(),
            format!("claude mcp add --transport http --scope user clickhouse-cloud {URL}"),
        ]
    );
    let value = json(&run);
    assert_eq!(value["agent"], "claude");
    assert_eq!(value["name"], "clickhouse-cloud");
    assert_eq!(value["url"], URL);
    assert_eq!(value["registration"], "added");
    assert_eq!(value["signed_in"], false);
    assert_eq!(value["sign_in_command"], "/mcp");
    // The agent's own output goes to stderr, keeping stdout parseable.
    assert!(String::from_utf8_lossy(&run.output.stderr).contains("fake claude output"));
}

#[test]
fn claude_with_an_existing_entry_skips_add() {
    let run = run(&["claude"], &[], &["claude", "--name", "ch", "--json"]);
    assert_eq!(run.calls, ["claude mcp get ch"]);
    let value = json(&run);
    assert_eq!(value["registration"], "already_registered");
    assert_eq!(value["signed_in"], false);
}

#[test]
fn codex_signs_in_during_add() {
    let run = run(&["codex"], &[], &["codex", "--json"]);
    assert_eq!(
        run.calls,
        [format!("codex mcp add clickhouse-cloud --url {URL}")]
    );
    let value = json(&run);
    assert_eq!(value["registration"], "added");
    assert_eq!(value["signed_in"], true);
}

#[test]
fn gemini_registers_and_returns_the_in_session_sign_in_command() {
    let run = run(&["gemini"], &[], &["gemini", "--json"]);
    assert_eq!(
        run.calls,
        [format!(
            "gemini mcp add --scope user --transport http clickhouse-cloud {URL}"
        )]
    );
    let value = json(&run);
    assert_eq!(value["signed_in"], false);
    assert_eq!(value["sign_in_command"], "/mcp auth clickhouse-cloud");
}

#[test]
fn a_failed_agent_step_passes_its_exit_status_through() {
    let failed_login = run(&["pi"], &[("FAKE_LOGIN_EXIT", "7")], &["pi", "--json"]);
    assert_eq!(failed_login.output.status.code(), Some(7));
    assert_eq!(failed_login.calls.len(), 2);
    assert!(failed_login.output.stdout.is_empty());

    let failed_add = run(&["opencode"], &[("FAKE_ADD_EXIT", "3")], &["opencode"]);
    assert_eq!(failed_add.output.status.code(), Some(3));
    assert_eq!(failed_add.calls.len(), 1, "no sign-in after a failed add");
}

#[test]
fn a_missing_agent_cli_is_an_error() {
    let run = run(&[], &[], &["claude", "--json"]);
    assert_eq!(run.output.status.code(), Some(1));
    assert!(run.output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&run.output.stderr).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("claude")
    );
}
