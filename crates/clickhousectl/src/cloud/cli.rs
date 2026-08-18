pub(crate) use crate::cloud::activity::ActivityCommands;
pub(crate) use crate::cloud::api_keys::KeyCommands;
pub(crate) use crate::cloud::auth::AuthCommands;
#[allow(unused_imports)]
pub(crate) use crate::cloud::backups::{BackupCommands, BackupConfigCommands};
#[allow(unused_imports)]
pub(crate) use crate::cloud::clickpipes::{
    BigQueryCreateArgs, ClickPipeCommands, ClickPipeCreateCommands,
    ClickPipeSchemaDiscoverCommands, ClickPipeSettingsCommands, KafkaCreateArgs, KafkaSourceFields,
    KinesisCreateArgs, KinesisSourceFields, MongoDbCreateArgs, MySqlCreateArgs,
    ObjectStorageCreateArgs, PostgresCreateArgs,
};
pub(crate) use crate::cloud::organizations::{InvitationCommands, MemberCommands, OrgCommands};
#[allow(unused_imports)]
pub(crate) use crate::cloud::services::{
    PrivateEndpointCommands, QueryEndpointCommands, ServiceCommands,
};
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CloudArgs {
    /// API key override (highest precedence; see `cloud --help` for all sources)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret override (highest precedence; see `cloud --help` for all sources)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Print debug info (e.g. the credential source used) to stderr before running the command
    #[arg(long, global = true)]
    pub debug: bool,

    /// API base URL (default: auto-detect from OAuth tokens, or https://api.clickhouse.cloud)
    #[cfg_attr(debug_assertions, arg(long, global = true))]
    #[cfg_attr(not(debug_assertions), arg(long, global = true, hide = true))]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Create a ClickHouse Cloud account: `clickhousectl cloud auth signup`.

  `login` without flags uses OAuth device flow (interactive, read-only).
  Use API keys for write access (`login --api-key X --api-secret Y` or set CLICKHOUSE_CLOUD_API_KEY / CLICKHOUSE_CLOUD_API_SECRET).

  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl

  `logout` clears all saved credentials (OAuth tokens and API keys).

  Related: `clickhousectl cloud org list` to verify credentials work.")]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Organization commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud organizations. Subcommands: list, get, update, prometheus, usage.
  Org IDs are needed for most service and backup operations.
  Start with `clickhousectl cloud org list` to discover available org IDs.
  Related: `clickhousectl cloud service list` (uses org ID).")]
    Org {
        #[command(subcommand)]
        command: OrgCommands,
    },

    /// Service commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Most commands need a service ID — get it from `clickhousectl cloud service list`.
  Org ID is auto-detected if you have only one org; otherwise pass --org-id.
  Write commands (create, delete, start, stop, update, scale) require API key auth — OAuth is read-only.
  Use `query` to run SQL against a service over HTTP.
  Related: `clickhousectl cloud org list` for org IDs.")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Backup commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud backups. Subcommands: list, get.
  Requires a service ID — get it from `clickhousectl cloud service list`.
  Backup IDs from `backup list` can be used with `service create --backup-id` to restore.
  Related: `clickhousectl cloud service list` for service IDs.")]
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    // Clickpipe commands
    #[command(
        name = "clickpipe",
        after_help = "\
CONTEXT FOR AGENTS:
    Manage ClickPipes for ingesting data into ClickHouse Cloud.
    Subcommands: list, get, delete, start, stop, resync, scale, settings, create.
    Requires a service ID — get it from `clickhousectl cloud service list`."
    )]
    ClickPipe {
        #[command(subcommand)]
        command: Box<ClickPipeCommands>,
    },

    /// Manage organization members
    Member {
        #[command(subcommand)]
        command: MemberCommands,
    },

    /// Manage organization invitations
    Invitation {
        #[command(subcommand)]
        command: InvitationCommands,
    },

    /// Manage API keys
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// View activity log
    Activity {
        #[command(subcommand)]
        command: ActivityCommands,
    },

    /// Manage ClickHouse Cloud Postgres services (beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud managed Postgres services. Subcommands cover CRUD, lifecycle
  (restart/promote/switchover), CA certs, runtime config, password reset, read replicas,
  and point-in-time restore. Service IDs come from `postgres list`.
  Write commands require API key auth — OAuth is read-only.")]
    Postgres {
        #[command(subcommand)]
        command: crate::cloud::postgres::PostgresCommands,
    },
}

impl CloudCommands {
    /// Returns true if this command performs a write/mutating operation.
    /// OAuth (Bearer) auth is read-only and cannot execute write commands.
    ///
    /// Every variant is explicitly matched — no wildcards — so the compiler
    /// will error when a new command is added, forcing the developer to
    /// classify it as read or write.
    pub fn is_write_command(&self) -> bool {
        match self {
            CloudCommands::Auth { command } => command.is_write(),
            CloudCommands::Org { command } => command.is_write(),
            CloudCommands::Service { command } => command.is_write(),
            CloudCommands::Backup { command } => command.is_write(),
            CloudCommands::Member { command } => command.is_write(),
            CloudCommands::Invitation { command } => command.is_write(),
            CloudCommands::Key { command } => command.is_write(),
            CloudCommands::Activity { command } => command.is_write(),
            CloudCommands::Postgres { command } => command.is_write(),
            CloudCommands::ClickPipe { command } => command.is_write(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    /// Helper to assert a command parsed from CLI args is classified correctly.
    fn assert_write(args: &[&str], expected: bool) {
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        assert_eq!(
            cloud_args.command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    #[test]
    fn is_write_command_read_only_commands() {
        // Backup reads
        assert_write(
            &["clickhousectl", "cloud", "backup", "list", "svc-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "backup", "get", "svc-1", "bk-1"],
            false,
        );

        // Key reads
        assert_write(&["clickhousectl", "cloud", "key", "list"], false);
        assert_write(&["clickhousectl", "cloud", "key", "get", "key-1"], false);

        // Activity reads
        assert_write(&["clickhousectl", "cloud", "activity", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "activity", "get", "act-1"],
            false,
        );

        // Postgres reads
        assert_write(&["clickhousectl", "cloud", "postgres", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "postgres", "get", "pg-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "certs", "get", "pg-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "get",
                "pg-1",
            ],
            false,
        );
    }

    #[test]
    fn is_write_command_destructive_commands() {
        // Key writes
        assert_write(
            &["clickhousectl", "cloud", "key", "create", "--name", "k"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "key",
                "update",
                "key-1",
                "--name",
                "new",
            ],
            true,
        );
        assert_write(&["clickhousectl", "cloud", "key", "delete", "key-1"], true);

        // Postgres writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "create",
                "--name",
                "pg",
                "--region",
                "us-east-1",
                "--size",
                "m7i.2xlarge",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "update",
                "pg-1",
                "--size",
                "c6gd.large",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "delete", "pg-1"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                "/tmp/c.json",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "patch",
                "pg-1",
                "--set",
                "max_connections=500",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "reset-password",
                "pg-1",
                "--generate",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "r1",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "r",
                "--restore-target",
                "2026-04-16T12:00:00Z",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "restart", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "promote", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "switchover", "pg-1"],
            true,
        );
    }
}
