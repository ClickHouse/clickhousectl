pub(crate) use crate::cloud::activity::ActivityCommands;
pub(crate) use crate::cloud::api_keys::KeyCommands;
pub(crate) use crate::cloud::auth::AuthCommands;
#[allow(unused_imports)]
pub(crate) use crate::cloud::backups::{BackupCommands, BackupConfigCommands};
#[allow(unused_imports)]
pub(crate) use crate::cloud::clickpipe_endpoints::{
    ReversePrivateEndpointCommands, ReversePrivateEndpointCreateArgs,
};
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
    /// Cloud API key; overrides stored and environment credentials
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// Cloud API secret; overrides stored and environment credentials
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Print the resolved credential source and API URL to stderr
    #[arg(long, global = true)]
    pub debug: bool,

    /// Cloud API base URL override
    #[cfg_attr(debug_assertions, arg(long, global = true))]
    #[cfg_attr(not(debug_assertions), arg(long, global = true, hide = true))]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

impl CloudArgs {
    pub fn has_explicit_json_format_conflict(&self) -> bool {
        self.json
            && matches!(
                &self.command,
                CloudCommands::Service {
                    command: ServiceCommands::Query {
                        format: Some(_),
                        ..
                    }
                }
            )
    }

    /// The `clickpipe create <source>` validation message and the source
    /// subcommand it belongs to, if the flags cannot describe the chosen
    /// `--auth`. Covers both database sources whose credential flags depend on
    /// `--auth`'s value: `postgres` and `mysql`.
    pub fn clickpipe_create_validation_error(&self) -> Option<(&'static str, String)> {
        let CloudCommands::ClickPipe { command } = &self.command else {
            return None;
        };
        command.clickpipe_create_validation_error()
    }

    /// The `clickpipe reverse-private-endpoint create` validation message, if
    /// the flags cannot describe the chosen `--type`. clap cannot express
    /// "forbidden for this value of another argument", so the check runs after
    /// parsing and is reported as a usage error against the owning command.
    pub fn reverse_private_endpoint_validation_error(&self) -> Option<String> {
        let CloudCommands::ClickPipe { command } = &self.command else {
            return None;
        };
        command.reverse_private_endpoint_create_validation_error()
    }
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Manage organizations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `org list` is the source of the org IDs every other cloud command takes as --org-id.
  Next: `cloud service list`, `cloud member list`.")]
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

    /// View service backups
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Service IDs come from `cloud service list`; backup IDs from `cloud backup list <service-id>`.
  Restore a backup into a new service: `cloud service create --backup-id <backup-id>`.
  Change schedule or retention with `cloud service backup-config update`, not here.")]
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// Manage ClickPipes for data ingestion
    #[command(
        name = "clickpipe",
        after_help = "\
CONTEXT FOR AGENTS:
  Service ID: `clickhousectl cloud service list`. ClickPipe ID: `clickpipe list <SERVICE_ID>`.
  `start` only works on a Stopped or Failed pipe; `stop` works from any state.
  Typical flow: `clickpipe schema-discover <source>` -> `clickpipe create <source>` -> `clickpipe get`."
    )]
    ClickPipe {
        #[command(subcommand)]
        command: Box<ClickPipeCommands>,
    },

    /// Manage organization members
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  User IDs come from `cloud member list`; role IDs only from `cloud member list --json`
  (the table shows role names). `update` replaces the member's whole role set.
  `remove` takes effect immediately with no confirmation.
  Next: `cloud invitation create --email ...` to add someone who is not yet a member.")]
    Member {
        #[command(subcommand)]
        command: MemberCommands,
    },

    /// Manage organization invitations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Role IDs come from `cloud member list --json` (roleId), not from the human table.
  The invitee must accept from the email; `list` shows only invitations still pending.
  Next: `cloud member list` once the invitation is accepted.")]
    Invitation {
        #[command(subcommand)]
        command: InvitationCommands,
    },

    /// Manage API keys
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `create` prints the key secret exactly once — capture stdout or you must create a new key.
  Role IDs come from `cloud member list --json` (roleId) and must be UUIDs here.
  `update` replaces --role-id and --ip-allow wholesale; omitted flags are left as-is.
  Next: `cloud auth login --api-key <id> --api-secret <secret>` to use a new key.")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// View the organization activity log
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Audit log of org and service changes; without --from-date/--to-date the API picks the range.
  Activity IDs for `activity get` come from `activity list`.")]
    Activity {
        #[command(subcommand)]
        command: ActivityCommands,
    },

    /// Manage ClickHouse Cloud Postgres services (beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Write commands need API key auth; OAuth is read-only.
  Service IDs: `cloud postgres list`. Org ID auto-detects only with one org, else pass --org-id.
  Credentials come only from `create` and `reset-password`; `get` does not return them.
  promote/switchover are eventually consistent: pass --wait to confirm the new role.
  Typical flow: `create` -> `get <id>` until state is running -> `certs get` -> `config patch`.")]
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

        // ClickPipe reverse private endpoint reads
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
                "list",
                "svc-1",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
                "get",
                "svc-1",
                "rpe-1",
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

        // ClickPipe reverse private endpoint writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
                "create",
                "svc-1",
                "--type",
                "GCP_PSC_SERVICE_ATTACHMENT",
                "--description",
                "endpoint",
                "--gcp-service-attachment",
                "projects/p/regions/us-central1/serviceAttachments/s",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
                "update",
                "svc-1",
                "rpe-1",
                "--custom-private-dns-mapping",
                "db.example.com",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
                "delete",
                "svc-1",
                "rpe-1",
            ],
            true,
        );
    }

    #[test]
    fn every_cloud_subcommand_has_a_help_about() {
        use clap::CommandFactory;

        let mut command = Cli::command();
        let cloud = command
            .find_subcommand_mut("cloud")
            .expect("cloud subcommand");

        for sub in cloud.get_subcommands() {
            let about = sub.get_about().map(|a| a.to_string()).unwrap_or_default();
            assert!(
                !about.trim().is_empty(),
                "cloud subcommand `{}` has no about text",
                sub.get_name()
            );
        }
    }
}
