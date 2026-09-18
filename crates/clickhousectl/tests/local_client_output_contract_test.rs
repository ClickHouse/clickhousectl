//! Local clients hand SQL formatting to the native child in every wrapper output mode.

use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::process::{Command, Output, Stdio};

const VERSION: &str = "26.8.1.1760";

#[derive(Clone, Copy, Debug)]
enum Mode {
    Human,
    Json,
    Agent,
}

const MODES: [Mode; 3] = [Mode::Human, Mode::Json, Mode::Agent];

fn command(project: &Path, home: &Path, mode: Mode, postgres: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
    cmd.env_clear()
        .env("HOME", home)
        .env("PATH", home.join("bin"))
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .arg("local");
    match mode {
        Mode::Human => {}
        Mode::Json => {
            cmd.arg("--json");
        }
        Mode::Agent => {
            cmd.env("AI_AGENT", "1");
        }
    }
    if postgres {
        cmd.arg("postgres");
    }
    cmd.args(["client", "--host", "127.0.0.1"]);
    cmd
}

fn install(home: &Path, name: &str, binary: Option<&Path>) {
    let path = if name == "clickhouse" {
        home.join(".clickhouse/versions").join(VERSION).join(name)
    } else {
        home.join("bin").join(name)
    };
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    if let Some(binary) = binary {
        symlink(binary, path).unwrap();
    } else {
        std::fs::write(&path, "#!/bin/sh\nprintf '%s\\0' \"$@\"\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn json_and_agent_modes_leave_native_argv_unchanged() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    install(home.path(), "clickhouse", None);
    install(home.path(), "psql", None);
    for postgres in [false, true] {
        let base = if postgres {
            vec![
                "-h",
                "127.0.0.1",
                "-p",
                "5432",
                "-U",
                "postgres",
                "-d",
                "postgres",
            ]
        } else {
            vec!["client", "--host", "127.0.0.1", "--port", "9000"]
        };
        for (input, child) in [
            (vec![], vec![]), // Interactive invocation: no query source injected.
            (
                vec!["--query", "SELECT 'x'"],
                vec![if postgres { "-c" } else { "--query" }, "SELECT 'x'"],
            ),
            (
                vec!["--queries-file", "queries.sql"],
                vec![
                    if postgres { "-f" } else { "--queries-file" },
                    "queries.sql",
                ],
            ),
        ] {
            let formats = if postgres {
                vec![vec![], vec!["--csv"]]
            } else {
                vec![
                    vec![],
                    vec!["--format", "CSV"],
                    vec!["--output-format", "JSONEachRow"],
                ]
            };
            for format in formats {
                for mode in MODES {
                    let mut cmd = command(project.path(), home.path(), mode, postgres);
                    cmd.args(&input);
                    if !format.is_empty() {
                        cmd.arg("--").args(&format);
                    }
                    let output = cmd.output().unwrap();
                    assert!(output.status.success(), "{mode:?}: {output:?}");
                    let actual: Vec<_> = std::str::from_utf8(&output.stdout)
                        .unwrap()
                        .split_terminator('\0')
                        .collect();
                    let expected: Vec<_> =
                        base.iter().chain(&child).chain(&format).copied().collect();
                    assert_eq!(
                        actual, expected,
                        "{mode:?}, postgres={postgres}, input={input:?}"
                    );
                }
            }
        }
    }
}

fn output_with_stdin(mut cmd: Command, stdin: &str) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    output
}

/// Run against an explicitly provided disposable server; no existing service is mutated.
/// CHCTL_TEST_NATIVE_BINARY=/absolute/path/clickhouse CHCTL_TEST_NATIVE_PORT=19000
/// cargo test -p clickhousectl --test local_client_output_contract_test -- --ignored
#[test]
#[ignore = "requires CHCTL_TEST_NATIVE_BINARY and CHCTL_TEST_NATIVE_PORT for a disposable server"]
fn real_clickhouse_output_stays_native_in_every_wrapper_mode() {
    let binary =
        std::env::var_os("CHCTL_TEST_NATIVE_BINARY").expect("set CHCTL_TEST_NATIVE_BINARY");
    let port = std::env::var("CHCTL_TEST_NATIVE_PORT").expect("set CHCTL_TEST_NATIVE_PORT");
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    install(home.path(), "clickhouse", Some(Path::new(&binary)));
    std::fs::write(
        project.path().join("queries.sql"),
        "SELECT 'one' AS value; SELECT 'two' AS value;",
    )
    .unwrap();
    for mode in MODES {
        for (args, stdin, expected) in [
            (vec!["--query", "SELECT 'one' AS value"], "", "one\n"),
            (vec!["--queries-file", "queries.sql"], "", "one\ntwo\n"),
            (vec![], "SELECT 'one' AS value;\n", "one\n"),
            (
                vec!["--query", "SELECT 'one' AS value", "--", "--format", "CSV"],
                "",
                "\"one\"\n",
            ),
            (
                vec![
                    "--queries-file",
                    "queries.sql",
                    "--",
                    "--output-format",
                    "JSONEachRow",
                ],
                "",
                "{\"value\":\"one\"}\n{\"value\":\"two\"}\n",
            ),
            (
                vec!["--query", "SELECT 'one' AS value FORMAT CSV"],
                "",
                "\"one\"\n",
            ),
        ] {
            let mut cmd = command(project.path(), home.path(), mode, false);
            cmd.args(["--port", &port]).args(&args);
            let output = output_with_stdin(cmd, stdin);
            assert_eq!(output.stdout, expected.as_bytes(), "{mode:?}: {args:?}");
        }
        // Native precedence belongs to ClickHouse: compare the selected binary directly
        // instead of requiring every ClickHouse version to resolve competing formats alike.
        let native_args = [
            "--query",
            "SELECT 'one' AS value FORMAT CSV",
            "--format",
            "JSONEachRow",
        ];
        let mut native = Command::new(&binary);
        native
            .env_clear()
            .env("HOME", home.path())
            .current_dir(project.path())
            .args(["client", "--host", "127.0.0.1", "--port", &port])
            .args(native_args);
        let expected = output_with_stdin(native, "");
        let mut wrapped = command(project.path(), home.path(), mode, false);
        wrapped.args(["--port", &port, "--"]).args(native_args);
        let actual = output_with_stdin(wrapped, "");
        assert_eq!(
            actual.stdout, expected.stdout,
            "native precedence: {mode:?}"
        );

        // A PTY exercises the actual native interactive loop, rather than stdin batch mode.
        let mut cmd = command(project.path(), home.path(), mode, false);
        cmd.env("TERM", "dumb").args([
            "--port",
            &port,
            "--",
            "--format",
            "CSV",
            "--prompt",
            "contract-ready> ",
            "--history_file",
            "/dev/null",
            "--disable_suggestion",
            "--highlight",
            "0",
            "--progress",
            "0",
        ]);
        let stdout = interactive_output(cmd);
        assert!(
            stdout.contains("\"interactive-contract\""),
            "{mode:?}: {stdout}"
        );
    }
}

fn interactive_output(mut cmd: Command) -> String {
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty writes two owned descriptors; null optional arguments use defaults.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(result, 0, "openpty: {}", std::io::Error::last_os_error());
    // SAFETY: successful openpty returned fresh descriptors, each adopted exactly once.
    let mut master = unsafe { std::fs::File::from_raw_fd(master) };
    let slave = unsafe { std::fs::File::from_raw_fd(slave) };
    cmd.stdin(slave.try_clone().unwrap())
        .stdout(slave.try_clone().unwrap())
        .stderr(slave);
    let mut child = cmd.spawn().unwrap();
    drop(cmd); // Close the parent's copies of the slave so the reader sees EOF after exit.
    let mut reader = master.try_clone().unwrap();
    let (ready, received) = std::sync::mpsc::channel();
    let output = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    bytes.extend_from_slice(&buffer[..n]);
                    let text = String::from_utf8_lossy(&bytes);
                    if text
                        .split_once("\"interactive-contract\"")
                        .is_some_and(|(_, tail)| tail.contains("contract-ready> "))
                    {
                        let _ = ready.send(());
                    }
                }
                Err(e) if e.raw_os_error() == Some(libc::EIO) => break, // Linux PTY EOF
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => panic!("read PTY: {e}"),
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    });
    // Concatenation makes the result distinguishable from terminal-echoed SQL.
    master
        .write_all(b"SELECT concat('interactive-', 'contract') AS value;\n")
        .unwrap();
    if received
        .recv_timeout(std::time::Duration::from_secs(15))
        .is_err()
    {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("interactive result timed out: {}", output.join().unwrap());
    }
    // Send exit separately: native paste handling otherwise parses it as a SQL statement.
    master.write_all(b"exit\n").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("interactive client timed out: {}", output.join().unwrap());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = output.join().unwrap();
    assert!(status.success(), "{status}: {output}");
    output
}
