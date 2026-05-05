//! Docker integration for `local postgres`.
//!
//! All Docker work goes through the async Docker API — there is no shell-out
//! to the `docker` CLI anywhere in this crate, including for interactive
//! `psql` exec (which uses an attached exec stream + crossterm raw mode and
//! forwards SIGWINCH as `resize_exec`).
//!
//! Containers we create are tagged with these labels so we can later discover
//! them even if the local metadata file is missing:
//!
//!  * `clickhousectl.engine=postgres`
//!  * `clickhousectl.name=<server-name>`
//!  * `clickhousectl.project=<canonical project cwd>`
//!  * `created_by=clickhousectl_<crate-version>`

use crate::error::{Error, Result};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;

pub const LABEL_ENGINE: &str = "clickhousectl.engine";
pub const LABEL_NAME: &str = "clickhousectl.name";
pub const LABEL_PROJECT: &str = "clickhousectl.project";
pub const LABEL_CREATED_BY: &str = "created_by";

/// Value of the `created_by` label — `clickhousectl_<crate version>`.
pub fn created_by_value() -> String {
    format!("clickhousectl_{}", env!("CARGO_PKG_VERSION"))
}

/// Container name we use for a managed Postgres server.
pub fn container_name(server_name: &str) -> String {
    format!("clickhousectl-pg-{}", server_name)
}

/// Connect to the local Docker daemon and verify it's reachable.
pub async fn connect() -> Result<Docker> {
    let docker = Docker::connect_with_local_defaults().map_err(|e| {
        Error::DockerNotAvailable(format!("could not initialize docker client: {e}"))
    })?;
    docker.ping().await.map_err(|e| {
        Error::DockerNotAvailable(format!(
            "Docker daemon is not reachable ({e}). Install Docker Desktop or start the daemon."
        ))
    })?;
    Ok(docker)
}

/// Pull `postgres:<tag>`, streaming progress to stderr.
pub async fn pull_image(docker: &Docker, tag: &str) -> Result<()> {
    use bollard::query_parameters::CreateImageOptionsBuilder;
    let from = format!("postgres:{}", tag);
    let opts = CreateImageOptionsBuilder::default().from_image(&from).build();
    let mut stream = docker.create_image(Some(opts), None, None);
    while let Some(item) = stream.next().await {
        let info = item.map_err(|e| Error::DockerError(e.to_string()))?;
        if let Some(status) = info.status.as_deref() {
            eprintln!("  {}", status);
        }
    }
    Ok(())
}

/// Check whether `postgres:<tag>` is already present locally (no pull).
pub async fn image_exists(docker: &Docker, tag: &str) -> Result<bool> {
    use bollard::errors::Error as BErr;
    let name = format!("postgres:{}", tag);
    match docker.inspect_image(&name).await {
        Ok(_) => Ok(true),
        Err(BErr::DockerResponseServerError { status_code: 404, .. }) => Ok(false),
        Err(e) => Err(Error::DockerError(e.to_string())),
    }
}

pub struct PostgresRunOpts<'a> {
    pub server_name: &'a str,
    pub tag: &'a str,
    pub host_port: u16,
    pub data_dir: &'a std::path::Path,
    pub project_cwd: &'a str,
    pub user: &'a str,
    pub password: &'a str,
    pub database: &'a str,
    pub extra_env: Vec<String>,
}

/// Create + start a Postgres container; return its ID.
pub async fn run_postgres(docker: &Docker, opts: PostgresRunOpts<'_>) -> Result<String> {
    use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
    use bollard::query_parameters::{CreateContainerOptionsBuilder, StartContainerOptions};

    let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
    port_bindings.insert(
        "5432/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some(opts.host_port.to_string()),
        }]),
    );

    let canonical_data = opts
        .data_dir
        .canonicalize()
        .map_err(|e| Error::DockerError(format!("data dir canonicalize: {e}")))?;
    let bind = format!("{}:/var/lib/postgresql/data", canonical_data.display());

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        binds: Some(vec![bind]),
        ..Default::default()
    };

    let mut env: Vec<String> = vec![
        format!("POSTGRES_USER={}", opts.user),
        format!("POSTGRES_PASSWORD={}", opts.password),
        format!("POSTGRES_DB={}", opts.database),
    ];
    env.extend(opts.extra_env);

    let mut labels: HashMap<String, String> = HashMap::new();
    labels.insert(LABEL_ENGINE.into(), "postgres".into());
    labels.insert(LABEL_NAME.into(), opts.server_name.into());
    labels.insert(LABEL_PROJECT.into(), opts.project_cwd.into());
    labels.insert(LABEL_CREATED_BY.into(), created_by_value());

    let container_config = ContainerCreateBody {
        image: Some(format!("postgres:{}", opts.tag)),
        env: Some(env),
        host_config: Some(host_config),
        labels: Some(labels),
        ..Default::default()
    };

    let create_opts = CreateContainerOptionsBuilder::default()
        .name(&container_name(opts.server_name))
        .build();

    let created = docker
        .create_container(Some(create_opts), container_config)
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;

    docker
        .start_container(&created.id, None::<StartContainerOptions>)
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;

    Ok(created.id)
}

/// If a container with our managed name (`clickhousectl-pg-<name>`) exists in
/// any state, remove it — but only when it carries our labels for the current
/// project. Returns Ok(()) if the name is free or was successfully cleaned up,
/// or an actionable error if the name is held by an unrelated container.
pub async fn ensure_name_free(
    docker: &Docker,
    server_name: &str,
    project_cwd: &str,
) -> Result<()> {
    use bollard::errors::Error as BErr;
    let cname = container_name(server_name);
    match docker.inspect_container(&cname, None).await {
        Ok(info) => {
            let labels_match = info
                .config
                .as_ref()
                .and_then(|c| c.labels.as_ref())
                .map(|l| {
                    l.get(LABEL_ENGINE).map(String::as_str) == Some("postgres")
                        && l.get(LABEL_PROJECT).map(String::as_str) == Some(project_cwd)
                })
                .unwrap_or(false);
            if !labels_match {
                return Err(Error::DockerError(format!(
                    "container '{cname}' already exists but is not managed by clickhousectl. \
                     Remove it manually or pick a different --name."
                )));
            }
            let id = info.id.unwrap_or(cname.clone());
            let _ = stop_container(docker, &id).await;
            remove_container(docker, &id).await
        }
        Err(BErr::DockerResponseServerError { status_code: 404, .. }) => Ok(()),
        Err(e) => Err(Error::DockerError(e.to_string())),
    }
}

pub async fn is_container_running(docker: &Docker, id: &str) -> bool {
    match docker.inspect_container(id, None).await {
        Ok(resp) => resp.state.and_then(|s| s.running).unwrap_or(false),
        Err(_) => false,
    }
}

pub async fn stop_container(docker: &Docker, id: &str) -> Result<()> {
    use bollard::query_parameters::StopContainerOptionsBuilder;
    docker
        .stop_container(
            id,
            Some(StopContainerOptionsBuilder::default().t(10).build()),
        )
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    Ok(())
}

pub async fn remove_container(docker: &Docker, id: &str) -> Result<()> {
    use bollard::query_parameters::RemoveContainerOptionsBuilder;
    docker
        .remove_container(
            id,
            Some(RemoveContainerOptionsBuilder::default().force(true).build()),
        )
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    Ok(())
}

pub async fn container_logs_tail(docker: &Docker, id: &str, n: usize) -> Result<String> {
    use bollard::query_parameters::LogsOptionsBuilder;
    let opts = LogsOptionsBuilder::default()
        .stdout(true)
        .stderr(true)
        .tail(&n.to_string())
        .build();
    let mut stream = docker.logs(id, Some(opts));
    let mut buf = String::new();
    while let Some(line) = stream.next().await {
        let l = line.map_err(|e| Error::DockerError(e.to_string()))?;
        buf.push_str(&l.to_string());
    }
    Ok(buf)
}

pub struct DiscoveredContainer {
    pub container_id: String,
    pub server_name: String,
    pub image: String,
    pub host_port: Option<u16>,
}

/// Find Postgres containers we created in `project_cwd`. Filtered on the
/// engine + project labels — both unique to containers this CLI created — but
/// **not** on the version-stamped `created_by` label, so containers created by
/// older releases of the CLI remain manageable after upgrade.
pub async fn list_project_postgres(
    docker: &Docker,
    project_cwd: &str,
) -> Result<Vec<DiscoveredContainer>> {
    use bollard::query_parameters::ListContainersOptionsBuilder;

    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![
            format!("{}=postgres", LABEL_ENGINE),
            format!("{}={}", LABEL_PROJECT, project_cwd),
        ],
    );

    let opts = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let containers = docker
        .list_containers(Some(opts))
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;

    let mut out = Vec::new();
    for c in containers {
        let id = match c.id {
            Some(s) => s,
            None => continue,
        };
        let labels = c.labels.unwrap_or_default();
        let server_name = match labels.get(LABEL_NAME) {
            Some(s) => s.clone(),
            None => continue,
        };
        let image = c.image.unwrap_or_default();
        let host_port = c.ports.as_ref().and_then(|ports| {
            ports
                .iter()
                .find(|p| p.private_port == 5432)
                .and_then(|p| p.public_port)
        });
        out.push(DiscoveredContainer {
            container_id: id,
            server_name,
            image,
            host_port,
        });
    }
    Ok(out)
}

/// Run `psql` inside a container in non-interactive mode: no TTY, no raw
/// mode, no stdin. Streams stdout+stderr to the host. Used when the caller
/// passes `--query` or `--queries-file` so the output can be piped/scripted.
pub async fn exec_psql_one_shot(
    docker: &Docker,
    container_id: &str,
    psql_args: &[String],
) -> Result<()> {
    use bollard::exec::StartExecResults;
    use bollard::models::ExecConfig;
    use tokio::io::AsyncWriteExt;

    let mut cmd = vec!["psql".to_string()];
    cmd.extend(psql_args.iter().cloned());

    let exec = docker
        .create_exec(
            container_id,
            ExecConfig {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                attach_stdin: Some(false),
                tty: Some(false),
                cmd: Some(cmd),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    let exec_id = exec.id;

    let started = docker
        .start_exec(&exec_id, None)
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    let mut output = match started {
        StartExecResults::Attached { output, .. } => output,
        StartExecResults::Detached => return Ok(()),
    };

    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();
    while let Some(chunk) = output.next().await {
        match chunk {
            Ok(bollard::container::LogOutput::StdErr { message }) => {
                let _ = stderr.write_all(&message).await;
            }
            Ok(out) => {
                let _ = stdout.write_all(&out.into_bytes()).await;
            }
            Err(_) => break,
        }
    }
    let _ = stdout.flush().await;
    let _ = stderr.flush().await;

    if let Ok(info) = docker.inspect_exec(&exec_id).await
        && let Some(code) = info.exit_code
        && code != 0
    {
        std::process::exit(code as i32);
    }
    Ok(())
}

/// Run `psql` inside a container with a full interactive TTY:
/// host stdin/stdout are wired to the docker exec stream, the host terminal
/// goes into raw mode, and SIGWINCH is forwarded as `resize_exec`.
pub async fn exec_psql_in_container(
    docker: &Docker,
    container_id: &str,
    psql_args: &[String],
) -> Result<()> {
    use bollard::exec::StartExecResults;
    use bollard::models::ExecConfig;
    use bollard::query_parameters::ResizeExecOptionsBuilder;
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut cmd = vec!["psql".to_string()];
    cmd.extend(psql_args.iter().cloned());

    let exec = docker
        .create_exec(
            container_id,
            ExecConfig {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                attach_stdin: Some(true),
                tty: Some(true),
                cmd: Some(cmd),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    let exec_id = exec.id;

    let started = docker
        .start_exec(&exec_id, None)
        .await
        .map_err(|e| Error::DockerError(e.to_string()))?;
    let (mut output, mut input) = match started {
        StartExecResults::Attached { output, input } => (output, input),
        StartExecResults::Detached => return Ok(()),
    };

    // Initial size resize.
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        let _ = docker
            .resize_exec(
                &exec_id,
                ResizeExecOptionsBuilder::default()
                    .h(rows as i32)
                    .w(cols as i32)
                    .build(),
            )
            .await;
    }

    enable_raw_mode().map_err(|e| Error::DockerError(format!("raw mode: {e}")))?;
    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }
    let _guard = RawModeGuard;

    #[cfg(unix)]
    let resize_task = {
        use tokio::signal::unix::{signal, SignalKind};
        let docker_clone = docker.clone();
        let exec_id_clone = exec_id.clone();
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::window_change()) {
                while sig.recv().await.is_some() {
                    if let Ok((cols, rows)) = crossterm::terminal::size() {
                        let _ = docker_clone
                            .resize_exec(
                                &exec_id_clone,
                                ResizeExecOptionsBuilder::default()
                                    .h(rows as i32)
                                    .w(cols as i32)
                                    .build(),
                            )
                            .await;
                    }
                }
            }
        })
    };

    // Pump stdin into the exec stream.
    let stdin_task = tokio::spawn(async move {
        let mut stdin = tokio::io::stdin();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if input.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    let _ = input.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    // Pump exec output to stdout.
    let mut stdout = tokio::io::stdout();
    while let Some(chunk) = output.next().await {
        match chunk {
            Ok(out) => {
                let bytes = out.into_bytes();
                let _ = stdout.write_all(&bytes).await;
                let _ = stdout.flush().await;
            }
            Err(_) => break,
        }
    }

    stdin_task.abort();
    #[cfg(unix)]
    resize_task.abort();
    drop(_guard);

    if let Ok(info) = docker.inspect_exec(&exec_id).await
        && let Some(code) = info.exit_code
        && code != 0
    {
        std::process::exit(code as i32);
    }
    Ok(())
}

// ── Blocking shims (callable from sync `server.rs`) ─────────────────────────

/// Drive an async task to completion from sync code.
///
/// Inside a multi-thread tokio runtime (the CLI's default `#[tokio::main]`),
/// uses `block_in_place` so we don't deadlock the executor. Outside any
/// runtime (rare — basically only from non-tokio tests), spins up a fresh
/// runtime. Will panic if called inside a `current_thread` runtime; we don't
/// use one anywhere.
pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use tokio::runtime::Handle;
    match Handle::try_current() {
        Ok(h) => tokio::task::block_in_place(|| h.block_on(f)),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new()
                .expect("failed to build tokio runtime for docker blocking shim");
            rt.block_on(f)
        }
    }
}

pub fn is_container_running_blocking(id: &str) -> bool {
    let id = id.to_string();
    block_on(async move {
        let docker = match connect().await {
            Ok(d) => d,
            Err(_) => return false,
        };
        is_container_running(&docker, &id).await
    })
}

/// Stop a container, leaving it on disk so it can be `docker start`ed again.
pub fn stop_blocking(id: &str) -> Result<()> {
    let id = id.to_string();
    block_on(async move {
        let docker = connect().await?;
        stop_container(&docker, &id).await
    })
}

/// Best-effort stop, then remove the container.
pub fn stop_and_remove_blocking(id: &str) -> Result<()> {
    let id = id.to_string();
    block_on(async move {
        let docker = connect().await?;
        let _ = stop_container(&docker, &id).await;
        remove_container(&docker, &id).await
    })
}

/// `docker start` an existing stopped container.
pub fn start_existing_blocking(id: &str) -> Result<()> {
    use bollard::query_parameters::StartContainerOptions;
    let id = id.to_string();
    block_on(async move {
        let docker = connect().await?;
        docker
            .start_container(&id, None::<StartContainerOptions>)
            .await
            .map_err(|e| Error::DockerError(e.to_string()))
    })
}

/// Discover Postgres containers belonging to this project that don't yet have
/// a metadata file under `.clickhouse/servers/`, and write a `ServerInfo` for
/// each so they show up in `local server list` and can be managed.
///
/// Safe to call multiple times in one CLI invocation. When Docker isn't
/// reachable, `connect()` fails fast (no socket → immediate error, no I/O
/// timeout) and we return silently.
pub fn recover_project_postgres_blocking(project_cwd: &str) {
    use crate::local::server::{
        ensure_server_data_dir, save_server_info, server_meta_path_for_recovery, Engine,
        ServerInfo,
    };
    let cwd_owned = project_cwd.to_string();
    let _ = block_on(async move {
        let docker = match connect().await {
            Ok(d) => d,
            Err(_) => return Ok::<(), Error>(()),
        };
        let containers = match list_project_postgres(&docker, &cwd_owned).await {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };
        for c in containers {
            if server_meta_path_for_recovery(&c.server_name).exists() {
                continue;
            }
            let _ = ensure_server_data_dir(&c.server_name);
            let info = ServerInfo {
                name: c.server_name.clone(),
                pid: 0,
                version: c.image.clone(),
                http_port: 0,
                tcp_port: c.host_port.unwrap_or(0),
                started_at: "recovered".to_string(),
                cwd: cwd_owned.clone(),
                engine: Engine::Postgres,
                container_id: Some(c.container_id.clone()),
            };
            let _ = save_server_info(&info);
        }
        Ok(())
    });
}
