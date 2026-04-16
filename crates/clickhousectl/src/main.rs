mod cli;
mod cloud;
mod error;
mod init;
mod local;
mod paths;
mod skills;
mod update;
mod user_agent;
mod version_manager;

use clap::Parser;
use cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, Cli,
    ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSettingsCommands, CloudArgs,
    CloudCommands, Commands, InvitationCommands, KeyCommands, MemberCommands, OrgCommands,
    PrivateEndpointCommands, QueryEndpointCommands, ServiceCommands, SkillsArgs, UpdateArgs,
};
use clap::error::ErrorKind;

use cloud::CloudClient;
use error::{Error, Result};

#[tokio::main]
async fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // For --help and --version, show the update notice after the output
            if e.kind() == ErrorKind::DisplayHelp || e.kind() == ErrorKind::DisplayVersion {
                e.print().expect("failed to print output");
                update::print_cached_update_notice();
                std::process::exit(0);
            }
            e.exit();
        }
    };

    // Spawn a background task to refresh the update cache for non-update commands.
    // The notice itself is only shown on --help (above), not during normal execution.
    let is_update_cmd = matches!(cli.command, Commands::Update(_));
    let cache_refresh = if !is_update_cmd {
        Some(tokio::spawn(update::refresh_update_cache()))
    } else {
        None
    };

    let result = run(cli.command).await;

    // Give the cache refresh a brief window to finish so short-lived commands
    // don't always drop it before the write completes. The background HTTP
    // request itself has a 400ms timeout, so 500ms here is enough headroom.
    if let Some(handle) = cache_refresh {
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
    }

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Local(args) => local::run(args.command, args.json).await,
        Commands::Skills(args) => run_skills(args).await,
        Commands::Cloud(args) => run_cloud(*args).await,
        Commands::Update(args) => run_update(args).await,
    }
}

async fn run_update(args: UpdateArgs) -> Result<()> {
    if args.check {
        match update::check_for_update().await? {
            Some((current, latest)) => {
                println!(
                    "Update available: v{} → v{}",
                    current, latest
                );
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
                        Error::Cloud("--api-key is required when --api-secret is provided".into())
                    })?;
                    let secret = api_secret.ok_or_else(|| {
                        Error::Cloud("--api-secret is required when --api-key is provided".into())
                    })?;
                    let creds = cloud::credentials::Credentials {
                        api_key: key,
                        api_secret: secret,
                    };
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
                    println!("Tokens saved to {}", cloud::auth::tokens_path().display());
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
                let url = format!("https://console.{}/signUp?utm_source=clickhousectl", base_host);
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
                }

                let mut rows = Vec::new();

                match cloud::auth::load_tokens() {
                    Some(tokens) if cloud::auth::is_token_valid(&tokens) => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Active".into(),
                            scope: "read-only".into(),
                        });
                    }
                    Some(_) => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Expired".into(),
                            scope: "read-only".into(),
                        });
                    }
                    None => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Not configured".into(),
                            scope: "-".into(),
                        });
                    }
                }

                if cloud::credentials::load_credentials().is_some() {
                    rows.push(AuthRow {
                        auth_type: "API key".into(),
                        status: "Active".into(),
                        scope: "read/write".into(),
                    });
                } else {
                    rows.push(AuthRow {
                        auth_type: "API key".into(),
                        status: "Not configured".into(),
                        scope: "-".into(),
                    });
                }

                if args.json {
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                } else {
                    println!("{}", Table::new(rows).with(Style::rounded()));
                }
                Ok(())
            }
        };
    }

    // Refresh OAuth tokens if needed before creating the client
    cloud::auth::ensure_fresh_tokens()
        .await
        .map_err(|e| Error::Cloud(e.to_string()))?;

    let client = CloudClient::new(
        args.api_key.as_deref(),
        args.api_secret.as_deref(),
        args.url.as_deref(),
    )
    .map_err(|e| Error::Cloud(e.to_string()))?;

    // OAuth (Bearer) tokens are read-only. Block write commands early
    // to avoid fail loops where agents repeatedly hit 403 errors.
    if client.is_bearer_auth() && args.command.is_write_command() {
        return Err(Error::Cloud(
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

    let json = args.json;

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
            ServiceCommands::Client {
                name,
                id,
                query,
                queries_file,
                user,
                password,
                allow_mismatched_client_version,
                generate_password,
                org_id,
                args,
            } => {
                let opts = cloud::commands::ServiceClientOptions {
                    name,
                    id,
                    query,
                    queries_file,
                    user,
                    password,
                    allow_mismatched_client_version,
                    generate_password,
                    org_id,
                    args,
                };
                cloud::commands::service_client(&client, opts).await
            }
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
                cloud::commands::invitation_delete(
                    &client,
                    &invitation_id,
                    org_id.as_deref(),
                    json,
                )
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
        CloudCommands::ClickPipe { command } => match command {
            ClickPipeCommands::List { service_id, org_id } => {
                cloud::commands::clickpipe_list(&client, &service_id, org_id.as_deref(), json)
                    .await
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
                    let request = cloud::types::ClickPipeSettingsRequest {
                        streaming_max_insert_wait_ms,
                        object_storage_concurrency,
                        object_storage_polling_interval_ms,
                        object_storage_max_insert_bytes,
                        object_storage_max_file_count,
                        clickhouse_max_threads,
                        clickhouse_max_insert_threads,
                        object_storage_use_cluster_function,
                        clickhouse_parallel_view_processing,
                    };
                    cloud::commands::clickpipe_settings_update(
                        &client,
                        &service_id,
                        &clickpipe_id,
                        &request,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            }
            ClickPipeCommands::Create { command } => match command {
                ClickPipeCreateCommands::ObjectStorage {
                    service_id,
                    name,
                    source_url,
                    format,
                    database,
                    table,
                    columns,
                    storage_type,
                    compression,
                    continuous,
                    queue_url,
                    delimiter,
                    iam_role,
                    access_key_id,
                    secret_key,
                    connection_string,
                    azure_container_name,
                    path,
                    service_account_key,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_s3(
                        &client,
                        &service_id,
                        &name,
                        &source_url,
                        &format,
                        &database,
                        &table,
                        &columns,
                        &storage_type,
                        &compression,
                        continuous,
                        queue_url.as_deref(),
                        delimiter.as_deref(),
                        iam_role.as_deref(),
                        access_key_id.as_deref(),
                        secret_key.as_deref(),
                        connection_string.as_deref(),
                        azure_container_name.as_deref(),
                        path.as_deref(),
                        service_account_key.as_deref(),
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::Kafka {
                    service_id,
                    name,
                    brokers,
                    topics,
                    format,
                    database,
                    table,
                    columns,
                    kafka_type,
                    consumer_group,
                    auth,
                    username,
                    password,
                    iam_role,
                    access_key_id,
                    secret_key,
                    offset,
                    offset_timestamp,
                    schema_registry_url,
                    schema_registry_username,
                    schema_registry_password,
                    ca_certificate,
                    client_certificate,
                    client_key,
                    schema_registry_ca_certificate,
                    reverse_private_endpoint_ids,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_kafka(
                        &client,
                        &service_id,
                        &name,
                        &brokers,
                        &topics,
                        &format,
                        &database,
                        &table,
                        &columns,
                        &kafka_type,
                        consumer_group.as_deref(),
                        auth.as_deref(),
                        username.as_deref(),
                        password.as_deref(),
                        iam_role.as_deref(),
                        access_key_id.as_deref(),
                        secret_key.as_deref(),
                        &offset,
                        offset_timestamp.as_deref(),
                        schema_registry_url.as_deref(),
                        schema_registry_username.as_deref(),
                        schema_registry_password.as_deref(),
                        ca_certificate.as_deref(),
                        client_certificate.as_deref(),
                        client_key.as_deref(),
                        schema_registry_ca_certificate.as_deref(),
                        &reverse_private_endpoint_ids,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::Kinesis {
                    service_id,
                    name,
                    stream_name,
                    region,
                    format,
                    database,
                    table,
                    columns,
                    auth,
                    iam_role,
                    access_key_id,
                    secret_key,
                    iterator_type,
                    iterator_timestamp,
                    enhanced_fan_out,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_kinesis(
                        &client,
                        &service_id,
                        &name,
                        &stream_name,
                        &region,
                        &format,
                        &database,
                        &table,
                        &columns,
                        &auth,
                        iam_role.as_deref(),
                        access_key_id.as_deref(),
                        secret_key.as_deref(),
                        &iterator_type,
                        iterator_timestamp,
                        enhanced_fan_out,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::Postgres {
                    service_id,
                    name,
                    host,
                    port,
                    pg_database,
                    username,
                    password,
                    table_mappings,
                    postgres_type,
                    replication_mode,
                    auth,
                    iam_role,
                    tls_host,
                    ca_certificate,
                    publication_name,
                    replication_slot_name,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_postgres(
                        &client,
                        &service_id,
                        &name,
                        &host,
                        port,
                        &pg_database,
                        &username,
                        &password,
                        &table_mappings,
                        &postgres_type,
                        &replication_mode,
                        &auth,
                        iam_role.as_deref(),
                        tls_host.as_deref(),
                        ca_certificate.as_deref(),
                        publication_name.as_deref(),
                        replication_slot_name.as_deref(),
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::MySQL {
                    service_id,
                    name,
                    host,
                    port,
                    username,
                    password,
                    table_mappings,
                    mysql_type,
                    replication_mode,
                    replication_mechanism,
                    auth,
                    iam_role,
                    tls_host,
                    ca_certificate,
                    disable_tls,
                    skip_cert_verification,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_mysql(
                        &client,
                        &service_id,
                        &name,
                        &host,
                        port,
                        &username,
                        &password,
                        &table_mappings,
                        &mysql_type,
                        &replication_mode,
                        &replication_mechanism,
                        &auth,
                        iam_role.as_deref(),
                        tls_host.as_deref(),
                        ca_certificate.as_deref(),
                        disable_tls,
                        skip_cert_verification,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::MongoDB {
                    service_id,
                    name,
                    uri,
                    username,
                    password,
                    table_mappings,
                    replication_mode,
                    read_preference,
                    tls_host,
                    ca_certificate,
                    disable_tls,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_mongodb(
                        &client,
                        &service_id,
                        &name,
                        &uri,
                        &username,
                        &password,
                        &table_mappings,
                        &replication_mode,
                        &read_preference,
                        tls_host.as_deref(),
                        ca_certificate.as_deref(),
                        disable_tls,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                ClickPipeCreateCommands::BigQuery {
                    service_id,
                    name,
                    service_account_file,
                    staging_path,
                    table_mappings,
                    org_id,
                } => {
                    cloud::commands::clickpipe_create_bigquery(
                        &client,
                        &service_id,
                        &name,
                        &service_account_file,
                        &staging_path,
                        &table_mappings,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            }
        },
    };

    result.map_err(|e| Error::Cloud(e.to_string()))
}
