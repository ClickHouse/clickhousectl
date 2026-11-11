//! A compiled stand-in ClickHouse binary for subprocess tests that exercise
//! process discovery.
//!
//! Discovery reads the native process table (issue #1301), so a fake server
//! has to be a real process whose kernel name is `clickhouse`, run from the
//! fixture's data directory with the fixture's binary path as `argv[0]`. A
//! shell script is not one on macOS, where the kernel names it after its
//! interpreter, so `fake_clickhouse_program.rs` is compiled once with the
//! toolchain's own `rustc` and copied into each fixture.
//!
//! Each process the fake starts appends `<pid>|<role>|<ppid>|<cwd>` to
//! `FAKE_CLICKHOUSE_PID_FILE`; see the program for the other variables.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const PROGRAM: &str = include_str!("fake_clickhouse_program.rs");

/// The compiled fake, built at most once per test process and cached across
/// test processes by the program's content.
fn compiled() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-clickhouse");
        std::fs::create_dir_all(&dir).expect("create the fake ClickHouse build directory");
        let binary = dir.join(format!("fake-clickhouse-{:016x}", fnv1a(PROGRAM)));
        if binary.exists() {
            return binary;
        }

        let source = dir.join(format!("fake_clickhouse_{}.rs", std::process::id()));
        let partial = dir.join(format!("fake-clickhouse.partial-{}", std::process::id()));
        std::fs::write(&source, PROGRAM).expect("write the fake ClickHouse source");
        let output = Command::new(rustc())
            .args(["--edition", "2021", "-C", "debuginfo=0", "-o"])
            .arg(&partial)
            .arg(&source)
            .output()
            .expect("run rustc for the fake ClickHouse");
        assert!(
            output.status.success(),
            "compiling the fake ClickHouse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = std::fs::remove_file(&source);
        // A rename is atomic, so a concurrent test process sees either no
        // binary or a complete one.
        std::fs::rename(&partial, &binary).expect("publish the fake ClickHouse");
        binary
    })
}

/// The `rustc` beside the `cargo` that built these tests, so the toolchain
/// matches; `rustc` on `PATH` otherwise.
fn rustc() -> PathBuf {
    let beside_cargo = Path::new(env!("CARGO")).with_file_name("rustc");
    if beside_cargo.exists() {
        beside_cargo
    } else {
        PathBuf::from("rustc")
    }
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Install the fake as the `clickhouse` binary at `path`, with the
/// `clickhouse-watchdog` copy beside it that its watchdog mode re-executes.
pub fn install(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("fake binary parent"))
        .expect("create fake version directory");
    for target in [
        path.to_path_buf(),
        path.with_file_name("clickhouse-watchdog"),
    ] {
        std::fs::copy(compiled(), &target).expect("install the fake ClickHouse");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
            .expect("make the fake ClickHouse executable");
    }
}

/// One `<pid>|<role>|<ppid>|<cwd>` record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Not every test reads every field.
pub struct Record {
    pub pid: u32,
    pub role: String,
    pub ppid: u32,
    pub cwd: String,
}

/// Every record the fake processes have written to `pid_file` so far.
pub fn records(pid_file: &Path) -> Vec<Record> {
    std::fs::read_to_string(pid_file)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(4, '|');
            Some(Record {
                pid: fields.next()?.parse().ok()?,
                role: fields.next()?.to_string(),
                ppid: fields.next()?.parse().ok()?,
                cwd: fields.next()?.to_string(),
            })
        })
        .collect()
}

/// SIGKILLs every process recorded in `pid_file` on drop, so a failed
/// assertion cannot leak a sleeping fake.
pub struct ProcessGuard {
    pub pid_file: PathBuf,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        for record in records(&self.pid_file) {
            if let Ok(pid) = i32::try_from(record.pid) {
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
            }
        }
    }
}
