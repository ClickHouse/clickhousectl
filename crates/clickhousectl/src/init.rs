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

    create_project_scaffold()?;

    Ok(())
}

fn create_project_scaffold() -> Result<()> {
    let dir = project_dir();
    let subdirs = ["tables", "materialized_views", "queries", "seed"];

    let mut created = false;
    for subdir in &subdirs {
        let path = dir.join(subdir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            std::fs::write(path.join(".gitkeep"), "")?;
            created = true;
        }
    }

    if created {
        eprintln!(
            "Created project scaffold in {}/ (tables, materialized_views, queries, seed)",
            dir.display()
        );
    }

    Ok(())
}

/// Returns CLI flags that point ClickHouse data into the current directory.
pub fn server_flags() -> Vec<String> {
    vec!["--".into(), "--path=./".into()]
}

/// Returns CLI flags that enable the `system.query_log` table.
///
/// ClickHouse's embedded config (used when no `config.xml` is supplied) is
/// minimal and does not configure `system.query_log`. Package installs ship a
/// config that does — these flags mirror that behavior so `system.query_log`
/// is queryable on a managed local server. `flush_interval_milliseconds` is
/// lowered from the package default (7500) so dev-loop queries show up
/// quickly.
pub fn query_log_flags() -> Vec<String> {
    vec![
        "--query_log.database=system".into(),
        "--query_log.table=query_log".into(),
        "--query_log.flush_interval_milliseconds=1000".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_log_flags_configure_system_table() {
        let flags = query_log_flags();
        assert!(flags.iter().any(|f| f == "--query_log.database=system"));
        assert!(flags.iter().any(|f| f == "--query_log.table=query_log"));
        assert!(
            flags
                .iter()
                .any(|f| f.starts_with("--query_log.flush_interval_milliseconds="))
        );
    }
}
