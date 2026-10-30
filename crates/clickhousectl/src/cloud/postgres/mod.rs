mod cli;
mod client;
mod config;
mod lifecycle;
mod observability;
mod query;
mod topology;

use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

/// The GET that tells a missing service from a rejected request when a by-id
/// subresource or write fails with a 400 or 404 (see
/// `CloudClient::convert_postgres_by_id_result`).
const CONFIRM_POSTGRES_SERVICE: Conditional =
    Conditional::new("If the ID is rejected", &[&op::POSTGRES_SERVICE_GET]);

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("postgres list", &[&op::POSTGRES_SERVICE_GET_LIST]),
    Permission::api("postgres get", &[&op::POSTGRES_SERVICE_GET])
        .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])]),
    Permission::api("postgres logs", &[&op::POSTGRES_LOGS_GET_LIST])
        .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])]),
    Permission::api("postgres query", &[])
        .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])])
        .authorization("OAuth only; SQL uses database privileges and must be read-only."),
    Permission::api("postgres create", &[&op::POSTGRES_SERVICE_CREATE]),
    Permission::api(
        "postgres delete",
        &[&op::POSTGRES_SERVICE_GET, &op::POSTGRES_SERVICE_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])]),
    Permission::api("postgres certs get", &[&op::POSTGRES_SERVICE_CERTS_GET]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres config get", &[&op::POSTGRES_INSTANCE_CONFIG_GET]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api(
        "postgres config replace",
        &[&op::POSTGRES_INSTANCE_CONFIG_POST],
    )
    .when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api(
        "postgres config patch",
        &[&op::POSTGRES_INSTANCE_CONFIG_PATCH],
    )
    .when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api(
        "postgres reset-password",
        &[&op::POSTGRES_SERVICE_SET_PASSWORD],
    )
    .when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api(
        "postgres read-replica create",
        &[&op::POSTGRES_INSTANCE_CREATE_READ_REPLICA],
    )
    .when(&[
        Conditional::flag("source-name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres metrics", &[&op::POSTGRES_INSTANCE_METRICS_GET])
        .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])]),
    Permission::api(
        "postgres slow-queries list",
        &[&op::SLOW_QUERY_PATTERNS_GET_LIST],
    ),
    Permission::api("postgres slow-queries get", &[&op::SLOW_QUERY_PATTERN_GET]),
    Permission::api(
        "postgres prometheus service",
        &[&op::POSTGRES_INSTANCE_PROMETHEUS_GET],
    )
    .when(&[Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST])]),
    Permission::api(
        "postgres prometheus org",
        &[&op::POSTGRES_ORG_PROMETHEUS_GET],
    ),
    Permission::api(
        "postgres backup list",
        &[&op::POSTGRES_SERVICE_BACKUP_GET_LIST],
    ),
    Permission::api("postgres restore", &[&op::POSTGRES_INSTANCE_RESTORE]).when(&[
        Conditional::flag("source-name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        Conditional::new("Reading access rules", &[&op::POSTGRES_SERVICE_GET]),
        Conditional::new("Applying access rules", &[&op::POSTGRES_SERVICE_PATCH]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres restart", &[&op::POSTGRES_SERVICE_PATCH_STATE]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres promote", &[&op::POSTGRES_SERVICE_PATCH_STATE]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres switchover", &[&op::POSTGRES_SERVICE_PATCH_STATE]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
    Permission::api("postgres update", &[&op::POSTGRES_SERVICE_PATCH]).when(&[
        Conditional::flag("name", &[&op::POSTGRES_SERVICE_GET_LIST]),
        Conditional::new("Without --clear-tags", &[&op::POSTGRES_SERVICE_GET]),
        CONFIRM_POSTGRES_SERVICE,
    ]),
];

pub use cli::{
    CertsCommands, ConfigCommands, PostgresBackupCommands, PostgresCommands, PrometheusCommands,
    ReadReplicaCommands, SlowQueryCommands,
};

use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::shared::NamedResource;
use clickhouse_cloud_api::models::PostgresServiceSetStateCommand;
use config::{postgres_config_get, postgres_config_patch, postgres_config_replace};
use lifecycle::{
    PostgresCreateOptions, PostgresUpdateOptions, postgres_certs_get, postgres_create,
    postgres_delete, postgres_get, postgres_list, postgres_reset_password, postgres_state_change,
    postgres_update,
};
use observability::{
    SlowQueryDetailInput, SlowQueryListInput, build_postgres_logs_query, postgres_backup_list,
    postgres_logs, postgres_metrics, postgres_prometheus_org, postgres_prometheus_service,
    postgres_slow_queries_list, postgres_slow_query_get,
};
use query::postgres_query;
use topology::{
    PostgresReadReplicaOptions, PostgresRestoreOptions, PostgresRoleCommand,
    postgres_read_replica_create, postgres_restore, postgres_role_change,
};

pub async fn run(client: &CloudClient, command: PostgresCommands, json: bool) -> CloudResult<()> {
    match command {
        PostgresCommands::List { filter } => postgres_list(client, &filter, json).await,
        PostgresCommands::Get { postgres_id } => {
            postgres_get(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                json,
            )
            .await
        }
        PostgresCommands::Query {
            postgres_id,
            query,
            queries_file,
            database,
        } => {
            postgres_query(
                client,
                &postgres_id,
                query.as_deref(),
                queries_file.as_deref(),
                database.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Logs {
            postgres_id,
            from_date,
            to_date,
            body_contains,
            severity,
            sort_order,
            limit,
            offset,
        } => {
            let query = build_postgres_logs_query(
                &from_date,
                &to_date,
                body_contains.as_deref(),
                severity.as_deref(),
                sort_order.as_ref(),
                limit,
                offset,
            )?;
            postgres_logs(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                &query,
                json,
            )
            .await
        }
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
            ip_allow,
            block_public_access,
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
                ip_allow: &ip_allow,
                block_public_access,
            };
            postgres_create(client, opts, json).await
        }
        PostgresCommands::Update {
            postgres_id,
            name,
            size,
            ha_type,
            add_tag,
            remove_tag,
            clear_tags,
            add_ip_allow,
            remove_ip_allow,
        } => {
            let opts = PostgresUpdateOptions {
                name: name.as_deref(),
                size: size.as_deref(),
                ha_type: ha_type.as_deref(),
                add_tag: &add_tag,
                remove_tag: &remove_tag,
                clear_tags,
                add_ip_allow: &add_ip_allow,
                remove_ip_allow: &remove_ip_allow,
            };
            postgres_update(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                opts,
                json,
            )
            .await
        }
        PostgresCommands::Delete { postgres_id } => {
            postgres_delete(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                json,
            )
            .await
        }
        PostgresCommands::Certs(CertsCommands::Get {
            postgres_id,
            output,
        }) => {
            postgres_certs_get(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                output.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Config(ConfigCommands::Get { postgres_id }) => {
            postgres_config_get(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                json,
            )
            .await
        }
        PostgresCommands::Config(ConfigCommands::Replace { postgres_id, file }) => {
            postgres_config_replace(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                &file,
                json,
            )
            .await
        }
        PostgresCommands::Config(ConfigCommands::Patch {
            postgres_id,
            sets,
            file,
        }) => {
            postgres_config_patch(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                &sets,
                file.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::ResetPassword {
            postgres_id,
            password,
            generate,
        } => {
            postgres_reset_password(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                password.as_deref(),
                generate,
                json,
            )
            .await
        }
        PostgresCommands::ReadReplica(ReadReplicaCommands::Create {
            postgres_id,
            name,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
        }) => {
            let opts = PostgresReadReplicaOptions {
                name: &name,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
            };
            postgres_read_replica_create(client, &postgres_id.resolve(client).await?, opts, json)
                .await
        }
        PostgresCommands::Metrics {
            postgres_id,
            from_date,
            to_date,
            bucket_size_seconds,
        } => {
            postgres_metrics(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                &from_date,
                &to_date,
                bucket_size_seconds,
                json,
            )
            .await
        }
        PostgresCommands::SlowQueries(SlowQueryCommands::List {
            postgres_id,
            from_date,
            to_date,
            db_name,
            db_user,
            db_operation,
            app,
            sort_by,
            sort_order,
            limit,
            offset,
        }) => {
            let input = SlowQueryListInput {
                from_date: &from_date,
                to_date: &to_date,
                db_name: db_name.as_deref(),
                db_user: db_user.as_deref(),
                db_operation: db_operation.as_deref(),
                app: app.as_deref(),
                sort_by: sort_by.as_deref(),
                sort_order: sort_order.as_deref(),
                limit,
                offset,
            };
            postgres_slow_queries_list(client, &postgres_id, input, json).await
        }
        PostgresCommands::SlowQueries(SlowQueryCommands::Get {
            postgres_id,
            query_id,
            db_name,
            db_user,
            db_operation,
            app,
            timestamp,
        }) => {
            let input = SlowQueryDetailInput {
                db_name: &db_name,
                db_user: &db_user,
                db_operation: &db_operation,
                app: app.as_deref(),
                timestamp: timestamp.as_deref(),
            };
            postgres_slow_query_get(client, &postgres_id, &query_id, input, json).await
        }
        PostgresCommands::Prometheus(PrometheusCommands::Service { postgres_id }) => {
            postgres_prometheus_service(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                json,
            )
            .await
        }
        PostgresCommands::Prometheus(PrometheusCommands::Org) => {
            postgres_prometheus_org(client, json).await
        }
        PostgresCommands::Backup(PostgresBackupCommands::List {
            postgres_id,
            cursor,
            limit,
        }) => postgres_backup_list(client, &postgres_id, cursor.as_deref(), limit, json).await,
        PostgresCommands::Restore {
            postgres_id,
            name,
            restore_target,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
            ip_allow,
            block_public_access,
        } => {
            let opts = PostgresRestoreOptions {
                name: &name,
                restore_target: &restore_target,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                ip_allow: &ip_allow,
                block_public_access,
            };
            postgres_restore(client, &postgres_id.resolve(client).await?, opts, json).await
        }
        PostgresCommands::Restart { postgres_id } => {
            postgres_state_change(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                PostgresServiceSetStateCommand::Restart,
                json,
            )
            .await
        }
        PostgresCommands::Promote { postgres_id } => {
            postgres_role_change(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                PostgresRoleCommand::Promote,
                json,
            )
            .await
        }
        PostgresCommands::Switchover { postgres_id } => {
            postgres_role_change(
                client,
                &postgres_id.resolve(client, NamedResource::Postgres).await?,
                PostgresRoleCommand::Switchover,
                json,
            )
            .await
        }
    }
}
