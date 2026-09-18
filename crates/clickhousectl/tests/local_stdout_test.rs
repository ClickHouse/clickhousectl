//! CLI-owned output and native-client handoffs under a closed stdout reader.
use std::process::{Command, Output};

fn command(home: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
    command
        .env_clear()
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(home)
        .args(args);
    command
}

fn run_closed(command: &mut Command) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    command.stdout(writer).output().unwrap()
}

#[test]
fn closed_stdout_is_quiet_for_local_output_help_and_version() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["local", "list"],
        vec!["local", "list", "--json"],
        vec!["--help"],
        vec!["--version"],
    ] {
        let output = run_closed(&mut command(home.path(), &args));
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
    }
}

#[test]
fn closed_stdout_preserves_local_errors_and_usage_errors() {
    let home = tempfile::tempdir().unwrap();
    for (args, expected) in [
        (vec!["local", "remove", "missing-version"], 1),
        (vec!["local", "remove", "missing-version", "--json"], 1),
        (vec!["local", "missing-command"], 2),
    ] {
        let output = run_closed(&mut command(home.path(), &args));
        assert_eq!(output.status.code(), Some(expected), "{output:?}");
        assert!(!output.stderr.is_empty());
        if args.contains(&"--json") {
            let _: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn closed_stdout_preserves_native_client_exit_status() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let bin = home
        .path()
        .join(".clickhouse/versions/26.8.1.1760/clickhouse");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    // Deliberately handle SIGPIPE inside the native process and return its own
    // status. The wrapper must neither reinterpret nor suppress that status.
    std::fs::write(
        &bin,
        "#!/bin/sh\ntrap '' PIPE\nprintf 'native output\\n' 2>/dev/null\nexit 23\n",
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = run_closed(&mut command(
        home.path(),
        &[
            "local",
            "client",
            "--version",
            "26.8.1.1760",
            "--host",
            "127.0.0.1",
        ],
    ));
    assert_eq!(output.status.code(), Some(23), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[cfg(target_os = "linux")]
#[test]
fn non_broken_pipe_output_failure_is_not_success() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["local", "list"],
        vec!["local", "list", "--json"],
        vec!["--help"],
    ] {
        let output = command(home.path(), &args)
            .stdout(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(!output.stderr.is_empty());
    }
}
