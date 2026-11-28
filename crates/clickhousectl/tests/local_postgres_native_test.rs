//! Subprocess coverage for the native `local postgres` runtime, without the
//! network or a real PostgreSQL: the bundle source is a `file://` directory
//! holding a manifest and a tiny bundle whose programs are shell scripts.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const PG_VERSION: &str = "18.7";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        other => panic!("no bundle target for {other:?}"),
    }
}

fn bundle_name() -> String {
    format!("pg-{PG_VERSION}-pgch-0.11.1-{}", target())
}

/// The fake programs. `postgres` behaves like a postmaster as far as the CLI
/// can tell: it runs from the data directory, records its PID there and
/// exits on SIGINT. `FAKE_*` variables reach them through the CLI's
/// environment and select a failure.
const PROGRAMS: [(&str, &str); 5] = [
    (
        "initdb",
        r#"#!/bin/sh
if [ -n "$FAKE_ASSERT_NO_PG_ENV" ] && [ -n "${PGSSLMODE}${PGOPTIONS}${PGHOST}${PGDATA}${PGPASSWORD}" ]; then
  echo "leaked PG env into initdb" >&2
  exit 91
fi
data=""
while [ $# -gt 0 ]; do
  [ "$1" = "-D" ] && data="$2"
  printf '%s\n' "$1" >> "$FAKE_RECORD_DIR/initdb.args"
  case "$1" in --pwfile=*) cp "${1#--pwfile=}" "$FAKE_RECORD_DIR/pwfile" ;; esac
  shift
done
mkdir -p "$data" && echo 18 > "$data/PG_VERSION"
"#,
    ),
    (
        "postgres",
        r#"#!/bin/sh
if [ -n "$FAKE_ASSERT_NO_PG_ENV" ] && [ -n "${PGSSLMODE}${PGOPTIONS}${PGHOST}${PGDATA}${PGPASSWORD}" ]; then
  echo "leaked PG env into postgres" >&2
  exit 92
fi
[ -n "$FAKE_POSTGRES_EXIT" ] && { echo "FATAL: fake postmaster failure" >&2; exit "$FAKE_POSTGRES_EXIT"; }
while [ $# -gt 0 ]; do
  [ "$1" = "-D" ] && data="$2"
  shift
done
cd "$data" || exit 1
echo $$ > postmaster.pid
echo $$ > "$FAKE_RECORD_DIR/postmaster.pid"
exec sleep 600
"#,
    ),
    // Ready while the recorded postmaster is alive.
    (
        "pg_isready",
        r#"#!/bin/sh
if [ -n "$FAKE_ASSERT_NO_PG_ENV" ] && [ -n "${PGSSLMODE}${PGOPTIONS}${PGHOST}${PGDATA}${PGPASSWORD}" ]; then
  echo "leaked PG env into pg_isready" >&2
  exit 93
fi
kill -0 "$(cat "$FAKE_RECORD_DIR/postmaster.pid" 2>/dev/null)" 2>/dev/null
"#,
    ),
    (
        "createdb",
        r#"#!/bin/sh
if [ -n "$FAKE_ASSERT_NO_PG_ENV" ] && [ -n "${PGSSLMODE}${PGOPTIONS}${PGHOST}${PGDATA}" ]; then
  echo "leaked PG env into createdb" >&2
  exit 94
fi
if [ -n "$FAKE_EXPECT_PGPASSWORD" ] && [ "$PGPASSWORD" != "$FAKE_EXPECT_PGPASSWORD" ]; then
  echo "unexpected PGPASSWORD in createdb: $PGPASSWORD" >&2
  exit 95
fi
[ -n "$FAKE_CREATEDB_EXIT" ] && { echo 'fake createdb failure' >&2; exit "$FAKE_CREATEDB_EXIT"; }
exit 0
"#,
    ),
    (
        "psql",
        r#"#!/bin/sh
if [ -n "$FAKE_ASSERT_NO_PG_ENV" ] && [ -n "${PGSSLMODE}${PGOPTIONS}${PGHOST}${PGDATA}" ]; then
  echo "leaked PG env into psql" >&2
  exit 96
fi
if [ -n "$FAKE_EXPECT_PGPASSWORD" ] && [ "$PGPASSWORD" != "$FAKE_EXPECT_PGPASSWORD" ]; then
  echo "unexpected PGPASSWORD in psql: $PGPASSWORD" >&2
  exit 97
fi
if [ -n "$FAKE_EXPECT_PSQLRC" ] && [ "$PSQLRC" != "$FAKE_EXPECT_PSQLRC" ]; then
  echo "PSQLRC was lost in psql: $PSQLRC" >&2
  exit 98
fi
printf 'fake-psql'; printf ' %s' "$@"; printf '\n'
"#,
    ),
];

/// A bundle tarball laid out as the real ones are: one top-level directory
/// holding `bin/`.
fn bundle_tarball() -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);
    for (program, script) in PROGRAMS {
        let mut header = tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                format!("{}/bin/{program}", bundle_name()),
                script.as_bytes(),
            )
            .expect("append fake program");
    }
    archive
        .into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip")
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
    source: PathBuf,
    record: PathBuf,
    sha256: String,
}

impl Fixture {
    /// A project, a home and a bundle source whose manifest is `manifest`
    /// applied to the true description of the one bundle.
    fn with_manifest(manifest: impl FnOnce(&mut Value)) -> Self {
        let root = tempfile::tempdir().expect("create fixture root");
        // Canonical, as the CLI records it (`/var` is a symlink on macOS).
        let base = root.path().canonicalize().expect("canonical fixture root");
        let [home, project, source, record] =
            ["home", "project", "source", "record"].map(|dir| base.join(dir));
        for dir in [&home, &project, &source, &record] {
            std::fs::create_dir(dir).expect("create fixture dir");
        }
        std::fs::create_dir(project.join(".clickhouse")).expect("mark project");

        let tarball = bundle_tarball();
        let sha256 = sha256(&tarball);
        let file = format!("{}.tar.gz", bundle_name());
        std::fs::write(source.join(&file), &tarball).expect("write bundle");
        let mut document = json!({
            "schema": 1,
            "release": "2026.11.28",
            "built_at": "2026-11-16T00:00:00Z",
            "source": { "repo": "sdairs/postgres-binaries", "commit": "0000000" },
            "bundles": [{
                "postgres_version": PG_VERSION,
                "postgres_major": "18",
                "extensions": { "pg_clickhouse": "0.11.1" },
                "target": target(),
                "file": file,
                "sha256": sha256,
                "size": tarball.len(),
            }],
        });
        manifest(&mut document);
        std::fs::write(
            source.join("manifest.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .expect("write manifest");
        Self {
            _root: root,
            home,
            project,
            source,
            record,
            sha256,
        }
    }

    fn new() -> Self {
        Self::with_manifest(|_| {})
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            // No Docker: nothing here may need it.
            .env("DOCKER_HOST", "unix:///nonexistent/docker.sock")
            .env(
                "CLICKHOUSECTL_POSTGRES_BUNDLES_URL",
                format!("file://{}", self.source.display()),
            )
            .env("FAKE_RECORD_DIR", &self.record)
            .current_dir(&self.project)
            .args(["local", "--json", "postgres"])
            .args(args);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run clickhousectl")
    }

    fn ok(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("parse JSON stdout")
    }

    fn instance_dir(&self) -> PathBuf {
        self.project.join(".clickhouse/servers/default-pg18")
    }

    fn cache_dir(&self) -> PathBuf {
        self.home.join(".clickhouse/postgres")
    }

    fn cached_bundles(&self) -> Vec<String> {
        match std::fs::read_dir(self.cache_dir()) {
            Ok(entries) => entries
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The PID of the last fake postmaster this fixture's CLI spawned.
    fn postmaster_pid(&self) -> i32 {
        std::fs::read_to_string(self.record.join("postmaster.pid"))
            .expect("fake postmaster recorded its PID")
            .trim()
            .parse()
            .expect("PID is a number")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // A failed assertion must not leave a fake postmaster behind.
        if let Ok(pid) = std::fs::read_to_string(self.record.join("postmaster.pid"))
            && let Ok(pid) = pid.trim().parse::<i32>()
        {
            // SAFETY: `kill` takes plain integers and touches no memory.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
    }
}

fn process_is_gone(pid: i32) -> bool {
    for _ in 0..200 {
        // SAFETY: `kill` takes plain integers and touches no memory.
        if unsafe { libc::kill(pid, 0) } != 0 {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}

fn error_of(output: &Output) -> Value {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let start = stderr
        .find("{\n  \"error\"")
        .unwrap_or_else(|| panic!("no JSON error on stderr: {stderr}"));
    serde_json::from_str::<Value>(&stderr[start..]).expect("parse JSON error")["error"].clone()
}

#[test]
fn native_instance_installs_once_then_starts_stops_resumes_offline_and_is_removed() {
    let fixture = Fixture::new();
    let password = "example-password";

    let started = fixture.ok(&["start", "--password", password, "--database", "app"]);
    assert_eq!(started["name"], "default");
    assert_eq!(started["runtime"], "native");
    assert_eq!(started["image"], format!("postgres:{PG_VERSION}"));
    assert_eq!(started["extensions"]["pg_clickhouse"], "0.11.1");
    assert_eq!(started["password"], password);
    assert_eq!(started["database"], "app");
    assert!(started.get("container_id").is_none());
    let pid = fixture.postmaster_pid();
    assert_eq!(started["pid"], pid);
    let port = started["port"].as_u64().expect("port");

    // The cache directory is keyed on the bundle name and its sha256.
    assert_eq!(
        fixture.cached_bundles(),
        [format!("{}-{}", bundle_name(), &fixture.sha256[..12])]
    );
    // The password reached initdb through a file, never its arguments.
    let initdb_args = std::fs::read_to_string(fixture.record.join("initdb.args")).unwrap();
    assert!(!initdb_args.contains(password), "{initdb_args}");
    assert_eq!(
        std::fs::read_to_string(fixture.record.join("pwfile"))
            .unwrap()
            .trim_end(),
        password
    );
    // Credentials are stored for this user only, with no bundle path.
    let state_path = fixture.instance_dir().join("native.json");
    let mode = std::fs::metadata(&state_path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(state["postgres_major"], "18");
    assert_eq!(state["bundle"]["sha256"], fixture.sha256);

    // Listed where a Docker instance is: its own list and `local status`.
    let listed = fixture.ok(&["list"]);
    assert_eq!(listed["total_running_servers"], 1);
    let entry = &listed["servers"][0];
    assert_eq!(entry["name"], "default");
    assert_eq!(entry["engine"], "postgres");
    assert_eq!(entry["running"], true);
    assert_eq!(entry["pid"], pid);
    assert_eq!(entry["tcp_port"], port);
    assert!(entry.get("container_id").is_none());

    let dotenv = fixture.ok(&["dotenv", "--local"]);
    let var = |key: &str| {
        dotenv["vars"]
            .as_array()
            .unwrap()
            .iter()
            .find(|var| var["key"] == key)
            .unwrap_or_else(|| panic!("{key} missing"))["value"]
            .clone()
    };
    assert_eq!(var("POSTGRES_PORT"), port.to_string());
    assert_eq!(var("POSTGRES_PASSWORD"), password);
    assert_eq!(var("POSTGRES_DATABASE"), "app");

    // `client` runs the bundle's psql against the instance.
    let client = fixture.run(&["client", "--query", "SELECT 1"]);
    assert!(client.status.success());
    let psql = String::from_utf8_lossy(&client.stdout);
    assert!(psql.starts_with("fake-psql "), "{psql}");
    assert!(psql.contains(&format!("-p {port} ")), "{psql}");

    let stopped = fixture.ok(&["stop"]);
    assert_eq!(stopped["already_stopped"], false);
    assert!(process_is_gone(pid));
    assert_eq!(fixture.ok(&["list"])["total_running_servers"], 0);

    // A resume needs neither the bundle source nor a new install, and keeps
    // the stored port and credentials whatever is passed.
    let resumed = fixture
        .command(&["start", "--password", "ignored-on-resume"])
        .env(
            "CLICKHOUSECTL_POSTGRES_BUNDLES_URL",
            "file:///nonexistent/bundles",
        )
        .output()
        .expect("resume");
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let resumed: Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(resumed["port"], port);
    assert_eq!(resumed["password"], password);
    let resumed_pid = fixture.postmaster_pid();
    assert_ne!(resumed_pid, pid);
    assert_eq!(fixture.cached_bundles().len(), 1);

    // `remove` stops a running native instance and leaves nothing behind.
    fixture.ok(&["remove"]);
    assert!(process_is_gone(resumed_pid));
    assert!(!fixture.instance_dir().exists());
    assert_eq!(fixture.ok(&["list"])["total_servers"], 0);
}

#[test]
fn checksum_mismatch_installs_nothing_and_starts_nothing() {
    let fixture = Fixture::with_manifest(|manifest| {
        manifest["bundles"][0]["sha256"] = "0".repeat(64).into();
    });

    let output = fixture.run(&["start"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(error_of(&output)["code"], "checksum_mismatch");
    assert!(fixture.cached_bundles().is_empty());
    assert!(!fixture.instance_dir().exists());
    assert!(!fixture.record.join("initdb.args").exists());
}

#[test]
fn unusable_manifests_fail_with_a_bundle_error_before_any_install() {
    for edit in [
        (|manifest: &mut Value| manifest["schema"] = 2.into()) as fn(&mut Value),
        // Nothing published for this machine.
        |manifest| manifest["bundles"][0]["target"] = "riscv64-unknown-none".into(),
        // A manifest entry without a digest is never installed.
        |manifest| manifest["bundles"][0]["sha256"] = "".into(),
    ] {
        let fixture = Fixture::with_manifest(edit);

        let output = fixture.run(&["start"]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(error_of(&output)["code"], "postgres_bundle_unavailable");
        assert!(fixture.cached_bundles().is_empty());
        assert!(!fixture.instance_dir().exists());
    }
}

#[test]
fn postmaster_that_exits_before_ready_rolls_the_fresh_instance_back() {
    let fixture = Fixture::new();

    let output = fixture
        .command(&["start"])
        .env("FAKE_POSTGRES_EXIT", "1")
        .output()
        .expect("run start");

    assert_eq!(output.status.code(), Some(1));
    let error = error_of(&output);
    assert_eq!(error["code"], "startup_exit");
    // The postmaster's own output is carried for diagnosis.
    assert!(
        error["details"]
            .as_str()
            .is_some_and(|details| details.contains("fake postmaster failure")),
        "{error}"
    );
    assert!(!fixture.instance_dir().exists());
    assert_eq!(fixture.ok(&["list"])["total_servers"], 0);
    // The bundle stays installed for the retry, which then succeeds.
    assert_eq!(fixture.cached_bundles().len(), 1);
    fixture.ok(&["start"]);
    fixture.ok(&["remove"]);
}

#[test]
fn createdb_failure_stops_the_postmaster_and_leaves_no_instance() {
    let fixture = Fixture::new();

    let output = fixture
        .command(&["start", "--database", "app"])
        .env("FAKE_CREATEDB_EXIT", "1")
        .output()
        .expect("run start");

    assert_eq!(output.status.code(), Some(1));
    assert!(process_is_gone(fixture.postmaster_pid()));
    assert!(!fixture.instance_dir().exists());
    assert_eq!(fixture.ok(&["list"])["total_servers"], 0);
}

#[test]
fn docker_flag_on_a_native_instance_is_a_usage_error_that_changes_nothing() {
    let fixture = Fixture::new();
    fixture.ok(&["start"]);
    let pid = fixture.postmaster_pid();

    let output = fixture.run(&["start", "default", "--docker"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--docker"));
    assert_eq!(fixture.ok(&["list"])["servers"][0]["pid"], pid);
    fixture.ok(&["remove"]);
}

fn bundle_tarball_for(version: &str) -> (String, Vec<u8>, String) {
    let major = version.split('.').next().unwrap();
    let name = format!("pg-{version}-pgch-0.11.1-{}", target());
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);
    for (program, script) in PROGRAMS {
        let script = if program == "initdb" {
            script.replace("echo 18 >", &format!("echo {major} >"))
        } else {
            script.to_string()
        };
        let mut header = tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                format!("{name}/bin/{program}"),
                script.as_bytes(),
            )
            .expect("append fake program");
    }
    let bytes = archive
        .into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip");
    let digest = sha256(&bytes);
    (format!("{name}.tar.gz"), bytes, digest)
}

#[test]
fn latest_txt_multi_major_defaults_to_newest_and_honours_explicit_majors() {
    let root = tempfile::tempdir().expect("create fixture root");
    let base = root.path().canonicalize().expect("canonical fixture root");
    let [home, project, source, record] =
        ["home", "project", "source", "record"].map(|dir| base.join(dir));
    for dir in [&home, &project, &source, &record] {
        std::fs::create_dir(dir).expect("create fixture dir");
    }
    std::fs::create_dir(project.join(".clickhouse")).expect("mark project");

    // Immutable `<tag>/` layout pointed to by `latest.txt` (builds.clickhouse.com).
    let release_dir = source.join("2026.11.28");
    std::fs::create_dir(&release_dir).expect("create release dir");
    std::fs::write(source.join("latest.txt"), "2026.11.28\n").expect("write latest.txt");

    let mut bundles = Vec::new();
    for version in ["19.0", "18.7", "17.12"] {
        let major = version.split('.').next().unwrap();
        let (file, tarball, digest) = bundle_tarball_for(version);
        std::fs::write(release_dir.join(&file), &tarball).expect("write bundle");
        bundles.push(json!({
            "postgres_version": version,
            "postgres_major": major,
            "extensions": { "pg_clickhouse": "0.11.1" },
            "target": target(),
            "file": file,
            "sha256": digest,
            "size": tarball.len(),
        }));
    }
    let manifest = json!({
        "schema": 1,
        "release": "2026.11.28",
        "built_at": "2026-11-28T00:00:00Z",
        "source": { "repo": "ClickHouse/postgres-binaries", "commit": "0000000" },
        "bundles": bundles,
    });
    std::fs::write(
        release_dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .expect("write manifest");

    let run = |args: &[&str], bundles_url: Option<&str>| -> Output {
        let mut cmd = Command::new(clickhousectl_binary());
        cmd.env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin")
            .env("DOCKER_HOST", "unix:///nonexistent/docker.sock")
            .env(
                "CLICKHOUSECTL_POSTGRES_BUNDLES_URL",
                bundles_url
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("file://{}", source.display())),
            )
            .env("FAKE_RECORD_DIR", &record)
            .current_dir(&project)
            .args(["local", "--json"])
            .args(args);
        cmd.output().expect("run clickhousectl")
    };
    let ok = |args: &[&str], bundles_url: Option<&str>| -> Value {
        let output = run(args, bundles_url);
        assert!(
            output.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("parse JSON stdout")
    };

    // `local install postgres@latest` picks 19.0; `postgres@17` picks 17.12.
    let installed_latest = ok(&["install", "postgres@latest"], None);
    assert_eq!(installed_latest["version"], "postgres@19.0");
    let installed_17 = ok(&["install", "postgres@17"], None);
    assert_eq!(installed_17["version"], "postgres@17.12");

    // Omitting --version on a fresh native start picks the newest major (19.0).
    let started_default = ok(&["postgres", "start"], None);
    assert_eq!(started_default["image"], "postgres:19.0");
    assert!(project.join(".clickhouse/servers/default-pg19").is_dir());
    ok(&["postgres", "remove"], None);

    // Explicit --version 18 starts 18.7, and a later start without --version
    // resumes that existing PG 18 instance offline rather than creating PG 19.
    let started_18 = ok(&["postgres", "start", "app18", "--version", "18"], None);
    assert_eq!(started_18["image"], "postgres:18.7");
    assert!(project.join(".clickhouse/servers/app18-pg18").is_dir());
    ok(&["postgres", "stop", "app18"], None);

    let resumed_18 = ok(
        &["postgres", "start", "app18"],
        Some("file:///nonexistent/bundles"),
    );
    assert_eq!(resumed_18["image"], "postgres:18.7");
    assert!(!project.join(".clickhouse/servers/app18-pg19").exists());
    ok(&["postgres", "remove", "app18"], None);

    // --docker rejects --version latest with a usage error (exit 2).
    let docker_latest = run(
        &["postgres", "start", "--docker", "--version", "latest"],
        None,
    );
    assert_eq!(docker_latest.status.code(), Some(2));
}

#[test]
fn inherited_pg_env_vars_are_scrubbed_for_managed_native_commands_only() {
    let fixture = Fixture::new();

    // Inherited PG* client settings must not break start (initdb, postgres,
    // pg_isready, createdb) or managed native client, while the managed
    // PGPASSWORD and non-PG client vars (PSQLRC, PAGER) are kept.
    let start_output = fixture
        .command(&["start", "--database", "app", "--password", "managed-secret"])
        .env("FAKE_ASSERT_NO_PG_ENV", "1")
        .env("FAKE_EXPECT_PGPASSWORD", "managed-secret")
        .env("PGSSLMODE", "verify-full")
        .env("PGOPTIONS", "-c statement_timeout=1ms")
        .env("PGHOST", "unreachable.invalid")
        .env("PGDATA", "/nonexistent/pgdata")
        .env("PGPASSWORD", "inherited-wrong-password")
        .output()
        .expect("run start with PG* env");
    assert!(
        start_output.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&start_output.stderr)
    );

    let client_output = fixture
        .command(&["client", "-q", "SELECT 1"])
        .env("FAKE_ASSERT_NO_PG_ENV", "1")
        .env("FAKE_EXPECT_PGPASSWORD", "managed-secret")
        .env("FAKE_EXPECT_PSQLRC", "/tmp/custom-psqlrc")
        .env("PGSSLMODE", "verify-full")
        .env("PGOPTIONS", "-c statement_timeout=1ms")
        .env("PGHOST", "unreachable.invalid")
        .env("PGDATA", "/nonexistent/pgdata")
        .env("PGPASSWORD", "inherited-wrong-password")
        .env("PSQLRC", "/tmp/custom-psqlrc")
        .env("PAGER", "cat")
        .output()
        .expect("run managed client with PG* env");
    assert!(
        client_output.status.success(),
        "managed client failed: {}",
        String::from_utf8_lossy(&client_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&client_output.stdout).starts_with("fake-psql "),
        "{}",
        String::from_utf8_lossy(&client_output.stdout)
    );

    // Direct `--host`/`--port` client connections keep inherited `PG*` vars
    // (such as `PGSSLMODE=verify-full` for remote/Cloud Postgres).
    let host_bin = fixture.record.join("host-bin");
    std::fs::create_dir(&host_bin).expect("create host-bin");
    let host_psql = host_bin.join("psql");
    std::fs::write(
        &host_psql,
        r#"#!/bin/sh
[ "$1" = "--version" ] && exit 0
printf 'PGSSLMODE=%s\n' "$PGSSLMODE"
"#,
    )
    .expect("write host psql");
    std::fs::set_permissions(&host_psql, std::fs::Permissions::from_mode(0o755))
        .expect("chmod host psql");

    let direct_output = fixture
        .command(&["client", "--host", "127.0.0.1", "--port", "5432"])
        .env("PATH", format!("{}:/usr/bin:/bin", host_bin.display()))
        .env("PGSSLMODE", "verify-full")
        .output()
        .expect("run direct client with PGSSLMODE");
    assert!(
        direct_output.status.success(),
        "direct client failed: {}",
        String::from_utf8_lossy(&direct_output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&direct_output.stdout).trim(),
        "PGSSLMODE=verify-full"
    );

    fixture.ok(&["remove"]);
}
