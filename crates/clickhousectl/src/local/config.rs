//! Resolution, listing and staging of custom server config files.
//!
//! A server start merges three sources over ClickHouse's built-in defaults,
//! later winning: every file in the global `~/.clickhouse/config.d/`, every
//! file in the project's `clickhouse/config.d/`, then the one file `--config
//! <NAME>` selects from `~/.clickhouse/configs/`. Each is copied into the
//! server's own `config.d/` directory under a name that sorts in that order.
//! The launcher writes the data path and ports to its own `config.d/` file,
//! named to merge after every other file there, so the managed server
//! lifecycle is preserved regardless of what the config files contain. That
//! file, unlike command-line overrides, survives the config reloads ClickHouse
//! runs whenever `config.d/` changes on a running server.

use crate::error::{Error, Result};
use crate::paths;
use std::path::{Path, PathBuf};

/// Recognized config file extensions, in resolution priority order.
const CONFIG_EXTS: [&str; 3] = ["xml", "yaml", "yml"];

/// Filename stem for the chctl-managed `config.d` overlay file.
const OVERLAY_STEM: &str = "chctl-config";

/// File holding the settings the launcher manages: the data path and ports.
///
/// ClickHouse merges `config.d/` files in lexicographic order with later files
/// winning, so the name sorts after every other chctl file there (`chctl-*`),
/// including the `--config` overlay.
pub const MANAGED_CONFIG_FILE: &str = "zz-chctl-managed.xml";

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
pub(crate) fn validate_config_name(name: &str) -> Result<()> {
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

/// Prefix of the files staged from the global `~/.clickhouse/config.d/`.
const GLOBAL_PREFIX: &str = "chctl-0-global-";

/// Prefix of the files staged from the project's `clickhouse/config.d/`.
const PROJECT_PREFIX: &str = "chctl-1-project-";

/// Prefixes of every file [`apply_config_overlay`] stages from a layer. The
/// cleanup removes exactly these, so a file anyone else put in the server's
/// `config.d/` survives.
const LAYER_PREFIXES: [&str; 2] = ["chctl-0-", "chctl-1-"];

/// The project's always-applied config directory, relative to its root.
pub const PROJECT_CONFIG_D: &str = "clickhouse/config.d";

/// The project's always-applied config directory under this invocation's
/// project root.
pub fn project_config_d_dir() -> std::io::Result<PathBuf> {
    Ok(crate::init::project_root()?.dir.join(PROJECT_CONFIG_D))
}

/// The config sources a server start merges, from first to last applied. The
/// managed file ([`MANAGED_CONFIG_FILE`]) merges after all of them.
#[derive(Debug, Clone, Copy)]
pub struct ConfigLayers<'a> {
    /// Applied to every server on this machine (`~/.clickhouse/config.d/`).
    pub global_dir: &'a Path,
    /// Applied to every server in this project (`clickhouse/config.d/`).
    pub project_dir: &'a Path,
    /// The file `--config NAME` selected, if any.
    pub named: Option<&'a Path>,
}

/// One file [`apply_config_overlay`] copied into the server's `config.d/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedConfig {
    /// File name inside the server's `config.d/`.
    pub staged: String,
    /// The file it was copied from.
    pub source: PathBuf,
}

/// Lists the files a `config.d` layer contributes, sorted by name: regular
/// files (or links to them) with a recognized extension, skipping hidden
/// entries and subdirectories as ClickHouse does. A missing directory has
/// none; any other read failure is returned.
pub fn layer_files(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') || !has_config_ext(name) || !path.is_file() {
            continue;
        }
        files.push((name.to_string(), path));
    }
    files.sort();
    Ok(files)
}

/// Stages every config source in `layers` into `<data_dir>/config.d/` and
/// returns what it staged, in merge order.
///
/// ClickHouse merges the files in the `config.d/` directory next to its
/// working directory with its built-in defaults, in byte order of their names
/// with later files winning, so a partial override file takes effect without
/// replacing the whole config. The staged names keep that order: global
/// files as `chctl-0-global-<name>`, then project files as
/// `chctl-1-project-<name>`, then the named config as `chctl-config.<ext>`.
/// A file with the same name in both layers is staged twice, so both apply.
///
/// Everything staged by an earlier start is removed first, so a deleted
/// source or a start without `--config` stops applying. Only entries with
/// those names directly inside `config.d/` are removed; a symlink is removed
/// as a link, never followed, and a symlinked `config.d/` is refused.
pub fn apply_config_overlay(
    data_dir: &Path,
    layers: ConfigLayers<'_>,
) -> Result<Vec<StagedConfig>> {
    let config_d = data_dir.join("config.d");
    clear_staged(&config_d)?;

    let mut staged = Vec::new();
    for (prefix, dir) in [
        (GLOBAL_PREFIX, layers.global_dir),
        (PROJECT_PREFIX, layers.project_dir),
    ] {
        for (name, source) in layer_files(dir)? {
            staged.push(StagedConfig {
                staged: format!("{prefix}{name}"),
                source,
            });
        }
    }
    if let Some(source) = layers.named {
        // Preserve the source extension so ClickHouse parses XML vs YAML correctly.
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| CONFIG_EXTS.contains(e))
            .unwrap_or("xml");
        staged.push(StagedConfig {
            staged: format!("{OVERLAY_STEM}.{ext}"),
            source: source.to_path_buf(),
        });
    }

    if staged.is_empty() {
        return Ok(staged);
    }
    std::fs::create_dir_all(&config_d)?;
    for file in &staged {
        std::fs::copy(&file.source, config_d.join(&file.staged))?;
    }
    Ok(staged)
}

/// Removes every file an earlier [`apply_config_overlay`] staged in
/// `config_d`, and nothing else.
fn clear_staged(config_d: &Path) -> Result<()> {
    match std::fs::symlink_metadata(config_d) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(Error::Io(std::io::Error::other(format!(
                "{} is not a directory; refusing to stage config files through it",
                config_d.display()
            ))));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    for entry in std::fs::read_dir(config_d)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let ours = LAYER_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
            || CONFIG_EXTS
                .iter()
                .any(|ext| name == format!("{OVERLAY_STEM}.{ext}"));
        // `file_type` does not follow symlinks, so a link is removed as a link.
        if ours && !entry.file_type()?.is_dir() {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

/// Adds the files a failed start staged, with their sources, to a startup
/// error's details, so a broken file is traceable to where it came from.
pub fn with_staged_files(error: Error, staged: &[StagedConfig]) -> Error {
    match error {
        Error::StartupExit {
            kind,
            name,
            mut details,
        } if !staged.is_empty() => {
            append_staged(&mut details, staged);
            Error::StartupExit {
                kind,
                name,
                details,
            }
        }
        Error::StartupTimeout {
            kind,
            name,
            seconds,
            mut details,
        } if !staged.is_empty() => {
            append_staged(&mut details, staged);
            Error::StartupTimeout {
                kind,
                name,
                seconds,
                details,
            }
        }
        other => other,
    }
}

fn append_staged(details: &mut String, staged: &[StagedConfig]) {
    details.push_str("\nStaged config files (config.d/, merged in this order):");
    for file in staged {
        details.push_str(&format!(
            "\n  {} (from {})",
            file.staged,
            file.source.display()
        ));
    }
}

/// Renders the managed config file for a server listening on these ports.
fn managed_config_xml(http_port: u16, tcp_port: u16) -> String {
    format!(
        "<!-- Managed by clickhousectl; rewritten on every `local clickhouse start`. -->\n\
         <clickhouse>\n    \
         <path>./</path>\n    \
         <http_port>{http_port}</http_port>\n    \
         <tcp_port>{tcp_port}</tcp_port>\n\
         </clickhouse>\n"
    )
}

/// Writes the managed config file into `<data_dir>/config.d/`.
///
/// The file is left untouched when it already holds this content: any write
/// under `config.d/` makes a running ClickHouse reload its config, and this
/// runs only before the server starts, so it must never be the trigger itself.
pub fn write_managed_config(data_dir: &Path, http_port: u16, tcp_port: u16) -> Result<()> {
    let config_d = data_dir.join("config.d");
    let path = config_d.join(MANAGED_CONFIG_FILE);
    let content = managed_config_xml(http_port, tcp_port);
    if std::fs::read_to_string(&path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    std::fs::create_dir_all(&config_d)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Reads the ports from the managed config file in `<data_dir>/config.d/`.
///
/// Returns `(http_port, tcp_port)`, each absent when the file or the element is
/// missing or unparseable. Only the file [`write_managed_config`] writes is
/// read, so plain element matching is enough.
pub fn read_managed_ports(data_dir: &Path) -> (Option<u16>, Option<u16>) {
    let Ok(content) = std::fs::read_to_string(data_dir.join("config.d").join(MANAGED_CONFIG_FILE))
    else {
        return (None, None);
    };
    (
        element_u16(&content, "http_port"),
        element_u16(&content, "tcp_port"),
    )
}

fn element_u16(content: &str, tag: &str) -> Option<u16> {
    let open = format!("<{tag}>");
    let start = content.find(&open)? + open.len();
    let end = start + content[start..].find(&format!("</{tag}>"))?;
    content[start..end].trim().parse().ok()
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
        assert!(matches!(err, Error::ConfigNotFound(_)), "got: {msg}");
        assert!(msg.contains("'dev.xml'"), "got: {msg}");
        assert!(
            msg.contains(&tmp.path().display().to_string()),
            "got: {msg}"
        );
    }

    #[test]
    fn not_found_lists_available() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "dev.xml");
        write_file(tmp.path(), "prod.yaml");
        let err = resolve_config_in(tmp.path(), "missing").unwrap_err();
        let msg = err.to_string();
        assert!(matches!(err, Error::ConfigNotFound(_)), "got: {msg}");
        assert!(msg.contains("'missing'"), "got: {msg}");
        assert!(msg.contains("dev.xml"), "got: {msg}");
        assert!(msg.contains("prod.yaml"), "got: {msg}");
    }

    #[test]
    fn not_found_reports_none_when_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_config_in(tmp.path(), "missing").unwrap_err();
        let msg = err.to_string();
        assert!(matches!(err, Error::ConfigNotFound(_)), "got: {msg}");
        assert!(msg.contains("'missing'"), "got: {msg}");
        assert!(
            msg.contains(&tmp.path().display().to_string()),
            "got: {msg}"
        );
    }

    #[test]
    fn ambiguous_bare_name_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "shared.xml");
        write_file(tmp.path(), "shared.yaml");
        let err = resolve_config_in(tmp.path(), "shared").unwrap_err();
        let msg = err.to_string();
        assert!(matches!(err, Error::ConfigNotFound(_)), "got: {msg}");
        assert!(msg.contains("'shared'"), "got: {msg}");
        assert!(msg.contains("shared.xml"));
        assert!(msg.contains("shared.yaml"));
        // Not the not-found error, which would list the same two files.
        assert_ne!(msg, not_found_error(tmp.path(), "shared").to_string());
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
        assert!(matches!(err, Error::InvalidConfigName(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_parent_dir_escape_with_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside.xml");
        std::fs::write(&outside, "<clickhouse/>").unwrap();
        let configs = tmp.path().join("configs");
        std::fs::create_dir(&configs).unwrap();

        let err = resolve_config_in(&configs, "../outside.xml").unwrap_err();
        assert!(matches!(err, Error::InvalidConfigName(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_absolute_path() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "dev.xml");
        // Absolute path would make `dir.join` discard the configs dir entirely.
        let abs = tmp.path().join("dev.xml");
        let err = resolve_config_in(tmp.path(), abs.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfigName(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_dotdot() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_config_in(tmp.path(), "..").unwrap_err();
        assert!(matches!(err, Error::InvalidConfigName(_)), "got: {err:?}");
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

    /// Applies only a named config, with both layer directories absent.
    fn apply_named(data_dir: &Path, named: Option<&Path>) -> Vec<StagedConfig> {
        let missing = data_dir.join("no-such-layer");
        apply_config_overlay(
            data_dir,
            ConfigLayers {
                global_dir: &missing,
                project_dir: &missing,
                named,
            },
        )
        .unwrap()
    }

    /// The file names in `<data_dir>/config.d/`, sorted as ClickHouse merges them.
    fn staged_names(data_dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(data_dir.join("config.d"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// A temp tree with global and project layer directories and a data dir.
    struct Layers {
        _tmp: tempfile::TempDir,
        global: PathBuf,
        project: PathBuf,
        data: PathBuf,
    }

    impl Layers {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let global = tmp.path().join("home/.clickhouse/config.d");
            let project = tmp.path().join("app/clickhouse/config.d");
            let data = tmp.path().join("app/.clickhouse/servers/default/data");
            for dir in [&global, &project, &data] {
                std::fs::create_dir_all(dir).unwrap();
            }
            Self {
                _tmp: tmp,
                global,
                project,
                data,
            }
        }

        fn apply(&self, named: Option<&Path>) -> Vec<StagedConfig> {
            apply_config_overlay(
                &self.data,
                ConfigLayers {
                    global_dir: &self.global,
                    project_dir: &self.project,
                    named,
                },
            )
            .unwrap()
        }

        fn staged(&self, name: &str) -> String {
            std::fs::read_to_string(self.data.join("config.d").join(name)).unwrap()
        }
    }

    #[test]
    fn overlay_stages_file_with_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.xml");
        std::fs::write(&src, "<clickhouse><a>1</a></clickhouse>").unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();

        apply_named(&data_dir, Some(&src));

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

        apply_named(&data_dir, Some(&src));

        assert!(
            data_dir
                .join("config.d")
                .join("chctl-config.yaml")
                .is_file()
        );
    }

    #[test]
    fn overlay_none_clears_previous() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src.xml");
        std::fs::write(&src, "<clickhouse/>").unwrap();
        let data_dir = tmp.path().join("data");

        apply_named(&data_dir, Some(&src));
        assert!(data_dir.join("config.d").join("chctl-config.xml").is_file());

        apply_named(&data_dir, None);
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

        apply_named(&data_dir, Some(&xml));
        apply_named(&data_dir, Some(&yaml));

        let config_d = data_dir.join("config.d");
        assert!(!config_d.join("chctl-config.xml").exists());
        assert!(config_d.join("chctl-config.yaml").is_file());
    }

    #[test]
    fn overlay_with_no_sources_is_a_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        // Neither layer directory exists and config.d is not created.
        assert!(apply_named(&data_dir, None).is_empty());
        assert!(!data_dir.join("config.d").exists());

        // Empty layer directories stage nothing either.
        let layers = Layers::new();
        assert!(layers.apply(None).is_empty());
    }

    #[test]
    fn layers_merge_global_then_project_then_named_before_the_managed_files() {
        let layers = Layers::new();
        std::fs::write(layers.global.join("b.xml"), "global b").unwrap();
        std::fs::write(layers.global.join("a.yaml"), "global a").unwrap();
        std::fs::write(layers.project.join("c.yml"), "project c").unwrap();
        let named = layers.data.join("../named.xml");
        std::fs::write(&named, "named").unwrap();

        let staged = layers.apply(Some(&named));

        let order = [
            "chctl-0-global-a.yaml",
            "chctl-0-global-b.xml",
            "chctl-1-project-c.yml",
            "chctl-config.xml",
        ];
        // Byte order, which ClickHouse merges in, is the layer order.
        assert_eq!(staged_names(&layers.data), order);
        assert_eq!(
            staged.iter().map(|f| f.staged.as_str()).collect::<Vec<_>>(),
            order
        );
        assert_eq!(staged[0].source, layers.global.join("a.yaml"));
        assert_eq!(staged[2].source, layers.project.join("c.yml"));
        assert_eq!(staged[3].source, named);
        assert_eq!(layers.staged("chctl-0-global-b.xml"), "global b");
        assert_eq!(layers.staged("chctl-1-project-c.yml"), "project c");
    }

    #[test]
    fn a_file_name_in_both_layers_stages_both_with_the_project_last() {
        let layers = Layers::new();
        std::fs::write(layers.global.join("logging.xml"), "global").unwrap();
        std::fs::write(layers.project.join("logging.xml"), "project").unwrap();

        layers.apply(None);

        assert_eq!(
            staged_names(&layers.data),
            ["chctl-0-global-logging.xml", "chctl-1-project-logging.xml"]
        );
        assert_eq!(layers.staged("chctl-0-global-logging.xml"), "global");
        assert_eq!(layers.staged("chctl-1-project-logging.xml"), "project");
    }

    #[test]
    fn a_deleted_source_stops_applying_on_the_next_start() {
        let layers = Layers::new();
        std::fs::write(layers.global.join("g.xml"), "g").unwrap();
        std::fs::write(layers.project.join("keep.xml"), "keep").unwrap();
        std::fs::write(layers.project.join("drop.xml"), "drop").unwrap();
        layers.apply(None);

        std::fs::remove_file(layers.project.join("drop.xml")).unwrap();
        std::fs::remove_file(layers.global.join("g.xml")).unwrap();
        layers.apply(None);

        assert_eq!(staged_names(&layers.data), ["chctl-1-project-keep.xml"]);
    }

    #[test]
    fn hidden_files_other_extensions_and_subdirectories_are_skipped() {
        let layers = Layers::new();
        for name in [
            ".hidden.xml",
            "notes.txt",
            "README.md",
            "x.xml.bak",
            ".gitkeep",
        ] {
            std::fs::write(layers.project.join(name), "skip").unwrap();
        }
        std::fs::create_dir(layers.project.join("nested.xml")).unwrap();
        std::fs::write(layers.project.join("nested.xml/inner.xml"), "skip").unwrap();
        std::fs::write(layers.project.join("real.xml"), "real").unwrap();

        layers.apply(None);

        assert_eq!(staged_names(&layers.data), ["chctl-1-project-real.xml"]);
    }

    #[test]
    fn restaging_removes_only_chctl_files_and_never_follows_links() {
        let layers = Layers::new();
        let config_d = layers.data.join("config.d");
        std::fs::create_dir_all(&config_d).unwrap();
        // Files chctl does not own, including ones that look nearly like it.
        for name in [
            "user.xml",
            "chctl-2-other.xml",
            "chctl-udf.xml",
            MANAGED_CONFIG_FILE,
        ] {
            std::fs::write(config_d.join(name), "keep").unwrap();
        }
        // A stale staged link pointing outside config.d, and a directory with
        // a staged-looking name.
        let outside = layers.data.join("../outside.xml");
        std::fs::write(&outside, "outside").unwrap();
        std::os::unix::fs::symlink(&outside, config_d.join("chctl-1-project-link.xml")).unwrap();
        std::fs::create_dir(config_d.join("chctl-0-global-dir")).unwrap();
        std::fs::write(config_d.join("chctl-0-global-stale.xml"), "stale").unwrap();

        layers.apply(None);

        assert_eq!(
            staged_names(&layers.data),
            [
                "chctl-0-global-dir",
                "chctl-2-other.xml",
                "chctl-udf.xml",
                "user.xml",
                MANAGED_CONFIG_FILE,
            ]
        );
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "outside");
        assert_eq!(
            std::fs::read_to_string(config_d.join("user.xml")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn a_symlinked_config_d_is_refused_without_touching_its_target() {
        let layers = Layers::new();
        let elsewhere = layers.data.join("../elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("chctl-0-global-x.xml"), "theirs").unwrap();
        std::os::unix::fs::symlink(&elsewhere, layers.data.join("config.d")).unwrap();
        std::fs::write(layers.global.join("x.xml"), "ours").unwrap();

        let error = apply_config_overlay(
            &layers.data,
            ConfigLayers {
                global_dir: &layers.global,
                project_dir: &layers.project,
                named: None,
            },
        )
        .unwrap_err();

        assert!(matches!(error, Error::Io(_)), "{error:?}");
        assert_eq!(
            std::fs::read_to_string(elsewhere.join("chctl-0-global-x.xml")).unwrap(),
            "theirs"
        );
    }

    #[test]
    fn startup_errors_name_each_staged_file_and_its_source() {
        let staged = vec![StagedConfig {
            staged: "chctl-1-project-bad.xml".to_string(),
            source: PathBuf::from("/app/clickhouse/config.d/bad.xml"),
        }];
        let exit = Error::StartupExit {
            kind: crate::error::StartupKind::ClickHouse,
            name: "dev".to_string(),
            details: "exited".to_string(),
        };
        let timeout = Error::StartupTimeout {
            kind: crate::error::StartupKind::ClickHouse,
            name: "dev".to_string(),
            seconds: 30,
            details: "timed out".to_string(),
        };
        for error in [exit, timeout] {
            let details = match with_staged_files(error, &staged) {
                Error::StartupExit { details, .. } | Error::StartupTimeout { details, .. } => {
                    details
                }
                other => panic!("unexpected {other:?}"),
            };
            assert!(details.contains("chctl-1-project-bad.xml"), "{details}");
            assert!(
                details.contains("/app/clickhouse/config.d/bad.xml"),
                "{details}"
            );
        }

        // Nothing staged, or another error, is unchanged.
        let plain = with_staged_files(
            Error::StartupExit {
                kind: crate::error::StartupKind::ClickHouse,
                name: "dev".to_string(),
                details: "exited".to_string(),
            },
            &[],
        );
        assert!(matches!(plain, Error::StartupExit { ref details, .. } if details == "exited"));
        assert!(matches!(
            with_staged_files(Error::NoDefaultVersion, &staged),
            Error::NoDefaultVersion
        ));
    }

    #[test]
    fn project_config_d_is_in_the_init_scaffold() {
        let (parent, leaf) = PROJECT_CONFIG_D.split_once('/').unwrap();
        assert_eq!(parent, "clickhouse");
        assert!(crate::init::CLICKHOUSE_SCAFFOLD.contains(&leaf));
    }

    #[test]
    fn managed_config_sets_path_and_ports() {
        let tmp = tempfile::tempdir().unwrap();
        write_managed_config(tmp.path(), 18123, 19000).unwrap();

        let staged =
            std::fs::read_to_string(tmp.path().join("config.d").join(MANAGED_CONFIG_FILE)).unwrap();
        assert!(staged.contains("<path>./</path>"), "got: {staged}");
        assert_eq!(read_managed_ports(tmp.path()), (Some(18123), Some(19000)));
    }

    #[test]
    fn managed_config_rewrites_changed_ports() {
        let tmp = tempfile::tempdir().unwrap();
        write_managed_config(tmp.path(), 8123, 9000).unwrap();
        write_managed_config(tmp.path(), 8124, 9001).unwrap();
        assert_eq!(read_managed_ports(tmp.path()), (Some(8124), Some(9001)));
    }

    #[test]
    fn managed_config_unchanged_content_is_not_rewritten() {
        let tmp = tempfile::tempdir().unwrap();
        write_managed_config(tmp.path(), 8123, 9000).unwrap();
        let path = tmp.path().join("config.d").join(MANAGED_CONFIG_FILE);
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        write_managed_config(tmp.path(), 8123, 9000).unwrap();

        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
    }

    #[test]
    fn managed_config_merges_after_other_chctl_files() {
        // ClickHouse merges config.d in byte order, later files winning.
        for other in ["chctl-config.xml", "chctl-config.yaml", "chctl-udf.xml"] {
            assert!(
                other < MANAGED_CONFIG_FILE,
                "{other} would merge after {MANAGED_CONFIG_FILE}"
            );
        }
        // Both always-applied layers merge before the named config and the
        // UDF overlay, whatever the source file is called.
        for layer in [GLOBAL_PREFIX, PROJECT_PREFIX] {
            let last = format!("{layer}zzzz.yml");
            for later in ["chctl-config.xml", "chctl-udf.xml"] {
                assert!(last.as_str() < later, "{last} would merge after {later}");
            }
        }
        assert!(format!("{GLOBAL_PREFIX}zzzz.yml") < format!("{PROJECT_PREFIX}0.xml"));
    }

    #[test]
    fn managed_ports_absent_without_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read_managed_ports(tmp.path()), (None, None));
    }

    #[test]
    fn managed_ports_ignore_unparseable_values() {
        let tmp = tempfile::tempdir().unwrap();
        let config_d = tmp.path().join("config.d");
        std::fs::create_dir(&config_d).unwrap();
        std::fs::write(
            config_d.join(MANAGED_CONFIG_FILE),
            "<clickhouse><http_port>abc</http_port><tcp_port>9000</tcp_port></clickhouse>",
        )
        .unwrap();
        assert_eq!(read_managed_ports(tmp.path()), (None, Some(9000)));
    }
}
