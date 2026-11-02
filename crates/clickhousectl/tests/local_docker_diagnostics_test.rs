//! Subprocess coverage for local Postgres and Docker diagnostics.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run_postgres_start(home: &Path, project: &Path, docker_host: &str, json: bool) -> Output {
    let args: &[&str] = if json {
        &["local", "--json", "postgres", "start"]
    } else {
        &["local", "postgres", "start"]
    };
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home)
        .env("DOCKER_HOST", docker_host)
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

enum SocketState {
    Missing,
    PermissionDenied,
    Stale,
}

/// A real socket in each failure state reaches `classify_docker_failure`
/// through bollard's own error chain, which the unit tests only synthesize.
/// The guidance text itself is unit-tested per platform in `local/docker.rs`.
#[test]
fn unusable_docker_sockets_are_classified_without_leaking_the_endpoint() {
    let running_as_root = unsafe { libc::geteuid() } == 0;
    let mut causes = Vec::new();
    for state in [
        SocketState::Missing,
        SocketState::PermissionDenied,
        SocketState::Stale,
    ] {
        if running_as_root && matches!(state, SocketState::PermissionDenied) {
            continue;
        }
        let home = tempfile::tempdir().expect("create home tempdir");
        let project = tempfile::tempdir().expect("create project tempdir");
        let socket_path = home.path().join("docker-secret-token.sock");
        let _listener = match state {
            SocketState::Missing => None,
            SocketState::PermissionDenied => {
                let listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
                fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o000))
                    .expect("deny fake Docker socket access");
                Some(listener)
            }
            SocketState::Stale => {
                drop(UnixListener::bind(&socket_path).expect("bind fake Docker socket"));
                None
            }
        };
        let docker_host = format!("unix://{}", socket_path.display());

        let output = run_postgres_start(home.path(), project.path(), &docker_host, false);
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        let (first, guidance) = stderr.split_once('\n').expect("cause and guidance lines");
        assert!(guidance.contains("DOCKER_HOST"), "{stderr}");
        assert!(!stderr.contains("docker-secret-token"), "{stderr}");
        causes.push(first.to_owned());

        let json = run_postgres_start(home.path(), project.path(), &docker_host, true);
        assert_eq!(json.status.code(), Some(1));
        let error: serde_json::Value =
            serde_json::from_slice(&json.stderr).expect("parse JSON error");
        assert_eq!(error["error"]["code"], "docker_unavailable", "{error}");
        assert!(!String::from_utf8_lossy(&json.stderr).contains("docker-secret-token"));
    }
    // Each socket state is classified to its own cause.
    let distinct: std::collections::BTreeSet<_> = causes.iter().collect();
    assert_eq!(distinct.len(), causes.len(), "{causes:#?}");
}

#[test]
fn missing_psql_reports_which_program_could_not_run() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let empty_path = home.path().join("empty-path");
    fs::create_dir(&empty_path).expect("create empty PATH directory");

    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home.path())
        .env("PATH", &empty_path)
        .current_dir(project.path())
        .args(["local", "postgres", "client", "--host", "127.0.0.1"])
        .output()
        .expect("run clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("psql"), "{stderr}");

    // The failure comes from the Postgres client path, not the ClickHouse one.
    let json = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home.path())
        .env("PATH", &empty_path)
        .current_dir(project.path())
        .args([
            "local",
            "--json",
            "postgres",
            "client",
            "--host",
            "127.0.0.1",
        ])
        .output()
        .expect("run clickhousectl");
    assert_eq!(json.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&json.stderr).expect("parse JSON error");
    assert_eq!(error["error"]["code"], "postgres_error", "{error}");
}
