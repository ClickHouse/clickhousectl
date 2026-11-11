//! A live process that a project's server metadata can name as running.
//!
//! The CLI only treats a recorded PID as a ClickHouse server while that
//! process runs from the server's data directory (issue #1299), so a test that
//! needs a "running" server spawns `sleep` from there instead of recording its
//! own PID.

use std::path::Path;
use std::process::{Child, Command};

/// A `sleep` run from `<project>/.clickhouse/servers/<name>/data`, killed on
/// drop.
pub struct ServerProcess(Child);

impl ServerProcess {
    pub fn spawn(project: &Path, name: &str) -> Self {
        let data_dir = project.join(".clickhouse/servers").join(name).join("data");
        std::fs::create_dir_all(&data_dir).expect("create server data directory");
        let child = Command::new("sleep")
            .arg("60")
            .current_dir(&data_dir)
            .spawn()
            .expect("spawn the stand-in server process");
        Self(child)
    }

    pub fn pid(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
