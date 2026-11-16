//! `local clickhouse start` and `remove` refuse a local Postgres instance key
//! (#1296). Both engines share `.clickhouse/servers/`, so without the check
//! `remove dev-pg17` deleted PGDATA and `start dev-pg17` wrote ClickHouse
//! config into it. A stopped Postgres instance is seeded on disk; no Docker
//! or ClickHouse binary is needed because the refusal comes first.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Env {
    project: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Env {
    fn servers(&self) -> PathBuf {
        self.project.path().join(".clickhouse/servers")
    }

    /// A stopped `dev` Postgres 17: metadata plus a data directory holding a
    /// stand-in PGDATA file.
    fn with_stopped_postgres() -> Self {
        let env = Self {
            project: tempfile::tempdir().expect("create project"),
            home: tempfile::tempdir().expect("create home"),
        };
        let data = env.servers().join("dev-pg17/data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("PG_VERSION"), "17\n").unwrap();
        std::fs::write(
            env.servers().join("dev-pg17.json"),
            serde_json::json!({
                "name": "dev",
                "pid": 0,
                "version": "postgres:17",
                "http_port": 0,
                "tcp_port": 5432,
                "started_at": "2026-11-10T00:00:00Z",
                "cwd": env.project.path().display().to_string(),
                "engine": "postgres",
                "container_id": "abc123"
            })
            .to_string(),
        )
        .unwrap();
        env
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", self.home.path())
            .env("PATH", "/usr/bin:/bin")
            .current_dir(self.project.path())
            .args(args)
            .output()
            .expect("run clickhousectl")
    }
}

fn snapshot(dir: &Path) -> Vec<String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    entries.sort();
    entries
}

#[test]
fn clickhouse_start_and_remove_leave_a_postgres_instance_untouched() {
    let env = Env::with_stopped_postgres();
    let metadata = std::fs::read(env.servers().join("dev-pg17.json")).unwrap();
    let data = env.servers().join("dev-pg17/data");

    for args in [
        vec!["local", "clickhouse", "remove", "dev-pg17"],
        vec!["local", "clickhouse", "remove", "dev-pg17", "--json"],
        vec!["local", "clickhouse", "start", "dev-pg17"],
        vec![
            "local",
            "clickhouse",
            "start",
            "--name",
            "dev-pg17",
            "--json",
        ],
    ] {
        let output = env.run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?} {output:?}");
        assert!(output.stdout.is_empty(), "{args:?} {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("'dev-pg17'"), "{args:?} {stderr}");
        assert!(stderr.contains("local postgres"), "{args:?} {stderr}");

        assert_eq!(snapshot(&data), ["PG_VERSION"], "{args:?}");
        assert_eq!(
            std::fs::read(env.servers().join("dev-pg17.json")).unwrap(),
            metadata,
            "{args:?}"
        );
    }
}
