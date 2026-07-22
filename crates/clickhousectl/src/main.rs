mod cli;
mod cloud;
mod dotenv;
mod error;
mod http;
mod init;
mod local;
mod paths;
mod skills;
#[cfg(feature = "telemetry")]
mod telemetry;
mod update;
mod user_agent;
mod version_manager;

use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches};
use cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, Cli, ClickPipeCommands,
    ClickPipeCreateCommands, ClickPipeSettingsCommands, CloudArgs, CloudCommands, Commands,
    InvitationCommands, KeyCommands, MemberCommands, OrgCommands, PostgresCertsCommands,
    PostgresCommands, PostgresConfigCommands, PostgresReadReplicaCommands,
    PrivateEndpointCommands, QueryEndpointCommands, ServiceCommands, SkillsArgs, UpdateArgs,
};

use cloud::CloudClient;
use error::{Error, Result};

#[tokio::main]
async fn main() {
    // Snapshot any project-local `.env` before anything else so credential
    // resolution can use it. Safe to call here even though tokio has worker
    // threads — we populate an in-process `OnceLock` rather than touching
    // libc's environ.
    dotenv::init();

    // Parse via ArgMatches (rather than `Cli::try_parse()`) so the telemetry
    // capture below can read the command path and passed-flag *names* from the
    // clap definitions — argument values are never consulted.
    let mut cmd = Cli::command();
    let matches = match cmd.try_get_matches_from_mut(std::env::args_os()) {
        Ok(matches) => matches,
        Err(e) => {
            match e.kind() {
                // --version always hits the network to refresh the cache + timer,
                // then prints the notice from the freshly-updated cache.
                ErrorKind::DisplayVersion => {
                    e.print().expect("failed to print output");
                    update::force_refresh_update_cache().await;
                    update::print_cached_update_notice();
                    std::process::exit(0);
                }
                // --help shows the notice from cache (no blocking network call).
                ErrorKind::DisplayHelp => {
                    e.print().expect("failed to print output");
                    update::print_cached_update_notice();
                    std::process::exit(0);
                }
                // Parse errors exit here, before telemetry capture: a mistyped
                // invocation never produces an event.
                _ => e.exit(),
            }
        }
    };

    #[cfg(feature = "telemetry")]
    let telemetry_invocation = telemetry::capture(&cmd, &matches);

    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // The hidden send child does exactly one POST and exits: no update-cache
    // refresh, no dispatch, and no telemetry hook of its own (a send can never
    // trigger another send).
    #[cfg(feature = "telemetry")]
    if matches!(
        cli.command,
        Commands::Telemetry(cli::TelemetryArgs {
            command: cli::TelemetryCommands::Send
        })
    ) {
        telemetry::run_child_send().await;
        std::process::exit(0);
    }

    // Spawn a background task to refresh the update cache for non-update
    // commands. The refresh is gated to one network call per 24h; the notice
    // below is driven off whatever the cache currently holds.
    let is_update_cmd = matches!(cli.command, Commands::Update(_));
    let cache_refresh = if !is_update_cmd {
        Some(tokio::spawn(update::refresh_update_cache()))
    } else {
        None
    };

    // Decide whether to surface the update notice before `run` consumes the
    // command. Shown on every command that does not emit machine-readable JSON.
    let show_notice = should_show_update_notice(&cli.command);

    let result = run(cli.command).await;

    // Give the cache refresh a brief window to finish so short-lived commands
    // don't always drop it before the write completes. The background HTTP
    // request itself has a 400ms timeout, so 500ms here is enough headroom.
    if let Some(handle) = cache_refresh {
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    }

    let exit_code = match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            e.exit_code()
        }
    };

    // Always print the notice at the very end, after the command's own output
    // (stdout) and any error message.
    if show_notice {
        update::print_cached_update_notice();
    }

    // Consent is evaluated here, after the command ran, so `telemetry disable`
    // silences its own event and `telemetry enable` sends one.
    #[cfg(feature = "telemetry")]
    telemetry::finalize(telemetry_invocation, exit_code);

    std::process::exit(exit_code);
}

/// The explicit `--json` flag for a command, or `None` for commands that never
/// surface the update notice (the `update` command itself). `Skills` has no
/// `--json` flag, so it reports `false`. Kept separate from agent detection so
/// the mapping is deterministic and unit-testable regardless of environment.
fn command_json_flag(cmd: &Commands) -> Option<bool> {
    match cmd {
        Commands::Update(_) => None,
        Commands::Local(args) => Some(args.json),
        Commands::Cloud(args) => Some(args.json),
        Commands::Skills(_) => Some(false),
        #[cfg(feature = "telemetry")]
        Commands::Telemetry(_) => Some(false),
    }
}

/// Whether to surface the cached update notice for this invocation. Shown for
/// every command that does not emit machine-readable JSON (`--json` or a
/// detected coding agent both suppress it), except the `update` command itself.
fn should_show_update_notice(cmd: &Commands) -> bool {
    match command_json_flag(cmd) {
        None => false,
        Some(flag) => !json_output(flag),
    }
}

/// Resolve whether to emit machine-readable JSON. True when `--json` was passed
/// or we're running under a known coding agent (same detection as the outbound
/// User-Agent in `user_agent.rs`). Pipes/redirects stay human-readable unless
/// `--json` is passed, matching `gh`/`kubectl` norms.
fn json_output(flag: bool) -> bool {
    flag || is_ai_agent::detect().is_some()
}

async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Local(args) => local::run(args.command, json_output(args.json)).await,
        Commands::Skills(args) => run_skills(args).await,
        Commands::Cloud(args) => run_cloud(*args).await,
        Commands::Update(args) => run_update(args).await,
        #[cfg(feature = "telemetry")]
        Commands::Telemetry(args) => telemetry::run_command(args.command),
    }
}

async fn run_update(args: UpdateArgs) -> Result<()> {
    if args.check {
        match update::check_for_update().await? {
            Some((current, latest)) => {
                println!("Update available: v{} → v{}", current, latest);
                println!("Run `clickhousectl update` to upgrade.");
            }
            None => {
                println!("Already up to date (v{}).", env!("CARGO_PKG_VERSION"));
            }
        }
        Ok(())
    } else {
        update::perform_update().await
    }
}

async fn run_skills(args: SkillsArgs) -> Result<()> {
    skills::install(args).await
}

async fn run_cloud(args: CloudArgs) -> Result<()> {
    // Auth subcommands don't need a client
    if let CloudCommands::Auth { command } = args.command {
        return match command {
            AuthCommands::Login {
                interactive,
                api_key,
                api_secret,
            } => {
                if interactive {
                    // Interactive prompt for API key/secret
                    cloud::commands::auth_interactive().map_err(|e| Error::Cloud(e.to_string()))
                } else if api_key.is_some() || api_secret.is_some() {
                    // Non-interactive API key login
                    let key = api_key.ok_or_else(|| {
                        Error::AuthRequired(
                            "--api-key is required when --api-secret is provided".into(),
                        )
                    })?;
                    let secret = api_secret.ok_or_else(|| {
                        Error::AuthRequired(
                            "--api-secret is required when --api-key is provided".into(),
                        )
                    })?;
                    let mut creds = cloud::credentials::load_credentials().unwrap_or_default();
                    creds.api_key = Some(key);
                    creds.api_secret = Some(secret);
                    cloud::credentials::save_credentials(&creds)
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    println!(
                        "Credentials saved to {}",
                        cloud::credentials::credentials_path().display()
                    );
                    Ok(())
                } else {
                    // Default: OAuth device flow
                    let url = args
                        .url
                        .as_deref()
                        .unwrap_or("https://api.clickhouse.cloud");
                    let tokens = cloud::auth::device_auth_login(url)
                        .await
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    cloud::auth::save_tokens(&tokens).map_err(|e| Error::Cloud(e.to_string()))?;
                    println!("Logged in successfully.");
                    let tokens_path = cloud::auth::tokens_path()
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    println!("Tokens saved to {}", tokens_path.display());
                    Ok(())
                }
            }
            AuthCommands::Signup => {
                let api_url = args
                    .url
                    .as_deref()
                    .unwrap_or("https://api.clickhouse.cloud");
                let parsed = url::Url::parse(api_url)
                    .map_err(|e| Error::Cloud(format!("Invalid URL: {}", e)))?;
                let host = parsed.host_str().unwrap_or("api.clickhouse.cloud");
                let base_host = host.strip_prefix("api.").unwrap_or(host);
                let url = format!(
                    "https://console.{}/signUp?utm_source=clickhousectl",
                    base_host
                );
                println!("Opening ClickHouse Cloud sign-up page...");
                if open::that(&url).is_err() {
                    println!("Could not open browser. Please visit: {}", url);
                }
                Ok(())
            }
            AuthCommands::Logout { oauth, api_keys } => {
                match (oauth, api_keys) {
                    (true, false) => {
                        cloud::auth::clear_tokens();
                        println!("OAuth tokens cleared. API keys unchanged.");
                    }
                    (false, true) => {
                        cloud::credentials::clear_credentials();
                        println!("API keys cleared. OAuth tokens unchanged.");
                    }
                    _ => {
                        cloud::auth::clear_tokens();
                        cloud::credentials::clear_credentials();
                        println!("Logged out. All saved credentials cleared.");
                    }
                }
                Ok(())
            }
            AuthCommands::Status => {
                use serde::Serialize;
                use tabled::{Table, Tabled, settings::Style};

                #[derive(Serialize, Tabled)]
                struct AuthRow {
                    #[tabled(rename = "Type")]
                    #[serde(rename = "type")]
                    auth_type: String,
                    #[tabled(rename = "Status")]
                    status: String,
                    #[tabled(rename = "Scope")]
                    scope: String,
                    #[tabled(rename = "Active")]
                    active: String,
                }

                // Determine which source would actually win precedence right now.
                // CLI --api-key/--api-secret aren't relevant to `auth status` itself.
                let active = cloud::resolve_active_auth_source();
                let mark = |src: cloud::AuthSource| -> String {
                    if active == Some(src) { "yes".into() } else { "-".into() }
                };

                let mut rows = Vec::new();

                match cloud::auth::load_tokens() {
                    Some(tokens) if cloud::auth::is_token_valid(&tokens) => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Active".into(),
                            scope: "read-only".into(),
                            active: mark(cloud::AuthSource::OAuthTokens),
                        });
                    }
                    Some(_) => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Expired".into(),
                            scope: "read-only".into(),
                            active: "-".into(),
                        });
                    }
                    None => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Not configured".into(),
                            scope: "-".into(),
                            active: "-".into(),
                        });
                    }
                }

                if cloud::credentials::load_credentials().is_some() {
                    rows.push(AuthRow {
                        auth_type: "API key".into(),
                        status: "Active".into(),
                        scope: "read/write".into(),
                        active: mark(cloud::AuthSource::CredentialsFile),
                    });
                } else {
                    rows.push(AuthRow {
                        auth_type: "API key".into(),
                        status: "Not configured".into(),
                        scope: "-".into(),
                        active: "-".into(),
                    });
                }

                // Presence is computed through the same `env_or_dotenv` merge
                // the resolver uses (shell env with `.env` fallback, empties
                // treated as absent) so this table can't disagree with which
                // source actually wins.
                let env_creds = cloud::env_cred_presence();
                let has_key = env_creds.key;
                let has_secret = env_creds.secret;

                match (has_key, has_secret) {
                    (true, true) => {
                        // Only label the `.env` path when BOTH credentials
                        // come exclusively from it — otherwise the status
                        // would imply the file was the source even though
                        // one value is actually exported in the shell. Use
                        // the same rule as `dotenv_env_provenance()` so the
                        // table and `--debug describe()` stay consistent.
                        let status = match cloud::dotenv_env_provenance() {
                            Some(path) => format!("Active (from {})", path.display()),
                            None => "Active".into(),
                        };
                        rows.push(AuthRow {
                            auth_type: "Env vars".into(),
                            status,
                            scope: "read/write".into(),
                            active: mark(cloud::AuthSource::EnvVars),
                        });
                    }
                    (true, false) => {
                        rows.push(AuthRow {
                            auth_type: "Env vars".into(),
                            status: "Incomplete (missing CLICKHOUSE_CLOUD_API_SECRET)".into(),
                            scope: "-".into(),
                            active: "-".into(),
                        });
                    }
                    (false, true) => {
                        rows.push(AuthRow {
                            auth_type: "Env vars".into(),
                            status: "Incomplete (missing CLICKHOUSE_CLOUD_API_KEY)".into(),
                            scope: "-".into(),
                            active: "-".into(),
                        });
                    }
                    (false, false) => {
                        rows.push(AuthRow {
                            auth_type: "Env vars".into(),
                            status: "Not configured".into(),
                            scope: "-".into(),
                            active: "-".into(),
                        });
                    }
                }

                if args.debug {
                    match active {
                        Some(src) => {
                            eprintln!("[debug] auth source: {}", src.describe());
                        }
                        None => eprintln!("[debug] auth source: none (no credentials configured)"),
                    }
                }

                if json_output(args.json) {
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                } else {
                    println!("{}", Table::new(rows).with(Style::markdown()));
                }
                Ok(())
            }
        };
    }

    // Refresh OAuth tokens if needed. Errors here are filesystem failures
    // (refresh-rpc failures are swallowed and tokens cleared), so this stays
    // a generic error rather than `AuthRequired`.
    cloud::auth::ensure_fresh_tokens()
        .await
        .map_err(|e| Error::Cloud(e.to_string()))?;

    let client = CloudClient::new(
        args.api_key.as_deref(),
        args.api_secret.as_deref(),
        args.url.as_deref(),
    )
    .map_err(cloud_error_to_top_level)?;

    if args.debug {
        eprintln!("[debug] auth source: {}", client.auth_source().describe());
        eprintln!("[debug] api url: {}", client.base_url());
    }

    // OAuth (Bearer) tokens are read-only. Block write commands early
    // to avoid fail loops where agents repeatedly hit 403 errors.
    if client.is_bearer_auth() && args.command.is_write_command() {
        return Err(Error::AuthRequired(
            "This command requires API key authentication. \
             OAuth (browser login) provides read-only access.\n\n\
             To authenticate with an API key:\n  \
             clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET\n\n\
             Or set environment variables:\n  \
             export CLICKHOUSE_CLOUD_API_KEY=your-key\n  \
             export CLICKHOUSE_CLOUD_API_SECRET=your-secret\n\n\
             Learn how to create API keys:\n  \
             https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl"
                .into(),
        ));
    }

    let json = json_output(args.json);

    let result = match args.command {
        CloudCommands::Auth { .. } => unreachable!("handled above"),
        CloudCommands::Org { command } => match command {
            OrgCommands::List => cloud::commands::org_list(&client, json).await,
            OrgCommands::Get { org_id } => cloud::commands::org_get(&client, &org_id, json).await,
            OrgCommands::Update {
                org_id,
                name,
                remove_private_endpoint,
                enable_core_dumps,
            } => {
                let opts = cloud::commands::OrgUpdateOptions {
                    name,
                    remove_private_endpoints: remove_private_endpoint,
                    enable_core_dumps,
                };
                cloud::commands::org_update(&client, &org_id, opts, json).await
            }
            OrgCommands::Prometheus {
                org_id,
                filtered_metrics,
            } => cloud::commands::org_prometheus(&client, &org_id, filtered_metrics, json).await,
            OrgCommands::Usage {
                org_id,
                from_date,
                to_date,
                filter,
            } => {
                cloud::commands::org_usage(&client, &org_id, &from_date, &to_date, &filter, json)
                    .await
            }
        },
        CloudCommands::Service { command } => match command {
            ServiceCommands::List { org_id, filter } => {
                cloud::commands::service_list(&client, org_id.as_deref(), &filter, json).await
            }
            ServiceCommands::Get { service_id, org_id } => {
                cloud::commands::service_get(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Create {
                name,
                provider,
                region,
                min_replica_memory_gb,
                max_replica_memory_gb,
                num_replicas,
                min_replicas,
                max_replicas,
                autoscaling_mode,
                idle_scaling,
                idle_timeout_minutes,
                ip_allow,
                backup_id,
                release_channel,
                data_warehouse_id,
                readonly,
                encryption_key,
                encryption_role,
                enable_tde,
                compliance_type,
                profile,
                tag,
                enable_endpoint,
                disable_endpoint,
                private_preview_terms_checked,
                enable_core_dumps,
                org_id,
            } => {
                let opts = cloud::commands::CreateServiceOptions {
                    name,
                    provider,
                    region,
                    min_replica_memory_gb,
                    max_replica_memory_gb,
                    num_replicas,
                    min_replicas,
                    max_replicas,
                    autoscaling_mode,
                    idle_scaling,
                    idle_timeout_minutes,
                    ip_allow,
                    backup_id,
                    release_channel,
                    data_warehouse_id,
                    is_readonly: readonly,
                    encryption_key,
                    encryption_role,
                    enable_tde,
                    compliance_type,
                    profile,
                    tags: tag,
                    enable_endpoints: enable_endpoint,
                    disable_endpoints: disable_endpoint,
                    private_preview_terms_checked,
                    enable_core_dumps,
                    org_id,
                };
                cloud::commands::service_create(&client, opts, json).await
            }
            ServiceCommands::Delete {
                service_id,
                force,
                org_id,
            } => {
                cloud::commands::service_delete(
                    &client,
                    &service_id,
                    force,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ServiceCommands::Start { service_id, org_id } => {
                cloud::commands::service_start(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Stop { service_id, org_id } => {
                cloud::commands::service_stop(&client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Update {
                service_id,
                name,
                add_ip_allow,
                remove_ip_allow,
                add_private_endpoint_id,
                remove_private_endpoint_id,
                release_channel,
                enable_endpoint,
                disable_endpoint,
                transparent_data_encryption_key_id,
                add_tag,
                remove_tag,
                enable_core_dumps,
                org_id,
            } => {
                let opts = cloud::commands::ServiceUpdateOptions {
                    name,
                    add_ip_allow,
                    remove_ip_allow,
                    add_private_endpoint_ids: add_private_endpoint_id,
                    remove_private_endpoint_ids: remove_private_endpoint_id,
                    release_channel,
                    enable_endpoints: enable_endpoint,
                    disable_endpoints: disable_endpoint,
                    transparent_data_encryption_key_id,
                    add_tags: add_tag,
                    remove_tags: remove_tag,
                    enable_core_dumps,
                    org_id,
                };
                cloud::commands::service_update(&client, &service_id, opts, json).await
            }
            ServiceCommands::Scale {
                service_id,
                min_replica_memory_gb,
                max_replica_memory_gb,
                num_replicas,
                min_replicas,
                max_replicas,
                autoscaling_mode,
                idle_scaling,
                idle_timeout_minutes,
                org_id,
            } => {
                cloud::commands::service_scale(
                    &client,
                    &service_id,
                    cloud::commands::ServiceScaleOptions {
                        min_replica_memory_gb,
                        max_replica_memory_gb,
                        num_replicas,
                        min_replicas,
                        max_replicas,
                        autoscaling_mode,
                        idle_scaling,
                        idle_timeout_minutes,
                        org_id,
                    },
                    json,
                )
                .await
            }
            ServiceCommands::ResetPassword {
                service_id,
                new_password_hash,
                new_double_sha1_hash,
                org_id,
            } => {
                let opts = cloud::commands::ServiceResetPasswordOptions {
                    new_password_hash,
                    new_double_sha1_hash,
                    org_id,
                };
                cloud::commands::service_reset_password(&client, &service_id, opts, json).await
            }
            ServiceCommands::QueryEndpoint { command } => match command {
                QueryEndpointCommands::Get { service_id, org_id } => {
                    cloud::commands::query_endpoint_get(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                QueryEndpointCommands::Create {
                    service_id,
                    role,
                    open_api_key,
                    allowed_origins,
                    org_id,
                } => {
                    let opts = cloud::commands::QueryEndpointCreateOptions {
                        roles: role,
                        open_api_keys: open_api_key,
                        allowed_origins,
                        org_id,
                    };
                    cloud::commands::query_endpoint_create(&client, &service_id, opts, json).await
                }
                QueryEndpointCommands::Delete { service_id, org_id } => {
                    cloud::commands::query_endpoint_delete(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ServiceCommands::PrivateEndpoint { command } => match command {
                PrivateEndpointCommands::Create {
                    service_id,
                    endpoint_id,
                    description,
                    org_id,
                } => {
                    cloud::commands::private_endpoint_create(
                        &client,
                        &service_id,
                        &endpoint_id,
                        description.as_deref(),
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                PrivateEndpointCommands::GetConfig { service_id, org_id } => {
                    cloud::commands::private_endpoint_get_config(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ServiceCommands::BackupConfig { command } => match command {
                BackupConfigCommands::Get { service_id, org_id } => {
                    cloud::commands::backup_config_get(
                        &client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                BackupConfigCommands::Update {
                    service_id,
                    backup_period_hours,
                    backup_retention_period_hours,
                    backup_start_time,
                    org_id,
                } => {
                    let opts = cloud::commands::BackupConfigUpdateOptions {
                        backup_period_hours,
                        backup_retention_period_hours,
                        backup_start_time,
                        org_id,
                    };
                    cloud::commands::backup_config_update(&client, &service_id, opts, json).await
                }
            },
            ServiceCommands::Prometheus {
                service_id,
                org_id,
                filtered_metrics,
            } => {
                cloud::commands::service_prometheus(
                    &client,
                    &service_id,
                    org_id.as_deref(),
                    filtered_metrics,
                )
                .await
            }
            ServiceCommands::Query {
                name,
                id,
                query,
                queries_file,
                database,
                format,
                org_id,
                no_auto_enable,
            } => {
                let opts = cloud::commands::ServiceQueryOptions {
                    name,
                    id,
                    query,
                    queries_file,
                    database,
                    format,
                    org_id,
                    no_auto_enable,
                };
                cloud::commands::service_query(&client, opts).await
            }
        },
        CloudCommands::Member { command } => match command {
            MemberCommands::List { org_id } => {
                cloud::commands::member_list(&client, org_id.as_deref(), json).await
            }
            MemberCommands::Get { user_id, org_id } => {
                cloud::commands::member_get(&client, &user_id, org_id.as_deref(), json).await
            }
            MemberCommands::Update {
                user_id,
                role_id,
                org_id,
            } => {
                cloud::commands::member_update(&client, &user_id, &role_id, org_id.as_deref(), json)
                    .await
            }
            MemberCommands::Remove { user_id, org_id } => {
                cloud::commands::member_remove(&client, &user_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Invitation { command } => match command {
            InvitationCommands::List { org_id } => {
                cloud::commands::invitation_list(&client, org_id.as_deref(), json).await
            }
            InvitationCommands::Create {
                email,
                role_id,
                org_id,
            } => {
                cloud::commands::invitation_create(
                    &client,
                    &email,
                    &role_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            InvitationCommands::Get {
                invitation_id,
                org_id,
            } => {
                cloud::commands::invitation_get(&client, &invitation_id, org_id.as_deref(), json)
                    .await
            }
            InvitationCommands::Delete {
                invitation_id,
                org_id,
            } => {
                cloud::commands::invitation_delete(&client, &invitation_id, org_id.as_deref(), json)
                    .await
            }
        },
        CloudCommands::Key { command } => match command {
            KeyCommands::List { org_id } => {
                cloud::commands::key_list(&client, org_id.as_deref(), json).await
            }
            KeyCommands::Create {
                name,
                role_id,
                expires_at,
                state,
                ip_allow,
                hash_key_id,
                hash_key_id_suffix,
                hash_key_secret,
                org_id,
            } => {
                let opts = cloud::commands::KeyCreateOptions {
                    name,
                    role_ids: role_id,
                    expires_at,
                    state,
                    ip_allow,
                    hash_key_id,
                    hash_key_id_suffix,
                    hash_key_secret,
                    org_id,
                };
                cloud::commands::key_create(&client, opts, json).await
            }
            KeyCommands::Get { key_id, org_id } => {
                cloud::commands::key_get(&client, &key_id, org_id.as_deref(), json).await
            }
            KeyCommands::Update {
                key_id,
                name,
                role_id,
                expires_at,
                state,
                ip_allow,
                org_id,
            } => {
                let opts = cloud::commands::KeyUpdateOptions {
                    name,
                    role_ids: role_id,
                    expires_at,
                    state,
                    ip_allow,
                    org_id,
                };
                cloud::commands::key_update(&client, &key_id, opts, json).await
            }
            KeyCommands::Delete { key_id, org_id } => {
                cloud::commands::key_delete(&client, &key_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Activity { command } => match command {
            ActivityCommands::List {
                org_id,
                from_date,
                to_date,
            } => {
                cloud::commands::activity_list(
                    &client,
                    org_id.as_deref(),
                    from_date.as_deref(),
                    to_date.as_deref(),
                    json,
                )
                .await
            }
            ActivityCommands::Get {
                activity_id,
                org_id,
            } => {
                cloud::commands::activity_get(&client, &activity_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Backup { command } => match command {
            BackupCommands::List { service_id, org_id } => {
                cloud::commands::backup_list(&client, &service_id, org_id.as_deref(), json).await
            }
            BackupCommands::Get {
                service_id,
                backup_id,
                org_id,
            } => {
                cloud::commands::backup_get(
                    &client,
                    &service_id,
                    &backup_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
        },
        CloudCommands::Postgres { command } => run_postgres(&client, command, json).await,
        CloudCommands::ClickPipe { command } => match *command {
            ClickPipeCommands::List { service_id, org_id } => {
                cloud::commands::clickpipe_list(&client, &service_id, org_id.as_deref(), json).await
            }
            ClickPipeCommands::Get {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                cloud::commands::clickpipe_get(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Delete {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                cloud::commands::clickpipe_delete(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Start {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                cloud::commands::clickpipe_state(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    "start",
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Stop {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                cloud::commands::clickpipe_state(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    "stop",
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Resync {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                cloud::commands::clickpipe_state(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    "resync",
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Scale {
                service_id,
                clickpipe_id,
                replicas,
                cpu_millicores,
                memory_gb,
                org_id,
            } => {
                cloud::commands::clickpipe_scale(
                    &client,
                    &service_id,
                    &clickpipe_id,
                    replicas,
                    cpu_millicores,
                    memory_gb,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Settings { command } => match command {
                ClickPipeSettingsCommands::Get {
                    service_id,
                    clickpipe_id,
                    org_id,
                } => {
                    cloud::commands::clickpipe_settings_get(
                        &client,
                        &service_id,
                        &clickpipe_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeSettingsCommands::Update {
                    service_id,
                    clickpipe_id,
                    streaming_max_insert_wait_ms,
                    object_storage_concurrency,
                    object_storage_polling_interval_ms,
                    object_storage_max_insert_bytes,
                    object_storage_max_file_count,
                    clickhouse_max_threads,
                    clickhouse_max_insert_threads,
                    object_storage_use_cluster_function,
                    clickhouse_parallel_view_processing,
                    org_id,
                } => {
                    cloud::commands::clickpipe_settings_update(
                        &client,
                        &service_id,
                        &clickpipe_id,
                        streaming_max_insert_wait_ms,
                        object_storage_concurrency,
                        object_storage_polling_interval_ms,
                        object_storage_max_insert_bytes,
                        object_storage_max_file_count,
                        clickhouse_max_threads,
                        clickhouse_max_insert_threads,
                        object_storage_use_cluster_function,
                        clickhouse_parallel_view_processing,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ClickPipeCommands::SchemaDiscover {
                service_id,
                command,
                org_id,
            } => {
                cloud::commands::clickpipe_schema_discover(
                    &client,
                    &service_id,
                    &command,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Create { command } => match command {
                ClickPipeCreateCommands::ObjectStorage(args) => {
                    cloud::commands::clickpipe_create_s3(&client, &args, json).await
                }
                ClickPipeCreateCommands::Kafka(args) => {
                    cloud::commands::clickpipe_create_kafka(&client, &args, json).await
                }
                ClickPipeCreateCommands::Kinesis(args) => {
                    cloud::commands::clickpipe_create_kinesis(&client, &args, json).await
                }
                ClickPipeCreateCommands::Postgres(args) => {
                    cloud::commands::clickpipe_create_postgres(&client, &args, json).await
                }
                ClickPipeCreateCommands::MySQL(args) => {
                    cloud::commands::clickpipe_create_mysql(&client, &args, json).await
                }
                ClickPipeCreateCommands::MongoDB(args) => {
                    cloud::commands::clickpipe_create_mongodb(&client, &args, json).await
                }
                ClickPipeCreateCommands::BigQuery(args) => {
                    cloud::commands::clickpipe_create_bigquery(&client, &args, json).await
                }
            },
        },
    };

    result.map_err(boxed_cloud_error_to_top_level)
}

fn cloud_error_to_top_level(e: cloud::CloudError) -> Error {
    match e.kind {
        cloud::CloudErrorKind::Auth => Error::AuthRequired(e.message),
        cloud::CloudErrorKind::Generic => Error::Cloud(e.message),
    }
}

// Cloud command fns return `Box<dyn std::error::Error>`, so the `CloudError.kind`
// only survives via downcast — without it, auth-flagged errors silently fall back
// to `Error::Cloud` (exit 1) instead of `Error::AuthRequired` (exit 4).
fn boxed_cloud_error_to_top_level(e: Box<dyn std::error::Error>) -> Error {
    match e.downcast::<cloud::CloudError>() {
        Ok(ce) => cloud_error_to_top_level(*ce),
        Err(other) => Error::Cloud(other.to_string()),
    }
}

async fn run_postgres(
    client: &CloudClient,
    command: PostgresCommands,
    json: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::PostgresServiceSetStateCommand;
    use cloud::postgres::{
        self as pg, PostgresCreateOptions, PostgresReadReplicaOptions, PostgresRestoreOptions,
        PostgresUpdateOptions,
    };

    match command {
        PostgresCommands::List { org_id, filter } => {
            pg::postgres_list(client, org_id.as_deref(), &filter, json).await
        }
        PostgresCommands::Get {
            postgres_id,
            org_id,
        } => pg::postgres_get(client, &postgres_id, org_id.as_deref(), json).await,
        PostgresCommands::Create {
            name,
            region,
            size,
            provider,
            pg_version,
            ha_type,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
            org_id,
        } => {
            let opts = PostgresCreateOptions {
                name: &name,
                region: &region,
                size: &size,
                provider: &provider,
                pg_version: pg_version.as_deref(),
                ha_type: ha_type.as_deref(),
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                org_id: org_id.as_deref(),
            };
            pg::postgres_create(client, opts, json).await
        }
        PostgresCommands::Update {
            postgres_id,
            size,
            ha_type,
            add_tag,
            remove_tag,
            org_id,
        } => {
            let opts = PostgresUpdateOptions {
                size: size.as_deref(),
                ha_type: ha_type.as_deref(),
                add_tag: &add_tag,
                remove_tag: &remove_tag,
                org_id: org_id.as_deref(),
            };
            pg::postgres_update(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Delete {
            postgres_id,
            org_id,
        } => pg::postgres_delete(client, &postgres_id, org_id.as_deref(), json).await,
        PostgresCommands::Certs(PostgresCertsCommands::Get {
            postgres_id,
            output,
            org_id,
        }) => {
            pg::postgres_certs_get(
                client,
                &postgres_id,
                output.as_deref(),
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Config(PostgresConfigCommands::Get {
            postgres_id,
            org_id,
        }) => pg::postgres_config_get(client, &postgres_id, org_id.as_deref(), json).await,
        PostgresCommands::Config(PostgresConfigCommands::Replace {
            postgres_id,
            file,
            org_id,
        }) => {
            pg::postgres_config_replace(client, &postgres_id, &file, org_id.as_deref(), json).await
        }
        PostgresCommands::Config(PostgresConfigCommands::Patch {
            postgres_id,
            sets,
            file,
            org_id,
        }) => {
            pg::postgres_config_patch(
                client,
                &postgres_id,
                &sets,
                file.as_deref(),
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::ResetPassword {
            postgres_id,
            password,
            generate,
            org_id,
        } => {
            pg::postgres_reset_password(
                client,
                &postgres_id,
                password.as_deref(),
                generate,
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::ReadReplica(PostgresReadReplicaCommands::Create {
            postgres_id,
            name,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
            org_id,
        }) => {
            let opts = PostgresReadReplicaOptions {
                name: &name,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                org_id: org_id.as_deref(),
            };
            pg::postgres_read_replica_create(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Restore {
            postgres_id,
            name,
            restore_target,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
            org_id,
        } => {
            let opts = PostgresRestoreOptions {
                name: &name,
                restore_target: &restore_target,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                org_id: org_id.as_deref(),
            };
            pg::postgres_restore(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Restart {
            postgres_id,
            org_id,
        } => {
            pg::postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Restart,
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Promote {
            postgres_id,
            org_id,
        } => {
            pg::postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Promote,
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Switchover {
            postgres_id,
            org_id,
        } => {
            pg::postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Switchover,
                org_id.as_deref(),
                json,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use cloud::{CloudError, CloudErrorKind};

    #[test]
    fn json_output_true_when_flag_set() {
        assert!(json_output(true));
    }

    fn parse(args: &[&str]) -> Commands {
        Cli::try_parse_from(args).unwrap().command
    }

    #[test]
    fn command_json_flag_tracks_each_command() {
        // Human-readable commands report an explicit `false` flag.
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "cloud", "service", "list"])),
            Some(false)
        );
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "local", "list"])),
            Some(false)
        );
        // --json is picked up on both cloud and local (global flag).
        assert_eq!(
            command_json_flag(&parse(&[
                "clickhousectl",
                "cloud",
                "--json",
                "service",
                "list"
            ])),
            Some(true)
        );
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "local", "--json", "list"])),
            Some(true)
        );
        // Skills has no --json flag, so it always reports `false`.
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "skills"])),
            Some(false)
        );
        // The update command never surfaces the notice.
        assert_eq!(command_json_flag(&parse(&["clickhousectl", "update"])), None);
        // Telemetry management commands are human-readable output.
        #[cfg(feature = "telemetry")]
        assert_eq!(
            command_json_flag(&parse(&["clickhousectl", "telemetry", "status"])),
            Some(false)
        );
    }

    #[test]
    fn update_notice_suppressed_for_json_and_update() {
        // --json suppresses the notice so machine output stays clean,
        // regardless of agent detection.
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "cloud",
            "--json",
            "service",
            "list"
        ])));
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "local",
            "--json",
            "list"
        ])));
        // The update command never nags about itself.
        assert!(!should_show_update_notice(&parse(&[
            "clickhousectl",
            "update"
        ])));
    }

    #[test]
    fn cloud_error_kind_routes_to_top_level() {
        assert!(matches!(
            cloud_error_to_top_level(CloudError::auth("nope")),
            Error::AuthRequired(_)
        ));
        assert!(matches!(
            cloud_error_to_top_level(CloudError::new("boom")),
            Error::Cloud(_)
        ));
        // Default kind is Generic.
        assert_eq!(CloudError::new("x").kind, CloudErrorKind::Generic);
    }

    #[test]
    fn boxed_cloud_error_preserves_auth_kind_through_downcast() {
        let boxed: Box<dyn std::error::Error> = Box::new(CloudError::auth("nope"));
        assert!(matches!(
            boxed_cloud_error_to_top_level(boxed),
            Error::AuthRequired(_)
        ));
    }

    #[test]
    fn boxed_non_cloud_error_falls_back_to_generic() {
        // Anything that isn't a CloudError must not downcast to AuthRequired —
        // it falls back to Error::Cloud (exit 1). This pins the contract that a
        // handler stringifying a CloudError before boxing silently loses exit 4.
        let boxed: Box<dyn std::error::Error> = "plain string error".into();
        assert!(matches!(
            boxed_cloud_error_to_top_level(boxed),
            Error::Cloud(_)
        ));
    }
}
