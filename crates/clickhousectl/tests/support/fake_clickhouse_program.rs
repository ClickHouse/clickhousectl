// A stand-in ClickHouse server for subprocess tests, compiled by
// `fake_clickhouse.rs` with plain `rustc` (std only, edition 2021).
//
// Process discovery reads the real process table, so the stand-in has to be a
// real executable whose kernel name is `clickhouse`, which a shell script is
// not on macOS. It ignores its arguments, records itself, and sleeps.
//
// Environment:
// - FAKE_CLICKHOUSE_PID_FILE: append `<pid>|<role>|<ppid>|<cwd>` per process.
// - FAKE_CLICKHOUSE_WATCHDOG=1: run as a watchdog that supervises a server
//   child, like ClickHouse's own. The watchdog re-executes itself from the
//   `clickhouse-watchdog` copy next to this binary with `argv[0]` rewritten to
//   `clickhouse-watchdog`, so neither its kernel name nor its arguments say
//   `clickhouse` alone.
// - FAKE_CLICKHOUSE_IGNORE_SIGTERM=1: the watchdog ignores SIGTERM instead of
//   forwarding it to the server and exiting.
// - FAKE_CLICKHOUSE_VERSION_OUTPUT: what `--version` prints, instead of a
//   ClickHouse version line; `--version` never records or sleeps.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
    fn getppid() -> i32;
    fn _exit(status: i32) -> !;
}

const SIGTERM: i32 = 15;
const SIG_DFL: usize = 0;
const SIG_IGN: usize = 1;
const ROLE: &str = "FAKE_CLICKHOUSE_ROLE";

static CHILD: AtomicI32 = AtomicI32::new(0);

extern "C" fn forward_sigterm(_: i32) {
    let child = CHILD.load(Ordering::SeqCst);
    unsafe {
        if child > 0 {
            kill(child, SIGTERM);
        }
        _exit(0);
    }
}

fn record(pid: u32, role: &str, ppid: u32) {
    let Some(path) = env::var_os("FAKE_CLICKHOUSE_PID_FILE") else {
        return;
    };
    let cwd = env::current_dir().unwrap_or_default();
    let line = format!("{pid}|{role}|{ppid}|{}\n", cwd.display());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn sleep_forever() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(300));
    }
}

fn main() {
    if env::args().nth(1).as_deref() == Some("--version") {
        match env::var("FAKE_CLICKHOUSE_VERSION_OUTPUT") {
            Ok(output) => println!("{output}"),
            Err(_) => println!("ClickHouse local version 26.9.1.4242 (fake build)."),
        }
        return;
    }
    let ppid = unsafe { getppid() } as u32;
    match env::var(ROLE).as_deref() {
        Ok("watchdog") => watchdog(ppid),
        // A supervised server: its watchdog has already recorded it.
        Ok("server") => server(),
        _ if env::var("FAKE_CLICKHOUSE_WATCHDOG").as_deref() == Ok("1") => {
            let server_binary = env::current_exe().expect("own path");
            let watchdog_binary = server_binary.with_file_name("clickhouse-watchdog");
            let error = Command::new(watchdog_binary)
                .arg0("clickhouse-watchdog")
                .args(env::args_os().skip(1))
                .env(ROLE, "watchdog")
                .env("FAKE_CLICKHOUSE_SERVER_BINARY", server_binary)
                .exec();
            panic!("exec the fake watchdog: {error}");
        }
        _ => {
            record(std::process::id(), "server", ppid);
            server();
        }
    }
}

fn server() -> ! {
    // The watchdog's ignored SIGTERM would otherwise be inherited.
    unsafe {
        signal(SIGTERM, SIG_DFL);
    }
    sleep_forever()
}

fn watchdog(ppid: u32) -> ! {
    let ignore_sigterm = env::var("FAKE_CLICKHOUSE_IGNORE_SIGTERM").as_deref() == Ok("1");
    // The real watchdog forwards SIGTERM to the server it supervises. The
    // handler goes in first so no SIGTERM finds the default action.
    let handler = if ignore_sigterm {
        SIG_IGN
    } else {
        forward_sigterm as usize
    };
    unsafe {
        signal(SIGTERM, handler);
    }
    // Recorded before forking, so a test's cleanup can always reach it.
    let pid = std::process::id();
    record(pid, "watchdog", ppid);
    let mut child = Command::new(env::var_os("FAKE_CLICKHOUSE_SERVER_BINARY").expect("server"))
        .args(env::args_os().skip(1))
        .env(ROLE, "server")
        .spawn()
        .expect("spawn the fake server");
    CHILD.store(child.id() as i32, Ordering::SeqCst);
    // Recorded once SIGTERM would reach it, so a test that waits for this
    // record never signals a watchdog that cannot forward yet.
    record(child.id(), "server", pid);
    let _ = child.wait();
    unsafe { _exit(0) }
}
