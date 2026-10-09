//! Subprocess coverage for `local udf deploy|list|remove|reload` when the
//! target server is not running: staging, the managed overlay, and the
//! structured error contract. Running-server behaviour is covered by
//! `local_udf_reload_test.rs`.

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

struct Env {
    project: tempfile::TempDir,
    home: tempfile::TempDir,
    /// Holds a fake `python3.11`; prepended to `PATH`.
    bin: PathBuf,
}

fn setup() -> Env {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let bin = home.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    write_executable(&bin.join("python3.11"), "#!/bin/sh\nexit 0\n");
    write_executable(
        &home
            .path()
            .join(".clickhouse/versions")
            .join(VERSION)
            .join("clickhouse"),
        "#!/bin/sh\nexec sleep 30\n",
    );
    Env { project, home, bin }
}

fn write_executable(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn command(env: &Env) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", env.home.path())
        .env("PATH", format!("{}:/usr/bin:/bin", env.bin.display()))
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

fn error_json(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "structured errors leave stdout empty"
    );
    serde_json::from_slice(&output.stderr).expect("stderr is a single JSON object")
}

/// `local udf` never creates servers: start the fake ClickHouse under `name`
/// and stop it again so the data directory exists but nothing is running.
fn create_stopped_server(env: &Env, name: &str) {
    let started = run(
        env,
        &[
            "local",
            "--json",
            "server",
            "start",
            name,
            "--no-wait",
            "--version",
            VERSION,
        ],
    );
    success_json(&started);
    let stopped = run(env, &["local", "--json", "server", "stop", name]);
    assert!(
        stopped.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(data_dir(env, name).is_dir());
}

fn write_python_udf(env: &Env, name: &str, with_requirements: bool) -> PathBuf {
    let dir = env.project.path().join("clickhouse/udfs").join(name);
    write_python_udf_at(&dir, name, with_requirements);
    dir
}

fn write_python_udf_at(dir: &Path, name: &str, with_requirements: bool) {
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::create_dir_all(dir.join("__pycache__")).unwrap();
    std::fs::write(
        dir.join("udf.json"),
        serde_json::json!({
            "functionName": name,
            "type": "executable",
            "runtime": "python3.11",
            "arguments": [{"name": "value", "type": "String"}],
            "returnType": "String",
            "memoryLimitMib": 128
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
    std::fs::write(dir.join("lib/helper.py"), "x = 1\n").unwrap();
    std::fs::write(dir.join(".hidden"), "secret\n").unwrap();
    std::fs::write(dir.join("__pycache__/main.pyc"), "").unwrap();
    if with_requirements {
        std::fs::write(dir.join("requirements.txt"), "requests>=2\n").unwrap();
    }
}

fn write_native_udf(env: &Env, name: &str) -> PathBuf {
    let dir = env.project.path().join("clickhouse/udfs").join(name);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("udf.json"),
        serde_json::json!({
            "functionName": name,
            "type": "executable_pool",
            "runtime": "native",
            "arguments": [{"name": "value", "type": "UInt64"}],
            "returnType": "UInt64",
            "poolSize": 2
        })
        .to_string(),
    )
    .unwrap();
    for arch in ["amd64", "arm64"] {
        std::fs::create_dir_all(dir.join(arch)).unwrap();
        // Deliberately not executable: deploy must set the bit.
        std::fs::write(dir.join(arch).join("main"), "#!/bin/sh\ncat\n").unwrap();
    }
    std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
    dir
}

fn data_dir(env: &Env, server: &str) -> PathBuf {
    env.project
        .path()
        .join(".clickhouse/servers")
        .join(server)
        .join("data")
}

#[cfg(target_os = "linux")]
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o111 == 0o111
}

fn assert_nothing_staged(env: &Env, server: &str) {
    let data = data_dir(env, server);
    assert!(!data.join("user_scripts").exists(), "scripts were staged");
    assert!(
        !data.join("user_defined_functions").exists(),
        "a function file was written"
    );
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
fn deploy_to_a_stopped_server_stages_files_and_overlay() {
    let env = setup();
    create_stopped_server(&env, "default");
    write_python_udf(&env, "my_fn", true);

    let output = run(&env, &["local", "udf", "deploy", "my_fn", "--json"]);
    let json = success_json(&output);
    assert_eq!(
        json,
        serde_json::json!({
            "name": "my_fn",
            "server": "default",
            "type": "executable",
            "runtime": "python3.11",
            "server_running": false,
            "reloaded": false,
            "loaded": null,
            "interpreter": env.bin.join("python3.11").display().to_string(),
            "ignored_fields": ["memoryLimitMib"],
            "ignored_files": ["requirements.txt"],
            "function_config": ".clickhouse/servers/default/data/user_defined_functions/my_fn_function.xml",
            "scripts_dir": ".clickhouse/servers/default/data/user_scripts/my_fn"
        })
    );
    assert!(
        output.stderr.is_empty(),
        "JSON mode prints no notice: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let data = data_dir(&env, "default");
    let overlay = std::fs::read_to_string(data.join("config.d/chctl-udf.xml")).unwrap();
    let data_abs = data.canonicalize().unwrap();
    assert!(overlay.contains(&format!(
        "<user_defined_executable_functions_config>{}/user_defined_functions/*_function.xml</user_defined_executable_functions_config>",
        data_abs.display()
    )), "{overlay}");
    assert!(
        overlay.contains(&format!(
            "<user_scripts_path>{}/user_scripts/</user_scripts_path>",
            data_abs.display()
        )),
        "{overlay}"
    );

    let xml =
        std::fs::read_to_string(data.join("user_defined_functions/my_fn_function.xml")).unwrap();
    assert!(xml.contains("<name>my_fn</name>"), "{xml}");
    assert!(xml.contains("<execute_direct>0</execute_direct>"), "{xml}");
    assert!(
        xml.contains(&format!(
            "<command>'{}' '{}/user_scripts/my_fn/main.py'</command>",
            env.bin.join("python3.11").display(),
            data_abs.display()
        )),
        "{xml}"
    );
    assert!(xml.contains("<format>TabSeparated</format>"), "{xml}");
    assert!(!xml.contains("memory"), "{xml}");

    let sidecar: Value = serde_json::from_str(
        &std::fs::read_to_string(data.join("user_defined_functions/my_fn.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(sidecar["functionName"], "my_fn");

    let scripts = data.join("user_scripts/my_fn");
    assert!(scripts.join("main.py").is_file());
    assert!(scripts.join("lib/helper.py").is_file());
    assert!(
        scripts.join("requirements.txt").is_file(),
        "copied, just not installed"
    );
    assert!(!scripts.join("udf.json").exists());
    assert!(!scripts.join(".hidden").exists());
    assert!(!scripts.join("__pycache__").exists());

    let leftovers: Vec<String> = std::fs::read_dir(data.join("user_defined_functions"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftovers.iter().all(|name| !name.contains(".tmp-")),
        "{leftovers:?}"
    );
}

#[test]
fn deploy_without_requirements_reports_no_ignored_files() {
    let env = setup();
    create_stopped_server(&env, "default");
    write_python_udf(&env, "my_fn", false);
    let json = success_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    assert_eq!(json["ignored_files"], serde_json::json!([]));
}

#[cfg(not(target_os = "linux"))]
#[test]
fn deploy_native_is_unsupported_off_linux_and_writes_nothing() {
    let env = setup();
    create_stopped_server(&env, "default");
    write_native_udf(&env, "native_fn");

    let error = error_json(&run(
        &env,
        &["local", "udf", "deploy", "native_fn", "--json"],
    ));
    assert_eq!(error["error"]["code"], "udf_runtime_unsupported");
    assert_eq!(
        error["error"]["message"],
        "native UDFs are Linux binaries and cannot run on this host's local server; deploy them on Linux or use runtime python3.11"
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf deploy --help"
    );
    assert_nothing_staged(&env, "default");
}

#[cfg(target_os = "linux")]
#[test]
fn deploy_native_picks_the_host_architecture_and_marks_both_binaries_executable() {
    let env = setup();
    create_stopped_server(&env, "dev");
    write_native_udf(&env, "native_fn");

    let json = success_json(&run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "native_fn",
            "--server",
            "dev",
            "--json",
        ],
    ));
    assert_eq!(json["server"], "dev");
    assert_eq!(json["type"], "executable_pool");
    assert_eq!(json["runtime"], "native");
    assert_eq!(json["interpreter"], Value::Null);
    assert_eq!(json["ignored_files"], serde_json::json!([]));

    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("unexpected test host CPU {other}"),
    };
    let data = data_dir(&env, "dev");
    let xml = std::fs::read_to_string(data.join("user_defined_functions/native_fn_function.xml"))
        .unwrap();
    assert!(xml.contains("<execute_direct>1</execute_direct>"), "{xml}");
    assert!(
        xml.contains(&format!("<command>native_fn/{arch}/main</command>")),
        "{xml}"
    );
    assert!(xml.contains("<pool_size>2</pool_size>"), "{xml}");
    let scripts = data.join("user_scripts/native_fn");
    for arch in ["amd64", "arm64"] {
        assert!(is_executable(&scripts.join(arch).join("main")), "{arch}");
    }
    assert!(
        !scripts.join("src").exists(),
        "sources outside amd64/arm64 are not copied"
    );
    assert!(!scripts.join("Cargo.toml").exists());
}

#[test]
fn redeploy_replaces_previous_scripts_and_human_output_explains_next_start() {
    let env = setup();
    create_stopped_server(&env, "default");
    let dir = write_python_udf(&env, "my_fn", true);
    success_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    let scripts = data_dir(&env, "default").join("user_scripts/my_fn");
    std::fs::write(scripts.join("stale.txt"), "old\n").unwrap();
    std::fs::remove_file(dir.join("lib/helper.py")).unwrap();

    let output = run(&env, &["local", "udf", "deploy", "my_fn"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!scripts.join("stale.txt").exists());
    assert!(!scripts.join("lib/helper.py").exists());
    assert!(scripts.join("main.py").is_file());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("Deployed UDF my_fn to server 'default' (python3.11, executable)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  ignored Cloud-only fields: memoryLimitMib\n"),
        "{stdout}"
    );
    assert!(
        stdout
            .ends_with("Server 'default' is not running; the function loads on its next start.\n"),
        "{stdout}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "requirements.txt is not installed locally; install its packages into the interpreter you deploy with (e.g. a virtualenv passed with --python).\n"
    );
}

#[test]
fn deploy_reads_the_udf_from_another_parent_directory_with_dir() {
    let env = setup();
    create_stopped_server(&env, "default");
    let shared = tempfile::tempdir().unwrap();
    write_python_udf_at(&shared.path().join("udfs/shared_fn"), "shared_fn", false);

    let json = success_json(&run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "shared_fn",
            "--dir",
            shared.path().join("udfs").to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(json["name"], "shared_fn");
    assert!(
        data_dir(&env, "default")
            .join("user_scripts/shared_fn/main.py")
            .is_file()
    );
    assert!(
        !env.project
            .path()
            .join("clickhouse/udfs/shared_fn")
            .exists()
    );
}

#[test]
fn server_start_writes_the_udf_overlay_before_any_udf_exists() {
    let env = setup();
    let output = run(
        &env,
        &[
            "local",
            "--json",
            "server",
            "start",
            "--no-wait",
            "--version",
            VERSION,
        ],
    );
    let json = success_json(&output);
    let _guard = ProcessGuard(json["pid"].as_u64().expect("pid") as u32);

    let overlay = data_dir(&env, "default").join("config.d/chctl-udf.xml");
    let contents = std::fs::read_to_string(&overlay).expect("overlay written on start");
    assert!(contents.contains("<user_defined_executable_functions_config>"));
    assert!(contents.contains("user_defined_functions/*_function.xml"));
}

#[test]
fn list_and_remove_work_without_a_running_server() {
    let env = setup();
    create_stopped_server(&env, "default");
    write_python_udf(&env, "my_fn", false);
    write_python_udf(&env, "other_fn", false);
    for name in ["my_fn", "other_fn"] {
        success_json(&run(&env, &["local", "udf", "deploy", name, "--json"]));
    }

    let list = success_json(&run(&env, &["local", "udf", "list", "--json"]));
    assert_eq!(list["server"], "default");
    assert_eq!(list["server_running"], false);
    assert_eq!(
        list["udfs"],
        serde_json::json!([
            {"name": "my_fn", "type": "executable", "runtime": "python3.11", "loaded": null},
            {"name": "other_fn", "type": "executable", "runtime": "python3.11", "loaded": null}
        ])
    );
    let human = run(&env, &["local", "udf", "list"]);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("| my_fn"), "{stdout}");
    assert!(stdout.contains("| other_fn"), "{stdout}");
    assert!(
        stdout.ends_with(
            "Server 'default' is not running; loaded state is unknown until it starts.\n"
        ),
        "{stdout}"
    );

    let removed = success_json(&run(&env, &["local", "udf", "remove", "my_fn", "--json"]));
    assert_eq!(
        removed,
        serde_json::json!({"name": "my_fn", "server": "default", "reloaded": false})
    );
    let data = data_dir(&env, "default");
    assert!(
        !data
            .join("user_defined_functions/my_fn_function.xml")
            .exists()
    );
    assert!(!data.join("user_defined_functions/my_fn.json").exists());
    assert!(!data.join("user_scripts/my_fn").exists());
    assert!(data.join("user_scripts/other_fn").exists());

    let again = error_json(&run(&env, &["local", "udf", "remove", "my_fn", "--json"]));
    assert_eq!(again["error"]["code"], "udf_not_found");
    assert_eq!(
        again["error"]["message"],
        "UDF 'my_fn' is not deployed to server 'default'"
    );
    assert_eq!(
        again["error"]["command"],
        "clickhousectl local udf list --server default"
    );

    let human = run(&env, &["local", "udf", "remove", "other_fn"]);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "Removed UDF other_fn from server 'default'; the change applies on its next start\n"
    );
    let empty = run(&env, &["local", "udf", "list"]);
    assert_eq!(
        String::from_utf8_lossy(&empty.stdout),
        "No UDFs deployed to server 'default'\n"
    );
}

#[test]
fn definition_and_source_problems_are_structured_errors_before_anything_is_staged() {
    let env = setup();
    create_stopped_server(&env, "default");
    let dir = write_python_udf(&env, "my_fn", false);
    let args = ["local", "udf", "deploy", "my_fn", "--json"];

    std::fs::write(dir.join("udf.json"), "{ not json").unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(error["error"]["code"], "udf_definition_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF definition is not valid JSON"
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf init --help"
    );

    std::fs::write(
        dir.join("udf.json"),
        r#"{"functionName":"my_fn","type":"executable","runtime":"python3.11","arguments":[],"returnType":"String","typo":1}"#,
    )
    .unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(error["error"]["code"], "udf_definition_invalid");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("clickhouse/udfs/my_fn/udf.json"),
        "{message}"
    );
    assert!(message.contains("unknown field `typo`"), "{message}");

    std::fs::write(
        dir.join("udf.json"),
        r#"{"type":"executable","runtime":"python3.11","arguments":[],"returnType":"String"}"#,
    )
    .unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(
        error["error"]["message"],
        "UDF definition 'clickhouse/udfs/my_fn/udf.json' is invalid: UDF definition requires functionName"
    );

    std::fs::write(
        dir.join("udf.json"),
        r#"{"functionName":"other","type":"executable","runtime":"python3.11","arguments":[],"returnType":"String"}"#,
    )
    .unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(error["error"]["code"], "udf_definition_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF definition 'clickhouse/udfs/my_fn/udf.json' is invalid: functionName is other, but the command targets my_fn"
    );

    std::fs::remove_file(dir.join("udf.json")).unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(error["error"]["code"], "udf_source_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF source directory 'clickhouse/udfs/my_fn' has no udf.json"
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf deploy --help"
    );

    write_python_udf(&env, "my_fn", false);
    std::fs::remove_file(dir.join("main.py")).unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(error["error"]["code"], "udf_source_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF source directory 'clickhouse/udfs/my_fn' is missing main.py, the entrypoint required by runtime python3.11"
    );

    std::fs::write(dir.join("main.py"), "").unwrap();
    std::os::unix::fs::symlink(dir.join("main.py"), dir.join("link.py")).unwrap();
    let error = error_json(&run(&env, &args));
    assert_eq!(
        error["error"]["message"],
        "UDF source directory 'clickhouse/udfs/my_fn' contains a symbolic link at link.py"
    );
    std::fs::remove_file(dir.join("link.py")).unwrap();

    std::os::unix::fs::symlink(&dir, env.project.path().join("clickhouse/udfs/linked")).unwrap();
    let error = error_json(&run(&env, &["local", "udf", "deploy", "linked", "--json"]));
    assert_eq!(error["error"]["code"], "udf_source_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF source directory 'clickhouse/udfs/linked' is a symbolic link"
    );

    let missing_dir = error_json(&run(&env, &["local", "udf", "deploy", "nope", "--json"]));
    assert_eq!(missing_dir["error"]["code"], "udf_source_invalid");
    assert_eq!(
        missing_dir["error"]["message"],
        "UDF source directory 'clickhouse/udfs/nope' is not a directory"
    );

    assert_nothing_staged(&env, "default");
}

#[test]
fn missing_python_interpreter_is_a_structured_error_and_explicit_python_is_honoured() {
    let env = setup();
    create_stopped_server(&env, "default");
    write_python_udf(&env, "my_fn", false);

    let output = command(&env)
        .env("PATH", "/nonexistent")
        .args(["local", "udf", "deploy", "my_fn", "--json"])
        .output()
        .unwrap();
    let error = error_json(&output);
    assert_eq!(error["error"]["code"], "udf_interpreter_not_found");
    assert_eq!(
        error["error"]["message"],
        "No python3.11 or python3 interpreter found on PATH; pass --python <PATH>"
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local udf deploy --help"
    );
    assert_nothing_staged(&env, "default");

    let custom = env.home.path().join("custom/python");
    write_executable(&custom, "#!/bin/sh\nexit 0\n");
    let output = command(&env)
        .env("PATH", "/nonexistent")
        .args([
            "local",
            "udf",
            "deploy",
            "my_fn",
            "--python",
            custom.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    let json = success_json(&output);
    assert_eq!(json["interpreter"], custom.display().to_string());

    let output = run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "my_fn",
            "--python",
            "/nonexistent/python",
            "--json",
        ],
    );
    let error = error_json(&output);
    assert_eq!(error["error"]["code"], "udf_interpreter_not_found");
    assert_eq!(
        error["error"]["message"],
        "Python interpreter '/nonexistent/python' is not an executable file"
    );
}

#[test]
fn unknown_servers_are_never_created_and_stopped_servers_reject_reload() {
    let env = setup();
    write_python_udf(&env, "my_fn", false);

    let error = error_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    assert_eq!(error["error"]["code"], "server_not_found");
    assert_eq!(
        error["error"]["message"],
        "Server 'default' not found; create it with `clickhousectl local server start default`"
    );
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local server start default"
    );
    assert!(
        !env.project
            .path()
            .join(".clickhouse/servers/default")
            .exists(),
        "deploy must not create a server"
    );

    for args in [
        vec!["local", "udf", "reload", "--server", "ghost", "--json"],
        vec!["local", "udf", "list", "--server", "ghost", "--json"],
        vec![
            "local", "udf", "remove", "my_fn", "--server", "ghost", "--json",
        ],
        vec![
            "local", "udf", "deploy", "my_fn", "--server", "ghost", "--json",
        ],
    ] {
        let error = error_json(&run(&env, &args));
        assert_eq!(error["error"]["code"], "server_not_found", "{args:?}");
        assert_eq!(
            error["error"]["command"], "clickhousectl local server start ghost",
            "{args:?}"
        );
    }
    assert!(
        !env.project
            .path()
            .join(".clickhouse/servers/ghost")
            .exists()
    );

    create_stopped_server(&env, "default");
    success_json(&run(&env, &["local", "udf", "deploy", "my_fn", "--json"]));
    let error = error_json(&run(&env, &["local", "udf", "reload", "--json"]));
    assert_eq!(error["error"]["code"], "server_not_running");
    assert_eq!(error["error"]["command"], "clickhousectl local server list");
}

#[test]
fn server_naming_a_local_postgres_instance_is_rejected_before_anything_is_written() {
    let env = setup();
    write_python_udf(&env, "my_fn", false);
    let servers = env.project.path().join(".clickhouse/servers");
    // A stopped local Postgres: metadata with the Postgres engine and a data
    // directory. `old-pg16` has only the data directory, as after a crash.
    std::fs::create_dir_all(servers.join("dev-pg18/data")).unwrap();
    std::fs::write(
        servers.join("dev-pg18.json"),
        serde_json::json!({
            "name": "dev",
            "pid": 0,
            "version": "postgres:18",
            "http_port": 0,
            "tcp_port": 5432,
            "started_at": "2026-10-09T00:00:00Z",
            "cwd": env.project.path().display().to_string(),
            "engine": "postgres"
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(servers.join("old-pg16/data")).unwrap();

    for server in ["dev-pg18", "old-pg16"] {
        for args in [
            vec![
                "local", "udf", "deploy", "my_fn", "--server", server, "--json",
            ],
            vec!["local", "udf", "list", "--server", server, "--json"],
            vec!["local", "udf", "reload", "--server", server, "--json"],
            vec![
                "local", "udf", "remove", "my_fn", "--server", server, "--json",
            ],
        ] {
            let error = error_json(&run(&env, &args));
            assert_eq!(error["error"]["code"], "server_not_found", "{args:?}");
            assert_eq!(
                error["error"]["message"],
                format!("'{server}' is a local Postgres instance, not a ClickHouse server"),
                "{args:?}"
            );
            assert_eq!(
                error["error"]["command"], "clickhousectl local server list",
                "{args:?}"
            );
        }
        let entries: Vec<_> = std::fs::read_dir(servers.join(server).join("data"))
            .unwrap()
            .collect();
        assert!(entries.is_empty(), "{server}: {entries:?}");
    }
}
