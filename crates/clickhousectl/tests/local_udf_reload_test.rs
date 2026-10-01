//! Subprocess coverage for `local udf` against a running server: deploy
//! reloads and verifies, `reload` and `list` talk HTTP, a function the
//! server never loads is an error, and server errors are redacted in JSON.
//! The server is this test binary re-executed as a fake ClickHouse that
//! answers the readiness probes and logs every statement.

use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn write_executable(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn unused_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct ProcessGuard(u32);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}

struct Env {
    project: tempfile::TempDir,
    home: tempfile::TempDir,
    bin: PathBuf,
    query_log: PathBuf,
    /// While this file exists the fake rejects `SYSTEM RELOAD FUNCTIONS`.
    fail_reload_marker: PathBuf,
    http_port: u16,
    tcp_port: u16,
}

fn setup() -> Env {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("bin");
    write_executable(&bin.join("python3.11"), "#!/bin/sh\nexit 0\n");
    let test_binary = std::env::current_exe().unwrap();
    write_executable(
        &home
            .path()
            .join(".clickhouse/versions")
            .join(VERSION)
            .join("clickhouse"),
        &format!(
            "#!/bin/sh\nexec '{}' --exact fake_clickhouse_process --nocapture\n",
            test_binary.display()
        ),
    );
    Env {
        query_log: home.path().join("queries.log"),
        fail_reload_marker: home.path().join("fail-reload"),
        project,
        home,
        bin,
        http_port: unused_port(),
        tcp_port: unused_port(),
    }
}

fn command(env: &Env) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", env.home.path())
        .env("PATH", format!("{}:/usr/bin:/bin", env.bin.display()))
        .env("FAKE_CLICKHOUSE_HTTP_PORT", env.http_port.to_string())
        .env("FAKE_CLICKHOUSE_PORT", env.tcp_port.to_string())
        .env("FAKE_CLICKHOUSE_QUERY_LOG", &env.query_log)
        .env("FAKE_CLICKHOUSE_FAIL_RELOAD", &env.fail_reload_marker)
        .current_dir(env.project.path());
    command
}

fn run(env: &Env, args: &[&str]) -> Output {
    command(env).args(args).output().expect("run clickhousectl")
}

fn success_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is a single JSON object")
}

fn start_server(env: &Env) -> ProcessGuard {
    let output = run(
        env,
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            VERSION,
            "--http-port",
            &env.http_port.to_string(),
            "--tcp-port",
            &env.tcp_port.to_string(),
        ],
    );
    let json = success_json(&output);
    ProcessGuard(json["pid"].as_u64().expect("pid") as u32)
}

fn stop_server(env: &Env) {
    let output = run(env, &["local", "--json", "server", "stop"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_udf(env: &Env, name: &str) {
    let dir = env.project.path().join("clickhouse/udfs").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("udf.json"),
        serde_json::json!({
            "functionName": name,
            "type": "executable",
            "runtime": "python3.11",
            "arguments": [{"name": "value", "type": "String"}],
            "returnType": "String"
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
}

fn deploy(env: &Env, name: &str) -> Value {
    success_json(&run(env, &["local", "udf", "deploy", name, "--json"]))
}

fn logged_queries(env: &Env) -> Vec<String> {
    std::fs::read_to_string(&env.query_log)
        .unwrap_or_default()
        .split("\n---\n")
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Re-executed by the fake ClickHouse script. Answers the TCP and HTTP
/// readiness probes, then serves HTTP statements until killed: `my_fn` is the
/// only loaded executable UDF, `SYSTEM RELOAD FUNCTIONS` fails while the
/// marker file exists, and anything else succeeds.
#[test]
fn fake_clickhouse_process() {
    let (Ok(http_port), Ok(tcp_port), Ok(log), Ok(fail_reload)) = (
        std::env::var("FAKE_CLICKHOUSE_HTTP_PORT"),
        std::env::var("FAKE_CLICKHOUSE_PORT"),
        std::env::var("FAKE_CLICKHOUSE_QUERY_LOG"),
        std::env::var("FAKE_CLICKHOUSE_FAIL_RELOAD"),
    ) else {
        return;
    };
    let tcp = std::net::TcpListener::bind(("127.0.0.1", tcp_port.parse::<u16>().unwrap()))
        .expect("bind fake TCP port");
    std::thread::spawn(move || {
        for stream in tcp.incoming() {
            drop(stream);
        }
    });
    let http = std::net::TcpListener::bind(("127.0.0.1", http_port.parse::<u16>().unwrap()))
        .expect("bind fake HTTP port");
    for stream in http.incoming() {
        let Ok(mut stream) = stream else { continue };
        let (request_line, body) = read_http_request(&mut stream);
        let (status, response) = if request_line.starts_with("GET /ping") {
            ("200 OK", "Ok.\n".to_string())
        } else {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log)
                .unwrap();
            write!(file, "{body}\n---\n").unwrap();
            if body.starts_with("SYSTEM RELOAD FUNCTIONS") && Path::new(&fail_reload).exists() {
                (
                    "500 Internal Server Error",
                    "Code: 36. DB::Exception: Function configuration is invalid. (BAD_ARGUMENTS)\n"
                        .to_string(),
                )
            } else if body.contains("system.functions") {
                ("200 OK", "my_fn\n".to_string())
            } else {
                ("200 OK", "Ok.\n".to_string())
            }
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        );
        let _ = stream.flush();
    }
}

fn read_http_request(stream: &mut std::net::TcpStream) -> (String, String) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            break buffer.len();
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let request_line = head.lines().next().unwrap_or_default().to_string();
    let content_length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    (request_line, String::from_utf8_lossy(&body).into_owned())
}

#[test]
fn deploy_to_a_running_server_reloads_and_reports_loaded() {
    let env = setup();
    let _server = start_server(&env);
    write_udf(&env, "my_fn");

    let json = deploy(&env, "my_fn");
    assert_eq!(json["server_running"], true);
    assert_eq!(json["reloaded"], true);
    assert_eq!(json["loaded"], true);

    let queries = logged_queries(&env);
    assert!(
        queries
            .iter()
            .any(|query| query == "SYSTEM RELOAD FUNCTIONS"),
        "{queries:?}"
    );
    assert!(
        queries
            .iter()
            .any(|query| query.contains("system.functions")
                && query.contains("ExecutableUserDefined")),
        "{queries:?}"
    );

    let human = run(&env, &["local", "udf", "deploy", "my_fn"]);
    assert!(human.status.success());
    assert!(
        String::from_utf8_lossy(&human.stdout).ends_with("Reloaded functions; my_fn is loaded.\n"),
        "{}",
        String::from_utf8_lossy(&human.stdout)
    );
}

#[test]
fn a_function_the_server_never_loads_is_an_error_that_keeps_the_files() {
    let env = setup();
    let _server = start_server(&env);
    write_udf(&env, "other_fn");

    let output = run(&env, &["local", "udf", "deploy", "other_fn", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "udf_not_loaded");
    let log_path = env
        .project
        .path()
        .canonicalize()
        .unwrap()
        .join(".clickhouse/servers/default/server.log");
    assert_eq!(
        error["error"]["message"],
        format!(
            "UDF other_fn was deployed to server default but ClickHouse did not load it; the reason is in {}",
            log_path.display()
        )
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf deploy other_fn --server default"
    );
    let data = env.project.path().join(".clickhouse/servers/default/data");
    assert!(
        data.join("user_defined_functions/other_fn_function.xml")
            .is_file()
    );
    assert!(data.join("user_scripts/other_fn/main.py").is_file());

    let human = run(&env, &["local", "udf", "deploy", "other_fn"]);
    assert_eq!(human.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&human.stderr).contains(&log_path.display().to_string()),
        "{}",
        String::from_utf8_lossy(&human.stderr)
    );
}

#[test]
fn reload_sends_system_reload_functions_and_lists_loaded_names() {
    let env = setup();
    let _server = start_server(&env);

    let json = success_json(&run(&env, &["local", "udf", "reload", "--json"]));
    assert_eq!(
        json,
        serde_json::json!({"server": "default", "loaded": ["my_fn"]})
    );
    assert!(
        logged_queries(&env)
            .iter()
            .any(|query| query == "SYSTEM RELOAD FUNCTIONS")
    );

    let human = run(&env, &["local", "udf", "reload"]);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "Reloaded functions on server 'default'; loaded: my_fn\n"
    );
}

#[test]
fn server_errors_are_redacted_in_json_and_verbose_in_human_mode() {
    let env = setup();
    let _server = start_server(&env);
    std::fs::write(&env.fail_reload_marker, "").unwrap();

    let json_mode = run(&env, &["local", "udf", "reload", "--json"]);
    assert_eq!(json_mode.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&json_mode.stderr).unwrap();
    assert_eq!(error["error"]["code"], "udf_query_failed");
    assert_eq!(
        error["error"]["message"],
        "ClickHouse server 'default' rejected the query"
    );
    assert_eq!(error["error"]["command"], "clickhousectl local server list");
    assert!(!String::from_utf8_lossy(&json_mode.stderr).contains("DB::Exception"));

    let human = run(&env, &["local", "udf", "reload"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.contains("ClickHouse server 'default' rejected the query: Code: 36. DB::Exception"),
        "{stderr}"
    );
}

#[test]
fn list_marks_loaded_functions_and_remove_reloads_on_a_running_server() {
    let env = setup();
    write_udf(&env, "my_fn");
    write_udf(&env, "other_fn");
    // `my_fn` deploys while running (the fake reports it loaded); `other_fn`
    // is staged while stopped, since the fake would never report it.
    let first = start_server(&env);
    deploy(&env, "my_fn");
    stop_server(&env);
    drop(first);
    let staged = deploy(&env, "other_fn");
    assert_eq!(staged["loaded"], Value::Null);
    let _server = start_server(&env);

    let list = success_json(&run(&env, &["local", "udf", "list", "--json"]));
    assert_eq!(list["server_running"], true);
    assert_eq!(
        list["udfs"],
        serde_json::json!([
            {"name": "my_fn", "type": "executable", "runtime": "python3.11", "loaded": true},
            {"name": "other_fn", "type": "executable", "runtime": "python3.11", "loaded": false}
        ])
    );
    let human = run(&env, &["local", "udf", "list"]);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("| yes"), "{stdout}");
    assert!(stdout.contains("| no"), "{stdout}");
    assert!(!stdout.contains("not running"), "{stdout}");

    let removed = success_json(&run(
        &env,
        &["local", "udf", "remove", "other_fn", "--json"],
    ));
    assert_eq!(removed["reloaded"], true);
    assert!(
        logged_queries(&env)
            .iter()
            .any(|query| query == "SYSTEM RELOAD FUNCTIONS")
    );
    let human = run(&env, &["local", "udf", "remove", "my_fn"]);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "Removed UDF my_fn from server 'default' and reloaded functions\n"
    );
}
