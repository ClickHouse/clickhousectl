pub mod activity;
pub mod api_keys;
pub mod auth;
pub mod backups;
pub mod cli;
pub mod clickpipe_endpoints;
pub mod clickpipes;
pub mod clickstack;
pub mod client;
mod config;
pub mod credentials;
pub mod organizations;
pub mod output;
pub mod postgres;
pub mod service_query;
pub mod services;
mod shared;
pub mod types;
pub mod udfs;

#[cfg(test)]
mod types_test;

pub use client::{
    AuthSource, CloudClient, CloudError, CloudErrorKind, EnvCredPresence, dotenv_env_provenance,
    env_cred_presence, resolve_active_auth_source,
};

use crate::error::{Error, Result};
use cli::{CloudArgs, CloudCommands};

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
        return auth::run(command, args.url.as_deref(), args.debug, json).await;
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
        .map_err(cloud_error_to_top_level)
}

fn cloud_error_to_top_level(e: CloudError) -> Error {
    match (e.kind, e.details) {
        // Auth failures own the exit code (4) and have their own remediation
        // text, so a structured detail never displaces them.
        (CloudErrorKind::Auth, _) => Error::AuthRequired(e.message),
        // A structured detail replaces the prose *only* in JSON mode; its
        // `message` is the same text, so human output is unchanged (#644).
        (CloudErrorKind::Generic, Some(details)) => Error::CloudDetailed(details),
        (CloudErrorKind::Generic, None) => Error::Cloud(e.message),
    }
}

async fn dispatch(client: &CloudClient, command: CloudCommands, json: bool) -> client::Result<()> {
    match command {
        CloudCommands::Auth { .. } => unreachable!("handled above"),
        CloudCommands::Org { command } => organizations::run_org(client, command, json).await,
        CloudCommands::Service { command } => services::run(client, command, json).await,
        CloudCommands::Member { command } => organizations::run_member(client, command, json).await,
        CloudCommands::Invitation { command } => {
            organizations::run_invitation(client, command, json).await
        }
        CloudCommands::Key { command } => api_keys::run(client, command, json).await,
        CloudCommands::Udf(args) => udfs::run(client, args, json).await,
        CloudCommands::Activity { command } => activity::run(client, command, json).await,
        CloudCommands::Backup { command } => backups::run(client, command, json).await,
        CloudCommands::Postgres { command } => postgres::run(client, command, json).await,
        CloudCommands::ClickPipe { command } => clickpipes::run(client, *command, json).await,
        CloudCommands::ClickStack { command } => clickstack::run(client, command, json).await,
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
        let auth = cloud_error_to_top_level(CloudError::auth("nope"));
        assert!(matches!(&auth, Error::AuthRequired(message) if message == "nope"));
        assert_eq!(auth.exit_code(), 4);

        let generic = cloud_error_to_top_level(CloudError::new("boom"));
        assert!(matches!(&generic, Error::Cloud(message) if message == "boom"));
        assert_eq!(generic.exit_code(), 1);
        assert_eq!(CloudError::new("x").kind, CloudErrorKind::Generic);
    }

    /// A structured detail travels to the top level, where JSON mode emits
    /// it; human mode keeps rendering the same message (#644).
    #[test]
    fn cloud_error_details_reach_the_top_level_error() {
        let detail = crate::cloud::output::CloudErrorDetail {
            code: crate::cloud::output::CloudErrorCode::QueryTimeout,
            message: "timed out".into(),
            host: Some("demo.clickhouse.cloud".into()),
            port: Some(9440),
            command: Some("clickhouse client".into()),
            api_key_id: None,
            ip_access_list: None,
        };
        let error = cloud_error_to_top_level(CloudError::new("timed out").with_details(detail));
        match &error {
            Error::CloudDetailed(details) => {
                assert_eq!(
                    details.code,
                    crate::cloud::output::CloudErrorCode::QueryTimeout
                );
                assert_eq!(details.host.as_deref(), Some("demo.clickhouse.cloud"));
            }
            other => panic!("expected CloudDetailed, got {other:?}"),
        }
        // Human output is unchanged, and the exit code is the ordinary 1.
        assert_eq!(error.to_string(), "timed out");
        assert_eq!(error.exit_code(), 1);
    }

    /// An auth failure owns exit code 4 and its own remediation, so a detail
    /// never displaces it.
    #[test]
    fn auth_errors_are_not_displaced_by_details() {
        let detail = crate::cloud::output::CloudErrorDetail {
            code: crate::cloud::output::CloudErrorCode::QueryTimeout,
            message: "nope".into(),
            host: None,
            port: None,
            command: None,
            api_key_id: None,
            ip_access_list: None,
        };
        let error = cloud_error_to_top_level(CloudError::auth("nope").with_details(detail));
        assert!(matches!(&error, Error::AuthRequired(message) if message == "nope"));
        assert_eq!(error.exit_code(), 4);
    }

    #[test]
    fn concrete_cloud_error_preserves_auth_kind() {
        assert!(matches!(
            cloud_error_to_top_level(CloudError::auth("nope")),
            Error::AuthRequired(_)
        ));
    }
}
