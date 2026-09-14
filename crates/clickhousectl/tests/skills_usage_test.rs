//! Non-interactive selection errors use the post-parse usage path.
use std::process::{Command, Stdio};

#[test]
fn missing_selection_without_a_terminal_is_a_usage_error() {
    for flags in [vec![], vec!["--global"]] {
        let dir = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
            .env_clear()
            .env("HOME", dir.path())
            .env("DO_NOT_TRACK", "1")
            .current_dir(dir.path())
            .arg("skills")
            .args(flags)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Usage: clickhousectl skills"), "{stderr}");
        assert!(output.stdout.is_empty());
        assert!(!dir.path().join(".agents").exists());
    }
}
