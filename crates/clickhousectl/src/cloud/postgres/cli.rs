use super::lifecycle::{
    PostgresListFilter, parse_postgres_ip_access_entry, parse_postgres_list_filter,
};
use super::observability::{parse_known_postgres_logs_sort_order, parse_postgres_window_datetime};
use crate::cloud::shared::{NameSelector, SourceSelector, parse_datetime};
use clap::builder::TypedValueParser;
use clap::{ArgGroup, Subcommand};
use clickhouse_cloud_api::models::{
    PgHaType, PgProvider, PgVersion, PostgresIpAccessListEntry, PostgresLogsGetListSortorder,
    SlowQueryPatternsGetListSortby, SlowQueryPatternsGetListSortorder,
};
use std::path::PathBuf;

const POSTGRES_LOG_SORT_ORDERS: &[&str] = &["asc", "desc"];

#[derive(Subcommand)]
pub enum PostgresCommands {
    /// List Postgres services in the organization
    List {
        /// Filter results client-side, repeatable (KEY=VALUE)
        ///
        /// Keys: state, region, name, provider, isPrimary. Every filter must
        /// match, and a field the API omitted matches nothing.
        #[arg(long, value_parser = parse_postgres_list_filter)]
        filter: Vec<PostgresListFilter>,
    },

    /// Get Postgres service details
    Get {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },

    /// Run a read-only SQL query on a Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Requires OAuth login and Postgres query access; API keys are unsupported.
  Remove higher-priority API key credentials before using OAuth.
  With no SQL flag, reads stdin; --query never reads stdin.
  --json streams arrays: column names, column types, then result rows.
  Queries are read-only; use psql for writes or interactive sessions.")]
    Query {
        #[command(flatten)]
        postgres_id: NameSelector,
        /// SQL to run; conflicts with --queries-file
        #[arg(long, short = 'q', conflicts_with = "queries_file")]
        query: Option<String>,
        /// SQL file, or - for stdin; conflicts with --query
        #[arg(long, conflicts_with = "query")]
        queries_file: Option<PathBuf>,
        /// Database name (server default: postgres)
        #[arg(long)]
        database: Option<String>,
    },

    /// List Postgres server logs
    Logs {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// Inclusive start of the time window (RFC 3339)
        #[arg(long, value_parser = parse_datetime)]
        from_date: String,
        /// Inclusive end of the time window (RFC 3339)
        #[arg(long, value_parser = parse_datetime)]
        to_date: String,
        /// Case-sensitive substring the log body must contain
        #[arg(long)]
        body_contains: Option<String>,
        /// PostgreSQL severity, such as ERROR, WARNING, or LOG
        #[arg(long)]
        severity: Option<String>,
        /// Sort order
        #[arg(
            long,
            value_parser = clap::builder::PossibleValuesParser::new(
                POSTGRES_LOG_SORT_ORDERS
            ).try_map(parse_known_postgres_logs_sort_order)
        )]
        sort_order: Option<PostgresLogsGetListSortorder>,
        /// Maximum number of log entries
        #[arg(long, allow_hyphen_values = true, value_parser = clap::value_parser!(i64).range(1..=2000))]
        limit: Option<i64>,
        /// Number of log entries to skip
        #[arg(
            long,
            allow_hyphen_values = true,
            value_parser = clap::value_parser!(i64).range(0..)
        )]
        offset: Option<i64>,
    },

    /// Create a Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The password and connection string are returned once, here — save them.
  Treat `get` as never returning credentials; `reset-password` is the only way back.
  Not usable until `cloud postgres get <id>` reports state=running.
  Omitting --ip-allow allows all addresses on 5432 (Postgres) and 6432 (PgBouncer); with --ip-allow
    only the listed rules apply, so add one for each port you need.")]
    Create {
        /// Service name
        #[arg(long)]
        name: String,
        /// Cloud region (e.g. us-east-1)
        #[arg(long)]
        region: String,
        /// Instance size (e.g. c6gd.xlarge); validated by the server
        #[arg(long)]
        size: String,
        /// Cloud provider
        #[arg(
            long,
            default_value = "aws",
            value_parser = clap::builder::PossibleValuesParser::new(PgProvider::VALUES)
        )]
        provider: String,
        /// Postgres major version
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(PgVersion::VALUES))]
        pg_version: Option<String>,
        /// High-availability type
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(PgHaType::VALUES))]
        ha_type: Option<String>,
        /// Resource tag (repeatable), e.g. --tag env=prod
        #[arg(long)]
        tag: Vec<String>,
        /// Path to a JSON file with a PgConfig object
        #[arg(long)]
        pg_config_file: Option<PathBuf>,
        /// JSON file of PgBouncer parameters with string values
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
        /// Allowed IPv4 address or CIDR and ports (repeatable)
        ///
        /// PORTS is a port or inclusive range, e.g. 5432 (Postgres), 6432
        /// (PgBouncer) or 5432..6432.
        #[arg(
            long = "ip-allow",
            value_name = "IP_OR_CIDR:PORTS[=DESCRIPTION]",
            value_parser = parse_postgres_ip_access_entry
        )]
        ip_allow: Vec<PostgresIpAccessListEntry>,
        /// Create with no access rules, blocking public access
        #[arg(long, conflicts_with = "ip_allow")]
        block_public_access: bool,
    },

    /// Update a Postgres service's name, size, HA, tags or access rules
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  --new-name changes the service hostname; re-read `cloud postgres get` and update connection strings.
  --remove-ip-allow takes rule IDs from `cloud postgres get`; removals apply before additions.
  Access rule changes can partially succeed: after a failure, read the rules with
    `cloud postgres get` before retrying.")]
    Update {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// New service name
        #[arg(long = "new-name", id = "new_name")]
        name: Option<String>,
        /// New instance size (e.g. c6gd.xlarge); validated by the server
        #[arg(long)]
        size: Option<String>,
        /// New high-availability type
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(PgHaType::VALUES))]
        ha_type: Option<String>,
        /// Add a tag (repeatable; conflicts with --clear-tags)
        #[arg(long, conflicts_with = "clear_tags")]
        add_tag: Vec<String>,
        /// Remove a tag by key (repeatable; conflicts with --clear-tags)
        #[arg(long, conflicts_with = "clear_tags")]
        remove_tag: Vec<String>,
        /// Remove all tags; conflicts with --add-tag and --remove-tag
        #[arg(long, conflicts_with_all = ["add_tag", "remove_tag"])]
        clear_tags: bool,
        /// IPv4 access rule to create, as in `create --ip-allow` (repeatable)
        #[arg(
            long = "add-ip-allow",
            value_name = "IP_OR_CIDR:PORTS[=DESCRIPTION]",
            value_parser = parse_postgres_ip_access_entry
        )]
        add_ip_allow: Vec<PostgresIpAccessListEntry>,
        /// Access rule ID to remove (repeatable)
        #[arg(
            long = "remove-ip-allow",
            value_name = "RULE_ID",
            value_parser = clap::builder::NonEmptyStringValueParser::new()
        )]
        remove_ip_allow: Vec<String>,
    },

    /// Delete a Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Irreversible: the service and its data cannot be recovered.
  Deletes from any state, including running — no stop first, unlike `cloud service delete`.")]
    Delete {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },

    /// Manage Postgres CA certificates
    #[command(subcommand)]
    Certs(CertsCommands),

    /// Manage Postgres runtime configuration
    #[command(
        subcommand,
        after_help = "\
CONTEXT FOR AGENTS:
  `replace` sends the whole object — start from `config get --json` output, not a fragment.
  `patch --set` only touches pgConfig; use --file to patch pgBouncerConfig.
  Exit 0 from `patch` or `replace` means accepted; confirm stored values with `config get`.
  If the response requires a restart, run `cloud postgres restart <id>` (or --name <name>),
  then reconnect and use `SHOW <setting>` to confirm a Postgres setting is active."
    )]
    Config(ConfigCommands),

    /// Reset the Postgres service password
    #[command(
        group(ArgGroup::new("password_source").required(true).args(["password", "generate"])),
        after_help = "\
CONTEXT FOR AGENTS:
  The response contains no endpoint; use `cloud postgres get <id>` for host and username.
  Prefer a psql password prompt; if a URI embeds the password, percent-encode it first."
    )]
    ResetPassword {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// New password (min 12, must include upper, lower, digit)
        #[arg(long, conflicts_with = "generate")]
        password: Option<String>,
        /// Generate a random compliant password and print it once
        #[arg(long, conflicts_with = "password")]
        generate: bool,
    },

    /// Manage Postgres read replicas
    #[command(name = "read-replica", subcommand)]
    ReadReplica(ReadReplicaCommands),

    /// Get Postgres service metrics
    Metrics {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// Start time (RFC 3339, at most millisecond precision)
        #[arg(long, value_parser = parse_postgres_window_datetime)]
        from_date: String,
        /// End time (RFC 3339, at most millisecond precision)
        #[arg(long, value_parser = parse_postgres_window_datetime)]
        to_date: String,
        /// Time bucket size in seconds; omit to let the API choose
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        bucket_size_seconds: Option<i64>,
    },

    /// Inspect Postgres slow query patterns
    #[command(name = "slow-queries", subcommand)]
    SlowQueries(SlowQueryCommands),

    /// Get raw Postgres Prometheus metrics
    #[command(
        subcommand,
        after_help = "\
CONTEXT FOR AGENTS:
  Human output is raw Prometheus exposition text.
  --json, including automatic coding-agent mode, returns that text as a JSON string."
    )]
    Prometheus(PrometheusCommands),

    /// Inspect Postgres service backups
    #[command(subcommand)]
    Backup(PostgresBackupCommands),

    /// Restore a Postgres service to a point in time
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Creates a NEW service from the source's backups; the source is untouched.
  --restore-target must fall inside the source's backup retention window.
  Copies the source's current IPv4 access rules unless --ip-allow or --block-public-access is given.
  The response has no password; use its ID with `get`, then reset it if no valid password is known.
  Do not assume the source's current password matches the historical restore.")]
    Restore {
        /// Source Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: SourceSelector,
        /// Name for the restored service
        #[arg(long)]
        name: String,
        /// Point-in-time target (ISO 8601 / RFC 3339, e.g. 2026-04-16T12:00:00Z)
        #[arg(long, value_parser = parse_datetime)]
        restore_target: String,
        /// Resource tag (repeatable), e.g. --tag env=prod
        #[arg(long)]
        tag: Vec<String>,
        /// Path to a JSON file with a PgConfig object
        #[arg(long)]
        pg_config_file: Option<PathBuf>,
        /// JSON file of PgBouncer parameters with string values
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
        /// Allowed IPv4 address or CIDR and ports, replacing the source's rules (repeatable)
        ///
        /// PORTS is a port or inclusive range, e.g. 5432 (Postgres), 6432
        /// (PgBouncer) or 5432..6432.
        #[arg(
            long = "ip-allow",
            value_name = "IP_OR_CIDR:PORTS[=DESCRIPTION]",
            value_parser = parse_postgres_ip_access_entry
        )]
        ip_allow: Vec<PostgresIpAccessListEntry>,
        /// Restore with no access rules, blocking public access
        #[arg(long, conflicts_with = "ip_allow")]
        block_public_access: bool,
    },

    /// Restart a Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Before restarting, record `SELECT pg_postmaster_start_time()` through psql.
  Exit 0 means accepted, not completed; state=running or readiness alone does not prove a restart.
  After connections recover, run it again; a later timestamp confirms a restart after the baseline.")]
    Restart {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },

    /// Promote a read replica to primary
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Exit 0 means accepted, not completed; promotion is asynchronous.
  The replica becomes an independent primary service; its source remains primary.
  Poll `cloud postgres get <replica-id> --json` until isPrimary=true.")]
    Promote {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },

    /// Switch a primary over to an HA standby
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Exit 0 means accepted, not completed; switchover is asynchronous.
  The standby takes over inside the same HA service.
  Service isPrimary and state, plus database readiness, do not identify the active HA node.
  Monitor client connectivity separately; these service fields expose no completion check.")]
    Switchover {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum CertsCommands {
    /// Get the CA certificate bundle (PEM) for a Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Prints raw PEM to stdout; --json wraps it as {\"certificate\": ...}, and coding agents
  get --json automatically — use --output to write a usable file.")]
    Get {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// Write PEM to the given file (mode 0600 on unix) instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get the runtime configuration (pgConfig + pgBouncerConfig)
    Get {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },
    /// Replace the whole runtime configuration from a file
    Replace {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// JSON file with a complete PostgresInstanceConfig object
        ///
        /// Not a fragment: use a document obtained from `config get --json`.
        #[arg(long, value_name = "PATH")]
        file: PathBuf,
    },
    /// Change selected runtime configuration fields
    #[command(
        group(ArgGroup::new("patch_source").required(true).args(["sets", "file"]))
    )]
    Patch {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
        /// Set a pgConfig field (repeatable), e.g. --set max_connections=500
        ///
        /// Values parse as JSON first, falling back to a string
        /// (statement_timeout=5s). Last value wins on a duplicate key.
        #[arg(long = "set", conflicts_with = "file")]
        sets: Vec<String>,
        /// JSON file with explicit pgConfig and pgBouncerConfig objects
        #[arg(long, value_name = "PATH", conflicts_with = "sets")]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ReadReplicaCommands {
    /// Create a read replica of an existing Postgres service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The replica inherits the source's provider, region, size and version.
  The response has no password; use its ID with `get` for the replica endpoint.
  Do not assume the source's password works on the replica.
  Next: `cloud postgres promote <replica-id>` to make it primary.")]
    Create {
        /// Source Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: SourceSelector,
        /// Name for the new replica
        #[arg(long)]
        name: String,
        /// Resource tag (repeatable), e.g. --tag env=prod
        #[arg(long)]
        tag: Vec<String>,
        /// Path to a JSON file with a PgConfig object
        #[arg(long)]
        pg_config_file: Option<PathBuf>,
        /// JSON file of PgBouncer parameters with string values
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum PostgresBackupCommands {
    /// List Postgres service backups
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Restore targets a point in time, not a backup key: `cloud postgres restore --restore-target`.")]
    List {
        /// Postgres service ID (from `cloud postgres list`)
        postgres_id: String,
        /// Cursor from nextCursor
        #[arg(long)]
        cursor: Option<String>,
        /// Maximum records per page (1–100)
        #[arg(long, allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..=100))]
        limit: Option<i64>,
    },
}

#[derive(Subcommand)]
pub enum PrometheusCommands {
    /// Get metrics for one Postgres service
    Service {
        /// Postgres service ID (from `cloud postgres list`)
        #[command(flatten)]
        postgres_id: NameSelector,
    },
    /// Get metrics for all Postgres services in an organization
    Org,
}

#[derive(Subcommand)]
pub enum SlowQueryCommands {
    /// List slow query patterns
    List {
        /// Postgres service ID (from `cloud postgres list`)
        postgres_id: String,
        /// Inclusive start time (RFC 3339, at most millisecond precision)
        #[arg(long, value_parser = parse_postgres_window_datetime)]
        from_date: String,
        /// Exclusive end time (RFC 3339, at most millisecond precision)
        #[arg(long, value_parser = parse_postgres_window_datetime)]
        to_date: String,
        /// Filter by database name
        #[arg(long)]
        db_name: Option<String>,
        /// Filter by database user
        #[arg(long)]
        db_user: Option<String>,
        /// Filter by database operation
        #[arg(long)]
        db_operation: Option<String>,
        /// Filter by application name
        #[arg(long)]
        app: Option<String>,
        /// Field used to sort results
        #[arg(
            long,
            value_parser = clap::builder::PossibleValuesParser::new(
                SlowQueryPatternsGetListSortby::VALUES
            )
        )]
        sort_by: Option<String>,
        /// Sort order
        #[arg(
            long,
            value_parser = clap::builder::PossibleValuesParser::new(
                SlowQueryPatternsGetListSortorder::VALUES
            )
        )]
        sort_order: Option<String>,
        /// Maximum number of patterns to return
        #[arg(long, allow_hyphen_values = true, value_parser = clap::value_parser!(i64).range(1..=500))]
        limit: Option<i64>,
        /// Number of patterns to skip
        #[arg(
            long,
            allow_hyphen_values = true,
            value_parser = clap::value_parser!(i64).range(0..)
        )]
        offset: Option<i64>,
    },
    /// Get a slow query pattern with recent executions
    Get {
        /// Postgres service ID (from `cloud postgres list`)
        postgres_id: String,
        /// Stable query pattern ID (from `slow-queries list`)
        query_id: String,
        /// Database name from the list result
        #[arg(long)]
        db_name: String,
        /// Database user from the list result
        #[arg(long)]
        db_user: String,
        /// Database operation from the list result
        #[arg(long)]
        db_operation: String,
        /// Application name from the list result
        #[arg(long)]
        app: Option<String>,
        /// Timestamp of a specific execution (ISO 8601 / RFC 3339)
        #[arg(long, value_parser = parse_datetime)]
        timestamp: Option<String>,
    },
}

impl PostgresCommands {
    pub fn is_write(&self) -> bool {
        match self {
            PostgresCommands::List { .. }
            | PostgresCommands::Get { .. }
            | PostgresCommands::Query { .. }
            | PostgresCommands::Metrics { .. }
            | PostgresCommands::Logs { .. }
            | PostgresCommands::SlowQueries(_)
            | PostgresCommands::Prometheus(_) => false,
            PostgresCommands::Backup(PostgresBackupCommands::List { .. }) => false,
            PostgresCommands::Certs(CertsCommands::Get { .. }) => false,
            PostgresCommands::Config(ConfigCommands::Get { .. }) => false,

            PostgresCommands::Create { .. }
            | PostgresCommands::Update { .. }
            | PostgresCommands::Delete { .. }
            | PostgresCommands::ResetPassword { .. }
            | PostgresCommands::Restore { .. }
            | PostgresCommands::Restart { .. }
            | PostgresCommands::Promote { .. }
            | PostgresCommands::Switchover { .. } => true,
            PostgresCommands::Config(ConfigCommands::Replace { .. })
            | PostgresCommands::Config(ConfigCommands::Patch { .. }) => true,
            PostgresCommands::ReadReplica(ReadReplicaCommands::Create { .. }) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::cloud::postgres::lifecycle::tests::access_rule;
    use clap::Parser;
    use std::path::Path;

    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "postgres", "config", "replace", "pg-1"],
            "file",
            &[],
        );
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "postgres", "config", "patch", "pg-1"],
            "file",
            &[],
        );
    }

    #[derive(Parser)]
    struct PostgresCli {
        #[command(subcommand)]
        command: PostgresCommands,
    }

    fn parse_postgres(args: &[&str]) -> PostgresCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let crate::cli::Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud, args);
        let crate::cloud::cli::CloudCommands::Postgres { command } = cloud.command else {
            panic!("expected postgres command");
        };
        command
    }

    #[test]
    fn parses_backup_list_pagination_as_read() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "backup",
            "list",
            "pg-1",
        ]);
        assert!(!cmd.is_write());
        let PostgresCommands::Backup(PostgresBackupCommands::List {
            postgres_id,
            cursor,
            limit,
        }) = cmd
        else {
            panic!("expected backup list");
        };
        assert_eq!(postgres_id, "pg-1");
        assert!(cursor.is_none());
        assert!(limit.is_none());

        for limit_value in ["1", "100"] {
            let cmd = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "backup",
                "list",
                "pg-1",
                "--cursor",
                "next+/=",
                "--limit",
                limit_value,
                "--org-id",
                "org-1",
            ]);
            assert!(!cmd.is_write());
            let PostgresCommands::Backup(PostgresBackupCommands::List { cursor, limit, .. }) = cmd
            else {
                panic!("expected backup list");
            };
            assert_eq!(cursor.as_deref(), Some("next+/="));
            assert_eq!(limit, Some(limit_value.parse().unwrap()));
        }
    }

    #[test]
    fn backup_list_requires_an_id_and_bounds_limit() {
        let base = ["clickhousectl", "cloud", "postgres", "backup", "list"];
        assert_eq!(
            Cli::try_parse_from(base).err().unwrap().kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        for value in ["-1", "0", "101", "many"] {
            let mut args = base.to_vec();
            args.extend(["pg-1", "--limit", value]);
            assert_eq!(
                Cli::try_parse_from(args).err().unwrap().kind(),
                clap::error::ErrorKind::ValueValidation,
                "{value}"
            );
        }
        // The group itself is not executable without a leaf.
        assert!(Cli::try_parse_from(["clickhousectl", "cloud", "postgres", "backup"]).is_err());
    }

    #[test]
    fn parses_postgres_list_with_filters() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "list",
            "--filter",
            "state=running",
            "--filter",
            "region=us-east-1",
            "--filter",
            "isPrimary=false",
        ]);
        let PostgresCommands::List { filter, .. } = cmd else {
            panic!("expected list");
        };
        assert_eq!(
            filter,
            vec![
                PostgresListFilter::State("running".to_string()),
                PostgresListFilter::Region("us-east-1".to_string()),
                PostgresListFilter::IsPrimary(false),
            ]
        );
    }

    fn list_filter_error(filter: &str) -> String {
        let err = PostgresCli::try_parse_from(["clickhousectl", "list", "--filter", filter])
            .err()
            .expect("expected parse error");
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
        assert_eq!(err.exit_code(), 2, "filter errors must be usage errors");
        err.to_string()
    }

    #[test]
    fn rejects_an_unknown_postgres_list_filter_key() {
        let message = list_filter_error("bogus=1");
        assert!(message.contains("unknown filter key 'bogus'"), "{message}");
        // The valid keys are listed so the user can correct the invocation.
        for key in ["state", "region", "name", "provider", "isPrimary"] {
            assert!(message.contains(key), "{key} missing from: {message}");
        }
    }

    #[test]
    fn rejects_a_postgres_list_filter_without_a_value() {
        let message = list_filter_error("state=");
        assert!(
            message.contains("filter 'state' requires a value"),
            "{message}"
        );
        // Whitespace-only is empty too.
        let message = list_filter_error("name=   ");
        assert!(
            message.contains("filter 'name' requires a value"),
            "{message}"
        );
    }

    #[test]
    fn rejects_a_postgres_list_filter_without_an_equals_sign() {
        let message = list_filter_error("state");
        assert!(
            message.contains("invalid filter 'state': expected KEY=VALUE"),
            "{message}"
        );
    }

    #[test]
    fn rejects_a_non_boolean_is_primary_postgres_list_filter() {
        let message = list_filter_error("isPrimary=maybe");
        assert!(
            message.contains("invalid value 'maybe' for filter 'isPrimary'"),
            "{message}"
        );
    }

    #[test]
    fn parses_postgres_get() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "get", "pg-1"]);
        let PostgresCommands::Get { postgres_id, .. } = cmd else {
            panic!("expected get");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
    }

    #[test]
    fn parses_postgres_query_sources_database_and_selectors() {
        let minimal = parse_postgres(&["clickhousectl", "cloud", "postgres", "query", "pg-1"]);
        assert!(!minimal.is_write());
        let PostgresCommands::Query {
            postgres_id,
            query,
            queries_file,
            database,
        } = minimal
        else {
            panic!("expected query");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert!(query.is_none() && queries_file.is_none() && database.is_none());

        for flag in ["--query", "-q"] {
            let command = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "query",
                "--name",
                "analytics",
                flag,
                "SELECT 1",
                "--database",
                "reporting",
            ]);
            assert!(!command.is_write());
            let PostgresCommands::Query {
                postgres_id,
                query,
                queries_file,
                database,
            } = command
            else {
                panic!("expected query");
            };
            assert_eq!(postgres_id.name.as_deref(), Some("analytics"));
            assert_eq!(query.as_deref(), Some("SELECT 1"));
            assert_eq!(database.as_deref(), Some("reporting"));
            assert!(queries_file.is_none());
        }
        for path in ["queries.sql", "-"] {
            let command = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "query",
                "pg-1",
                "--queries-file",
                path,
            ]);
            let PostgresCommands::Query { queries_file, .. } = command else {
                panic!("expected query")
            };
            assert_eq!(queries_file.as_deref(), Some(Path::new(path)));
        }
    }

    #[test]
    fn postgres_query_rejects_conflicting_sources_and_selectors() {
        for arguments in [
            vec!["pg-1", "--query", "SELECT 1", "--queries-file", "-"],
            vec!["pg-1", "--name", "analytics"],
            vec!["pg-1", "--query", "SELECT 1", "--query", "SELECT 2"],
        ] {
            let error = Cli::try_parse_from(
                ["clickhousectl", "cloud", "postgres", "query"]
                    .into_iter()
                    .chain(arguments),
            )
            .err()
            .expect("conflicting arguments must fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
        let error = Cli::try_parse_from(["clickhousectl", "cloud", "postgres", "query"])
            .err()
            .expect("selector is required");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn parses_postgres_logs_minimal() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "logs",
            "pg-1",
            "--from-date",
            "2026-08-01T00:00:00Z",
            "--to-date",
            "2026-08-02T00:00:00Z",
        ]);
        let PostgresCommands::Logs {
            postgres_id,
            from_date,
            to_date,
            body_contains,
            severity,
            sort_order,
            limit,
            offset,
            ..
        } = &cmd
        else {
            panic!("expected logs");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert_eq!(from_date, "2026-08-01T00:00:00Z");
        assert_eq!(to_date, "2026-08-02T00:00:00Z");
        assert_eq!(body_contains, &None);
        assert_eq!(severity, &None);
        assert_eq!(sort_order, &None);
        assert_eq!(limit, &None);
        assert_eq!(offset, &None);
        assert!(!cmd.is_write());
    }

    #[test]
    fn parses_postgres_logs_maximal() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "logs",
            "pg-1",
            "--from-date",
            "2026-08-01T00:00:00+01:00",
            "--to-date",
            "2026-08-02T00:00:00+01:00",
            "--body-contains",
            "checkpoint complete",
            "--severity",
            "LOG",
            "--sort-order",
            "asc",
            "--limit",
            "2000",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ]);
        let PostgresCommands::Logs {
            body_contains,
            severity,
            sort_order,
            limit,
            offset,
            ..
        } = cmd
        else {
            panic!("expected logs");
        };
        assert_eq!(body_contains.as_deref(), Some("checkpoint complete"));
        assert_eq!(severity.as_deref(), Some("LOG"));
        assert_eq!(sort_order, Some(PostgresLogsGetListSortorder::Asc));
        assert_eq!(limit, Some(2000));
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn postgres_logs_sort_order_exposes_every_supported_value() {
        use clap::CommandFactory;

        let command = PostgresCli::command();
        let sort_order = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == "logs")
            .and_then(|logs| {
                logs.get_arguments()
                    .find(|arg| arg.get_id() == "sort_order")
            })
            .expect("logs --sort-order argument");
        let possible_values: Vec<_> = sort_order
            .get_possible_values()
            .into_iter()
            .map(|value| value.get_name().to_string())
            .collect();
        assert_eq!(possible_values, POSTGRES_LOG_SORT_ORDERS);

        for &value in POSTGRES_LOG_SORT_ORDERS {
            let cmd = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "logs",
                "pg-1",
                "--from-date",
                "2026-08-01T00:00:00Z",
                "--to-date",
                "2026-08-02T00:00:00Z",
                "--sort-order",
                value,
            ]);
            let PostgresCommands::Logs { sort_order, .. } = cmd else {
                panic!("expected logs");
            };
            assert_eq!(
                sort_order.as_ref().map(ToString::to_string).as_deref(),
                Some(value)
            );
        }
    }

    #[test]
    fn rejects_invalid_postgres_logs_flag_values() {
        let invalid_datetime = PostgresCli::try_parse_from([
            "clickhousectl",
            "logs",
            "pg-1",
            "--from-date",
            "yesterday",
            "--to-date",
            "2026-08-02T00:00:00Z",
        ])
        .err()
        .expect("expected invalid datetime");
        assert_eq!(
            invalid_datetime.kind(),
            clap::error::ErrorKind::ValueValidation
        );

        for (flag, value, expected_kind) in [
            (
                "--sort-order",
                "newest",
                clap::error::ErrorKind::InvalidValue,
            ),
            ("--limit", "-1", clap::error::ErrorKind::ValueValidation),
            ("--limit", "0", clap::error::ErrorKind::ValueValidation),
            ("--limit", "2001", clap::error::ErrorKind::ValueValidation),
            ("--offset", "-1", clap::error::ErrorKind::ValueValidation),
        ] {
            let mut args = vec![
                "clickhousectl",
                "logs",
                "pg-1",
                "--from-date",
                "2026-08-01T00:00:00Z",
                "--to-date",
                "2026-08-02T00:00:00Z",
            ];
            args.extend([flag, value]);
            let error = PostgresCli::try_parse_from(args)
                .err()
                .expect("expected invalid logs option");
            assert_eq!(error.kind(), expected_kind);
        }
    }

    #[test]
    fn parses_postgres_create_minimal() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "m7i.2xlarge",
        ]);
        let PostgresCommands::Create {
            name,
            region,
            size,
            provider,
            pg_version,
            ha_type,
            ..
        } = cmd
        else {
            panic!("expected create");
        };
        assert_eq!(name, "pg1");
        assert_eq!(region, "us-east-1");
        assert_eq!(size, "m7i.2xlarge");
        assert_eq!(provider, "aws");
        assert!(pg_version.is_none());
        assert!(ha_type.is_none());
    }

    #[test]
    fn postgres_create_provider_uses_the_api_value_set() {
        for provider in PgProvider::VALUES {
            let cmd = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "create",
                "--name",
                "pg1",
                "--region",
                "us-east-1",
                "--size",
                "m7i.2xlarge",
                "--provider",
                provider,
            ]);
            let PostgresCommands::Create {
                provider: parsed, ..
            } = cmd
            else {
                panic!("expected create");
            };
            assert_eq!(parsed, *provider);
        }

        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "m7i.2xlarge",
            "--provider",
            "azure",
        ])
        .err()
        .expect("invalid provider must be rejected by clap");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        assert_eq!(error.exit_code(), 2);
        let message = error.to_string();
        for provider in PgProvider::VALUES {
            assert!(
                message.contains(provider),
                "missing `{provider}`: {message}"
            );
        }
    }

    #[test]
    fn parses_postgres_create_with_all_flags() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "m7i.2xlarge",
            "--pg-version",
            "17",
            "--ha-type",
            "sync",
            "--tag",
            "env=prod",
            "--tag",
            "owner=data",
        ]);
        let PostgresCommands::Create {
            pg_version,
            ha_type,
            tag,
            ..
        } = cmd
        else {
            panic!("expected create");
        };
        assert_eq!(pg_version.as_deref(), Some("17"));
        assert_eq!(ha_type.as_deref(), Some("sync"));
        assert_eq!(tag, vec!["env=prod", "owner=data"]);
    }

    #[test]
    fn rejects_postgres_create_missing_required() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            // missing --size
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("--size"));
    }

    #[test]
    fn rejects_postgres_create_invalid_pg_version() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "m7i.2xlarge",
            "--pg-version",
            "15",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("invalid value"));
    }

    #[test]
    fn rejects_postgres_create_pg_version_16() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "create",
            "--name",
            "pg1",
            "--region",
            "us-east-1",
            "--size",
            "m7i.2xlarge",
            "--pg-version",
            "16",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("invalid value"));
    }

    #[test]
    fn parses_postgres_update_tag_diff_flags() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "update",
            "pg-1",
            "--size",
            "c6gd.large",
            "--add-tag",
            "env=prod",
            "--add-tag",
            "team=data",
            "--remove-tag",
            "old",
        ]);
        let PostgresCommands::Update {
            postgres_id,
            size,
            add_tag,
            remove_tag,
            clear_tags,
            ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert_eq!(size.as_deref(), Some("c6gd.large"));
        assert_eq!(add_tag, vec!["env=prod", "team=data"]);
        assert_eq!(remove_tag, vec!["old"]);
        assert!(!clear_tags);
    }

    #[test]
    fn parses_postgres_update_no_fields() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "update", "pg-1"]);
        let PostgresCommands::Update {
            postgres_id,
            name,
            size,
            add_tag,
            remove_tag,
            clear_tags,
            ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert!(name.is_none());
        assert!(size.is_none());
        assert!(add_tag.is_empty());
        assert!(remove_tag.is_empty());
        assert!(!clear_tags);
    }

    #[test]
    fn parses_postgres_update_clear_tags() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "update",
            "pg-1",
            "--clear-tags",
        ]);
        let PostgresCommands::Update {
            add_tag,
            remove_tag,
            clear_tags,
            ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert!(add_tag.is_empty());
        assert!(remove_tag.is_empty());
        assert!(clear_tags);
    }

    #[test]
    fn rejects_conflicting_postgres_tag_changes() {
        for flags in [
            ["--add-tag", "env=prod", "--clear-tags"],
            ["--clear-tags", "--add-tag", "env=prod"],
            ["--remove-tag", "env", "--clear-tags"],
            ["--clear-tags", "--remove-tag", "env"],
        ] {
            let result = Cli::try_parse_from(
                ["clickhousectl", "cloud", "postgres", "update", "pg-1"]
                    .into_iter()
                    .chain(flags),
            );
            let Err(error) = result else {
                panic!("tag diff and clear flags must conflict");
            };
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    const CREATE_ARGS: [&str; 10] = [
        "clickhousectl",
        "cloud",
        "postgres",
        "create",
        "--name",
        "pg1",
        "--region",
        "us-east-1",
        "--size",
        "m7i.2xlarge",
    ];

    #[test]
    fn parses_postgres_create_access_rule_flags() {
        let PostgresCommands::Create {
            ip_allow,
            block_public_access,
            ..
        } = parse_postgres(&CREATE_ARGS)
        else {
            panic!("expected create");
        };
        assert!(ip_allow.is_empty());
        assert!(!block_public_access);

        let args: Vec<&str> = CREATE_ARGS
            .into_iter()
            .chain([
                "--ip-allow",
                "203.0.113.0/24:5432=office",
                "--ip-allow",
                "198.51.100.7:5432..6432",
            ])
            .collect();
        let PostgresCommands::Create {
            ip_allow,
            block_public_access,
            ..
        } = parse_postgres(&args)
        else {
            panic!("expected create");
        };
        assert_eq!(
            ip_allow,
            vec![
                access_rule("203.0.113.0/24", "5432", Some("office")),
                access_rule("198.51.100.7", "5432..6432", None),
            ]
        );
        assert!(!block_public_access);

        let args: Vec<&str> = CREATE_ARGS
            .into_iter()
            .chain(["--block-public-access"])
            .collect();
        let PostgresCommands::Create {
            ip_allow,
            block_public_access,
            ..
        } = parse_postgres(&args)
        else {
            panic!("expected create");
        };
        assert!(ip_allow.is_empty());
        assert!(block_public_access);
    }

    #[test]
    fn rejects_postgres_create_access_rule_conflicts_and_bad_rules() {
        for flags in [
            &["--ip-allow", "203.0.113.7:5432", "--block-public-access"][..],
            &["--block-public-access", "--ip-allow", "203.0.113.7:5432"][..],
        ] {
            let error = Cli::try_parse_from(CREATE_ARGS.into_iter().chain(flags.iter().copied()))
                .err()
                .expect("--ip-allow and --block-public-access must conflict");
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
        for rule in ["203.0.113.7", "2001:db8::/32:5432", "203.0.113.7:70000"] {
            let error = Cli::try_parse_from(CREATE_ARGS.into_iter().chain(["--ip-allow", rule]))
                .err()
                .expect("malformed rule must be rejected by clap");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert_eq!(error.exit_code(), 2);
        }
    }

    #[test]
    fn parses_postgres_update_access_rule_flags() {
        let PostgresCommands::Update {
            add_ip_allow,
            remove_ip_allow,
            ..
        } = parse_postgres(&["clickhousectl", "cloud", "postgres", "update", "pg-1"])
        else {
            panic!("expected update");
        };
        assert!(add_ip_allow.is_empty());
        assert!(remove_ip_allow.is_empty());

        let PostgresCommands::Update {
            add_ip_allow,
            remove_ip_allow,
            clear_tags,
            ..
        } = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "update",
            "pg-1",
            "--add-ip-allow",
            "203.0.113.0/24:5432",
            "--remove-ip-allow",
            "rule-1",
            "--add-ip-allow",
            "203.0.113.0/24:6432=pooler",
            "--remove-ip-allow",
            "rule-2",
            "--clear-tags",
        ])
        else {
            panic!("expected update");
        };
        assert_eq!(
            add_ip_allow,
            vec![
                access_rule("203.0.113.0/24", "5432", None),
                access_rule("203.0.113.0/24", "6432", Some("pooler")),
            ]
        );
        assert_eq!(remove_ip_allow, vec!["rule-1", "rule-2"]);
        assert!(clear_tags);

        for flags in [["--add-ip-allow", "203.0.113.7"], ["--remove-ip-allow", ""]] {
            let error = Cli::try_parse_from(
                ["clickhousectl", "cloud", "postgres", "update", "pg-1"]
                    .into_iter()
                    .chain(flags),
            )
            .err()
            .expect("malformed access rule flag must be rejected by clap");
            assert_eq!(error.exit_code(), 2, "{flags:?}");
        }
    }

    #[test]
    fn parses_postgres_update_name() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "update",
            "pg-1",
            "--new-name",
            "renamed-pg",
        ]);
        let PostgresCommands::Update {
            postgres_id, name, ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert_eq!(name.as_deref(), Some("renamed-pg"));
    }

    #[test]
    fn parses_postgres_delete() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "delete", "pg-1"]);
        let PostgresCommands::Delete { postgres_id, .. } = cmd else {
            panic!("expected delete");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
    }

    #[test]
    fn parses_postgres_certs_get_stdout_and_output() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "certs", "get", "pg-1"]);
        let PostgresCommands::Certs(CertsCommands::Get { output, .. }) = cmd else {
            panic!("expected certs get");
        };
        assert!(output.is_none());

        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "certs",
            "get",
            "pg-1",
            "--output",
            "/tmp/ca.pem",
        ]);
        let PostgresCommands::Certs(CertsCommands::Get { output, .. }) = cmd else {
            panic!("expected certs get");
        };
        assert_eq!(output, Some(PathBuf::from("/tmp/ca.pem")));
    }

    #[test]
    fn parses_postgres_config_get() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "get",
            "pg-1",
        ]);
        assert!(matches!(
            cmd,
            PostgresCommands::Config(ConfigCommands::Get { .. })
        ));
    }

    #[test]
    fn parses_postgres_config_replace_requires_file() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "replace",
            "pg-1",
            "--file",
            "/tmp/cfg.json",
        ]);
        let PostgresCommands::Config(ConfigCommands::Replace { file, .. }) = cmd else {
            panic!("expected replace");
        };
        assert_eq!(file, PathBuf::from("/tmp/cfg.json"));

        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "replace",
            "pg-1",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("--file"));
    }

    #[test]
    fn parses_postgres_config_patch_with_set_entries() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--set",
            "max_connections=500",
            "--set",
            "random_page_cost=1.1",
        ]);
        let PostgresCommands::Config(ConfigCommands::Patch { sets, file, .. }) = cmd else {
            panic!("expected patch");
        };
        assert_eq!(sets, vec!["max_connections=500", "random_page_cost=1.1"]);
        assert!(file.is_none());
    }

    #[test]
    fn parses_postgres_config_patch_with_file() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--file",
            "/tmp/p.json",
        ]);
        let PostgresCommands::Config(ConfigCommands::Patch { sets, file, .. }) = cmd else {
            panic!("expected patch");
        };
        assert!(sets.is_empty());
        assert_eq!(file, Some(PathBuf::from("/tmp/p.json")));
    }

    #[test]
    fn rejects_postgres_config_patch_set_and_file_together() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "patch",
            "pg-1",
            "--set",
            "max_connections=500",
            "--file",
            "/tmp/p.json",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("cannot be used"));
    }

    #[test]
    fn rejects_postgres_config_patch_without_set_or_file() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "config",
            "patch",
            "pg-1",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn parses_postgres_reset_password_with_password_and_generate() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "reset-password",
            "pg-1",
            "--password",
            "Hunter2345678",
        ]);
        let PostgresCommands::ResetPassword {
            password, generate, ..
        } = cmd
        else {
            panic!("expected reset-password");
        };
        assert_eq!(password.as_deref(), Some("Hunter2345678"));
        assert!(!generate);

        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "reset-password",
            "pg-1",
            "--generate",
        ]);
        let PostgresCommands::ResetPassword {
            password, generate, ..
        } = cmd
        else {
            panic!("expected reset-password");
        };
        assert!(password.is_none());
        assert!(generate);
    }

    #[test]
    fn rejects_postgres_reset_password_without_password_or_generate() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "reset-password",
            "pg-1",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        let message = err.to_string();
        assert!(message.contains("--password <PASSWORD>"), "{message}");
        assert!(message.contains("--generate"), "{message}");
    }

    #[test]
    fn rejects_postgres_reset_password_both() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "reset-password",
            "pg-1",
            "--password",
            "abc",
            "--generate",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("cannot be used"));
    }

    #[test]
    fn parses_postgres_restore_valid_rfc3339() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "restore",
            "pg-1",
            "--name",
            "restored",
            "--restore-target",
            "2026-04-16T12:00:00Z",
        ]);
        let PostgresCommands::Restore {
            name,
            restore_target,
            ..
        } = cmd
        else {
            panic!("expected restore");
        };
        assert_eq!(name, "restored");
        assert_eq!(restore_target, "2026-04-16T12:00:00Z");
    }

    #[test]
    fn parses_postgres_restore_access_rule_flags() {
        let base = [
            "clickhousectl",
            "cloud",
            "postgres",
            "restore",
            "pg-1",
            "--name",
            "restored",
            "--restore-target",
            "2026-04-16T12:00:00Z",
        ];
        let restore = |extra: &[&str]| {
            let args: Vec<&str> = base.iter().chain(extra).copied().collect();
            let PostgresCommands::Restore {
                ip_allow,
                block_public_access,
                ..
            } = parse_postgres(&args)
            else {
                panic!("expected restore");
            };
            (ip_allow, block_public_access)
        };

        assert_eq!(restore(&[]), (Vec::new(), false));
        assert_eq!(restore(&["--block-public-access"]), (Vec::new(), true));
        assert_eq!(
            restore(&[
                "--ip-allow",
                "203.0.113.0/24:5432",
                "--ip-allow",
                "198.51.100.7:6432=pooler",
            ]),
            (
                vec![
                    access_rule("203.0.113.0/24", "5432", None),
                    access_rule("198.51.100.7", "6432", Some("pooler")),
                ],
                false
            )
        );

        let args: Vec<&str> = base
            .iter()
            .copied()
            .chain(["--ip-allow", "203.0.113.0/24:5432", "--block-public-access"])
            .collect();
        let err = Cli::try_parse_from(args).err().expect("flags conflict");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn rejects_postgres_restore_invalid_datetime() {
        let err = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "restore",
            "pg-1",
            "--name",
            "restored",
            "--restore-target",
            "yesterday",
        ])
        .err()
        .expect("expected parse error");
        assert!(err.to_string().contains("invalid datetime"));
    }

    #[test]
    fn parses_postgres_metrics_with_all_query_inputs() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--bucket-size-seconds",
            "60",
            "--org-id",
            "org-1",
        ]);
        assert!(!cmd.is_write());
        let PostgresCommands::Metrics {
            postgres_id,
            from_date,
            to_date,
            bucket_size_seconds,
        } = cmd
        else {
            panic!("expected metrics");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert_eq!(from_date, "2026-04-16T11:00:00.000Z");
        assert_eq!(to_date, "2026-04-16T12:00:00.000Z");
        assert_eq!(bucket_size_seconds, Some(60));
    }

    #[test]
    fn postgres_metrics_requires_dates_without_defaulting_bucket_size() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
        ]);
        let PostgresCommands::Metrics {
            bucket_size_seconds,
            ..
        } = cmd
        else {
            panic!("expected metrics");
        };
        assert_eq!(bucket_size_seconds, None);

        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "metrics",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn postgres_metrics_rejects_invalid_dates_and_bucket_sizes() {
        for (flag, value) in [
            ("--from-date", "yesterday"),
            ("--to-date", "tomorrow"),
            ("--from-date", "2026-04-16T12:00:00.123456Z"),
            ("--to-date", "2026-04-16T13:00:00.123456789Z"),
            ("--from-date", "2026-04-16T12:00:00.0000000001Z"),
            ("--bucket-size-seconds", "0"),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "postgres",
                "metrics",
                "pg-1",
                "--from-date",
                "2026-04-16T12:00:00Z",
                "--to-date",
                "2026-04-16T13:00:00Z",
            ];
            let position = args.iter().position(|arg| *arg == flag);
            if let Some(position) = position {
                args[position + 1] = value;
            } else {
                args.extend([flag, value]);
            }
            let error = Cli::try_parse_from(args)
                .err()
                .expect("expected parse error");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn parses_slow_query_list_with_all_query_inputs_as_read() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00+01:00",
            "--to-date",
            "2026-04-16T13:00:00+01:00",
            "--db-name",
            "app db",
            "--db-user",
            "reader+worker",
            "--db-operation",
            "SELECT",
            "--app",
            "reporting/api",
            "--sort-by",
            "p95_duration",
            "--sort-order",
            "asc",
            "--limit",
            "500",
            "--offset",
            "0",
            "--org-id",
            "org-1",
        ]);
        assert!(!cmd.is_write());
        let PostgresCommands::SlowQueries(SlowQueryCommands::List {
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
        }) = cmd
        else {
            panic!("expected slow-query list");
        };
        assert_eq!(postgres_id, "pg-1");
        assert_eq!(from_date, "2026-04-16T11:00:00.000Z");
        assert_eq!(to_date, "2026-04-16T12:00:00.000Z");
        assert_eq!(db_name.as_deref(), Some("app db"));
        assert_eq!(db_user.as_deref(), Some("reader+worker"));
        assert_eq!(db_operation.as_deref(), Some("SELECT"));
        assert_eq!(app.as_deref(), Some("reporting/api"));
        assert_eq!(sort_by.as_deref(), Some("p95_duration"));
        assert_eq!(sort_order.as_deref(), Some("asc"));
        assert_eq!(limit, Some(500));
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn slow_query_list_accepts_zero_and_positive_offsets() {
        for (value, expected) in [("0", 0), ("37", 37)] {
            let cmd = parse_postgres(&[
                "clickhousectl",
                "cloud",
                "postgres",
                "slow-queries",
                "list",
                "pg-1",
                "--from-date",
                "2026-04-16T12:00:00Z",
                "--to-date",
                "2026-04-16T13:00:00Z",
                "--offset",
                value,
            ]);
            let PostgresCommands::SlowQueries(SlowQueryCommands::List { offset, .. }) = cmd else {
                panic!("expected slow-query list");
            };
            assert_eq!(offset, Some(expected));
        }
    }

    #[test]
    fn slow_query_list_requires_dates_and_validates_schema_bounds() {
        let missing_date = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(
            missing_date.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );

        for (flag, value, expected_kind) in [
            (
                "--from-date",
                "yesterday",
                clap::error::ErrorKind::ValueValidation,
            ),
            (
                "--to-date",
                "tomorrow",
                clap::error::ErrorKind::ValueValidation,
            ),
            (
                "--sort-by",
                "duration",
                clap::error::ErrorKind::InvalidValue,
            ),
            (
                "--sort-order",
                "sideways",
                clap::error::ErrorKind::InvalidValue,
            ),
            ("--limit", "-1", clap::error::ErrorKind::ValueValidation),
            ("--limit", "0", clap::error::ErrorKind::ValueValidation),
            ("--limit", "501", clap::error::ErrorKind::ValueValidation),
            ("--offset", "-1", clap::error::ErrorKind::ValueValidation),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "postgres",
                "slow-queries",
                "list",
                "pg-1",
                "--from-date",
                "2026-04-16T12:00:00Z",
                "--to-date",
                "2026-04-16T13:00:00Z",
            ];
            let position = args.iter().position(|arg| *arg == flag);
            if let Some(position) = position {
                args[position + 1] = value;
            } else {
                args.extend([flag, value]);
            }
            let error = Cli::try_parse_from(args)
                .err()
                .expect("expected parse error");
            assert_eq!(error.kind(), expected_kind);
        }

        let negative_offset_equals = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "list",
            "pg-1",
            "--from-date",
            "2026-04-16T12:00:00Z",
            "--to-date",
            "2026-04-16T13:00:00Z",
            "--offset=-1",
        ])
        .err()
        .expect("negative offset should fail during clap parsing");
        assert_eq!(
            negative_offset_equals.kind(),
            clap::error::ErrorKind::ValueValidation
        );
        assert_eq!(negative_offset_equals.exit_code(), 2);
    }

    #[test]
    fn negative_limits_use_range_validation_with_both_flag_forms() {
        for command in [vec!["logs"], vec!["slow-queries", "list"]] {
            for limit_args in [vec!["--limit", "-1"], vec!["--limit=-1"]] {
                let mut args = vec!["clickhousectl", "cloud", "postgres"];
                args.extend(command.iter().copied());
                args.extend([
                    "pg-1",
                    "--from-date",
                    "2026-04-16T12:00:00Z",
                    "--to-date",
                    "2026-04-16T13:00:00Z",
                ]);
                args.extend(limit_args);
                let error = Cli::try_parse_from(args).err().expect("negative limit");
                assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
                assert_eq!(error.exit_code(), 2);
            }
        }
    }

    #[test]
    fn parses_slow_query_detail_with_its_distinct_inputs_as_read() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app",
            "--db-user",
            "reader",
            "--db-operation",
            "SELECT",
            "--app",
            "reporter",
            "--timestamp",
            "2026-04-16T12:30:00Z",
            "--org-id",
            "org-1",
        ]);
        assert!(!cmd.is_write());
        let PostgresCommands::SlowQueries(SlowQueryCommands::Get {
            postgres_id,
            query_id,
            db_name,
            db_user,
            db_operation,
            app,
            timestamp,
        }) = cmd
        else {
            panic!("expected slow-query detail");
        };
        assert_eq!(postgres_id, "pg-1");
        assert_eq!(query_id, "query-1");
        assert_eq!(db_name, "app");
        assert_eq!(db_user, "reader");
        assert_eq!(db_operation, "SELECT");
        assert_eq!(app.as_deref(), Some("reporter"));
        assert_eq!(timestamp.as_deref(), Some("2026-04-16T12:30:00Z"));
    }

    #[test]
    fn slow_query_detail_requires_identity_filters_and_validates_timestamp() {
        let missing_operation = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app",
            "--db-user",
            "reader",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(
            missing_operation.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let invalid_timestamp = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "slow-queries",
            "get",
            "pg-1",
            "query-1",
            "--db-name",
            "app",
            "--db-user",
            "reader",
            "--db-operation",
            "SELECT",
            "--timestamp",
            "now",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(
            invalid_timestamp.kind(),
            clap::error::ErrorKind::ValueValidation
        );
    }

    #[test]
    fn parses_postgres_prometheus_service_and_org_scopes_as_reads() {
        let service = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "prometheus",
            "service",
            "pg-1",
            "--org-id",
            "org-1",
        ]);
        assert!(!service.is_write());
        let PostgresCommands::Prometheus(PrometheusCommands::Service { postgres_id }) = service
        else {
            panic!("expected service prometheus");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));

        let org = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "prometheus",
            "org",
            "--org-id",
            "org-2",
        ]);
        assert!(!org.is_write());
        let PostgresCommands::Prometheus(PrometheusCommands::Org) = org else {
            panic!("expected organization prometheus");
        };
    }

    #[test]
    fn postgres_prometheus_does_not_advertise_unsupported_filters() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "postgres",
            "prometheus",
            "service",
            "pg-1",
            "--filtered-metrics",
            "false",
        ])
        .err()
        .expect("expected parse error");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_postgres_read_replica_create() {
        let cmd = parse_postgres(&[
            "clickhousectl",
            "cloud",
            "postgres",
            "read-replica",
            "create",
            "pg-1",
            "--name",
            "replica1",
            "--tag",
            "role=read",
        ]);
        let PostgresCommands::ReadReplica(ReadReplicaCommands::Create {
            postgres_id,
            name,
            tag,
            ..
        }) = cmd
        else {
            panic!("expected read-replica create");
        };
        assert_eq!(postgres_id.id.as_deref(), Some("pg-1"));
        assert_eq!(name, "replica1");
        assert_eq!(tag, vec!["role=read"]);
    }

    #[test]
    fn parses_postgres_restart_promote_switchover() {
        assert!(matches!(
            parse_postgres(&["clickhousectl", "cloud", "postgres", "restart", "pg-1"]),
            PostgresCommands::Restart { .. }
        ));
        assert!(matches!(
            parse_postgres(&["clickhousectl", "cloud", "postgres", "promote", "pg-1"]),
            PostgresCommands::Promote { .. }
        ));
        assert!(matches!(
            parse_postgres(&["clickhousectl", "cloud", "postgres", "switchover", "pg-1"]),
            PostgresCommands::Switchover { .. }
        ));
    }

    // --- role change (promote / switchover) parsing, issue #834 ---

    #[test]
    fn promote_and_switchover_reject_removed_wait_flags() {
        for command in ["promote", "switchover"] {
            for args in [["--wait", ""], ["--wait-timeout", "30"]] {
                let mut argv = vec!["clickhousectl", command, "pg-1", args[0]];
                if !args[1].is_empty() {
                    argv.push(args[1]);
                }
                let err = match PostgresCli::try_parse_from(argv) {
                    Ok(_) => panic!("{command} {} must be rejected", args[0]),
                    Err(err) => err,
                };
                assert_eq!(
                    err.kind(),
                    clap::error::ErrorKind::UnknownArgument,
                    "{command} {}: {err}",
                    args[0]
                );
                assert_eq!(err.exit_code(), 2, "{command} {}: {err}", args[0]);
            }
        }
    }

    /// `--org-id` is a shared flag, so every `cloud postgres` screen must
    /// describe it identically, and the `<POSTGRES_ID>` positional must always
    /// carry a description. Both properties are structural: the assertions
    /// compare occurrences to each other, never to a fixed sentence.
    #[test]
    fn postgres_screens_describe_org_id_consistently_and_document_the_positional() {
        use clap::CommandFactory;

        fn walk(cmd: &clap::Command, path: &str, org_id: &mut Option<(String, String)>) {
            for arg in cmd.get_arguments() {
                let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
                match arg.get_id().as_str() {
                    "org_id" => {
                        assert!(!help.is_empty(), "{path}: --org-id has no help text");
                        match org_id {
                            Some((first_path, first_help)) => assert_eq!(
                                &help, first_help,
                                "{path}: --org-id help differs from {first_path}"
                            ),
                            None => *org_id = Some((path.to_string(), help)),
                        }
                    }
                    "postgres_id" => {
                        assert!(!help.is_empty(), "{path}: <POSTGRES_ID> has no help text")
                    }
                    _ => {}
                }
            }
            for sub in cmd.get_subcommands() {
                walk(sub, &format!("{path} {}", sub.get_name()), org_id);
            }
        }

        let mut cli = Cli::command();
        cli.build();
        let cloud = cli.find_subcommand("cloud").expect("cloud command");
        let postgres = cloud
            .find_subcommand("postgres")
            .expect("cloud postgres command");
        let mut org_id = None;
        walk(postgres, "cloud postgres", &mut org_id);
        assert!(org_id.is_some());
    }
}
