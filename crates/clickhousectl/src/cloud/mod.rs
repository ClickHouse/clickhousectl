pub mod auth;
pub mod cli;
pub mod client;
pub mod commands;
pub mod credentials;
pub mod output;
pub mod postgres;
pub mod service_query;
mod shared;
pub mod types;

#[cfg(test)]
mod types_test;

pub use client::{
    AuthSource, CloudClient, CloudError, CloudErrorKind, EnvCredPresence, dotenv_env_provenance,
    env_cred_presence, resolve_active_auth_source,
};

use crate::error::{Error, Result};
use cli::{
    ActivityCommands, AuthCommands, BackupCommands, BackupConfigCommands, ClickPipeCommands,
    ClickPipeCreateCommands, ClickPipeSettingsCommands, CloudArgs, CloudCommands,
    InvitationCommands, KeyCommands, MemberCommands, OrgCommands, PrivateEndpointCommands,
    QueryEndpointCommands, ServiceCommands,
};

/// Explain when a configured environment credential cannot participate in
/// authentication because a higher-precedence source won. Keep this notice on
/// stderr so it does not contaminate JSON output.
fn ignored_env_credentials_notice(
    active: AuthSource,
    env_creds: EnvCredPresence,
) -> Option<String> {
    let configured = match (env_creds.key, env_creds.secret) {
        (true, true) => {
            "CLICKHOUSE_CLOUD_API_KEY and CLICKHOUSE_CLOUD_API_SECRET are set but ignored"
        }
        (true, false) => "CLICKHOUSE_CLOUD_API_KEY is set but ignored",
        (false, true) => "CLICKHOUSE_CLOUD_API_SECRET is set but ignored",
        (false, false) => return None,
    };
    let winner = match active {
        AuthSource::CliFlags => "CLI flags",
        AuthSource::CredentialsFile => "credentials file",
        AuthSource::EnvVars | AuthSource::OAuthTokens => return None,
    };

    Some(format!("note: {configured}; using {winner} — see --debug"))
}

pub async fn run(args: CloudArgs, json: bool) -> Result<()> {
    // Auth subcommands don't need a client.
    if let CloudCommands::Auth { command } = args.command {
        return match command {
            AuthCommands::Login {
                interactive,
                api_key,
                api_secret,
            } => {
                if interactive {
                    commands::auth_interactive().map_err(|e| Error::Cloud(e.to_string()))
                } else if api_key.is_some() || api_secret.is_some() {
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
                    let mut creds = credentials::load_credentials().unwrap_or_default();
                    creds.api_key = Some(key);
                    creds.api_secret = Some(secret);
                    credentials::save_credentials(&creds)
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    println!(
                        "Credentials saved to {}",
                        credentials::credentials_path().display()
                    );
                    Ok(())
                } else {
                    let url = args
                        .url
                        .as_deref()
                        .unwrap_or("https://api.clickhouse.cloud");
                    let tokens = auth::device_auth_login(url)
                        .await
                        .map_err(|e| Error::Cloud(e.to_string()))?;
                    auth::save_tokens(&tokens).map_err(|e| Error::Cloud(e.to_string()))?;
                    println!("Logged in successfully.");
                    let tokens_path =
                        auth::tokens_path().map_err(|e| Error::Cloud(e.to_string()))?;
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
                        auth::clear_tokens();
                        println!("OAuth tokens cleared. API keys unchanged.");
                    }
                    (false, true) => {
                        credentials::clear_credentials();
                        println!("API keys cleared. OAuth tokens unchanged.");
                    }
                    _ => {
                        auth::clear_tokens();
                        credentials::clear_credentials();
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
                let active = resolve_active_auth_source();
                let mark = |src: AuthSource| -> String {
                    if active == Some(src) {
                        "yes".into()
                    } else {
                        "-".into()
                    }
                };

                let mut rows = Vec::new();

                match auth::load_tokens() {
                    Some(tokens) if auth::is_token_valid(&tokens) => {
                        rows.push(AuthRow {
                            auth_type: "OAuth".into(),
                            status: "Active".into(),
                            scope: "read-only".into(),
                            active: mark(AuthSource::OAuthTokens),
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

                if credentials::load_credentials().is_some() {
                    rows.push(AuthRow {
                        auth_type: "API key".into(),
                        status: "Active".into(),
                        scope: "read/write".into(),
                        active: mark(AuthSource::CredentialsFile),
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
                let env_creds = env_cred_presence();
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
                        let provenance = dotenv_env_provenance()
                            .map(|path| format!(" (from {})", path.display()))
                            .unwrap_or_default();
                        let status = match active {
                            Some(AuthSource::EnvVars) => format!("Active{provenance}"),
                            Some(AuthSource::CredentialsFile) => format!(
                                "Configured{provenance} (inactive, outranked by credentials file)"
                            ),
                            _ => format!("Configured{provenance} (inactive)"),
                        };
                        rows.push(AuthRow {
                            auth_type: "Env vars".into(),
                            status,
                            scope: "read/write".into(),
                            active: mark(AuthSource::EnvVars),
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
                        Some(src) => eprintln!("[debug] auth source: {}", src.describe()),
                        None => eprintln!("[debug] auth source: none (no credentials configured)"),
                    }
                }

                if json {
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
    auth::ensure_fresh_tokens()
        .await
        .map_err(|e| Error::Cloud(e.to_string()))?;

    let client = CloudClient::new(
        args.api_key.as_deref(),
        args.api_secret.as_deref(),
        args.url.as_deref(),
    )
    .map_err(cloud_error_to_top_level)?;

    if let Some(notice) = ignored_env_credentials_notice(client.auth_source(), env_cred_presence())
    {
        eprintln!("{notice}");
    }

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

    dispatch(&client, args.command, json)
        .await
        .map_err(boxed_cloud_error_to_top_level)
}

fn cloud_error_to_top_level(e: CloudError) -> Error {
    match e.kind {
        CloudErrorKind::Auth => Error::AuthRequired(e.message),
        CloudErrorKind::Generic => Error::Cloud(e.message),
    }
}

// Cloud command fns return `Box<dyn std::error::Error>`, so the `CloudError.kind`
// only survives via downcast — without it, auth-flagged errors silently fall back
// to `Error::Cloud` (exit 1) instead of `Error::AuthRequired` (exit 4).
fn boxed_cloud_error_to_top_level(e: Box<dyn std::error::Error>) -> Error {
    match e.downcast::<CloudError>() {
        Ok(ce) => cloud_error_to_top_level(*ce),
        Err(other) => Error::Cloud(other.to_string()),
    }
}

async fn dispatch(
    client: &CloudClient,
    command: CloudCommands,
    json: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match command {
        CloudCommands::Auth { .. } => unreachable!("handled above"),
        CloudCommands::Org { command } => match command {
            OrgCommands::List => commands::org_list(client, json).await,
            OrgCommands::Get { org_id } => commands::org_get(client, &org_id, json).await,
            OrgCommands::Update {
                org_id,
                name,
                remove_private_endpoint,
                enable_core_dumps,
            } => {
                let opts = commands::OrgUpdateOptions {
                    name,
                    remove_private_endpoints: remove_private_endpoint,
                    enable_core_dumps,
                };
                commands::org_update(client, &org_id, opts, json).await
            }
            OrgCommands::Prometheus {
                org_id,
                legacy_org_id,
                filtered_metrics,
            } => {
                let org_id = org_id.as_deref().or(legacy_org_id.as_deref());
                commands::org_prometheus(client, org_id, filtered_metrics, json).await
            }
            OrgCommands::Usage {
                org_id,
                legacy_org_id,
                from_date,
                to_date,
                filter,
            } => {
                let org_id = org_id.as_deref().or(legacy_org_id.as_deref());
                commands::org_usage(client, org_id, &from_date, &to_date, &filter, json).await
            }
        },
        CloudCommands::Service { command } => match command {
            ServiceCommands::List { org_id, filter } => {
                commands::service_list(client, org_id.as_deref(), &filter, json).await
            }
            ServiceCommands::Get { service_id, org_id } => {
                commands::service_get(client, &service_id, org_id.as_deref(), json).await
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
                let opts = commands::CreateServiceOptions {
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
                commands::service_create(client, opts, json).await
            }
            ServiceCommands::Delete {
                service_id,
                force,
                org_id,
            } => {
                commands::service_delete(client, &service_id, force, org_id.as_deref(), json).await
            }
            ServiceCommands::Start { service_id, org_id } => {
                commands::service_start(client, &service_id, org_id.as_deref(), json).await
            }
            ServiceCommands::Stop { service_id, org_id } => {
                commands::service_stop(client, &service_id, org_id.as_deref(), json).await
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
                let opts = commands::ServiceUpdateOptions {
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
                commands::service_update(client, &service_id, opts, json).await
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
                commands::service_scale(
                    client,
                    &service_id,
                    commands::ServiceScaleOptions {
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
                let opts = commands::ServiceResetPasswordOptions {
                    new_password_hash,
                    new_double_sha1_hash,
                    org_id,
                };
                commands::service_reset_password(client, &service_id, opts, json).await
            }
            ServiceCommands::QueryEndpoint { command } => match command {
                QueryEndpointCommands::Get { service_id, org_id } => {
                    commands::query_endpoint_get(client, &service_id, org_id.as_deref(), json).await
                }
                QueryEndpointCommands::Create {
                    service_id,
                    role,
                    open_api_key,
                    allowed_origins,
                    org_id,
                } => {
                    let opts = commands::QueryEndpointCreateOptions {
                        roles: role,
                        open_api_keys: open_api_key,
                        allowed_origins,
                        org_id,
                    };
                    commands::query_endpoint_create(client, &service_id, opts, json).await
                }
                QueryEndpointCommands::Delete { service_id, org_id } => {
                    commands::query_endpoint_delete(client, &service_id, org_id.as_deref(), json)
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
                    commands::private_endpoint_create(
                        client,
                        &service_id,
                        &endpoint_id,
                        description.as_deref(),
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
                PrivateEndpointCommands::GetConfig { service_id, org_id } => {
                    commands::private_endpoint_get_config(
                        client,
                        &service_id,
                        org_id.as_deref(),
                        json,
                    )
                    .await
                }
            },
            ServiceCommands::BackupConfig { command } => match command {
                BackupConfigCommands::Get { service_id, org_id } => {
                    commands::backup_config_get(client, &service_id, org_id.as_deref(), json).await
                }
                BackupConfigCommands::Update {
                    service_id,
                    backup_period_hours,
                    backup_retention_period_hours,
                    backup_start_time,
                    org_id,
                } => {
                    let opts = commands::BackupConfigUpdateOptions {
                        backup_period_hours,
                        backup_retention_period_hours,
                        backup_start_time,
                        org_id,
                    };
                    commands::backup_config_update(client, &service_id, opts, json).await
                }
            },
            ServiceCommands::Prometheus {
                service_id,
                org_id,
                filtered_metrics,
            } => {
                commands::service_prometheus(
                    client,
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
                let opts = commands::ServiceQueryOptions {
                    name,
                    id,
                    query,
                    queries_file,
                    database,
                    format,
                    json,
                    org_id,
                    no_auto_enable,
                };
                commands::service_query(client, opts).await
            }
        },
        CloudCommands::Member { command } => match command {
            MemberCommands::List { org_id } => {
                commands::member_list(client, org_id.as_deref(), json).await
            }
            MemberCommands::Get { user_id, org_id } => {
                commands::member_get(client, &user_id, org_id.as_deref(), json).await
            }
            MemberCommands::Update {
                user_id,
                role_id,
                org_id,
            } => commands::member_update(client, &user_id, &role_id, org_id.as_deref(), json).await,
            MemberCommands::Remove { user_id, org_id } => {
                commands::member_remove(client, &user_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Invitation { command } => match command {
            InvitationCommands::List { org_id } => {
                commands::invitation_list(client, org_id.as_deref(), json).await
            }
            InvitationCommands::Create {
                email,
                role_id,
                org_id,
            } => {
                commands::invitation_create(client, &email, &role_id, org_id.as_deref(), json).await
            }
            InvitationCommands::Get {
                invitation_id,
                org_id,
            } => commands::invitation_get(client, &invitation_id, org_id.as_deref(), json).await,
            InvitationCommands::Delete {
                invitation_id,
                org_id,
            } => commands::invitation_delete(client, &invitation_id, org_id.as_deref(), json).await,
        },
        CloudCommands::Key { command } => match command {
            KeyCommands::List { org_id } => {
                commands::key_list(client, org_id.as_deref(), json).await
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
                let opts = commands::KeyCreateOptions {
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
                commands::key_create(client, opts, json).await
            }
            KeyCommands::Get { key_id, org_id } => {
                commands::key_get(client, &key_id, org_id.as_deref(), json).await
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
                let opts = commands::KeyUpdateOptions {
                    name,
                    role_ids: role_id,
                    expires_at,
                    state,
                    ip_allow,
                    org_id,
                };
                commands::key_update(client, &key_id, opts, json).await
            }
            KeyCommands::Delete { key_id, org_id } => {
                commands::key_delete(client, &key_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Activity { command } => match command {
            ActivityCommands::List {
                org_id,
                from_date,
                to_date,
            } => {
                commands::activity_list(
                    client,
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
            } => commands::activity_get(client, &activity_id, org_id.as_deref(), json).await,
        },
        CloudCommands::Backup { command } => match command {
            BackupCommands::List { service_id, org_id } => {
                commands::backup_list(client, &service_id, org_id.as_deref(), json).await
            }
            BackupCommands::Get {
                service_id,
                backup_id,
                org_id,
            } => {
                commands::backup_get(client, &service_id, &backup_id, org_id.as_deref(), json).await
            }
        },
        CloudCommands::Postgres { command } => postgres::run(client, command, json).await,
        CloudCommands::ClickPipe { command } => match *command {
            ClickPipeCommands::List { service_id, org_id } => {
                commands::clickpipe_list(client, &service_id, org_id.as_deref(), json).await
            }
            ClickPipeCommands::Get {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                commands::clickpipe_get(client, &service_id, &clickpipe_id, org_id.as_deref(), json)
                    .await
            }
            ClickPipeCommands::Delete {
                service_id,
                clickpipe_id,
                org_id,
            } => {
                commands::clickpipe_delete(
                    client,
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
                commands::clickpipe_state(
                    client,
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
                commands::clickpipe_state(
                    client,
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
                commands::clickpipe_state(
                    client,
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
                commands::clickpipe_scale(
                    client,
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
                    commands::clickpipe_settings_get(
                        client,
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
                    commands::clickpipe_settings_update(
                        client,
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
                commands::clickpipe_schema_discover(
                    client,
                    &service_id,
                    &command,
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            ClickPipeCommands::Create { command } => match command {
                ClickPipeCreateCommands::ObjectStorage(args) => {
                    commands::clickpipe_create_s3(client, &args, json).await
                }
                ClickPipeCreateCommands::Kafka(args) => {
                    commands::clickpipe_create_kafka(client, &args, json).await
                }
                ClickPipeCreateCommands::Kinesis(args) => {
                    commands::clickpipe_create_kinesis(client, &args, json).await
                }
                ClickPipeCreateCommands::Postgres(args) => {
                    commands::clickpipe_create_postgres(client, &args, json).await
                }
                ClickPipeCreateCommands::MySQL(args) => {
                    commands::clickpipe_create_mysql(client, &args, json).await
                }
                ClickPipeCreateCommands::MongoDB(args) => {
                    commands::clickpipe_create_mongodb(client, &args, json).await
                }
                ClickPipeCreateCommands::BigQuery(args) => {
                    commands::clickpipe_create_bigquery(client, &args, json).await
                }
            },
        },
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[test]
    fn ignored_env_notice_names_the_overridden_variables_and_winner() {
        assert_eq!(
            ignored_env_credentials_notice(
                AuthSource::CredentialsFile,
                EnvCredPresence {
                    key: true,
                    secret: true,
                },
            )
            .as_deref(),
            Some(
                "note: CLICKHOUSE_CLOUD_API_KEY and CLICKHOUSE_CLOUD_API_SECRET are set but \
                 ignored; using credentials file — see --debug"
            )
        );
        assert_eq!(
            ignored_env_credentials_notice(
                AuthSource::CliFlags,
                EnvCredPresence {
                    key: true,
                    secret: false,
                },
            )
            .as_deref(),
            Some(
                "note: CLICKHOUSE_CLOUD_API_KEY is set but ignored; using CLI flags — see --debug"
            )
        );
    }

    #[test]
    fn ignored_env_notice_is_absent_when_env_wins_or_is_unset() {
        assert!(
            ignored_env_credentials_notice(
                AuthSource::EnvVars,
                EnvCredPresence {
                    key: true,
                    secret: true,
                },
            )
            .is_none()
        );
        assert!(
            ignored_env_credentials_notice(
                AuthSource::CredentialsFile,
                EnvCredPresence {
                    key: false,
                    secret: false,
                },
            )
            .is_none()
        );
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
        // Anything that isn't a CloudError must not downcast to AuthRequired.
        let boxed: Box<dyn std::error::Error> = "plain string error".into();
        assert!(matches!(
            boxed_cloud_error_to_top_level(boxed),
            Error::Cloud(_)
        ));
    }
}
