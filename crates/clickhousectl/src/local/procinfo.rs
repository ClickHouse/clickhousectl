//! Native process inspection: the process table, and one process's working
//! directory and arguments.
//!
//! Linux reads `/proc`; macOS asks the kernel through `libproc` and `sysctl`.
//! Nothing here runs an external tool, so discovery works in a slim container
//! without `pgrep`, `ps` or `lsof`, and costs no subprocess startup (issue #1301).
//!
//! A failure to read the process table at all is an error, so callers can tell
//! "could not look" from "nothing is running". A single process that cannot be
//! read (it exited mid-scan, or belongs to another user) is left out of the
//! table instead, as `pgrep` would.

use std::io;
use std::path::PathBuf;

/// One row of the process table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessEntry {
    pub pid: u32,
    pub ppid: u32,
    /// The kernel's short process name, the one `pgrep -x` matches: the
    /// executable's basename, truncated by the kernel (15 bytes on Linux, 16 on
    /// macOS). Unlike `argv[0]`, a process cannot rewrite it by editing its
    /// own argument memory.
    pub name: String,
}

/// Every process the caller can see.
pub(crate) fn process_table() -> io::Result<Vec<ProcessEntry>> {
    imp::process_table()
}

/// The current working directory of `pid`.
///
/// `NotFound` means the process is gone; `PermissionDenied` that it belongs to
/// another user. Exposed on its own so a caller can check one process's
/// identity without scanning the table.
pub(crate) fn process_cwd(pid: u32) -> io::Result<PathBuf> {
    imp::process_cwd(pid)
}

/// The command-line arguments of `pid`, `argv[0]` first, as the process
/// currently presents them: a process that rewrites its `argv[0]` (the
/// ClickHouse watchdog does) reports the rewritten name.
pub(crate) fn process_args(pid: u32) -> io::Result<Vec<String>> {
    imp::process_args(pid)
}

/// The direct children of `pid`, from one read of the process table.
pub(crate) fn child_pids(pid: u32) -> io::Result<Vec<u32>> {
    Ok(children_in(&process_table()?, pid))
}

fn children_in(table: &[ProcessEntry], pid: u32) -> Vec<u32> {
    table
        .iter()
        .filter(|entry| entry.ppid == pid && entry.pid != pid && entry.pid != 0)
        .map(|entry| entry.pid)
        .collect()
}

/// Parse `/proc/<pid>/stat` into the process name and parent PID.
///
/// The name is the second field, in parentheses, and may itself contain spaces
/// and parentheses, so it runs to the *last* `)`. The state and the parent PID
/// follow it.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_stat(stat: &[u8]) -> Option<(String, u32)> {
    let open = stat.iter().position(|&byte| byte == b'(')?;
    let close = stat.iter().rposition(|&byte| byte == b')')?;
    if close < open {
        return None;
    }
    let name = String::from_utf8_lossy(&stat[open + 1..close]).into_owned();
    let rest = std::str::from_utf8(&stat[close + 1..]).ok()?;
    let mut fields = rest.split_ascii_whitespace();
    let _state = fields.next()?;
    let ppid = fields.next()?.parse().ok()?;
    Some((name, ppid))
}

/// Split NUL-terminated strings, as `/proc/<pid>/cmdline` holds them.
///
/// Only the final terminator is dropped, so the empty strings a process leaves
/// behind when it zeroes its argument area (the watchdog's rewrite does) stay
/// visible as empty arguments, exactly as `ps` pads them with spaces.
#[cfg(any(target_os = "linux", test))]
fn parse_nul_separated(bytes: &[u8]) -> Vec<String> {
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes
        .split(|&byte| byte == 0)
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect()
}

/// Parse the `KERN_PROCARGS2` buffer of a macOS process into its arguments.
///
/// The layout is a native-endian `int argc`, the executable path, NUL padding,
/// then `argc` NUL-terminated arguments followed by the environment, which is
/// never read.
#[cfg(any(target_os = "macos", test))]
fn parse_procargs2(buffer: &[u8]) -> Option<Vec<String>> {
    let (argc, rest) = buffer.split_first_chunk::<4>()?;
    let argc = usize::try_from(i32::from_ne_bytes(*argc)).ok()?;
    let exec_path_end = rest.iter().position(|&byte| byte == 0)?;
    let rest = &rest[exec_path_end..];
    let args_start = rest.iter().position(|&byte| byte != 0)?;
    let args = rest[args_start..]
        .split(|&byte| byte == 0)
        .take(argc)
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect();
    Some(args)
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{ProcessEntry, parse_nul_separated, parse_proc_stat};
    use std::io;
    use std::path::PathBuf;

    pub(super) fn process_table() -> io::Result<Vec<ProcessEntry>> {
        let mut table = Vec::new();
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            else {
                continue;
            };
            // The process may have exited since the directory was listed.
            let Ok(stat) = std::fs::read(format!("/proc/{pid}/stat")) else {
                continue;
            };
            if let Some((name, ppid)) = parse_proc_stat(&stat) {
                table.push(ProcessEntry { pid, ppid, name });
            }
        }
        Ok(table)
    }

    pub(super) fn process_cwd(pid: u32) -> io::Result<PathBuf> {
        std::fs::read_link(format!("/proc/{pid}/cwd"))
    }

    pub(super) fn process_args(pid: u32) -> io::Result<Vec<String>> {
        Ok(parse_nul_separated(&std::fs::read(format!(
            "/proc/{pid}/cmdline"
        ))?))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{ProcessEntry, parse_procargs2};
    use std::ffi::{CStr, OsStr, c_int, c_void};
    use std::io;
    use std::mem::{MaybeUninit, size_of};
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;

    /// Room for processes started between sizing the PID buffer and filling it.
    const PID_HEADROOM: usize = 64;

    pub(super) fn process_table() -> io::Result<Vec<ProcessEntry>> {
        // SAFETY: a null buffer of size 0 only asks for the number of PIDs.
        let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
        let count = usize::try_from(count)
            .ok()
            .filter(|&count| count > 0)
            .ok_or_else(io::Error::last_os_error)?;

        let mut pids: Vec<c_int> = vec![0; count + PID_HEADROOM];
        let size = c_int::try_from(pids.len() * size_of::<c_int>())
            .map_err(|_| io::Error::other("process table too large"))?;
        // SAFETY: `pids` is a live, writable buffer of exactly `size` bytes.
        let filled = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast::<c_void>(), size) };
        let filled = usize::try_from(filled)
            .ok()
            .filter(|&filled| filled > 0)
            .ok_or_else(io::Error::last_os_error)?;
        pids.truncate(filled.min(pids.len()));

        Ok(pids
            .into_iter()
            .filter_map(|pid| {
                let pid = u32::try_from(pid).ok().filter(|&pid| pid > 0)?;
                short_info(pid)
            })
            .collect())
    }

    /// One process's PID, parent and short name, or `None` once it has exited
    /// or cannot be inspected.
    fn short_info(pid: u32) -> Option<ProcessEntry> {
        let info: libc::proc_bsdshortinfo = pid_info(pid, libc::PROC_PIDT_SHORTBSDINFO).ok()?;
        Some(ProcessEntry {
            pid,
            ppid: info.pbsi_ppid,
            name: c_chars_to_string(&info.pbsi_comm),
        })
    }

    pub(super) fn process_cwd(pid: u32) -> io::Result<PathBuf> {
        let info: libc::proc_vnodepathinfo = pid_info(pid, libc::PROC_PIDVNODEPATHINFO)?;
        let bytes: Vec<u8> = info
            .pvi_cdir
            .vip_path
            .iter()
            .flatten()
            .map(|&byte| byte as u8)
            .take_while(|&byte| byte != 0)
            .collect();
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("process {pid} has no working directory"),
            ));
        }
        Ok(PathBuf::from(OsStr::from_bytes(&bytes)))
    }

    pub(super) fn process_args(pid: u32) -> io::Result<Vec<String>> {
        let pid = c_int::try_from(pid).map_err(|_| io::Error::from(io::ErrorKind::NotFound))?;
        let mut buffer = vec![0u8; arg_max()?];
        let mut size = buffer.len();
        let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
        // SAFETY: `mib` names three valid integers, `buffer` is writable for
        // `size` bytes, and the kernel writes back how many it filled.
        let status = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as libc::c_uint,
                buffer.as_mut_ptr().cast::<c_void>(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        buffer.truncate(size);
        parse_procargs2(&buffer).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unreadable arguments for process {pid}"),
            )
        })
    }

    /// The kernel's limit on the size of a process's arguments and environment,
    /// which bounds the `KERN_PROCARGS2` buffer.
    fn arg_max() -> io::Result<usize> {
        let mut arg_max: c_int = 0;
        let mut size = size_of::<c_int>();
        let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
        // SAFETY: `arg_max` is a writable `c_int` and `size` says so.
        let status = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as libc::c_uint,
                (&raw mut arg_max).cast::<c_void>(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        usize::try_from(arg_max).map_err(|_| io::Error::other("invalid KERN_ARGMAX"))
    }

    /// Fill one `proc_pidinfo` flavor's struct for `pid`.
    ///
    /// `T` must be the plain C struct the flavor writes, for which all-zero
    /// bytes are a valid value (every libproc info struct is integers and
    /// arrays of them).
    fn pid_info<T>(pid: u32, flavor: c_int) -> io::Result<T> {
        let pid = c_int::try_from(pid).map_err(|_| io::Error::from(io::ErrorKind::NotFound))?;
        let size = c_int::try_from(size_of::<T>()).expect("libproc structs are small");
        let mut info = MaybeUninit::<T>::zeroed();
        // SAFETY: `info` is writable for `size` bytes; the kernel writes at
        // most that many.
        let written =
            unsafe { libc::proc_pidinfo(pid, flavor, 0, info.as_mut_ptr().cast::<c_void>(), size) };
        if written != size {
            let error = io::Error::last_os_error();
            // `proc_pidinfo` reports a vanished process as 0 bytes, sometimes
            // without setting errno.
            return Err(match error.raw_os_error() {
                Some(0) | None => io::Error::from(io::ErrorKind::NotFound),
                Some(libc::ESRCH) => io::Error::from(io::ErrorKind::NotFound),
                Some(_) => error,
            });
        }
        // SAFETY: the kernel filled all `size` bytes, and all-zero was
        // already a valid `T` (see above).
        Ok(unsafe { info.assume_init() })
    }

    fn c_chars_to_string(chars: &[libc::c_char]) -> String {
        let bytes: Vec<u8> = chars.iter().map(|&c| c as u8).collect();
        CStr::from_bytes_until_nul(&bytes)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_yields_the_name_and_the_parent_pid() {
        for (stat, expected) in [
            (
                &b"4211 (clickhouse) S 4210 4211 4211 0 -1 4194560 2390"[..],
                Some(("clickhouse", 4210)),
            ),
            // The name may contain spaces and parentheses, so it ends at the
            // last `)`.
            (
                b"77 (my (odd) name) R 1 77 77 0 -1",
                Some(("my (odd) name", 1)),
            ),
            (
                b"4210 (clickhouse-watc) S 1 4210",
                Some(("clickhouse-watc", 1)),
            ),
            (b"", None),
            (b"4211 (clickhouse) S", None),
            (b"4211 (clickhouse) S notapid", None),
            (b"4211 clickhouse) S 1", None),
        ] {
            let expected = expected.map(|(name, ppid)| (name.to_string(), ppid));
            assert_eq!(
                parse_proc_stat(stat),
                expected,
                "{}",
                String::from_utf8_lossy(stat)
            );
        }
    }

    #[test]
    fn nul_separated_arguments_keep_the_empty_ones_a_rewrite_leaves() {
        assert_eq!(
            parse_nul_separated(b"/opt/clickhouse\0server\0--\0--path=./\0"),
            ["/opt/clickhouse", "server", "--", "--path=./"]
        );
        // The watchdog's rewritten `argv[0]` followed by its zeroed argument
        // area.
        assert_eq!(
            parse_nul_separated(b"clickhouse-watchdog\0\0\0"),
            ["clickhouse-watchdog", "", ""]
        );
        // A kernel thread or zombie has no command line.
        assert!(parse_nul_separated(b"").is_empty());
    }

    fn procargs2(argc: i32, body: &[u8]) -> Vec<u8> {
        let mut buffer = argc.to_ne_bytes().to_vec();
        buffer.extend_from_slice(body);
        buffer
    }

    #[test]
    fn procargs2_yields_argc_arguments_and_never_the_environment() {
        let buffer = procargs2(
            3,
            b"/Users/al/.clickhouse/versions/26.9.1.531/clickhouse\0\0\0\0\
              /Users/al/.clickhouse/versions/26.9.1.531/clickhouse\0server\0--path=./\0\
              HOME=/Users/al\0PATH=/usr/bin\0",
        );

        assert_eq!(
            parse_procargs2(&buffer).unwrap(),
            [
                "/Users/al/.clickhouse/versions/26.9.1.531/clickhouse",
                "server",
                "--path=./",
            ]
        );
    }

    #[test]
    fn procargs2_reports_a_rewritten_argv0() {
        let buffer = procargs2(
            4,
            b"/Users/al/.clickhouse/versions/26.9.1.531/clickhouse\0\0\0\
              clickhouse-watchdog\0\0\0\0\0\0\0\0HOME=/Users/al\0",
        );

        let args = parse_procargs2(&buffer).unwrap();
        assert_eq!(args[0], "clickhouse-watchdog");
        assert!(args[1..].iter().all(String::is_empty), "{args:?}");
    }

    #[test]
    fn a_truncated_procargs2_buffer_is_unreadable() {
        for buffer in [&b""[..], b"\x01\0", &procargs2(1, b"/bin/sleep")] {
            assert_eq!(parse_procargs2(buffer), None, "{buffer:?}");
        }
    }

    #[test]
    fn children_are_the_rows_whose_parent_is_the_pid() {
        let row = |pid, ppid| ProcessEntry {
            pid,
            ppid,
            name: "clickhouse".into(),
        };
        let table = [
            row(10, 1),
            row(11, 10),
            row(12, 10),
            row(13, 11),
            row(0, 10),
        ];

        assert_eq!(children_in(&table, 10), [11, 12]);
        assert!(children_in(&table, 13).is_empty());
    }

    /// Spawns `sleep` from a directory the test owns, and kills it on drop.
    struct Sleeper {
        child: std::process::Child,
        cwd: tempfile::TempDir,
    }

    impl Sleeper {
        fn spawn() -> Self {
            let cwd = tempfile::tempdir().unwrap();
            let child = std::process::Command::new("sleep")
                .arg("30")
                .current_dir(cwd.path())
                .spawn()
                .expect("spawn sleep");
            Self { child, cwd }
        }

        fn pid(&self) -> u32 {
            self.child.id()
        }
    }

    impl Drop for Sleeper {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    #[test]
    fn the_native_scan_reads_a_live_childs_parent_name_cwd_and_arguments() {
        let sleeper = Sleeper::spawn();
        let pid = sleeper.pid();

        let table = process_table().expect("read the process table");
        let entry = table
            .iter()
            .find(|entry| entry.pid == pid)
            .expect("the child is in the process table");
        assert_eq!(entry.ppid, std::process::id());
        assert_eq!(entry.name, "sleep");

        assert!(child_pids(std::process::id()).unwrap().contains(&pid));
        assert_eq!(
            process_cwd(pid).unwrap(),
            sleeper.cwd.path().canonicalize().unwrap()
        );
        let args = process_args(pid).unwrap();
        assert!(args[0].ends_with("sleep"), "{args:?}");
        assert_eq!(args[1..], ["30"]);
    }

    #[test]
    fn a_process_that_exited_has_no_cwd_or_arguments() {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();

        assert!(process_cwd(pid).is_err());
        assert!(process_args(pid).is_err());
    }
}
