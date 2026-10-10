//! Subprocess coverage for `local udf` against a running server: deploy
//! reloads and verifies, `reload` and `list` talk HTTP, a function the
//! server never loads is an error, a rejected reload names the broken
//! function, an unreachable server is not a rejection, and server errors are
//! redacted in JSON.
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
    /// While this file exists the fake rejects `SYSTEM RELOAD FUNCTIONS`,
    /// answering with the file's text (a generic exception when empty).
    fail_reload_marker: PathBuf,
    /// While this file exists the fake accepts the first reload and rejects
    /// every later one, like a function ClickHouse has never loaded.
    fail_retry_marker: PathBuf,
    /// The names `system.user_defined_functions` reports as failed, one per
    /// line. Without it the fake rejects that query, like ClickHouse < 26.2.
    failed_functions: PathBuf,
    /// While this file exists the fake drops HTTP connections unanswered.
    drop_http_marker: PathBuf,
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
        fail_retry_marker: home.path().join("fail-retry"),
        failed_functions: home.path().join("failed-functions"),
        drop_http_marker: home.path().join("drop-http"),
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
        .env("FAKE_CLICKHOUSE_FAIL_RETRY", &env.fail_retry_marker)
        .env("FAKE_CLICKHOUSE_FAILED_FUNCTIONS", &env.failed_functions)
        .env("FAKE_CLICKHOUSE_DROP_HTTP", &env.drop_http_marker)
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

fn write_udf_at(env: &Env, parent: &str, name: &str) {
    write_udf(env, name);
    let target = env.project.path().join(parent);
    std::fs::create_dir_all(&target).unwrap();
    std::fs::rename(
        env.project.path().join("clickhouse/udfs").join(name),
        target.join(name),
    )
    .unwrap();
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
/// only loaded executable UDF, `SYSTEM RELOAD FUNCTIONS` and
/// `system.user_defined_functions` follow the marker files in [`Env`], and
/// anything else succeeds.
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
    let fail_retry = std::env::var("FAKE_CLICKHOUSE_FAIL_RETRY").unwrap();
    let failed_functions = std::env::var("FAKE_CLICKHOUSE_FAILED_FUNCTIONS").unwrap();
    let drop_http = std::env::var("FAKE_CLICKHOUSE_DROP_HTTP").unwrap();
    let mut reloads_since_retry_marker = 0;
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
        if Path::new(&drop_http).exists() {
            continue;
        }
        let rejection = |text: String| ("500 Internal Server Error", text);
        let (status, response) = if request_line.starts_with("GET /ping") {
            ("200 OK", "Ok.\n".to_string())
        } else {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log)
                .unwrap();
            write!(file, "{body}\n---\n").unwrap();
            let is_reload = body.starts_with("SYSTEM RELOAD FUNCTIONS");
            if is_reload && Path::new(&fail_retry).exists() {
                reloads_since_retry_marker += 1;
            } else if is_reload {
                reloads_since_retry_marker = 0;
            }
            if is_reload && Path::new(&fail_reload).exists() {
                let text = std::fs::read_to_string(&fail_reload).unwrap_or_default();
                rejection(if text.is_empty() {
                    "Code: 36. DB::Exception: Function configuration is invalid. (BAD_ARGUMENTS)\n"
                        .to_string()
                } else {
                    text
                })
            } else if is_reload && reloads_since_retry_marker > 1 {
                rejection(
                    "Code: 50. DB::Exception: Unknown data type family: NotAType. (UNKNOWN_TYPE)\n"
                        .to_string(),
                )
            } else if body.contains("system.user_defined_functions") {
                match std::fs::read_to_string(&failed_functions) {
                    Ok(names) => ("200 OK", names),
                    Err(_) => rejection(
                        "Code: 60. DB::Exception: Unknown table expression identifier \
                         'system.user_defined_functions'. (UNKNOWN_TABLE)\n"
                            .to_string(),
                    ),
                }
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

#[test]
fn deploy_commands_in_errors_name_a_non_default_dir() {
    let env = setup();
    let _server = start_server(&env);
    write_udf_at(&env, "shared/udfs", "other_fn");

    let output = run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "other_fn",
            "--dir",
            "shared/udfs",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "udf_not_loaded");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf deploy other_fn --dir shared/udfs --server default"
    );
}

#[test]
fn a_definition_the_server_rejects_on_reload_keeps_the_files_and_names_the_way_out() {
    let env = setup();
    let _server = start_server(&env);
    write_udf_at(&env, "shared/udfs", "my_fn");
    std::fs::write(&env.fail_reload_marker, "").unwrap();
    std::fs::write(&env.failed_functions, "my_fn\n").unwrap();

    let output = run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "my_fn",
            "--dir",
            "shared/udfs",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "udf_rejected");
    assert_eq!(
        error["error"]["message"],
        "ClickHouse rejected UDF my_fn on server default. Its files stay deployed, and every \
         function reload on this server fails until it is fixed. Fix shared/udfs/my_fn and rerun \
         `clickhousectl local udf deploy my_fn --dir shared/udfs --server default`, restore the \
         previous working copy and rerun it, or remove it with \
         `clickhousectl local udf remove my_fn --server default`."
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove my_fn --server default"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("DB::Exception"));
    let data = env.project.path().join(".clickhouse/servers/default/data");
    assert!(
        data.join("user_defined_functions/my_fn_function.xml")
            .is_file()
    );
    assert!(data.join("user_scripts/my_fn/main.py").is_file());

    let human = run(
        &env,
        &["local", "udf", "deploy", "my_fn", "--dir", "shared/udfs"],
    );
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.contains("ClickHouse rejected UDF my_fn"), "{stderr}");
    assert!(stderr.contains("Code: 36. DB::Exception"), "{stderr}");
}

fn error_json(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stderr).expect("stderr is a single JSON object")
}

#[test]
fn a_function_rejected_only_by_the_retry_reload_is_reported_as_rejected() {
    let env = setup();
    let _server = start_server(&env);
    write_udf(&env, "other_fn");
    // The first reload succeeds; the fake never lists `other_fn`, so deploy
    // reloads again, and that reload is rejected.
    std::fs::write(&env.fail_retry_marker, "").unwrap();
    std::fs::write(&env.failed_functions, "other_fn\n").unwrap();

    let error = error_json(&run(
        &env,
        &["local", "udf", "deploy", "other_fn", "--json"],
    ));
    assert_eq!(error["error"]["code"], "udf_rejected");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove other_fn --server default"
    );
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("ClickHouse rejected UDF other_fn on server default."),
        "{error}"
    );
    let reloads = logged_queries(&env)
        .iter()
        .filter(|query| *query == "SYSTEM RELOAD FUNCTIONS")
        .count();
    assert!(reloads >= 2, "{reloads}");
}

#[test]
fn a_rejected_reload_names_the_broken_function_not_the_one_deployed() {
    let env = setup();
    let _server = start_server(&env);
    write_udf(&env, "bad");
    write_udf(&env, "my_fn");
    std::fs::write(&env.fail_reload_marker, "").unwrap();
    std::fs::write(&env.failed_functions, "bad\n").unwrap();
    let error = error_json(&run(&env, &["local", "udf", "deploy", "bad", "--json"]));
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove bad --server default"
    );

    // ClickHouse >= 26.2 reports the failed function's load status.
    let error = error_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    assert_eq!(error["error"]["code"], "udf_rejected");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove bad --server default"
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("UDF bad is broken"), "{message}");
    assert!(!message.contains("remove my_fn"), "{message}");

    // Older servers: only a name clash names the function, in the error text.
    std::fs::remove_file(&env.failed_functions).unwrap();
    std::fs::write(
        &env.fail_reload_marker,
        "Code: 609. DB::Exception: The function 'bad' already exists. \
         (FUNCTION_ALREADY_EXISTS) (version 25.12.10.7 (official build))",
    )
    .unwrap();
    let error = error_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove bad --server default"
    );

    // No culprit identified, or one that is not deployed here: blame no one.
    std::fs::write(
        &env.fail_reload_marker,
        "Code: 50. DB::Exception: Unknown data type family: NotAType. (UNKNOWN_TYPE)",
    )
    .unwrap();
    for failed in [None, Some("handmade\n")] {
        if let Some(failed) = failed {
            std::fs::write(&env.failed_functions, failed).unwrap();
        }
        let error = error_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
        assert_eq!(error["error"]["code"], "udf_rejected");
        assert_eq!(
            error["error"]["command"],
            "clickhousectl local udf list --server default"
        );
        let message = error["error"]["message"].as_str().unwrap();
        assert!(!message.contains("udf remove"), "{message}");
        assert!(!message.contains("handmade"), "{message}");
    }
}

#[test]
fn remove_during_a_blocked_reload_says_what_was_removed_and_what_blocks() {
    let env = setup();
    let _server = start_server(&env);
    write_udf(&env, "my_fn");
    deploy(&env, "my_fn");
    write_udf(&env, "bad");
    std::fs::write(&env.fail_reload_marker, "").unwrap();
    std::fs::write(&env.failed_functions, "bad\n").unwrap();
    error_json(&run(&env, &["local", "udf", "deploy", "bad", "--json"]));

    let error = error_json(&run(&env, &["local", "udf", "remove", "my_fn", "--json"]));
    assert_eq!(error["error"]["code"], "udf_reload_blocked");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf remove bad --server default"
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.starts_with("Removed UDF my_fn from server default"),
        "{message}"
    );
    assert!(message.contains("UDF bad is broken"), "{message}");
    assert!(!error.to_string().contains("DB::Exception"));
    let data = env.project.path().join(".clickhouse/servers/default/data");
    assert!(
        !data
            .join("user_defined_functions/my_fn_function.xml")
            .exists()
    );
    assert!(!data.join("user_scripts/my_fn").exists());

    std::fs::remove_file(&env.failed_functions).unwrap();
    write_udf(&env, "other_fn");
    std::fs::remove_file(&env.fail_reload_marker).unwrap();
    std::fs::write(&env.fail_retry_marker, "").unwrap();
    error_json(&run(
        &env,
        &["local", "udf", "deploy", "other_fn", "--json"],
    ));
    std::fs::remove_file(&env.fail_retry_marker).unwrap();
    std::fs::write(&env.fail_reload_marker, "").unwrap();
    let human = run(&env, &["local", "udf", "remove", "other_fn"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.contains("Removed UDF other_fn from server default"),
        "{stderr}"
    );
    assert!(
        stderr.contains("another deployed UDF is broken"),
        "{stderr}"
    );
    assert!(stderr.contains("Code: 36. DB::Exception"), "{stderr}");
}

#[test]
fn an_unreachable_server_is_not_reported_as_a_rejection() {
    let env = setup();
    let _server = start_server(&env);
    std::fs::write(&env.drop_http_marker, "").unwrap();

    let error = error_json(&run(&env, &["local", "udf", "reload", "--json"]));
    assert_eq!(error["error"]["code"], "udf_server_unreachable");
    assert_eq!(
        error["error"]["message"],
        format!("Could not reach server 'default' on port {}", env.http_port)
    );
    assert_eq!(error["error"]["command"], "clickhousectl local server list");

    let human = run(&env, &["local", "udf", "reload"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.contains(&format!(
            "Could not reach server 'default' on port {}: ",
            env.http_port
        )),
        "{stderr}"
    );
    assert!(!stderr.contains("rejected"), "{stderr}");
}
