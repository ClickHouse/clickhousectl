use crate::error::Result;
use std::path::PathBuf;

pub fn local_dir() -> PathBuf {
    std::env::current_dir()
        .expect("failed to get current directory")
        .join(".clickhouse")
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

pub fn is_initialized() -> bool {
    local_dir().exists()
}

pub fn init() -> Result<()> {
    let dir = local_dir();

    if is_initialized() {
        eprintln!("Already initialized at {}", dir.display());
    } else {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(".gitignore"), "*\n")?;
        eprintln!("Initialized ClickHouse project in {}", dir.display());
    }

    create_project_scaffold(
        project_dir(),
        &["tables", "materialized_views", "queries", "seed"],
    )?;
    create_project_scaffold(
        postgres_project_dir(),
        &["tables", "materialized_views", "queries", "seed"],
    )?;

    Ok(())
}

fn create_project_scaffold(dir: PathBuf, subdirs: &[&str]) -> Result<()> {
    let mut created = false;
    for subdir in subdirs {
        let path = dir.join(subdir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            std::fs::write(path.join(".gitkeep"), "")?;
            created = true;
        }
    }

    if created {
        eprintln!(
            "Created project scaffold in {}/ ({})",
            dir.display(),
            subdirs.join(", ")
        );
    }

    Ok(())
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

        create_project_scaffold(dir.clone(), &subdirs).unwrap();

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

        create_project_scaffold(dir.clone(), &subdirs).unwrap();
        // Running again over an existing scaffold must not error.
        create_project_scaffold(dir.clone(), &subdirs).unwrap();

        assert!(dir.join("tables").join(".gitkeep").is_file());
    }
}
