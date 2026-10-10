//! Coverage for the managed `config.d` file that carries a local server's data
//! path and ports (issue #1066).
//!
//! The ports used to be command-line overrides, which ClickHouse 26.1 and
//! earlier drop on the first config reload, rebinding to 8123/9000. They now
//! live in `config.d/zz-chctl-managed.xml`, which merges after every other file
//! there and survives reloads.
//!
//! The fake-binary test runs everywhere. The live test needs a real ClickHouse
//! (ideally ≤ 26.1, where the bug reproduced) and is ignored by default:
//! `CLICKHOUSECTL_LIVE_CLICKHOUSE_VERSION=26.1.13.2 cargo test -p clickhousectl
//! --test local_server_managed_config_test -- --ignored` uses
//! `~/.clickhouse/versions/<version>/clickhouse`.

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const FAKE_VERSION: &str = "25.12.9.61";
const MANAGED_FILE: &str = "zz-chctl-managed.xml";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env(
            "FAKE_CLICKHOUSE_ARGS_FILE",
            home.join("clickhouse-args.txt"),
        )
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home
        .join(".clickhouse/versions")
        .join(FAKE_VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(
        &binary,
        b"#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_CLICKHOUSE_ARGS_FILE\"\nexec sleep 30\n",
    )
    .expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn read_file_eventually(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(contents) = std::fs::read_to_string(path)
            && !contents.is_empty()
        {
            return contents;
        }
        assert!(
            Instant::now() < deadline,
            "{} never appeared",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn unused_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind temporary port");
    listener.local_addr().unwrap().port()
}

/// Stops a real server through the CLI, so a failed assertion does not strand
/// the server process its watchdog supervises.
struct StopGuard<'a> {
    project: &'a Path,
    home: &'a Path,
}

impl Drop for StopGuard<'_> {
    fn drop(&mut self) {
        run(self.project, self.home, &["local", "server", "stop"]);
    }
}

struct ProcessGuard(u32);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}

#[test]
fn start_writes_ports_to_a_config_file_that_merges_after_the_user_overlay() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());
    let configs = home.path().join(".clickhouse/configs");
    std::fs::create_dir_all(&configs).unwrap();
    std::fs::write(
        configs.join("ports.xml"),
        "<clickhouse><http_port>8123</http_port><tcp_port>9000</tcp_port></clickhouse>",
    )
    .unwrap();
    let (http, tcp) = (unused_port().to_string(), unused_port().to_string());

    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--no-wait",
            "--version",
            FAKE_VERSION,
            "--http-port",
            &http,
            "--tcp-port",
            &tcp,
            "--config",
            "ports",
            "--",
            "--logger.level=trace",
        ],
    );
    assert_success(&output);
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    let _process = ProcessGuard(body["pid"].as_u64().expect("start PID") as u32);

    // No managed setting travels on the command line; pass-through args still do.
    let args = read_file_eventually(&home.path().join("clickhouse-args.txt"));
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args, ["server", "--", "--logger.level=trace"]);

    let config_d = project
        .path()
        .join(".clickhouse/servers/default/data/config.d");
    let managed = std::fs::read_to_string(config_d.join(MANAGED_FILE)).expect("managed config");
    assert!(managed.contains("<path>./</path>"), "got: {managed}");
    assert!(
        managed.contains(&format!("<http_port>{http}</http_port>")),
        "got: {managed}"
    );
    assert!(
        managed.contains(&format!("<tcp_port>{tcp}</tcp_port>")),
        "got: {managed}"
    );

    // The user overlay is staged, and ClickHouse merges it first, so the
    // assigned ports win over the ones it sets.
    let mut staged: Vec<String> = std::fs::read_dir(&config_d)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    staged.sort();
    assert_eq!(staged, ["chctl-config.xml", MANAGED_FILE]);

    // Server metadata records the same assigned ports.
    assert_eq!(body["http_port"].as_u64().unwrap().to_string(), http);
    assert_eq!(body["tcp_port"].as_u64().unwrap().to_string(), tcp);
}

/// The ports a real ClickHouse logged it is listening on over HTTP.
fn http_listen_ports(log: &str) -> Vec<u16> {
    log.lines()
        .filter_map(|line| line.split("Listening for http://").nth(1))
        .filter_map(|addr| addr.rsplit(':').next()?.trim().parse().ok())
        .collect()
}

#[test]
#[ignore = "needs a real ClickHouse; set CLICKHOUSECTL_LIVE_CLICKHOUSE_VERSION"]
fn a_config_reload_keeps_a_running_server_on_its_assigned_ports() {
    let version = std::env::var("CLICKHOUSECTL_LIVE_CLICKHOUSE_VERSION")
        .expect("set CLICKHOUSECTL_LIVE_CLICKHOUSE_VERSION to an installed version");
    let real_home = PathBuf::from(std::env::var("HOME").unwrap());
    let real_binary = real_home
        .join(".clickhouse/versions")
        .join(&version)
        .join("clickhouse");
    assert!(real_binary.is_file(), "{} missing", real_binary.display());

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let version_dir = home.path().join(".clickhouse/versions").join(&version);
    std::fs::create_dir_all(&version_dir).unwrap();
    std::os::unix::fs::symlink(&real_binary, version_dir.join("clickhouse")).unwrap();
    let (http, tcp) = (unused_port(), unused_port());

    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            &version,
            "--http-port",
            &http.to_string(),
            "--tcp-port",
            &tcp.to_string(),
        ],
    );
    let _server = StopGuard {
        project: project.path(),
        home: home.path(),
    };
    assert_success(&output);

    // Any change under config.d makes ClickHouse reload its config.
    let server_dir = project.path().join(".clickhouse/servers/default");
    let config_d = server_dir.join("data/config.d");
    std::fs::create_dir_all(&config_d).unwrap();
    std::fs::write(
        config_d.join("reload-trigger.xml"),
        "<clickhouse><http_port>8123</http_port><tcp_port>9000</tcp_port></clickhouse>",
    )
    .unwrap();
    let log_path = server_dir.join("server.log");
    let deadline = Instant::now() + Duration::from_secs(30);
    let log = loop {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        // The reload merges the new file; give a rebind time to be logged.
        if log.contains("config.d/reload-trigger.xml") {
            std::thread::sleep(Duration::from_secs(3));
            break std::fs::read_to_string(&log_path).unwrap();
        }
        assert!(Instant::now() < deadline, "config was never reloaded");
        std::thread::sleep(Duration::from_millis(200));
    };

    let ports = http_listen_ports(&log);
    assert!(!ports.is_empty(), "no HTTP listener logged");
    assert!(ports.iter().all(|&p| p == http), "listened on {ports:?}");
    let ping = Command::new("curl")
        .args(["-sf", &format!("http://127.0.0.1:{http}/ping")])
        .output()
        .expect("run curl");
    assert!(ping.status.success(), "server left its assigned HTTP port");
}
