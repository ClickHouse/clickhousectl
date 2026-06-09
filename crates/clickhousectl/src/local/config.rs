//! Resolution and listing of custom server config files.
//!
//! Users drop named ClickHouse config files into `~/.clickhouse/configs/` and
//! reference them by name with `clickhousectl local server start --config-file
//! <NAME>`. The file is passed to ClickHouse as `--config-file`; the launcher
//! still forces `--path=./` and the ports as command-line overrides (which beat
//! config-file values), so the managed server lifecycle is preserved regardless
//! of what the config file contains.

use crate::error::{Error, Result};
use crate::paths;
use std::path::{Path, PathBuf};

/// Recognized config file extensions, in resolution priority order.
const CONFIG_EXTS: [&str; 3] = ["xml", "yaml", "yml"];

/// Filename stem for the chctl-managed `config.d` overlay file.
const OVERLAY_STEM: &str = "chctl-config";

/// Returns true if `name` already ends in a recognized config extension.
fn has_config_ext(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| CONFIG_EXTS.contains(&e))
        .unwrap_or(false)
}

/// Lists the names of config files in `dir` (those with a recognized
/// extension), sorted. Returns an empty vec if the directory does not exist.
pub fn list_configs_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let path = e.path();
            let ext = path.extension().and_then(|e| e.to_str())?;
            if CONFIG_EXTS.contains(&ext) {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

/// Rejects config names that would resolve outside `dir`.
///
/// A named config is, by design, a file living directly in the configs store.
/// Without this guard `dir.join(name)` would let an absolute path or `..`
/// segments escape the store (`Path::join` replaces the base on an absolute
/// argument), copying arbitrary files into the server overlay. Mirrors
/// `server::validate_server_name`.
fn validate_config_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
    {
        return Err(Error::InvalidConfigName(name.to_string()));
    }
    Ok(())
}

/// Resolves a config `name` to a file path within `dir`.
///
/// If `name` already carries a recognized extension, that exact file must
/// exist. Otherwise each known extension is tried; exactly one match must be
/// found. Missing or ambiguous names produce a helpful error. Names that would
/// escape `dir` (path separators or `..`) are rejected.
pub fn resolve_config_in(dir: &Path, name: &str) -> Result<PathBuf> {
    validate_config_name(name)?;

    if has_config_ext(name) {
        let path = dir.join(name);
        if path.is_file() {
            return Ok(path);
        }
        return Err(not_found_error(dir, name));
    }

    let matches: Vec<PathBuf> = CONFIG_EXTS
        .iter()
        .map(|ext| dir.join(format!("{name}.{ext}")))
        .filter(|p| p.is_file())
        .collect();

    match matches.len() {
        1 => Ok(matches.into_iter().next().unwrap()),
        0 => Err(not_found_error(dir, name)),
        _ => {
            let exts = matches
                .iter()
                .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
                .collect::<Vec<_>>()
                .join(", ");
            Err(Error::ConfigNotFound(format!(
                "config '{name}' is ambiguous in {} ({exts}); specify the file extension",
                dir.display()
            )))
        }
    }
}

fn not_found_error(dir: &Path, name: &str) -> Error {
    let available = list_configs_in(dir);
    let avail = if available.is_empty() {
        "none".to_string()
    } else {
        available.join(", ")
    };
    Error::ConfigNotFound(format!(
        "config '{name}' not found in {} (available: {avail})",
        dir.display()
    ))
}

/// Resolves a named config from `~/.clickhouse/configs/` to its absolute path.
pub fn resolve_config(name: &str) -> Result<PathBuf> {
    resolve_config_in(&paths::configs_dir()?, name)
}

/// Lists the available config file names in `~/.clickhouse/configs/`.
pub fn list_configs() -> Result<Vec<String>> {
    Ok(list_configs_in(&paths::configs_dir()?))
}

/// Stages (or clears) the chctl-managed config overlay in `<data_dir>/config.d/`.
///
/// ClickHouse merges files in the `config.d/` directory next to its working
/// directory with its built-in defaults, so a partial override file takes
/// effect without replacing the whole config. We own a single file there named
/// `chctl-config.<ext>`; any previously staged overlay (in any recognized
/// extension) is removed first, so restarting a server without `--config-file`
/// reverts cleanly to plain defaults.
pub fn apply_config_overlay(data_dir: &Path, source: Option<&Path>) -> Result<()> {
    let config_d = data_dir.join("config.d");

    // Drop any overlay we previously staged before applying the new state.
    for ext in CONFIG_EXTS {
        let stale = config_d.join(format!("{OVERLAY_STEM}.{ext}"));
        if stale.exists() {
            std::fs::remove_file(&stale)?;
        }
    }

    let Some(source) = source else {
        return Ok(());
    };

    // Preserve the source extension so ClickHouse parses XML vs YAML correctly.
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| CONFIG_EXTS.contains(e))
        .unwrap_or("xml");
    std::fs::create_dir_all(&config_d)?;
    std::fs::copy(source, config_d.join(format!("{OVERLAY_STEM}.{ext}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), "<clickhouse/>").unwrap();
    }

    #[test]
    fn resolves_by_bare_name() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "analytics.xml");
        let path = resolve_config_in(tmp.path(), "analytics").unwrap();
        assert_eq!(path, tmp.path().join("analytics.xml"));
    }

    #[test]
    fn resolves_yaml_by_bare_name() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "prod.yaml");
        let path = resolve_config_in(tmp.path(), "prod").unwrap();
        assert_eq!(path, tmp.path().join("prod.yaml"));
    }

    #[test]
    fn resolves_with_explicit_extension() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "dev.xml");
        let path = resolve_config_in(tmp.path(), "dev.xml").unwrap();
        assert_eq!(path, tmp.path().join("dev.xml"));
    }

    #[test]
    fn explicit_extension_missing_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_config_in(tmp.path(), "dev.xml").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {msg}");
    }

    #[test]
    fn not_found_lists_available() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "dev.xml");
        write_file(tmp.path(), "prod.yaml");
        let err = resolve_config_in(tmp.path(), "missing").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("missing"), "got: {msg}");
        assert!(msg.contains("dev.xml"), "got: {msg}");
        assert!(msg.contains("prod.yaml"), "got: {msg}");
    }

    #[test]
    fn not_found_reports_none_when_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_config_in(tmp.path(), "missing").unwrap_err();
        assert!(err.to_string().contains("available: none"));
    }

    #[test]
    fn ambiguous_bare_name_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "shared.xml");
        write_file(tmp.path(), "shared.yaml");
        let err = resolve_config_in(tmp.path(), "shared").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ambiguous"), "got: {msg}");
        assert!(msg.contains("shared.xml"));
        assert!(msg.contains("shared.yaml"));
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let tmp = tempfile::tempdir().unwrap();
        // A real file outside the configs dir that an escape could target.
        let outside = tmp.path().join("outside.xml");
        std::fs::write(&outside, "<clickhouse/>").unwrap();
        let configs = tmp.path().join("configs");
        std::fs::create_dir(&configs).unwrap();

        let err = resolve_config_in(&configs, "../outside").unwrap_err();
        assert!(
            matches!(err, Error::InvalidConfigName(_)),
            "got: {err:?}"
        );
    }

    #[test]
    fn rejects_parent_dir_escape_with_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside.xml");
        std::fs::write(&outside, "<clickhouse/>").unwrap();
        let configs = tmp.path().join("configs");
        std::fs::create_dir(&configs).unwrap();

        let err = resolve_config_in(&configs, "../outside.xml").unwrap_err();
        assert!(
            matches!(err, Error::InvalidConfigName(_)),
            "got: {err:?}"
        );
    }

    #[test]
    fn rejects_absolute_path() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "dev.xml");
        // Absolute path would make `dir.join` discard the configs dir entirely.
        let abs = tmp.path().join("dev.xml");
        let err = resolve_config_in(tmp.path(), abs.to_str().unwrap()).unwrap_err();
        assert!(
            matches!(err, Error::InvalidConfigName(_)),
            "got: {err:?}"
        );
    }

    #[test]
    fn rejects_dotdot() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_config_in(tmp.path(), "..").unwrap_err();
        assert!(
            matches!(err, Error::InvalidConfigName(_)),
            "got: {err:?}"
        );
    }

    #[test]
    fn list_filters_and_sorts() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "prod.yaml");
        write_file(tmp.path(), "dev.xml");
        write_file(tmp.path(), "notes.txt");
        std::fs::create_dir(tmp.path().join("subdir.xml")).unwrap();
        let configs = list_configs_in(tmp.path());
        assert_eq!(configs, vec!["dev.xml", "prod.yaml"]);
    }

    #[test]
    fn list_nonexistent_dir_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does-not-exist");
        assert!(list_configs_in(&missing).is_empty());
    }

    #[test]
    fn overlay_stages_file_with_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.xml");
        std::fs::write(&src, "<clickhouse><a>1</a></clickhouse>").unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();

        apply_config_overlay(&data_dir, Some(&src)).unwrap();

        let staged = data_dir.join("config.d").join("chctl-config.xml");
        assert!(staged.is_file());
        assert_eq!(
            std::fs::read_to_string(&staged).unwrap(),
            "<clickhouse><a>1</a></clickhouse>"
        );
    }

    #[test]
    fn overlay_preserves_yaml_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.yaml");
        std::fs::write(&src, "a: 1").unwrap();
        let data_dir = tmp.path().join("data");

        apply_config_overlay(&data_dir, Some(&src)).unwrap();

        assert!(data_dir.join("config.d").join("chctl-config.yaml").is_file());
    }

    #[test]
    fn overlay_none_clears_previous() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.xml");
        std::fs::write(&src, "<clickhouse/>").unwrap();
        let data_dir = tmp.path().join("data");

        apply_config_overlay(&data_dir, Some(&src)).unwrap();
        assert!(data_dir.join("config.d").join("chctl-config.xml").is_file());

        apply_config_overlay(&data_dir, None).unwrap();
        assert!(!data_dir.join("config.d").join("chctl-config.xml").exists());
    }

    #[test]
    fn overlay_switching_extension_removes_old() {
        let tmp = tempfile::tempdir().unwrap();
        let xml = tmp.path().join("a.xml");
        std::fs::write(&xml, "<clickhouse/>").unwrap();
        let yaml = tmp.path().join("b.yaml");
        std::fs::write(&yaml, "a: 1").unwrap();
        let data_dir = tmp.path().join("data");

        apply_config_overlay(&data_dir, Some(&xml)).unwrap();
        apply_config_overlay(&data_dir, Some(&yaml)).unwrap();

        let config_d = data_dir.join("config.d");
        assert!(!config_d.join("chctl-config.xml").exists());
        assert!(config_d.join("chctl-config.yaml").is_file());
    }

    #[test]
    fn overlay_none_on_empty_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        // Should not error even though config.d does not exist.
        apply_config_overlay(&data_dir, None).unwrap();
    }
}
