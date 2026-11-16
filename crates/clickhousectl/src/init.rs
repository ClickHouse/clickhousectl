use crate::error::Result;
use std::fs::Metadata;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The directory that marks a project root and holds its runtime state.
pub const PROJECT_STATE_DIR: &str = ".clickhouse";

/// Whether a `.clickhouse/` marker selected the project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRootKind {
    /// The root holds a `.clickhouse/` directory: the current directory's own,
    /// or the nearest qualifying one in an ancestor.
    Marked,
    /// No marker was found, so the current directory stands in as the root.
    /// Read-only commands report an empty project; commands that create state
    /// create `.clickhouse/` here.
    Unmarked,
    /// [`PROJECT_DIR_ENV`] named the root, whether or not it holds
    /// `.clickhouse/` yet. Discovery does not run, and changing directory
    /// never selects another project.
    Override,
}

/// Names the project root explicitly, replacing discovery (#250).
pub const PROJECT_DIR_ENV: &str = "CLICKHOUSECTL_PROJECT_DIR";

/// Why [`PROJECT_DIR_ENV`] was refused. Carried inside the `io::Error` that
/// [`project_root`] returns, so every caller's `?` keeps working and
/// `From<io::Error> for Error` turns it back into its own variant.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{PROJECT_DIR_ENV}={value} {reason}; set it to an existing project directory or unset it")]
pub struct ProjectDirOverrideError {
    /// The variable's value as given.
    pub value: String,
    pub reason: ProjectDirOverrideReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectDirOverrideReason {
    /// The path does not exist or cannot be resolved.
    Missing,
    /// The path exists but is not a directory.
    NotADirectory,
    /// The path is the home directory, whose `.clickhouse/` is the global one.
    HomeDirectory,
}

impl std::fmt::Display for ProjectDirOverrideReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Missing => "does not exist",
            Self::NotADirectory => "is not a directory",
            Self::HomeDirectory => {
                "is the home directory, whose .clickhouse/ is global, not a project"
            }
        })
    }
}

/// The project this invocation acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    /// Canonical directory containing (or that would contain) `.clickhouse/`.
    pub dir: PathBuf,
    pub kind: ProjectRootKind,
}

impl ProjectRoot {
    /// A project was found or named: a `.clickhouse/` marker or the
    /// [`PROJECT_DIR_ENV`] override. False only when the current directory
    /// stands in because nothing was found, the one case where changing
    /// directory can select a project.
    pub fn is_marked(&self) -> bool {
        self.kind != ProjectRootKind::Unmarked
    }

    /// `<root>/.clickhouse`, which may not exist yet.
    pub fn state_dir(&self) -> PathBuf {
        self.dir.join(PROJECT_STATE_DIR)
    }
}

/// Find the project root for `cwd`, the git way: the nearest directory holding
/// `.clickhouse/`.
///
/// `cwd`'s own `.clickhouse` directory always wins, including when `cwd` is the
/// home directory. Ancestors are searched nearest first, but only below `home`:
/// `~/.clickhouse` is the global directory (installed versions, tokens,
/// configs), never a project for the directories beneath it, and nothing above
/// home is considered. An ancestor's marker counts only when it is a real
/// directory (not a symlink) for which `owned` holds, so a `.clickhouse/`
/// another user planted in a shared parent such as `/tmp` is never adopted.
/// Without a match the root is `cwd`, unmarked.
///
/// `cwd` and `home` must be canonical for the home comparison to hold.
pub fn discover_project_root(
    cwd: &Path,
    home: Option<&Path>,
    owned: impl Fn(&Path, &Metadata) -> bool,
) -> ProjectRoot {
    let marked = |dir: &Path| ProjectRoot {
        dir: dir.to_path_buf(),
        kind: ProjectRootKind::Marked,
    };
    if cwd.join(PROJECT_STATE_DIR).is_dir() {
        return marked(cwd);
    }
    if Some(cwd) != home {
        for ancestor in cwd.ancestors().skip(1) {
            if Some(ancestor) == home {
                break;
            }
            let marker = ancestor.join(PROJECT_STATE_DIR);
            if let Ok(metadata) = std::fs::symlink_metadata(&marker)
                && metadata.is_dir()
                && owned(&marker, &metadata)
            {
                return marked(ancestor);
            }
        }
    }
    ProjectRoot {
        dir: cwd.to_path_buf(),
        kind: ProjectRootKind::Unmarked,
    }
}

/// Owned by the effective user, the only owner whose marker is trusted.
fn owned_by_current_user(_path: &Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid has no preconditions and cannot fail.
    metadata.uid() == unsafe { libc::geteuid() }
}

/// The root [`PROJECT_DIR_ENV`] names, or `None` when it is unset or empty.
///
/// A relative value is resolved against `cwd`, then canonicalized. It must be
/// an existing directory other than `home` (canonical), whose `.clickhouse/`
/// is the global directory. Unlike an ancestor marker in discovery, the
/// directory is not checked for ownership or symlinks: it was named
/// explicitly, as the current directory is, rather than adopted silently.
pub fn override_project_root(
    value: Option<&std::ffi::OsStr>,
    cwd: &Path,
    home: Option<&Path>,
) -> std::io::Result<Option<ProjectRoot>> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let refuse = |reason| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            ProjectDirOverrideError {
                value: value.to_string_lossy().into_owned(),
                reason,
            },
        )
    };
    let dir = cwd
        .join(value)
        .canonicalize()
        .map_err(|_| refuse(ProjectDirOverrideReason::Missing))?;
    if !dir.is_dir() {
        return Err(refuse(ProjectDirOverrideReason::NotADirectory));
    }
    if Some(dir.as_path()) == home {
        return Err(refuse(ProjectDirOverrideReason::HomeDirectory));
    }
    Ok(Some(ProjectRoot {
        dir,
        kind: ProjectRootKind::Override,
    }))
}

/// The override when [`PROJECT_DIR_ENV`] is set, else discovery from `cwd`.
fn resolve_project_root(
    value: Option<&std::ffi::OsStr>,
    cwd: &Path,
    home: Option<&Path>,
    owned: impl Fn(&Path, &Metadata) -> bool,
) -> std::io::Result<ProjectRoot> {
    match override_project_root(value, cwd, home)? {
        Some(root) => Ok(root),
        None => Ok(discover_project_root(cwd, home, owned)),
    }
}

static PROJECT_ROOT: OnceLock<ProjectRoot> = OnceLock::new();

/// The project root of this invocation, resolved on first use and then fixed
/// for the rest of the process, so a concurrent `local init` in an ancestor
/// cannot move a command's state half-way through. A refused
/// [`PROJECT_DIR_ENV`] is an error, not cached, on every call.
pub fn project_root() -> std::io::Result<&'static ProjectRoot> {
    if let Some(root) = PROJECT_ROOT.get() {
        return Ok(root);
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let home = dirs::home_dir().and_then(|home| home.canonicalize().ok());
    let root = resolve_project_root(
        std::env::var_os(PROJECT_DIR_ENV).as_deref(),
        &cwd,
        home.as_deref(),
        owned_by_current_user,
    )?;
    Ok(PROJECT_ROOT.get_or_init(|| root))
}

/// `<project root>/.clickhouse`: where this invocation's project state lives.
/// Fails when the current directory cannot be resolved, for example when it
/// was deleted underneath the process.
pub fn local_dir() -> std::io::Result<PathBuf> {
    Ok(project_root()?.state_dir())
}

/// The project that encloses the current directory when `local init` would
/// create a nested one here: the current directory has no `.clickhouse/` and an
/// ancestor project was found. Never with [`PROJECT_DIR_ENV`] set, since `init`
/// then initializes the named root.
pub fn enclosing_project() -> Result<Option<PathBuf>> {
    let root = project_root()?;
    let cwd = std::env::current_dir()?.canonicalize()?;
    Ok((root.kind == ProjectRootKind::Marked && root.dir != cwd).then(|| root.dir.clone()))
}

/// The directory `local init` initializes: the [`PROJECT_DIR_ENV`] root when
/// set, so later commands find what it created, else the exact current
/// directory.
pub fn init_dir() -> Result<PathBuf> {
    let root = project_root()?;
    if root.kind == ProjectRootKind::Override {
        return Ok(root.dir.clone());
    }
    Ok(std::env::current_dir()?)
}

/// Which project-local paths `init()` created during this invocation. The
/// caller renders this in both the human-readable and `--json` output, so
/// `init()` itself prints nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InitResult {
    pub clickhouse_dir_created: bool,
    pub runtime_gitignore_created: bool,
    pub clickhouse_scaffold_created: bool,
    pub postgres_scaffold_created: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeIgnoreResult {
    pub directory_created: bool,
    pub gitignore_created: bool,
}

/// Ensure project-local runtime state is ignored without replacing a custom
/// ignore file. The create-new write also preserves a file created by a
/// concurrent process, and every other I/O failure reaches the caller.
pub fn ensure_runtime_gitignore() -> Result<RuntimeIgnoreResult> {
    ensure_runtime_gitignore_in(&local_dir()?)
}

fn ensure_runtime_gitignore_in(dir: &Path) -> Result<RuntimeIgnoreResult> {
    let directory_created = !dir.exists();
    std::fs::create_dir_all(dir)?;

    let gitignore = dir.join(".gitignore");
    let gitignore_created = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&gitignore)
    {
        Ok(mut file) => {
            file.write_all(b"*\n")?;
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && gitignore.is_file() => {
            false
        }
        Err(error) => return Err(error.into()),
    };

    Ok(RuntimeIgnoreResult {
        directory_created,
        gitignore_created,
    })
}

/// Subdirectories `local init` creates under `clickhouse/`. `config.d` is the
/// project's always-applied server config layer.
pub const CLICKHOUSE_SCAFFOLD: [&str; 6] = [
    "tables",
    "materialized_views",
    "queries",
    "seed",
    "udfs",
    "config.d",
];

/// Initialize [`init_dir`], like `git init`: inside another project this
/// creates a nested one, which then wins for this directory.
pub fn init() -> Result<InitResult> {
    let dir = init_dir()?;
    let runtime_ignore = ensure_runtime_gitignore_in(&dir.join(PROJECT_STATE_DIR))?;

    let clickhouse_scaffold_created =
        create_project_scaffold(dir.join("clickhouse"), &CLICKHOUSE_SCAFFOLD)?;
    let postgres_scaffold_created = create_project_scaffold(
        dir.join("postgres"),
        &["tables", "views", "functions", "queries", "seed"],
    )?;

    Ok(InitResult {
        clickhouse_dir_created: runtime_ignore.directory_created,
        runtime_gitignore_created: runtime_ignore.gitignore_created,
        clickhouse_scaffold_created,
        postgres_scaffold_created,
    })
}

fn create_project_scaffold(dir: PathBuf, subdirs: &[&str]) -> Result<bool> {
    let mut created = false;
    for subdir in subdirs {
        let path = dir.join(subdir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            std::fs::write(path.join(".gitkeep"), "")?;
            created = true;
        }
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canonical temporary tree, so paths compare equal to discovered ones.
    struct Tree(tempfile::TempDir);

    impl Tree {
        fn new() -> Self {
            Self(tempfile::tempdir().unwrap())
        }

        fn path(&self, relative: &str) -> PathBuf {
            let root = self.0.path().canonicalize().unwrap();
            if relative.is_empty() {
                root
            } else {
                root.join(relative)
            }
        }

        /// Create `relative` and return its path.
        fn dir(&self, relative: &str) -> PathBuf {
            let path = self.path(relative);
            std::fs::create_dir_all(&path).unwrap();
            path
        }

        /// Mark `relative` as a project and return its path.
        fn mark(&self, relative: &str) -> PathBuf {
            let dir = self.dir(relative);
            std::fs::create_dir(dir.join(PROJECT_STATE_DIR)).unwrap();
            dir
        }

        /// A ceiling at the tree's root, so no test sees the real filesystem
        /// above it.
        fn ceiling(&self) -> PathBuf {
            self.path("")
        }
    }

    fn all_owned(_: &Path, _: &Metadata) -> bool {
        true
    }

    fn marked(dir: PathBuf) -> ProjectRoot {
        ProjectRoot {
            dir,
            kind: ProjectRootKind::Marked,
        }
    }

    fn unmarked(dir: PathBuf) -> ProjectRoot {
        ProjectRoot {
            dir,
            kind: ProjectRootKind::Unmarked,
        }
    }

    #[test]
    fn the_current_directory_marker_wins_over_an_ancestor() {
        let tree = Tree::new();
        tree.mark("app");
        let cwd = tree.mark("app/sub");
        assert_eq!(
            discover_project_root(&cwd, Some(&tree.ceiling()), all_owned),
            marked(cwd.clone())
        );
    }

    #[test]
    fn the_nearest_ancestor_project_wins() {
        let tree = Tree::new();
        tree.mark("app");
        let nested = tree.mark("app/nested");
        let cwd = tree.dir("app/nested/src/deep");
        assert_eq!(
            discover_project_root(&cwd, Some(&tree.ceiling()), all_owned),
            marked(nested)
        );
    }

    #[test]
    fn nothing_found_leaves_the_current_directory_unmarked() {
        let tree = Tree::new();
        let cwd = tree.dir("app/sub");
        assert_eq!(
            discover_project_root(&cwd, Some(&tree.ceiling()), all_owned),
            unmarked(cwd)
        );
    }

    #[test]
    fn the_home_directory_marker_is_global_not_a_project_for_subdirectories() {
        let tree = Tree::new();
        let home = tree.mark("home");
        let cwd = tree.dir("home/work/app");
        assert_eq!(
            discover_project_root(&cwd, Some(&home), all_owned),
            unmarked(cwd)
        );
    }

    #[test]
    fn nothing_above_the_home_directory_is_considered() {
        let tree = Tree::new();
        tree.mark("");
        let home = tree.dir("home");
        let cwd = tree.dir("home/work");
        assert_eq!(
            discover_project_root(&cwd, Some(&home), all_owned),
            unmarked(cwd)
        );
        // From home itself, too.
        assert_eq!(
            discover_project_root(&home, Some(&home), all_owned),
            unmarked(home.clone())
        );
    }

    #[test]
    fn the_home_directory_itself_can_be_the_current_project() {
        let tree = Tree::new();
        let home = tree.mark("home");
        assert_eq!(
            discover_project_root(&home, Some(&home), all_owned),
            marked(home.clone())
        );
    }

    #[test]
    fn a_project_below_home_is_found_from_its_subdirectories() {
        let tree = Tree::new();
        let home = tree.mark("home");
        let project = tree.mark("home/app");
        let cwd = tree.dir("home/app/src");
        assert_eq!(
            discover_project_root(&cwd, Some(&home), all_owned),
            marked(project)
        );
    }

    #[test]
    fn outside_home_the_walk_continues_past_it() {
        let tree = Tree::new();
        let project = tree.mark("srv/app");
        let home = tree.mark("home");
        let cwd = tree.dir("srv/app/sub");
        assert_eq!(
            discover_project_root(&cwd, Some(&home), all_owned),
            marked(project)
        );
    }

    #[test]
    fn an_ancestor_marker_owned_by_someone_else_is_skipped() {
        let tree = Tree::new();
        let owned_project = tree.mark("shared");
        let foreign = tree.mark("shared/foreign");
        let cwd = tree.dir("shared/foreign/sub");
        let foreign_marker = foreign.join(PROJECT_STATE_DIR);
        let owned = |marker: &Path, _: &Metadata| marker != foreign_marker;
        assert_eq!(
            discover_project_root(&cwd, Some(&tree.ceiling()), owned),
            marked(owned_project)
        );
    }

    #[test]
    fn a_symlinked_ancestor_marker_is_ignored() {
        let tree = Tree::new();
        let target = tree.dir("elsewhere/.clickhouse");
        let app = tree.dir("app");
        std::os::unix::fs::symlink(&target, app.join(PROJECT_STATE_DIR)).unwrap();
        let cwd = tree.dir("app/sub");
        assert_eq!(
            discover_project_root(&cwd, Some(&tree.ceiling()), all_owned),
            unmarked(cwd)
        );
    }

    fn overridden(dir: PathBuf) -> ProjectRoot {
        ProjectRoot {
            dir,
            kind: ProjectRootKind::Override,
        }
    }

    fn refusal(error: std::io::Error) -> ProjectDirOverrideReason {
        error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<ProjectDirOverrideError>())
            .expect("an override refusal")
            .reason
    }

    #[test]
    fn the_override_replaces_discovery_with_or_without_a_marker() {
        let tree = Tree::new();
        let home = tree.dir("home");
        tree.mark("app");
        let cwd = tree.dir("app/sub");
        let unmarked_target = tree.dir("elsewhere");
        let marked_target = tree.mark("other");
        for target in [unmarked_target, marked_target] {
            assert_eq!(
                resolve_project_root(Some(target.as_os_str()), &cwd, Some(&home), all_owned)
                    .unwrap(),
                overridden(target)
            );
        }
    }

    #[test]
    fn a_relative_override_resolves_against_the_current_directory() {
        let tree = Tree::new();
        let target = tree.dir("app");
        let cwd = tree.dir("app/sub/deep");
        assert_eq!(
            override_project_root(Some("../..".as_ref()), &cwd, None).unwrap(),
            Some(overridden(target))
        );
    }

    #[test]
    fn an_unset_or_empty_override_falls_back_to_discovery() {
        let tree = Tree::new();
        let project = tree.mark("app");
        let cwd = tree.dir("app/sub");
        for value in [None, Some("".as_ref())] {
            assert_eq!(
                resolve_project_root(value, &cwd, Some(&tree.ceiling()), all_owned).unwrap(),
                marked(project.clone())
            );
        }
    }

    #[test]
    fn a_missing_file_or_home_override_is_refused() {
        let tree = Tree::new();
        let home = tree.mark("home");
        let cwd = tree.dir("work");
        std::fs::write(tree.path("file"), "").unwrap();
        for (value, reason) in [
            (tree.path("missing"), ProjectDirOverrideReason::Missing),
            (tree.path("file"), ProjectDirOverrideReason::NotADirectory),
            (home.clone(), ProjectDirOverrideReason::HomeDirectory),
            // A spelling of home that only canonicalization reveals.
            (
                tree.path("work/../home/."),
                ProjectDirOverrideReason::HomeDirectory,
            ),
        ] {
            let error = override_project_root(Some(value.as_os_str()), &cwd, Some(&home))
                .expect_err("refused");
            assert_eq!(refusal(error), reason, "{}", value.display());
        }
    }

    #[test]
    fn scaffold_creates_subdirs_with_gitkeep() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("postgres");
        let subdirs = ["tables", "materialized_views", "queries", "seed"];

        let created = create_project_scaffold(dir.clone(), &subdirs).unwrap();
        assert!(created);

        for subdir in &subdirs {
            let path = dir.join(subdir);
            assert!(path.is_dir(), "{} should be a directory", path.display());
            assert!(
                path.join(".gitkeep").is_file(),
                "{}/.gitkeep should exist",
                path.display()
            );
        }
    }

    #[test]
    fn scaffold_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("clickhouse");
        let subdirs = ["tables", "queries"];

        assert!(create_project_scaffold(dir.clone(), &subdirs).unwrap());
        // Running again over an existing scaffold must not error, and must
        // report that nothing new was created.
        assert!(!create_project_scaffold(dir.clone(), &subdirs).unwrap());

        assert!(dir.join("tables").join(".gitkeep").is_file());
    }

    #[test]
    fn scaffold_repair_of_any_one_subdir_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("postgres");
        let subdirs = ["tables", "views", "functions"];
        assert!(create_project_scaffold(dir.clone(), &subdirs).unwrap());

        std::fs::remove_dir_all(dir.join("views")).unwrap();
        assert!(create_project_scaffold(dir.clone(), &subdirs).unwrap());
        assert!(dir.join("views").join(".gitkeep").is_file());
    }
}
