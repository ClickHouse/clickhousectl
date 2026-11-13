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
}

/// The project this invocation acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    /// Canonical directory containing (or that would contain) `.clickhouse/`.
    pub dir: PathBuf,
    pub kind: ProjectRootKind,
}

impl ProjectRoot {
    pub fn is_marked(&self) -> bool {
        self.kind == ProjectRootKind::Marked
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

static PROJECT_ROOT: OnceLock<ProjectRoot> = OnceLock::new();

/// The project root of this invocation, resolved on first use and then fixed
/// for the rest of the process, so a concurrent `local init` in an ancestor
/// cannot move a command's state half-way through.
pub fn project_root() -> std::io::Result<&'static ProjectRoot> {
    if let Some(root) = PROJECT_ROOT.get() {
        return Ok(root);
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let home = dirs::home_dir().and_then(|home| home.canonicalize().ok());
    let root = discover_project_root(&cwd, home.as_deref(), owned_by_current_user);
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
/// ancestor project was found.
pub fn enclosing_project() -> Result<Option<PathBuf>> {
    let root = project_root()?;
    let cwd = std::env::current_dir()?.canonicalize()?;
    Ok((root.is_marked() && root.dir != cwd).then(|| root.dir.clone()))
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

/// Initialize the exact current directory, like `git init`: inside another
/// project this creates a nested one, which then wins for this directory.
pub fn init() -> Result<InitResult> {
    let cwd = std::env::current_dir()?;
    let runtime_ignore = ensure_runtime_gitignore_in(&cwd.join(PROJECT_STATE_DIR))?;

    let clickhouse_scaffold_created =
        create_project_scaffold(cwd.join("clickhouse"), &CLICKHOUSE_SCAFFOLD)?;
    let postgres_scaffold_created = create_project_scaffold(
        cwd.join("postgres"),
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
