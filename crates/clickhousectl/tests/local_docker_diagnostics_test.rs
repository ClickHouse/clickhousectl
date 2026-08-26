//! Subprocess coverage for local Postgres and Docker diagnostics.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run_postgres_start(home: &Path, project: &Path, docker_host: &str) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("DOCKER_HOST", docker_host)
        .current_dir(project)
        .args(["local", "--json", "postgres", "start"])
        .output()
        .expect("run clickhousectl")
}

fn assert_platform_guidance(stderr: &str) {
    #[cfg(target_os = "macos")]
    assert!(
        stderr.contains("On macOS, start Docker Desktop"),
        "{stderr}"
    );
    #[cfg(target_os = "linux")]
    assert!(
        stderr.contains("On Linux, start Docker Engine or Docker Desktop"),
        "{stderr}"
    );
    #[cfg(target_os = "windows")]
    assert!(
        stderr.contains("On Windows, start Docker Desktop or Docker Engine"),
        "{stderr}"
    );
    #[cfg(target_os = "windows")]
    assert!(stderr.contains("named pipe"), "{stderr}");
    #[cfg(not(target_os = "windows"))]
    assert!(stderr.contains("socket"), "{stderr}");
    assert!(stderr.contains("docker context show"), "{stderr}");
    assert!(stderr.contains("DOCKER_HOST"), "{stderr}");
}

#[test]
fn missing_socket_reports_constructor_guidance_without_leaking_endpoint() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker-secret-token.sock");
    let docker_host = format!("unix://{}", socket_path.display());

    let output = run_postgres_start(home.path(), project.path(), &docker_host);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains(
            "Docker is not available: could not initialize Docker client: Docker socket was not found."
        ),
        "{stderr}"
    );
    assert_platform_guidance(&stderr);
    assert!(!stderr.contains("docker-secret-token"), "{stderr}");
}

#[test]
fn permission_denied_socket_reports_ping_guidance_without_leaking_endpoint() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker-secret-token.sock");
    let _listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o000))
        .expect("deny fake Docker socket access");
    let docker_host = format!("unix://{}", socket_path.display());

    let output = run_postgres_start(home.path(), project.path(), &docker_host);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains(
            "Docker is not available: Docker daemon is not reachable: permission denied while opening the Docker socket."
        ),
        "{stderr}"
    );
    assert_platform_guidance(&stderr);
    assert!(!stderr.contains("docker-secret-token"), "{stderr}");
}

#[test]
fn stale_socket_reports_daemon_down_guidance_without_leaking_endpoint() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker-secret-token.sock");
    let listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
    drop(listener);
    let docker_host = format!("unix://{}", socket_path.display());

    let output = run_postgres_start(home.path(), project.path(), &docker_host);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains(
            "Docker is not available: Docker daemon is not reachable: the Docker daemon refused the connection."
        ),
        "{stderr}"
    );
    assert_platform_guidance(&stderr);
    assert!(!stderr.contains("docker-secret-token"), "{stderr}");
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
        .env("HOME", home.path())
        .env("PATH", empty_path)
        .current_dir(project.path())
        .args(["local", "postgres", "client", "--host", "127.0.0.1"])
        .output()
        .expect("run clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Postgres error: could not execute psql:"),
        "{stderr}"
    );
    assert!(!stderr.contains("Failed to execute ClickHouse"), "{stderr}");
}
