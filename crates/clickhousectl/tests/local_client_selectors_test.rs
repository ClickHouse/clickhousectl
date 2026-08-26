//! End-to-end coverage for local client selectors and query inputs (issues #466, #469, and #470).

use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};

const VERSION_A: &str = "25.12.9.61";
const VERSION_B: &str = "26.1.2.3";
const MISSING_VERSION: &str = "24.8.99.1";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_fake_clickhouse(home: &Path, version: &str) {
    let binary = home
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(
        &binary,
        format!("#!/bin/sh\nprintf 'binary={version}\\n'\nprintf '%s\\n' \"$@\"\n"),
    )
    .expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn set_default(home: &Path, version: &str) {
    let base = home.join(".clickhouse");
    std::fs::create_dir_all(&base).expect("create ClickHouse home");
    std::fs::write(base.join("default"), version).expect("write default version");
}

fn write_server_metadata(project: &Path, name: &str, pid: u32, version: &str, tcp_port: u16) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create servers dir");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "pid": pid,
            "version": version,
            "http_port": 8123,
            "tcp_port": tcp_port,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .expect("write server metadata");
}

struct ProcessGuard(Child);

impl ProcessGuard {
    fn id(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_managed_clickhouse(project: &Path, name: &str) -> ProcessGuard {
    let data = project.join(".clickhouse/servers").join(name).join("data");
    std::fs::create_dir_all(&data).expect("create server data directory");
    let binary = project.join("clickhouse");
    std::fs::write(
        &binary,
        b"#!/bin/sh\ntrap 'exit 0' TERM INT\nwhile :; do sleep 1; done\n",
    )
    .expect("write fake managed ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&binary, permissions).expect("make fake ClickHouse executable");
    ProcessGuard(
        Command::new(binary)
            .current_dir(data)
            .spawn()
            .expect("spawn fake managed ClickHouse"),
    )
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn assert_client(output: Output, version: &str, expected_args: &[&str]) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<_> = String::from_utf8(output.stdout)
        .expect("fake ClickHouse output should be UTF-8")
        .lines()
        .map(str::to_string)
        .collect();
    let mut expected = vec![format!("binary={version}")];
    expected.extend(expected_args.iter().map(|arg| (*arg).to_string()));
    assert_eq!(lines, expected);
}

#[test]
fn clickhouse_client_preserves_native_query_argv_across_supported_versions() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    install_fake_clickhouse(home.path(), VERSION_B);

    let cases = [
        (
            &[][..],
            &["client", "--host", "db.example", "--port", "9000"][..],
        ),
        (
            &["--query", "SELECT 1"][..],
            &[
                "client",
                "--host",
                "db.example",
                "--port",
                "9000",
                "--query",
                "SELECT 1",
            ][..],
        ),
        (
            &[
                "--query",
                "SELECT 1",
                "-q",
                "",
                "--query",
                "SELECT 3",
                "--",
                "--format",
                "JSONEachRow",
            ][..],
            &[
                "client",
                "--host",
                "db.example",
                "--port",
                "9000",
                "--query",
                "SELECT 1",
                "--query",
                "",
                "--query",
                "SELECT 3",
                "--format",
                "JSONEachRow",
            ][..],
        ),
        (
            &["--queries-file", "schema.sql"][..],
            &[
                "client",
                "--host",
                "db.example",
                "--port",
                "9000",
                "--queries-file",
                "schema.sql",
            ][..],
        ),
        (
            &[
                "--queries-file",
                "schema.sql",
                "seed.sql",
                "--queries-file",
                "",
                "verify.sql",
                "--",
                "--echo",
            ][..],
            &[
                "client",
                "--host",
                "db.example",
                "--port",
                "9000",
                "--queries-file",
                "schema.sql",
                "seed.sql",
                "",
                "verify.sql",
                "--echo",
            ][..],
        ),
    ];

    for version in [VERSION_A, VERSION_B] {
        for (input, expected) in cases {
            let mut args = vec![
                "local",
                "client",
                "--host",
                "db.example",
                "--version",
                version,
            ];
            args.extend_from_slice(input);
            assert_client(run(project.path(), home.path(), &args), version, expected);
        }
    }
}

#[test]
fn combined_clickhouse_client_query_sources_are_usage_errors_before_exec() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let cases = [
        &[
            "local",
            "client",
            "--query",
            "SELECT 1",
            "--queries-file",
            "queries.sql",
        ][..],
        &[
            "local",
            "client",
            "--queries-file",
            "queries.sql",
            "--query",
            "SELECT 1",
        ][..],
    ];

    for args in cases {
        let output = run(project.path(), home.path(), args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "child must not run: {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--query"), "stderr: {stderr}");
        assert!(stderr.contains("--queries-file"), "stderr: {stderr}");
        assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");
    }
}

#[test]
fn clickhouse_direct_client_defaults_missing_host_or_port() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    set_default(home.path(), VERSION_A);

    let cases = [
        (
            &["local", "client", "--host", "db.example"][..],
            &["client", "--host", "db.example", "--port", "9000"][..],
        ),
        (
            &["local", "client", "--port", "1"][..],
            &["client", "--host", "localhost", "--port", "1"][..],
        ),
        (
            &["local", "client", "--port", "65535"][..],
            &["client", "--host", "localhost", "--port", "65535"][..],
        ),
    ];

    for (args, expected) in cases {
        let output = run(project.path(), home.path(), args);
        assert!(
            output.status.success(),
            "args: {args:?}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let forwarded: Vec<_> = String::from_utf8(output.stdout)
            .expect("fake ClickHouse output should be UTF-8")
            .lines()
            .skip(1)
            .map(str::to_string)
            .collect();
        assert_eq!(forwarded, expected, "args: {args:?}");
    }
}

#[test]
fn direct_client_without_explicit_version_uses_only_a_valid_default() {
    struct Case {
        installed: &'static [&'static str],
        default: Option<&'static str>,
        expected_binary: Option<&'static str>,
        expected_error: Option<&'static str>,
    }

    let cases = [
        Case {
            installed: &[],
            default: None,
            expected_binary: None,
            expected_error: Some("No ClickHouse client version selected"),
        },
        Case {
            installed: &[VERSION_A],
            default: None,
            expected_binary: None,
            expected_error: Some("No ClickHouse client version selected"),
        },
        Case {
            installed: &[VERSION_A, VERSION_B],
            default: None,
            expected_binary: None,
            expected_error: Some("No ClickHouse client version selected"),
        },
        Case {
            installed: &[VERSION_A],
            default: Some(VERSION_A),
            expected_binary: Some(VERSION_A),
            expected_error: None,
        },
        Case {
            installed: &[VERSION_A, VERSION_B],
            default: Some(VERSION_B),
            expected_binary: Some(VERSION_B),
            expected_error: None,
        },
        Case {
            installed: &[VERSION_A],
            default: Some(MISSING_VERSION),
            expected_binary: None,
            expected_error: Some("Default ClickHouse version 24.8.99.1 is not installed"),
        },
    ];

    for case in cases {
        let project = tempfile::tempdir().expect("create project tempdir");
        let home = tempfile::tempdir().expect("create home tempdir");
        for version in case.installed {
            install_fake_clickhouse(home.path(), version);
        }
        if let Some(version) = case.default {
            set_default(home.path(), version);
        }

        let output = run(
            project.path(),
            home.path(),
            &["local", "client", "--host", "db.example"],
        );
        if let Some(version) = case.expected_binary {
            assert_client(
                output,
                version,
                &["client", "--host", "db.example", "--port", "9000"],
            );
        } else {
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains(case.expected_error.unwrap()),
                "installed: {:?}, default: {:?}\nstderr: {stderr}",
                case.installed,
                case.default
            );
            assert!(
                stderr.contains("clickhousectl local use")
                    || stderr.contains("clickhousectl local list"),
                "stderr should be actionable: {stderr}"
            );
        }
    }
}

#[test]
fn direct_client_explicit_version_matrix_is_installed_only_and_preserves_default() {
    struct Case {
        installed: &'static [&'static str],
        default: Option<&'static str>,
        requested: &'static str,
        expected_binary: Option<&'static str>,
    }

    let cases = [
        Case {
            installed: &[],
            default: None,
            requested: VERSION_A,
            expected_binary: None,
        },
        Case {
            installed: &[VERSION_A],
            default: None,
            requested: VERSION_A,
            expected_binary: Some(VERSION_A),
        },
        Case {
            installed: &[VERSION_A, VERSION_B],
            default: Some(VERSION_A),
            requested: VERSION_B,
            expected_binary: Some(VERSION_B),
        },
        Case {
            installed: &[VERSION_A, VERSION_B],
            default: Some(MISSING_VERSION),
            requested: VERSION_B,
            expected_binary: Some(VERSION_B),
        },
        Case {
            installed: &[VERSION_A],
            default: Some(VERSION_A),
            requested: VERSION_B,
            expected_binary: None,
        },
    ];

    for case in cases {
        let project = tempfile::tempdir().expect("create project tempdir");
        let home = tempfile::tempdir().expect("create home tempdir");
        for version in case.installed {
            install_fake_clickhouse(home.path(), version);
        }
        if let Some(version) = case.default {
            set_default(home.path(), version);
        }

        let output = run(
            project.path(),
            home.path(),
            &[
                "local",
                "client",
                "--port",
                "9440",
                "--version",
                case.requested,
            ],
        );
        if let Some(version) = case.expected_binary {
            assert_client(
                output,
                version,
                &["client", "--host", "localhost", "--port", "9440"],
            );
        } else {
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains(&format!(
                    "ClickHouse client version {} is not installed",
                    case.requested
                )),
                "stderr: {stderr}"
            );
            assert!(
                stderr.contains(&format!("clickhousectl local install {}", case.requested))
                    && stderr.contains("clickhousectl local list"),
                "stderr should be actionable: {stderr}"
            );
        }

        let default_file = home.path().join(".clickhouse/default");
        match case.default {
            Some(expected) => assert_eq!(
                std::fs::read_to_string(default_file).unwrap(),
                expected,
                "explicit selection must preserve the default"
            ),
            None => assert!(
                !default_file.exists(),
                "explicit selection must not create a default"
            ),
        }
    }
}

#[test]
fn named_client_uses_recorded_version_even_with_a_stale_default() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    install_fake_clickhouse(home.path(), VERSION_B);
    set_default(home.path(), MISSING_VERSION);
    let process = spawn_managed_clickhouse(project.path(), "dev");
    write_server_metadata(project.path(), "dev", process.id(), VERSION_B, 9440);

    let output = run(
        project.path(),
        home.path(),
        &["local", "client", "--name", "dev"],
    );
    assert_client(
        output,
        VERSION_B,
        &["client", "--host", "localhost", "--port", "9440"],
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join(".clickhouse/default")).unwrap(),
        MISSING_VERSION
    );
}

#[test]
fn invalid_client_selectors_are_usage_errors_before_resolution() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let cases = [
        &["local", "client", "--name", "dev", "--host", "db.example"][..],
        &["local", "client", "--port", "9000", "--name", "dev"][..],
        &["local", "client", "--port", "0"][..],
        &["local", "client", "--version", VERSION_A][..],
        &["local", "client", "--name", "dev", "--version", VERSION_A][..],
        &["local", "client", "--version", VERSION_A, "--name", "dev"][..],
        &[
            "local",
            "postgres",
            "client",
            "--host",
            "db.example",
            "--name",
            "dev",
        ][..],
        &[
            "local", "postgres", "client", "--name", "dev", "--port", "5432",
        ][..],
        &["local", "postgres", "client", "--port", "0"][..],
    ];

    for args in cases {
        let output = run(project.path(), home.path(), args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error:"),
            "args: {args:?}\nstderr: {stderr}"
        );
        assert!(
            !stderr.contains("No default version configured")
                && !stderr.contains("Server 'dev' not found"),
            "args: {args:?}\nstderr: {stderr}"
        );
    }
}
