use crate::error::Result;
use std::io::Write;
use std::path::PathBuf;

pub fn local_dir() -> PathBuf {
    std::env::current_dir()
        .expect("failed to get current directory")
        .join(".clickhouse")
}

/// The physical directory whose project-local state is selected by this
/// invocation. Local commands intentionally do not search parent directories.
pub fn canonical_project_dir() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.canonicalize()?)
}

pub fn project_dir() -> PathBuf {
    std::env::current_dir()
        .expect("failed to get current directory")
        .join("clickhouse")
}

pub fn postgres_project_dir() -> PathBuf {
    std::env::current_dir()
        .expect("failed to get current directory")
        .join("postgres")
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
    let dir = local_dir();
    let directory_created = !dir.exists();
    std::fs::create_dir_all(&dir)?;

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

pub fn init() -> Result<InitResult> {
    let runtime_ignore = ensure_runtime_gitignore()?;

    let clickhouse_scaffold_created = create_project_scaffold(
        project_dir(),
        &["tables", "materialized_views", "queries", "seed"],
    )?;
    let postgres_scaffold_created = create_project_scaffold(
        postgres_project_dir(),
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

/// Returns CLI flags that point ClickHouse data into the current directory.
pub fn server_flags() -> Vec<String> {
    vec!["--".into(), "--path=./".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
