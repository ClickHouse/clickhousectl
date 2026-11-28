//! The native runtime for `local postgres`: a postmaster from a cached
//! bundle (`local::pg_binaries`), run as a child process.
//!
//! Everything an instance owns lives in its directory under
//! `.clickhouse/servers/<name>-pg<major>/`: `data/` (PGDATA), `postgres.log`,
//! `run/` (the Unix socket) and `native.json` (credentials and the bundle
//! last used). This module owns those files and the processes; choosing an
//! instance and recording it in the project metadata is `local::postgres`.

use crate::error::{Error, Result, StartupKind};
use crate::local::pg_binaries::InstalledBundle;
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

/// The server setting for dynamic shared memory, which `initdb`'s bootstrap
/// and parallel queries allocate. On Linux the default,
/// POSIX, puts it in `/dev/shm`, which a container caps at 64 MiB unless told
/// otherwise, and a large parallel query then fails; System V segments have
/// no such mount behind them. macOS keeps its default: it has no `/dev/shm`
/// to run out of, and its System V limits (4 MiB a segment) are the small
/// ones there.
const SHARED_MEMORY_SETTING: Option<&str> = if cfg!(target_os = "linux") {
    Some("dynamic_shared_memory_type=sysv")
} else {
    None
};

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
        server::keep_owner(&path, &staged);
        std::fs::rename(&staged, &path)
    };
    write().map_err(|error| {
        let _ = std::fs::remove_file(&staged);
        error.into()
    })
}

/// The account `initdb` and the postmaster run as when this CLI is root.
///
/// PostgreSQL refuses to run as root, so a root invocation drops to an
/// unprivileged account for those two programs (issue #1464). Everything
/// else, the client tools included, keeps running as the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerUser {
    pub uid: u32,
    pub gid: u32,
    pub name: String,
}

/// Accounts tried for a new instance, in order. Nothing is ever created:
/// `nobody` exists on every system this was tried on.
const SERVER_USER_CANDIDATES: [&str; 2] = ["postgres", "nobody"];

fn effective_uid() -> u32 {
    // SAFETY: `geteuid` takes no arguments and cannot fail.
    unsafe { libc::geteuid() }
}

/// Read one passwd entry with `lookup`, which wraps `getpwnam_r` or
/// `getpwuid_r`.
fn passwd_entry(
    lookup: impl Fn(
        *mut libc::passwd,
        *mut libc::c_char,
        libc::size_t,
        *mut *mut libc::passwd,
    ) -> libc::c_int,
) -> Option<ServerUser> {
    let mut buffer = vec![0 as libc::c_char; 16 * 1024];
    // SAFETY: `passwd` is plain data; all-zero is a valid value to fill in.
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    if lookup(&mut entry, buffer.as_mut_ptr(), buffer.len(), &mut found) != 0 || found.is_null() {
        return None;
    }
    // SAFETY: on success `pw_name` points at a NUL-terminated string inside
    // `buffer`, which is still alive.
    let name = unsafe { std::ffi::CStr::from_ptr(entry.pw_name) }
        .to_string_lossy()
        .into_owned();
    Some(ServerUser {
        uid: entry.pw_uid,
        gid: entry.pw_gid,
        name,
    })
}

fn passwd_by_name(name: &str) -> Option<ServerUser> {
    let name = std::ffi::CString::new(name).ok()?;
    // SAFETY: every pointer is valid for the call and `name` is NUL-terminated.
    passwd_entry(|entry, buffer, len, found| unsafe {
        libc::getpwnam_r(name.as_ptr(), entry, buffer, len, found)
    })
}

fn passwd_by_uid(uid: u32) -> Option<ServerUser> {
    // SAFETY: every pointer is valid for the call.
    passwd_entry(|entry, buffer, len, found| unsafe {
        libc::getpwuid_r(uid, entry, buffer, len, found)
    })
}

/// The first candidate account that exists and is not itself root.
fn select_server_user(lookup: impl Fn(&str) -> Option<ServerUser>) -> Option<ServerUser> {
    SERVER_USER_CANDIDATES
        .iter()
        .filter_map(|name| lookup(name))
        .find(|user| user.uid != 0)
}

/// Who runs a new instance: `None` for the caller, or for root the
/// `postgres` account if there is one, else `nobody`.
pub(crate) fn user_for_new_instance() -> Result<Option<ServerUser>> {
    if effective_uid() != 0 {
        return Ok(None);
    }
    select_server_user(passwd_by_name).map(Some).ok_or_else(|| {
        Error::PostgresUsage(
            "PostgreSQL cannot run as root, and this system has neither a `postgres` nor a \
             `nobody` user to run it as. Run as a regular user, or pass `--docker`."
                .to_string(),
        )
    })
}

/// Who may run an instance whose data directory `owner` owns: PostgreSQL
/// only starts as that owner. A regular user must be the owner; root runs
/// the postmaster as the owner, whoever created the instance.
fn resolve_existing_user(
    euid: u32,
    owner: u32,
    lookup: impl Fn(u32) -> Option<ServerUser>,
) -> std::result::Result<Option<ServerUser>, String> {
    if euid != 0 {
        return if owner == euid {
            Ok(None)
        } else {
            Err(format!(
                "its data belongs to another user (uid {owner}); start it as that user or as root"
            ))
        };
    }
    if owner == 0 {
        return Err("its data directory is owned by root, which PostgreSQL cannot run as".into());
    }
    lookup(owner)
        .map(Some)
        .ok_or_else(|| format!("its data belongs to uid {owner}, which has no passwd entry"))
}

/// Who runs an existing instance; see [`resolve_existing_user`]. An instance
/// with no data directory resolves to the caller, and the start then reports
/// the missing data.
pub(crate) fn user_for_existing_instance(
    instance_dir: &Path,
    name: &str,
) -> Result<Option<ServerUser>> {
    use std::os::unix::fs::MetadataExt;
    let Ok(metadata) = std::fs::metadata(data_dir(instance_dir)) else {
        return Ok(None);
    };
    resolve_existing_user(effective_uid(), metadata.uid(), passwd_by_uid).map_err(|reason| {
        Error::PostgresUsage(format!(
            "Postgres server '{name}' cannot be started: {reason}"
        ))
    })
}

/// Owner, group and permission bits of a directory.
struct DirAccess {
    uid: u32,
    gid: u32,
    mode: u32,
}

fn can_search(access: &DirAccess, user: &ServerUser) -> bool {
    let bit = if access.uid == user.uid {
        0o100
    } else if access.gid == user.gid {
        0o010
    } else {
        0o001
    };
    access.mode & bit != 0
}

/// The outermost existing directory on the way to `path`, `path` included,
/// that `user` cannot enter. Directories that do not exist yet are skipped:
/// this CLI creates them.
fn first_unsearchable(
    path: &Path,
    user: &ServerUser,
    stat: &impl Fn(&Path) -> Option<DirAccess>,
) -> Option<PathBuf> {
    let mut ancestors: Vec<&Path> = path.ancestors().collect();
    ancestors.reverse();
    ancestors
        .into_iter()
        .find(|dir| stat(dir).is_some_and(|access| !can_search(&access, user)))
        .map(Path::to_path_buf)
}

/// Fail unless `user` can reach every one of `paths`: the bundle it executes
/// and the instance it writes. Nothing is chmodded to make it so; the
/// directories on the way belong to the caller.
pub(crate) fn ensure_searchable(paths: &[&Path], user: &ServerUser) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let stat = |dir: &Path| {
        std::fs::metadata(dir).ok().map(|metadata| DirAccess {
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
        })
    };
    match paths
        .iter()
        .find_map(|path| first_unsearchable(path, user, &stat))
    {
        Some(dir) => Err(Error::PostgresServerUserAccess {
            user: user.name.clone(),
            dir,
        }),
        None => Ok(()),
    }
}

/// Hand `paths` to `user`, who must own what the postmaster writes.
fn give_to(user: &ServerUser, paths: &[&Path]) -> Result<()> {
    for path in paths {
        std::os::unix::fs::chown(path, Some(user.uid), Some(user.gid))?;
    }
    Ok(())
}

/// Create the instance and data directories, owned by `run_as` when set.
pub(crate) fn create_instance_dirs(instance_dir: &Path, run_as: Option<&ServerUser>) -> Result<()> {
    if let Some(parent) = instance_dir.parent() {
        crate::paths::create_dir_all_managed(parent)?;
    }
    let data = data_dir(instance_dir);
    std::fs::create_dir_all(&data)?;
    match run_as {
        Some(user) => give_to(user, &[instance_dir, &data]),
        None => Ok(()),
    }
}

/// Strip inherited `PG*` environment variables (such as `PGSSLMODE`,
/// `PGOPTIONS`, `PGHOST`, `PGPORT`, `PGUSER`, `PGDATABASE`, `PGDATA`) so the
/// caller's shell configuration for remote Postgres servers cannot interfere
/// with managed local instances. Leaves non-`PG*` client settings such as
/// `PSQLRC`, `PAGER`, `EDITOR` and `TERM` untouched. Callers that need
/// `PGPASSWORD` set it explicitly after calling this helper.
pub(crate) fn scrub_pg_env(command: &mut std::process::Command) {
    scrub_pg_env_from(command, std::env::vars_os().map(|(key, _)| key));
}

fn scrub_pg_env_from(
    command: &mut std::process::Command,
    keys: impl IntoIterator<Item = std::ffi::OsString>,
) {
    use std::os::unix::ffi::OsStrExt;
    for key in keys {
        if key.as_bytes().starts_with(b"PG") {
            command.env_remove(key);
        }
    }
}

/// The step between fork and exec that turns a root child into `user`: leave
/// the caller's working directory, which `user` may not be able to enter,
/// then drop supplementary groups, group and user, in that order.
fn drop_privileges(
    user: &ServerUser,
) -> impl FnMut() -> std::io::Result<()> + Send + Sync + 'static {
    let (uid, gid) = (user.uid, user.gid);
    move || {
        // SAFETY: only async-signal-safe calls, on plain integers and a
        // static string.
        let dropped = unsafe {
            libc::chdir(c"/".as_ptr()) == 0
                && libc::setgroups(1, &gid) == 0
                && libc::setgid(gid) == 0
                && libc::setuid(uid) == 0
        };
        if dropped {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
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
/// counts only while that process is still this instance's postmaster: it
/// runs from the data directory, which a postmaster makes its working
/// directory, or it was started on it (`-D <data>`). The second test is for
/// root in a container, which may signal a postmaster running as another
/// user but not read its working directory.
pub(crate) fn running_pid(instance_dir: &Path) -> Option<u32> {
    let data = data_dir(instance_dir);
    let pid_file = std::fs::read_to_string(data.join("postmaster.pid")).ok()?;
    let pid: u32 = pid_file.lines().next()?.trim().parse().ok()?;
    let ours = server::is_server_process(pid, &data)
        || (server::is_process_alive(pid)
            && super::procinfo::process_args(pid)
                .is_ok_and(|args| started_on_data_dir(&args, &data)));
    ours.then_some(pid)
}

/// Whether `args`, a process's command line, start a server on `data`.
fn started_on_data_dir(args: &[String], data: &Path) -> bool {
    let Ok(data) = std::fs::canonicalize(data) else {
        return false;
    };
    args.windows(2).any(|pair| {
        pair[0] == "-D" && std::fs::canonicalize(&pair[1]).is_ok_and(|path| path == data)
    })
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
    /// The account to run as, when the caller is root.
    pub run_as: Option<&'a ServerUser>,
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
    if let Some(user) = options.run_as {
        // Still readable by one account only: the one initdb runs as.
        give_to(user, &[pwfile.path()])?;
    }

    let mut pwfile_arg = std::ffi::OsString::from("--pwfile=");
    pwfile_arg.push(pwfile.path());
    let mut command = tokio::process::Command::new(options.bundle.bin("initdb"));
    scrub_pg_env(command.as_std_mut());
    if let Some(user) = options.run_as {
        // SAFETY: the closure makes only async-signal-safe calls.
        unsafe { command.pre_exec(drop_privileges(user)) };
    }
    let output = command
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
        // initdb runs a server to bootstrap, which allocates shared memory
        // too, and it records the setting in postgresql.conf.
        .args(
            SHARED_MEMORY_SETTING
                .iter()
                .flat_map(|setting| ["-c", setting]),
        )
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
    run_as: Option<&ServerUser>,
) -> Result<Child> {
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(instance_dir.join(LOG_FILE))?;
    let socket_dir = socket_dir(instance_dir);
    if let Some(dir) = &socket_dir {
        std::fs::create_dir_all(dir)?;
        if let Some(user) = run_as {
            give_to(user, &[dir])?;
        }
    }
    let mut socket_setting = std::ffi::OsString::from("unix_socket_directories=");
    if let Some(dir) = &socket_dir {
        socket_setting.push(dir);
    }
    let mut command = std::process::Command::new(bundle.bin("postgres"));
    scrub_pg_env(&mut command);
    command
        .arg("-D")
        .arg(data_dir(instance_dir))
        .arg("-p")
        .arg(port.to_string())
        .arg("-c")
        .arg(format!("listen_addresses={LISTEN_ADDRESS}"))
        .arg("-c")
        .arg(socket_setting);
    if let Some(setting) = SHARED_MEMORY_SETTING {
        command.arg("-c").arg(setting);
    }
    if let Some(user) = run_as {
        // SAFETY: the closure makes only async-signal-safe calls.
        unsafe { command.pre_exec(drop_privileges(user)) };
    }
    command
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
        let mut probe_cmd = tokio::process::Command::new(bundle.bin("pg_isready"));
        scrub_pg_env(probe_cmd.as_std_mut());
        let probe = probe_cmd
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
    let mut command = tokio::process::Command::new(bundle.bin("createdb"));
    scrub_pg_env(command.as_std_mut());
    let output = command
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

    fn user(name: &str, uid: u32, gid: u32) -> ServerUser {
        ServerUser {
            uid,
            gid,
            name: name.to_string(),
        }
    }

    #[test]
    fn root_runs_new_instances_as_postgres_else_nobody_and_never_as_root() {
        let nobody = user("nobody", 65534, 65534);
        let postgres = user("postgres", 105, 110);
        let passwd = |entries: Vec<ServerUser>| {
            move |name: &str| entries.iter().find(|entry| entry.name == name).cloned()
        };
        assert_eq!(
            select_server_user(passwd(vec![nobody.clone(), postgres.clone()])),
            Some(postgres)
        );
        assert_eq!(
            select_server_user(passwd(vec![nobody.clone()])),
            Some(nobody.clone())
        );
        // An entry that is root under another name is no use.
        assert_eq!(
            select_server_user(passwd(vec![user("postgres", 0, 0), nobody.clone()])),
            Some(nobody)
        );
        assert_eq!(select_server_user(passwd(vec![])), None);
    }

    #[test]
    fn an_existing_instance_runs_as_the_owner_of_its_data() {
        let owner = user("al", 501, 20);
        let lookup = |uid: u32| (uid == 501).then(|| owner.clone());
        // The owner starts it as themselves; root starts it as the owner.
        assert_eq!(resolve_existing_user(501, 501, lookup), Ok(None));
        assert_eq!(
            resolve_existing_user(0, 501, lookup),
            Ok(Some(owner.clone()))
        );
        // Anyone else cannot, and neither can root for data it cannot map to
        // an account or that root itself owns.
        assert!(resolve_existing_user(502, 501, lookup).is_err());
        assert!(resolve_existing_user(501, 65534, lookup).is_err());
        assert!(resolve_existing_user(0, 4242, lookup).is_err());
        assert!(resolve_existing_user(0, 0, lookup).is_err());
    }

    #[test]
    fn the_outermost_directory_the_server_user_cannot_enter_is_reported() {
        let nobody = user("nobody", 65534, 65534);
        let tree = |entries: &'static [(&'static str, u32, u32, u32)]| {
            move |dir: &Path| {
                entries
                    .iter()
                    .find(|(path, ..)| Path::new(path) == dir)
                    .map(|&(_, uid, gid, mode)| DirAccess { uid, gid, mode })
            }
        };
        let blocked = |path: &str, stat: &dyn Fn(&Path) -> Option<DirAccess>| {
            first_unsearchable(Path::new(path), &nobody, &|dir| stat(dir))
        };

        // Root's home is closed to others; a project elsewhere is not.
        let home = tree(&[
            ("/", 0, 0, 0o755),
            ("/root", 0, 0, 0o700),
            ("/root/.clickhouse", 0, 0, 0o755),
            ("/srv", 0, 0, 0o755),
            ("/srv/app", 0, 0, 0o755),
        ]);
        assert_eq!(
            blocked("/root/.clickhouse/postgres/pg-18", &home),
            Some(PathBuf::from("/root"))
        );
        // Directories that do not exist yet are not an obstacle.
        assert_eq!(blocked("/srv/app/.clickhouse/servers/a-pg18", &home), None);

        // The class that applies decides: owner bits for the owner, group
        // bits for the group, other bits for everyone else.
        let classes = tree(&[
            ("/", 0, 0, 0o755),
            ("/owned", 65534, 0, 0o700),
            ("/owned-closed", 65534, 0, 0o077),
            ("/grouped", 0, 65534, 0o750),
            ("/grouped-closed", 0, 65534, 0o705),
            ("/other-closed", 0, 0, 0o770),
        ]);
        assert_eq!(blocked("/owned/x", &classes), None);
        assert_eq!(blocked("/grouped/x", &classes), None);
        for dir in ["/owned-closed", "/grouped-closed", "/other-closed"] {
            assert_eq!(
                blocked(&format!("{dir}/x"), &classes),
                Some(PathBuf::from(dir)),
                "{dir}"
            );
        }
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
    fn a_command_line_identifies_the_postmaster_of_a_data_directory() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let other = dir.path().join("other");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let args = |data_arg: &Path| -> Vec<String> {
            [
                "/bundle/bin/postgres",
                "-D",
                data_arg.to_str().unwrap(),
                "-p",
                "5432",
            ]
            .map(String::from)
            .to_vec()
        };
        assert!(started_on_data_dir(&args(&data), &data));
        // Another instance's postmaster, or a process that only mentions
        // the path, is not this one.
        assert!(!started_on_data_dir(&args(&other), &data));
        assert!(!started_on_data_dir(
            &["sleep".to_string(), data.display().to_string()],
            &data
        ));
        assert!(!started_on_data_dir(&[], &data));
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

    #[test]
    fn scrub_pg_env_removes_pg_vars_while_preserving_other_vars_and_explicit_password() {
        use std::ffi::{OsStr, OsString};
        let mut command = std::process::Command::new("psql");
        scrub_pg_env_from(
            &mut command,
            [
                "PGSSLMODE",
                "PGOPTIONS",
                "PGHOST",
                "PGPASSWORD",
                "PGDATA",
                "PSQLRC",
                "PAGER",
                "PATH",
            ]
            .into_iter()
            .map(OsString::from),
        );
        command.env("PGPASSWORD", "managed-secret");

        let envs: std::collections::BTreeMap<&OsStr, Option<&OsStr>> = command.get_envs().collect();
        for removed in ["PGSSLMODE", "PGOPTIONS", "PGHOST", "PGDATA"] {
            assert_eq!(envs.get(OsStr::new(removed)), Some(&None), "{removed}");
        }
        assert_eq!(
            envs.get(OsStr::new("PGPASSWORD")),
            Some(&Some(OsStr::new("managed-secret")))
        );
        for untouched in ["PSQLRC", "PAGER", "PATH"] {
            assert!(!envs.contains_key(OsStr::new(untouched)), "{untouched}");
        }
    }
}
