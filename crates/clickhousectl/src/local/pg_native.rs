//! The native runtime for `local postgres`: a postmaster from a cached
//! bundle (`local::pg_bundle`), run as a child process.
//!
//! Everything an instance owns lives in its directory under
//! `.clickhouse/servers/<name>-pg<major>/`: `data/` (PGDATA), `postgres.log`,
//! `run/` (the Unix socket) and `native.json` (credentials and the bundle
//! last used). This module owns those files and the processes; choosing an
//! instance and recording it in the project metadata is `local::postgres`.

use crate::error::{Error, Result, StartupKind};
use crate::local::pg_bundle::InstalledBundle;
use crate::local::server;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const STATE_FILE: &str = "native.json";
const LOG_FILE: &str = "postgres.log";
const SOCKET_DIR: &str = "run";
const LISTEN_ADDRESS: &str = "127.0.0.1";

const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(200);
/// Bounds one `pg_isready` run, which is told to give up after a second.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a fast shutdown (SIGINT) is given before an immediate one.
const FAST_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);
/// How long an immediate shutdown (SIGQUIT) is given.
const IMMEDIATE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const LOG_TAIL_LINES: usize = 50;
const LOG_TAIL_BYTES: u64 = 16 * 1024;

/// The bundle an instance last ran, by identity rather than by path: the
/// cache may be cleared or gain a newer minor between starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BundleIdentity {
    pub name: String,
    pub sha256: String,
    pub postgres_version: String,
}

impl BundleIdentity {
    pub(crate) fn of(bundle: &InstalledBundle) -> Self {
        Self {
            name: bundle.spec.name().to_string(),
            sha256: bundle.spec.sha256.clone(),
            postgres_version: bundle.spec.postgres_version.clone(),
        }
    }
}

/// What a native instance needs beyond the project metadata. Stored with
/// owner-only permissions because it holds the superuser password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NativeState {
    pub postgres_major: String,
    pub bundle: BundleIdentity,
    pub user: String,
    pub password: String,
    pub database: String,
}

pub(crate) fn load_state(instance_dir: &Path) -> Result<Option<NativeState>> {
    match std::fs::read(instance_dir.join(STATE_FILE)) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn save_state(instance_dir: &Path, state: &NativeState) -> Result<()> {
    let path = instance_dir.join(STATE_FILE);
    let staged = instance_dir.join(format!(".{STATE_FILE}.{}", std::process::id()));
    let _ = std::fs::remove_file(&staged);
    let write = || -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staged)?;
        file.write_all(&serde_json::to_vec_pretty(state)?)?;
        file.sync_all()?;
        std::fs::rename(&staged, &path)
    };
    write().map_err(|error| {
        let _ = std::fs::remove_file(&staged);
        error.into()
    })
}

/// Postgres refuses to run as root, and a bundle downloaded as root would be
/// unusable by the user afterwards, so this fails before any download.
/// Dropping privileges instead is being looked at in issue #1464.
pub(crate) fn reject_root() -> Result<()> {
    // SAFETY: `geteuid` takes no arguments and cannot fail.
    reject_root_euid(unsafe { libc::geteuid() })
}

fn reject_root_euid(euid: u32) -> Result<()> {
    if euid != 0 {
        return Ok(());
    }
    Err(Error::usage(
        clap::error::ErrorKind::InvalidValue,
        "PostgreSQL cannot run as root. Run `clickhousectl local postgres start` as a \
         regular user, or pass `--docker` to run Postgres in a container.",
    ))
}

fn data_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("data")
}

/// Whether `instance_dir` holds an initialized data directory.
pub(crate) fn is_initialized(instance_dir: &Path) -> bool {
    data_dir(instance_dir).join("PG_VERSION").is_file()
}

/// The running postmaster of an instance, if any.
///
/// `postmaster.pid` survives a crash and its PID can be reused, so the PID
/// counts only while that process still runs from the data directory, which
/// a postmaster makes its working directory.
pub(crate) fn running_pid(instance_dir: &Path) -> Option<u32> {
    let data = data_dir(instance_dir);
    let pid_file = std::fs::read_to_string(data.join("postmaster.pid")).ok()?;
    let pid: u32 = pid_file.lines().next()?.trim().parse().ok()?;
    server::is_server_process(pid, &data).then_some(pid)
}

pub(crate) fn is_running(instance_dir: &Path) -> bool {
    running_pid(instance_dir).is_some()
}

/// Ask a postmaster to shut down: fast (SIGINT, which ends sessions and
/// checkpoints), then immediate (SIGQUIT) if that takes too long. `wait`
/// reports whether the process went within the given time.
fn shutdown(pid: u32, mut wait: impl FnMut(Duration) -> bool) -> Result<()> {
    server::send_signal(pid, libc::SIGINT)?;
    let fast =
        crate::timing::duration_override(crate::timing::TERM_TIMEOUT_ENV, FAST_SHUTDOWN_TIMEOUT);
    if wait(fast) {
        return Ok(());
    }
    server::send_signal(pid, libc::SIGQUIT)?;
    if wait(IMMEDIATE_SHUTDOWN_TIMEOUT) {
        return Ok(());
    }
    Err(Error::Postgres(format!(
        "postmaster (PID {pid}) did not exit after SIGINT and SIGQUIT"
    )))
}

/// Stop the instance's postmaster, if it is running.
pub(crate) fn stop(instance_dir: &Path) -> Result<()> {
    match running_pid(instance_dir) {
        Some(pid) => shutdown(pid, |timeout| server::wait_for_exit(pid, timeout)),
        None => Ok(()),
    }
}

/// Stop a postmaster this process spawned. Unlike [`stop`] it reaps the
/// child, which a liveness probe would otherwise keep seeing as a zombie.
pub(crate) fn stop_child(child: &mut Child) -> Result<()> {
    let pid = child.id();
    let mut wait = |timeout: Duration| {
        let deadline = Instant::now() + timeout;
        loop {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    if wait(Duration::ZERO) {
        return Ok(());
    }
    shutdown(pid, wait)
}

/// The directory for the Unix socket, or `None` when its path would not fit
/// in a socket address (104 bytes on macOS, 108 on Linux). Postgres refuses
/// to start with a longer one, and TCP on loopback is all clients here use.
fn socket_dir(instance_dir: &Path) -> Option<PathBuf> {
    // SAFETY: `sockaddr_un` is plain data; all-zero is a valid value.
    let capacity = unsafe { std::mem::zeroed::<libc::sockaddr_un>() }
        .sun_path
        .len();
    let dir = instance_dir.join(SOCKET_DIR);
    socket_path_fits(&dir, capacity).then_some(dir)
}

fn socket_path_fits(dir: &Path, capacity: usize) -> bool {
    // "<dir>/.s.PGSQL.<port>" plus its terminating NUL, for any port.
    dir.as_os_str().len() + "/.s.PGSQL.65535".len() < capacity
}

/// The last lines of the instance's log, for a startup failure.
fn log_tail(instance_dir: &Path) -> String {
    let path = instance_dir.join(LOG_FILE);
    let read = || -> std::io::Result<String> {
        let mut file = std::fs::File::open(&path)?;
        let len = file.metadata()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(LOG_TAIL_BYTES)))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    };
    let text = read().unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let tail = lines[lines.len().saturating_sub(LOG_TAIL_LINES)..].join("\n");
    if tail.trim().is_empty() {
        format!("(no output in {})", path.display())
    } else {
        format!("--- last lines of {} ---\n{tail}", path.display())
    }
}

/// What a fresh instance is created with.
pub(crate) struct InitOptions<'a> {
    pub bundle: &'a InstalledBundle,
    pub instance_dir: &'a Path,
    /// The user-facing instance name, for error messages.
    pub name: &'a str,
    pub user: &'a str,
    pub password: &'a str,
}

/// Initialize the data directory. The password reaches `initdb` through a
/// file only this user can read, never through argv or the environment.
///
/// The locale is the builtin `C.UTF-8` provider with `C` for the libc
/// categories, so sorting and case mapping do not depend on the platform's
/// locale database or on this process's environment.
pub(crate) async fn initdb(options: &InitOptions<'_>) -> Result<()> {
    let mut pwfile = tempfile::Builder::new()
        .prefix(".pwfile")
        .tempfile_in(options.instance_dir)?;
    writeln!(pwfile, "{}", options.password)?;
    pwfile.flush()?;

    let mut pwfile_arg = std::ffi::OsString::from("--pwfile=");
    pwfile_arg.push(pwfile.path());
    let output = tokio::process::Command::new(options.bundle.bin("initdb"))
        .arg("-D")
        .arg(data_dir(options.instance_dir))
        .arg("-U")
        .arg(options.user)
        .arg(pwfile_arg)
        .args([
            "--auth=scram-sha-256",
            "-E",
            "UTF8",
            "--locale-provider=builtin",
            "--builtin-locale=C.UTF-8",
            "--no-locale",
            "--no-instructions",
        ])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| Error::Postgres(format!("could not run initdb: {error}")))?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::StartupExit {
        kind: StartupKind::Postgres,
        name: options.name.to_string(),
        details: format!(
            "initdb failed for Postgres server '{}' ({}).\n{}",
            options.name,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end()
        ),
    })
}

/// Start the postmaster for an initialized instance, listening on loopback
/// only. It gets its own process group, so a Ctrl-C aimed at this CLI (or a
/// later one) does not reach it, and it outlives this process.
pub(crate) fn spawn_postmaster(
    bundle: &InstalledBundle,
    instance_dir: &Path,
    port: u16,
) -> Result<Child> {
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(instance_dir.join(LOG_FILE))?;
    let socket_dir = socket_dir(instance_dir);
    if let Some(dir) = &socket_dir {
        std::fs::create_dir_all(dir)?;
    }
    let mut socket_setting = std::ffi::OsString::from("unix_socket_directories=");
    if let Some(dir) = &socket_dir {
        socket_setting.push(dir);
    }
    std::process::Command::new(bundle.bin("postgres"))
        .arg("-D")
        .arg(data_dir(instance_dir))
        .arg("-p")
        .arg(port.to_string())
        .arg("-c")
        .arg(format!("listen_addresses={LISTEN_ADDRESS}"))
        .arg("-c")
        .arg(socket_setting)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .spawn()
        .map_err(|error| Error::Postgres(format!("could not start postgres: {error}")))
}

enum Readiness {
    Ready,
    Exited(ExitStatus),
    TimedOut,
}

async fn poll_ready(
    child: &mut Child,
    bundle: &InstalledBundle,
    port: u16,
    timeout: Duration,
) -> Result<Readiness> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Readiness::Exited(status));
        }
        let probe = tokio::process::Command::new(bundle.bin("pg_isready"))
            .args(["-h", LISTEN_ADDRESS, "-p", &port.to_string(), "-t", "1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status();
        if let Ok(Ok(status)) = tokio::time::timeout(PROBE_TIMEOUT, probe).await
            && status.success()
        {
            // Something answered on the port; it is this postmaster only if
            // this postmaster is still running.
            return Ok(match child.try_wait()? {
                Some(status) => Readiness::Exited(status),
                None => Readiness::Ready,
            });
        }
        if Instant::now() >= deadline {
            return Ok(Readiness::TimedOut);
        }
        tokio::time::sleep(READINESS_POLL_INTERVAL).await;
    }
}

/// Wait until the postmaster accepts connections. The caller stops `child`
/// on an error.
pub(crate) async fn wait_ready(
    child: &mut Child,
    bundle: &InstalledBundle,
    instance_dir: &Path,
    name: &str,
    port: u16,
    timeout: Duration,
) -> Result<()> {
    match poll_ready(child, bundle, port, timeout).await? {
        Readiness::Ready => Ok(()),
        Readiness::Exited(status) => Err(Error::StartupExit {
            kind: StartupKind::Postgres,
            name: name.to_string(),
            details: format!(
                "Postgres server '{name}' exited before PostgreSQL became ready ({status}).\n{}",
                log_tail(instance_dir)
            ),
        }),
        Readiness::TimedOut => Err(Error::StartupTimeout {
            kind: StartupKind::Postgres,
            name: name.to_string(),
            seconds: timeout.as_secs(),
            details: format!(
                "PostgreSQL in server '{name}' did not become ready within {} seconds.\n{}",
                timeout.as_secs(),
                log_tail(instance_dir)
            ),
        }),
    }
}

/// Create `database` on a ready instance. The password goes to this
/// short-lived client through its environment, as `psql` takes it.
pub(crate) async fn create_database(
    bundle: &InstalledBundle,
    port: u16,
    user: &str,
    password: &str,
    database: &str,
) -> Result<()> {
    let output = tokio::process::Command::new(bundle.bin("createdb"))
        .args(["-h", LISTEN_ADDRESS, "-p", &port.to_string(), "-U", user])
        .arg("--no-password")
        .arg("--")
        .arg(database)
        .env("PGPASSWORD", password)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| Error::Postgres(format!("could not run createdb: {error}")))?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::Postgres(format!(
        "could not create database '{database}': {}",
        String::from_utf8_lossy(&output.stderr).trim_end()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn root_is_refused_as_a_usage_error_and_other_users_pass() {
        assert!(reject_root_euid(501).is_ok());
        let error = reject_root_euid(0).unwrap_err();
        assert!(matches!(error, Error::Usage(_)));
        assert_eq!(error.exit_code(), 2);
        // The way out is the Docker runtime.
        assert!(error.to_string().contains("--docker"));
    }

    #[test]
    fn state_round_trips_through_an_owner_only_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_state(dir.path()).unwrap(), None);
        let state = NativeState {
            postgres_major: "18".into(),
            bundle: BundleIdentity {
                name: "pg-18.7-pgch-0.11.1-aarch64-apple-darwin".into(),
                sha256: "ab".repeat(32),
                postgres_version: "18.7".into(),
            },
            user: "postgres".into(),
            password: "example-password".into(),
            database: "app".into(),
        };
        save_state(dir.path(), &state).unwrap();
        // Saving again replaces the file rather than failing on it.
        save_state(dir.path(), &state).unwrap();

        assert_eq!(load_state(dir.path()).unwrap(), Some(state));
        let mode = std::fs::metadata(dir.path().join(STATE_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        // No path to a bundle is stored: the cache may move or be cleared.
        let stored = std::fs::read_to_string(dir.path().join(STATE_FILE)).unwrap();
        assert!(!stored.contains('/'), "{stored}");
    }

    #[test]
    fn a_socket_directory_is_used_only_when_its_path_fits_the_address() {
        let short = Path::new("/p/.clickhouse/servers/default-pg18/run");
        assert!(socket_path_fits(short, 104));
        let long = PathBuf::from(format!("/{}/run", "d".repeat(100)));
        assert!(!socket_path_fits(&long, 104));
        // The boundary: path + "/.s.PGSQL.65535" + NUL must fit exactly.
        let exact = PathBuf::from(format!("/{}", "d".repeat(104 - 15 - 2)));
        assert!(socket_path_fits(&exact, 104));
        let over = PathBuf::from(format!("/{}", "d".repeat(104 - 15 - 1)));
        assert!(!socket_path_fits(&over, 104));
    }

    #[test]
    fn a_stale_pid_file_does_not_make_an_instance_running() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        assert!(!is_running(dir.path()));
        // This test process is alive, but it does not run from the data
        // directory, as a process that reused a postmaster's PID would not.
        std::fs::write(
            data.join("postmaster.pid"),
            format!("{}\n{}\n", std::process::id(), data.display()),
        )
        .unwrap();
        assert!(!is_running(dir.path()));
        std::fs::write(data.join("postmaster.pid"), "not a pid\n").unwrap();
        assert!(!is_running(dir.path()));
    }
}
