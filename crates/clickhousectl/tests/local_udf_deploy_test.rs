//! Subprocess coverage for `local udf deploy|list|remove|reload|call` when the
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

fn write_python_udf(env: &Env, name: &str) -> PathBuf {
    let dir = env.project.path().join("clickhouse/udfs").join(name);
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
    dir
}

fn write_native_udf(env: &Env, name: &str) -> PathBuf {
    let dir = env.project.path().join("clickhouse/udfs").join(name);
    std::fs::create_dir_all(&dir).unwrap();
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
    // Deliberately not executable: deploy must set the bit.
    std::fs::write(dir.join("main"), "#!/bin/sh\ncat\n").unwrap();
    dir
}

fn data_dir(env: &Env, server: &str) -> PathBuf {
    env.project
        .path()
        .join(".clickhouse/servers")
        .join(server)
        .join("data")
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o111 == 0o111
}

fn install_fake_clickhouse(env: &Env) {
    write_executable(
        &env.home
            .path()
            .join(".clickhouse/versions")
            .join(VERSION)
            .join("clickhouse"),
        "#!/bin/sh\nexec sleep 30\n",
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
fn deploy_without_a_running_server_stages_files_and_overlay() {
    let env = setup();
    write_python_udf(&env, "my_fn");

    let output = run(
        &env,
        &["local", "udf", "deploy", "clickhouse/udfs/my_fn", "--json"],
    );
    let json = success_json(&output);
    assert_eq!(json["name"], "my_fn");
    assert_eq!(json["server"], "default");
    assert_eq!(json["type"], "executable");
    assert_eq!(json["runtime"], "python3.11");
    assert_eq!(json["server_running"], false);
    assert_eq!(json["reloaded"], false);
    assert_eq!(json["loaded"], Value::Null);
    assert_eq!(
        json["ignored_fields"],
        serde_json::json!(["memoryLimitMib"])
    );
    assert_eq!(
        json["interpreter"],
        env.bin.join("python3.11").display().to_string()
    );
    assert_eq!(
        json["function_config"],
        ".clickhouse/servers/default/data/user_defined_functions/my_fn_function.xml"
    );
    assert_eq!(
        json["scripts_dir"],
        ".clickhouse/servers/default/data/user_scripts/my_fn"
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
fn deploy_native_marks_the_entrypoint_executable_and_uses_execute_direct() {
    let env = setup();
    write_native_udf(&env, "native_fn");

    let output = run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "clickhouse/udfs/native_fn",
            "--server",
            "dev",
            "--json",
        ],
    );
    let json = success_json(&output);
    assert_eq!(json["server"], "dev");
    assert_eq!(json["type"], "executable_pool");
    assert_eq!(json["runtime"], "native");
    assert_eq!(json["interpreter"], Value::Null);
    assert_eq!(json["ignored_fields"], serde_json::json!([]));

    let data = data_dir(&env, "dev");
    let xml = std::fs::read_to_string(data.join("user_defined_functions/native_fn_function.xml"))
        .unwrap();
    assert!(xml.contains("<execute_direct>1</execute_direct>"), "{xml}");
    assert!(xml.contains("<command>native_fn/main</command>"), "{xml}");
    assert!(xml.contains("<pool_size>2</pool_size>"), "{xml}");
    assert!(is_executable(&data.join("user_scripts/native_fn/main")));
}

#[test]
fn redeploy_replaces_previous_scripts_and_human_output_explains_next_start() {
    let env = setup();
    let dir = write_python_udf(&env, "my_fn");
    success_json(&run(
        &env,
        &["local", "udf", "deploy", "clickhouse/udfs/my_fn", "--json"],
    ));
    let scripts = data_dir(&env, "default").join("user_scripts/my_fn");
    std::fs::write(scripts.join("stale.txt"), "old\n").unwrap();
    std::fs::remove_file(dir.join("lib/helper.py")).unwrap();

    let output = run(&env, &["local", "udf", "deploy", "clickhouse/udfs/my_fn"]);
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
}

#[test]
fn server_start_writes_the_udf_overlay_before_any_udf_exists() {
    let env = setup();
    install_fake_clickhouse(&env);

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
    write_python_udf(&env, "my_fn");
    write_native_udf(&env, "native_fn");
    for name in ["my_fn", "native_fn"] {
        success_json(&run(
            &env,
            &[
                "local",
                "udf",
                "deploy",
                &format!("clickhouse/udfs/{name}"),
                "--json",
            ],
        ));
    }

    let list = success_json(&run(&env, &["local", "udf", "list", "--json"]));
    assert_eq!(list["server"], "default");
    assert_eq!(list["server_running"], false);
    assert_eq!(
        list["udfs"],
        serde_json::json!([
            {"name": "my_fn", "type": "executable", "runtime": "python3.11", "loaded": null},
            {"name": "native_fn", "type": "executable_pool", "runtime": "native", "loaded": null}
        ])
    );
    let human = run(&env, &["local", "udf", "list"]);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("| my_fn"), "{stdout}");
    assert!(stdout.contains("| native_fn"), "{stdout}");
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
    assert!(data.join("user_scripts/native_fn").exists());

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

    let human = run(&env, &["local", "udf", "remove", "native_fn"]);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "Removed UDF native_fn from server 'default'; the change applies on its next start\n"
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
    let dir = write_python_udf(&env, "my_fn");
    let args = ["local", "udf", "deploy", "clickhouse/udfs/my_fn", "--json"];

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
    assert_eq!(error["error"]["code"], "udf_definition_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF definition 'clickhouse/udfs/my_fn/udf.json' is invalid: UDF definition requires functionName"
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

    write_python_udf(&env, "my_fn");
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
    assert_eq!(error["error"]["code"], "udf_source_invalid");
    assert_eq!(
        error["error"]["message"],
        "UDF source directory 'clickhouse/udfs/my_fn' contains a symbolic link at link.py"
    );
    std::fs::remove_file(dir.join("link.py")).unwrap();

    let missing_dir = error_json(&run(
        &env,
        &["local", "udf", "deploy", "clickhouse/udfs/nope", "--json"],
    ));
    assert_eq!(missing_dir["error"]["code"], "udf_source_invalid");
    assert_eq!(
        missing_dir["error"]["message"],
        "UDF source directory 'clickhouse/udfs/nope' is not a directory"
    );

    assert!(
        !env.project
            .path()
            .join(".clickhouse/servers/default")
            .exists(),
        "nothing is staged when validation fails"
    );
}

#[test]
fn missing_python_interpreter_is_a_structured_error_and_explicit_python_is_honoured() {
    let env = setup();
    write_python_udf(&env, "my_fn");

    let output = command(&env)
        .env("PATH", "/nonexistent")
        .args(["local", "udf", "deploy", "clickhouse/udfs/my_fn", "--json"])
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

    let custom = env.home.path().join("custom/python");
    write_executable(&custom, "#!/bin/sh\nexit 0\n");
    let output = command(&env)
        .env("PATH", "/nonexistent")
        .args([
            "local",
            "udf",
            "deploy",
            "clickhouse/udfs/my_fn",
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
            "clickhouse/udfs/my_fn",
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
fn reload_call_and_list_report_server_state_errors() {
    let env = setup();

    for args in [
        vec!["local", "udf", "reload", "--json"],
        vec!["local", "udf", "call", "my_fn", "1", "--json"],
        vec!["local", "udf", "list", "--server", "ghost", "--json"],
        vec![
            "local", "udf", "remove", "my_fn", "--server", "ghost", "--json",
        ],
    ] {
        let error = error_json(&run(&env, &args));
        assert_eq!(error["error"]["code"], "server_not_found", "{args:?}");
    }

    write_python_udf(&env, "my_fn");
    success_json(&run(
        &env,
        &["local", "udf", "deploy", "clickhouse/udfs/my_fn", "--json"],
    ));
    for args in [
        vec!["local", "udf", "reload", "--json"],
        vec!["local", "udf", "call", "my_fn", "1", "--json"],
    ] {
        let error = error_json(&run(&env, &args));
        assert_eq!(error["error"]["code"], "server_not_running", "{args:?}");
        assert_eq!(
            error["error"]["command"], "clickhousectl local server list",
            "{args:?}"
        );
    }
}

#[test]
fn a_staged_server_is_not_listed_but_is_removable() {
    let env = setup();
    write_python_udf(&env, "my_fn");
    success_json(&run(
        &env,
        &[
            "local",
            "udf",
            "deploy",
            "clickhouse/udfs/my_fn",
            "--server",
            "staged",
            "--json",
        ],
    ));
    assert!(
        data_dir(&env, "staged")
            .join("user_scripts/my_fn/main.py")
            .is_file()
    );

    // A staged-but-never-started server is not listed, but it is removable.
    let list = success_json(&run(&env, &["local", "--json", "server", "list"]));
    assert_eq!(list["total_servers"], 0, "{list}");

    let removed = run(&env, &["local", "--json", "server", "remove", "staged"]);
    assert!(
        removed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(
        !env.project
            .path()
            .join(".clickhouse/servers/staged")
            .exists()
    );
}
