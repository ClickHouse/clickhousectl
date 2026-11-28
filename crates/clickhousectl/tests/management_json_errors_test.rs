//! `update` and `skills` runtime failures in JSON mode write one error object
//! to stderr and nothing to stdout (#1233).
use std::process::Command;

/// A proxy nothing listens on, so every outbound request fails at once
/// without touching the network.
fn closed_proxy() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

#[test]
fn update_and_skills_network_failures_print_a_json_error_envelope() {
    let proxy = closed_proxy();
    // Experimental builds make `update` inert (tested separately below).
    let cases = [
        #[cfg(not(feature = "experimental"))]
        (vec!["update", "--check", "--json"], "transport", false),
        // Never plain `update`: if the proxy were ignored it would replace
        // the test binary.
        #[cfg(not(feature = "experimental"))]
        (vec!["update", "--check"], "transport", true),
        (
            vec!["skills", "--agent", "claude", "--json"],
            "transport",
            false,
        ),
    ];
    for (args, code, agent) in cases {
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
        command
            .env_clear()
            .env("HOME", dir.path())
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HTTPS_PROXY", &proxy)
            .env("HTTP_PROXY", &proxy)
            .current_dir(dir.path())
            .args(&args);
        if agent {
            command.env("AI_AGENT", "1");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        let envelope: serde_json::Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|error| panic!("{args:?}: stderr is not one JSON object: {error}"));
        assert_eq!(envelope["error"]["code"], code, "{args:?}");
        assert!(envelope["error"]["message"].is_string(), "{args:?}");
        assert!(!dir.path().join(".agents").exists());
    }
}

/// The npm launcher's environment marker makes `update` refuse before any
/// request, naming npm's upgrade command (#1305). The closed proxy keeps a
/// missed refusal from replacing the test binary: it would fail as
/// `transport` instead.
#[cfg(not(feature = "experimental"))]
#[test]
fn update_refuses_an_npm_install_without_a_request() {
    let proxy = closed_proxy();
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
        .env_clear()
        .env("HOME", dir.path())
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HTTPS_PROXY", &proxy)
        .env("HTTP_PROXY", &proxy)
        .env("CLICKHOUSECTL_INSTALL_CHANNEL", "npm")
        .current_dir(dir.path())
        .args(["update", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["error"]["code"], "package_managed");
    assert_eq!(
        envelope["error"]["command"],
        "npm install -g clickhousectl@latest"
    );
}

/// A proxy that accepts the `skills` download and never answers: the
/// download gives up at its (overridden) timeout and reports `timeout` (#1310).
#[test]
fn a_stalled_skills_download_times_out() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    // Accepted connections are held open, unanswered, until the test ends.
    std::thread::spawn(move || {
        let held: Vec<_> = listener.incoming().collect();
        drop(held);
    });
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
        .env_clear()
        .env("HOME", dir.path())
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("CHCTL_SKILLS_DOWNLOAD_TIMEOUT_MS", "200")
        .env("HTTPS_PROXY", &proxy)
        .env("HTTP_PROXY", &proxy)
        .current_dir(dir.path())
        .args(["skills", "--agent", "claude", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["error"]["code"], "timeout");
    assert!(!dir.path().join(".agents").exists());
}

/// Experimental builds make `update` and `update --check` inert: they make no
/// network requests (even with a closed proxy) and always report the running
/// experimental build as up to date.
#[cfg(feature = "experimental")]
#[test]
fn experimental_update_is_inert_without_network_access() {
    let proxy = closed_proxy();
    for args in [
        vec!["update", "--check", "--json"],
        vec!["update", "--json"],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
            .env_clear()
            .env("HOME", dir.path())
            .env("DO_NOT_TRACK", "1")
            .env("HTTPS_PROXY", &proxy)
            .env("HTTP_PROXY", &proxy)
            .current_dir(dir.path())
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
        let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(body["action"], "up_to_date", "{args:?}");
        assert_eq!(body["current_version"], body["latest_version"], "{args:?}");
        assert!(
            body["current_version"]
                .as_str()
                .unwrap()
                .contains("experimental"),
            "{args:?}: {body}"
        );
        assert!(!dir.path().join(".clickhouse/last_update_check").exists());
    }
}
