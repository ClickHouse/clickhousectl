mod cli;
mod client;
mod endpoints;
mod lifecycle;
mod query;
pub mod query_key;
mod scaling_schedule;
mod settings;
mod upgrade_window;

use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("service list", &[&op::INSTANCE_GET_LIST]),
    Permission::api("service get", &[&op::INSTANCE_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service dotenv", &[&op::INSTANCE_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service profile list", &[&op::SERVICE_PROFILES_LIST]),
    Permission::api(
        "service settings list",
        &[&op::SERVICE_CLICKHOUSE_SETTINGS_LIST_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service settings get",
        &[&op::SERVICE_CLICKHOUSE_SETTING_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service settings schema",
        &[&op::SERVICE_CLICKHOUSE_SETTINGS_SCHEMA_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service settings set",
        &[&op::SERVICE_CLICKHOUSE_SETTINGS_UPDATE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service settings unset",
        &[&op::SERVICE_CLICKHOUSE_SETTING_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service scaling-schedule get", &[&op::SCALING_SCHEDULE_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service scaling-schedule set",
        &[&op::SCALING_SCHEDULE_UPSERT],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service scaling-schedule delete",
        &[&op::SCALING_SCHEDULE_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service start", &[&op::INSTANCE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service wake", &[&op::INSTANCE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service stop", &[&op::INSTANCE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service scale", &[&op::INSTANCE_REPLICA_SCALING_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service reset-password", &[&op::INSTANCE_PASSWORD_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service query-endpoint get",
        &[&op::INSTANCE_QUERY_ENDPOINT_GET],
    )
    .when(&[
        Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
        Conditional::new("If endpoint is absent", &[&op::INSTANCE_GET]),
    ]),
    Permission::api(
        "service query-endpoint create",
        &[
            &op::INSTANCE_QUERY_ENDPOINT_GET,
            &op::INSTANCE_QUERY_ENDPOINT_UPSERT,
        ],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service query-endpoint delete",
        &[&op::INSTANCE_QUERY_ENDPOINT_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service private-endpoint create",
        &[&op::INSTANCE_PRIVATE_ENDPOINT_CREATE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service private-endpoint get-config",
        &[&op::INSTANCE_PRIVATE_ENDPOINT_CONFIG_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service upgrade-window get", &[&op::UPGRADE_WINDOW_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service upgrade-window set", &[&op::UPGRADE_WINDOW_UPDATE])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service upgrade-window delete",
        &[&op::UPGRADE_WINDOW_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service replica-aware-routing get",
        &[&op::REPLICA_AWARE_ROUTING_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service replica-aware-routing update",
        &[&op::REPLICA_AWARE_ROUTING_UPDATE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service prometheus", &[&op::INSTANCE_PROMETHEUS_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service create", &[&op::INSTANCE_CREATE]).when(&[Conditional::new(
        "Dynamic BYOC profile",
        &[&op::SERVICE_PROFILES_LIST],
    )]),
    Permission::api("service delete", &[&op::INSTANCE_DELETE]).when(&[
        Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
        Conditional::flag("force", &[&op::INSTANCE_GET, &op::INSTANCE_STATE_UPDATE]),
        Conditional::new("Owned query-key cleanup", &[&op::OPENAPI_KEY_DELETE]),
    ]),
    Permission::api("service update", &[&op::INSTANCE_UPDATE]).when(&[
        Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
        Conditional::new("Removing IPs/endpoints/tags", &[&op::INSTANCE_GET]),
    ]),
    Permission::api("service query", &[])
        .when(&[
            Conditional::new("service ID", &[&op::INSTANCE_GET]),
            Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
            Conditional::new(
                "auto-enable",
                &[
                    &op::WHOAMI_GET,
                    &op::OPENAPI_KEY_GET,
                    &op::INSTANCE_QUERY_ENDPOINT_GET,
                    &op::INSTANCE_QUERY_ENDPOINT_UPSERT,
                ],
            ),
            Conditional::new(
                "key diagnosis",
                &[&op::OPENAPI_KEY_GET, &op::INSTANCE_QUERY_ENDPOINT_GET],
            ),
            Conditional::new("key cleanup", &[&op::OPENAPI_KEY_DELETE]),
        ])
        .authorization("SQL uses database privileges; OAuth permits read-only SELECT."),
    Permission::api(
        "service repair-query-key",
        &[
            &op::INSTANCE_QUERY_ENDPOINT_GET,
            &op::OPENAPI_KEY_CREATE,
            &op::INSTANCE_QUERY_ENDPOINT_UPSERT,
            &op::OPENAPI_KEY_DELETE,
        ],
    )
    .when(&[
        Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
        Conditional::new("Verifying the repaired key", &[&op::INSTANCE_GET]),
        Conditional::new(
            "Rollback of a new endpoint",
            &[&op::INSTANCE_QUERY_ENDPOINT_DELETE],
        ),
    ]),
];

pub use cli::{
    PrivateEndpointCommands, QueryEndpointCommands, ReplicaAwareRoutingCommands,
    ScalingScheduleCommands, ServiceCommands, ServiceProfileCommands, ServiceSettingsCommands,
    UpgradeWindowCommands,
};

use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::shared::NamedResource;
use endpoints::{
    QueryEndpointCreateOptions, private_endpoint_create, private_endpoint_get_config,
    query_endpoint_create, query_endpoint_delete, query_endpoint_get,
};
use lifecycle::{
    CreateServiceOptions, ServiceResetPasswordOptions, ServiceScaleOptions, ServiceUpdateOptions,
    read_backup_encryption_config, service_create, service_delete, service_dotenv, service_get,
    service_list, service_prometheus, service_reset_password, service_scale, service_start,
    service_stop, service_update, service_wake,
};
use query::{ServiceQueryOptions, service_query, service_query_key_repair};
use scaling_schedule::{scaling_schedule_delete, scaling_schedule_get, scaling_schedule_set};
use settings::{
    service_setting_get, service_setting_unset, service_settings_list, service_settings_schema,
    service_settings_set,
};
use upgrade_window::{
    build_replica_aware_routing_request, replica_aware_routing_get, replica_aware_routing_update,
    service_profile_list, upgrade_window_delete, upgrade_window_get, upgrade_window_set,
};

pub async fn run(client: &CloudClient, command: ServiceCommands, json: bool) -> CloudResult<()> {
    match command {
        ServiceCommands::List { filter } => service_list(client, &filter, json).await,
        ServiceCommands::Get { service_id } => {
            service_get(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
        }
        ServiceCommands::Dotenv { service_id, local } => {
            service_dotenv(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                local,
                json,
            )
            .await
        }
        ServiceCommands::Profile { command } => match command {
            ServiceProfileCommands::List { region, byoc_id } => {
                service_profile_list(client, region.as_deref(), byoc_id.as_deref(), json).await
            }
        },
        ServiceCommands::Settings { command } => match command {
            ServiceSettingsCommands::List { service_id } => {
                service_settings_list(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            ServiceSettingsCommands::Get { target } => {
                let (selector, setting_name) = target.parts();
                service_setting_get(
                    client,
                    &selector.resolve(client, NamedResource::Service).await?,
                    setting_name,
                    json,
                )
                .await
            }
            ServiceSettingsCommands::Schema { service_id } => {
                service_settings_schema(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            ServiceSettingsCommands::Set {
                service_id,
                setting,
                settings_file,
            } => {
                service_settings_set(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    &setting,
                    settings_file.as_deref(),
                    json,
                )
                .await
            }
            ServiceSettingsCommands::Unset { target } => {
                let (selector, setting_name) = target.parts();
                service_setting_unset(
                    client,
                    &selector.resolve(client, NamedResource::Service).await?,
                    setting_name,
                    json,
                )
                .await
            }
        },
        ServiceCommands::ScalingSchedule { command } => match command {
            ScalingScheduleCommands::Get { service_id } => {
                scaling_schedule_get(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            ScalingScheduleCommands::Set { service_id, file } => {
                scaling_schedule_set(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    &file,
                    json,
                )
                .await
            }
            ScalingScheduleCommands::Delete { service_id } => {
                scaling_schedule_delete(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
        },
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
            backup_encryption_config,
            release_channel,
            data_warehouse_id,
            readonly,
            enable_tde,
            compliance_type,
            profile,
            byoc_id,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            dotenv,
        } => {
            let options = CreateServiceOptions {
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
                backup_encryption_config: backup_encryption_config
                    .as_deref()
                    .map(read_backup_encryption_config)
                    .transpose()?,
                release_channel,
                data_warehouse_id,
                is_readonly: readonly,
                enable_tde,
                compliance_type,
                profile,
                byoc_id,
                tags: tag,
                enable_endpoints: enable_endpoint,
                disable_endpoints: disable_endpoint,
                private_preview_terms_checked,
                enable_core_dumps,
            };
            service_create(client, options, dotenv.as_deref(), json).await
        }
        ServiceCommands::Delete { service_id, force } => {
            service_delete(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                force,
                json,
            )
            .await
        }
        ServiceCommands::Start { service_id } => {
            service_start(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
        }
        ServiceCommands::Wake { service_id } => {
            service_wake(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
        }
        ServiceCommands::Stop { service_id } => {
            service_stop(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
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
        } => {
            let options = ServiceUpdateOptions {
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
            };
            service_update(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                options,
                json,
            )
            .await
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
        } => {
            service_scale(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                ServiceScaleOptions {
                    min_replica_memory_gb,
                    max_replica_memory_gb,
                    num_replicas,
                    min_replicas,
                    max_replicas,
                    autoscaling_mode,
                    idle_scaling,
                    idle_timeout_minutes,
                },
                json,
            )
            .await
        }
        ServiceCommands::ResetPassword {
            service_id,
            new_password_hash,
            new_double_sha1_hash,
        } => {
            let options = ServiceResetPasswordOptions {
                new_password_hash,
                new_double_sha1_hash,
            };
            service_reset_password(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                options,
                json,
            )
            .await
        }
        ServiceCommands::QueryEndpoint { command } => match command {
            QueryEndpointCommands::Get { service_id } => {
                query_endpoint_get(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            QueryEndpointCommands::Create {
                service_id,
                role,
                open_api_key,
                replace_open_api_keys,
                allowed_origins,
            } => {
                let options = QueryEndpointCreateOptions {
                    roles: role,
                    open_api_keys: open_api_key,
                    replace_open_api_keys,
                    allowed_origins,
                };
                query_endpoint_create(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    options,
                    json,
                )
                .await
            }
            QueryEndpointCommands::Delete { service_id } => {
                query_endpoint_delete(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
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
            } => {
                private_endpoint_create(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    &endpoint_id,
                    description.as_deref(),
                    json,
                )
                .await
            }
            PrivateEndpointCommands::GetConfig { service_id } => {
                private_endpoint_get_config(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
        },
        ServiceCommands::Snapshot { command } => {
            crate::cloud::backups::run_snapshot(client, command, json).await
        }
        ServiceCommands::BackupConfig { command } => {
            crate::cloud::backups::run_config(client, command, json).await
        }
        ServiceCommands::UpgradeWindow { command } => match command {
            UpgradeWindowCommands::Get { service_id } => {
                upgrade_window_get(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            UpgradeWindowCommands::Set {
                service_id,
                weekday,
                start_hour,
            } => {
                upgrade_window_set(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    weekday,
                    &start_hour,
                    json,
                )
                .await
            }
            UpgradeWindowCommands::Delete { service_id } => {
                upgrade_window_delete(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
        },
        ServiceCommands::ReplicaAwareRouting { command } => match command {
            ReplicaAwareRoutingCommands::Get { service_id } => {
                replica_aware_routing_get(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    json,
                )
                .await
            }
            ReplicaAwareRoutingCommands::Update {
                service_id,
                http_enabled,
                native_enabled,
            } => {
                replica_aware_routing_update(
                    client,
                    &service_id.resolve(client, NamedResource::Service).await?,
                    build_replica_aware_routing_request(http_enabled, native_enabled),
                    json,
                )
                .await
            }
        },
        ServiceCommands::Prometheus {
            service_id,
            filtered_metrics,
        } => {
            service_prometheus(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                filtered_metrics,
            )
            .await
        }
        ServiceCommands::Query {
            service_id,
            name,
            id,
            query,
            queries_file,
            database,
            format,
            no_auto_enable,
        } => {
            let options = ServiceQueryOptions {
                name,
                id: service_id.or(id),
                query,
                queries_file,
                database,
                format,
                json,
                no_auto_enable,
            };
            service_query(client, options).await
        }
        ServiceCommands::RepairQueryKey { service_id } => {
            service_query_key_repair(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
        }
    }
}
