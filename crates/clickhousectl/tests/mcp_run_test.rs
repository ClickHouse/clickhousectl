//! Subprocess coverage for `mcp run` and `mcp add` with fake `clickhouse` and
//! `uv` executables: which ClickHouse build reads the client config, the
//! environment handed to mcp-clickhouse, and the agent config written.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OLDER: &str = "25.12.9.61";
const NEWER: &str = "26.9.1.1312";

struct Sandbox {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
    bin: tempfile::TempDir,
}

/// A fake `clickhouse extract-from-config`: logs which build ran, answers keys
/// from `<config>.kv` (`key=value` lines) and fails like ClickHouse on a miss.
fn fake_clickhouse(version: &str, log: &Path) -> String {
    format!(
        r#"#!/bin/sh
echo {version} >> '{log}'
[ "$1" = extract-from-config ] || exit 99
config="$3"; key="$5"
while IFS= read -r line; do
  case "$line" in "$key="*) printf '%s\n' "${{line#*=}}"; exit 0;; esac
done < "$config.kv"
echo "Poco::Exception. Code: 1000, e.code() = 0, Not found: $key" >&2
exit 232
"#,
        log = log.display()
    )
}

fn write_executable(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Sandbox {
            home: tempfile::tempdir().unwrap(),
            project: tempfile::tempdir().unwrap(),
            bin: tempfile::tempdir().unwrap(),
        };
        // A fake `uv` that records its arguments and the CLICKHOUSE_* environment.
        write_executable(
            &sandbox.bin.path().join("uv"),
            &format!(
                "#!/bin/sh\necho \"$@\" > '{out}.args'\nenv | grep '^CLICKHOUSE_' | sort > '{out}.env'\n",
                out = sandbox.uv_out().display()
            ),
        );
        sandbox
    }

    fn uv_out(&self) -> PathBuf {
        self.bin.path().join("uv-out")
    }

    fn binary_log(&self) -> PathBuf {
        self.bin.path().join("clickhouse.log")
    }

    fn install(&self, version: &str) {
        let binary = self
            .home
            .path()
            .join(".clickhouse/versions")
            .join(version)
            .join("clickhouse");
        write_executable(&binary, &fake_clickhouse(version, &self.binary_log()));
    }

    fn set_default(&self, version: &str) {
        fs::write(self.home.path().join(".clickhouse/default"), version).unwrap();
    }

    fn config(&self, kv: &str) -> PathBuf {
        let config = self.project.path().join("client.xml");
        fs::write(&config, "<config/>").unwrap();
        fs::write(self.project.path().join("client.xml.kv"), kv).unwrap();
        config
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", self.home.path())
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.bin.path().display()),
            )
            .current_dir(self.project.path())
            .args(args);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn uv_env(&self) -> Vec<String> {
        let path = self.uv_out().with_extension("env");
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(String::from)
            .collect()
    }

    fn builds_used(&self) -> Vec<String> {
        fs::read_to_string(self.binary_log())
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

const PROD: &str = "\
user=topuser
connections_credentials.connection[0].name=prod
connections_credentials.connection[0].hostname=prod.example.com
connections_credentials.connection[0].port=9440
connections_credentials.connection[0].secure=1
connections_credentials.connection[0].password=s3cret
";

#[test]
fn run_reads_the_config_with_the_default_build_and_execs_mcp_clickhouse() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.install(NEWER);
    // The default is the older build; a newer install must not displace it.
    sandbox.set_default(OLDER);
    sandbox.config(PROD);

    let output = sandbox
        .command(&[
            "mcp",
            "run",
            "--client-config",
            "client.xml",
            "--connection",
            "prod",
        ])
        .env("CLICKHOUSE_DATABASE", "inherited")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // stdout belongs to the MCP protocol; clickhousectl writes nothing to it.
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let builds = sandbox.builds_used();
    assert!(!builds.is_empty());
    assert!(builds.iter().all(|b| b == OLDER), "{builds:?}");

    let args = fs::read_to_string(sandbox.uv_out().with_extension("args")).unwrap();
    assert_eq!(args.trim(), "tool run mcp-clickhouse");
    assert_eq!(
        sandbox.uv_env(),
        [
            "CLICKHOUSE_HOST=prod.example.com",
            "CLICKHOUSE_PASSWORD=s3cret",
            "CLICKHOUSE_PORT=8443",
            "CLICKHOUSE_SECURE=true",
            "CLICKHOUSE_USER=topuser",
            "CLICKHOUSE_VERIFY=true",
        ]
    );
}

#[test]
fn installed_builds_without_a_default_are_not_replaced_by_latest() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.config(PROD);

    let output = sandbox.run(&["mcp", "run", "--client-config", "client.xml"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("local use <version>"), "{stderr}");
    assert!(!sandbox.home.path().join(".clickhouse/default").exists());
    assert!(!sandbox.uv_out().with_extension("args").exists());
}

#[test]
fn a_stale_default_is_reported_rather_than_reinstalled() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.set_default(NEWER);
    sandbox.config(PROD);

    let output = sandbox.run(&["mcp", "run", "--client-config", "client.xml"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains(NEWER), "{stderr}");
    assert!(sandbox.builds_used().is_empty());
}

#[test]
fn missing_uv_is_an_actionable_error() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.set_default(OLDER);
    sandbox.config(PROD);
    fs::remove_file(sandbox.bin.path().join("uv")).unwrap();

    let output = sandbox.run(&["mcp", "run", "--client-config", "client.xml"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("`uv`"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn local_source_requires_a_running_project_server() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["mcp", "run", "--local"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("'default'"), "{stderr}");
    assert!(output.stdout.is_empty());
}

fn read_mcp_json(sandbox: &Sandbox) -> serde_json::Value {
    let text = fs::read_to_string(sandbox.project.path().join(".mcp.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn add_writes_a_launcher_entry_with_an_absolute_config_path() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.set_default(OLDER);
    let config = sandbox.config(PROD);
    fs::write(
        sandbox.project.path().join(".mcp.json"),
        r#"{"mcpServers": {"github": {"command": "gh"}}}"#,
    )
    .unwrap();

    let output = sandbox.run(&[
        "mcp",
        "add",
        "--agent",
        "claude",
        "--client-config",
        "client.xml",
        "--connection",
        "prod",
        "--json",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let reported: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reported["status"], "created");

    let document = read_mcp_json(&sandbox);
    assert_eq!(document["mcpServers"]["github"]["command"], "gh");
    let entry = &document["mcpServers"]["clickhouse"];
    assert_eq!(entry["command"], "clickhousectl");
    let config = fs::canonicalize(config).unwrap();
    assert_eq!(
        entry["args"],
        serde_json::json!([
            "mcp",
            "run",
            "--client-config",
            config.display().to_string(),
            "--connection",
            "prod"
        ])
    );
    // No credentials are written to the agent config.
    assert!(!document.to_string().contains("s3cret"));
}

#[test]
fn add_validates_the_connection_before_writing() {
    let sandbox = Sandbox::new();
    sandbox.install(OLDER);
    sandbox.set_default(OLDER);
    sandbox.config(PROD);

    let output = sandbox.run(&[
        "mcp",
        "add",
        "--agent",
        "claude",
        "--client-config",
        "client.xml",
        "--connection",
        "missing",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("available: prod"), "{stderr}");
    assert!(!sandbox.project.path().join(".mcp.json").exists());
}

#[test]
fn add_cloud_registers_the_hosted_server_without_a_clickhouse_build() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["mcp", "add", "--agent", "claude", "--cloud"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let entry = &read_mcp_json(&sandbox)["mcpServers"]["clickhouse"];
    assert_eq!(entry["type"], "http");
    assert_eq!(entry["url"], "https://mcp.clickhouse.cloud/mcp");
    assert!(!sandbox.home.path().join(".clickhouse/versions").exists());
}
