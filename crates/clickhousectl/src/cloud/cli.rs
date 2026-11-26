#[cfg(feature = "experimental-access-negotiation")]
pub(crate) use crate::cloud::access::AccessCommands;
pub(crate) use crate::cloud::activity::ActivityCommands;
pub(crate) use crate::cloud::api_keys::KeyCommands;
pub(crate) use crate::cloud::auth::AuthCommands;
pub(crate) use crate::cloud::backups::BackupCommands;
pub(crate) use crate::cloud::clickpipes::ClickPipeCommands;
pub(crate) use crate::cloud::clickstack::ClickStackCommands;
pub(crate) use crate::cloud::context::UseArgs;
pub(crate) use crate::cloud::organizations::{InvitationCommands, MemberCommands, OrgCommands};
pub(crate) use crate::cloud::query_api_endpoints::QueryApiEndpointArgs;
pub(crate) use crate::cloud::saved_queries::SavedQueryArgs;
pub(crate) use crate::cloud::services::ServiceCommands;
use crate::cloud::udfs::UdfArgs;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CloudArgs {
    /// Cloud API key (requires --api-secret for auth login)
    #[arg(long, global = true, display_order = crate::cli::help_order::API_KEY)]
    pub api_key: Option<String>,

    /// Cloud API secret (requires --api-key for auth login)
    #[arg(long, global = true, display_order = crate::cli::help_order::API_SECRET)]
    pub api_secret: Option<String>,

    /// Organization ID (else the `cloud use` pin, or your only org)
    #[arg(long, global = true, conflicts_with = "org_name", display_order = crate::cli::help_order::ORG_ID)]
    pub org_id: Option<String>,

    /// Exact organization name
    #[arg(long, global = true, conflicts_with = "org_id", display_order = crate::cli::help_order::ORG_NAME)]
    pub org_name: Option<String>,

    /// Output as JSON
    #[arg(long, global = true, display_order = crate::cli::help_order::JSON)]
    pub json: bool,

    /// Print the resolved credential source and API URL to stderr
    #[arg(long, global = true, display_order = crate::cli::help_order::DEBUG)]
    pub debug: bool,

    /// Cloud API base URL override
    #[arg(display_order = crate::cli::help_order::URL)]
    #[cfg_attr(debug_assertions, arg(long, global = true))]
    #[cfg_attr(not(debug_assertions), arg(long, global = true, hide = true))]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

impl CloudArgs {
    /// Reparse only access invocations without selector propagation so clap
    /// cannot erase conflicting values supplied at different command depths.
    #[cfg(feature = "experimental-access-negotiation")]
    pub fn validate_access_selectors(
        &self,
        argv: &[std::ffi::OsString],
    ) -> Result<(), clap::Error> {
        if !matches!(self.command, CloudCommands::Access { .. }) {
            return Ok(());
        }
        fn without_selector_globals(
            mut command: clap::Command,
            inherited: &[clap::Arg],
        ) -> clap::Command {
            for arg in inherited {
                if !command
                    .get_arguments()
                    .any(|local| local.get_id() == arg.get_id())
                {
                    command = command.arg(arg.clone());
                }
            }
            let selectors: Vec<_> = command
                .get_arguments()
                .filter(|arg| {
                    ["org", "org_id", "org_name"].contains(&arg.get_id().as_str())
                        && arg.is_global_set()
                })
                .cloned()
                .collect();
            for arg in &selectors {
                command = command.mut_arg(arg.get_id().clone(), |arg| arg.global(false));
            }
            for child in command.get_subcommands_mut() {
                *child = without_selector_globals(child.clone(), &selectors);
            }
            command
        }
        // Never mutate a built clap command: its cached parser indexes would
        // describe the old globals. Reuse fresh derive definitions instead.
        use clap::CommandFactory;
        let mut validation = without_selector_globals(crate::cli::Cli::command(), &[]);
        let matches = validation.try_get_matches_from_mut(argv)?;
        let mut current = &matches;
        let mut selected: Option<(&str, &str)> = None;
        loop {
            for id in ["org", "org_id", "org_name"] {
                if current.try_get_one::<String>(id).ok().flatten().is_none()
                    || current.value_source(id) != Some(clap::parser::ValueSource::CommandLine)
                {
                    continue;
                }
                let value = current.get_one::<String>(id).expect("explicit selector");
                let kind = if id == "org_name" { "name" } else { "id" };
                if selected.is_some_and(|previous| previous != (kind, value.as_str())) {
                    return Err(validation.error(
                        clap::error::ErrorKind::ArgumentConflict,
                        "Conflicting organization selectors: use one organization ID (--org or --org-id) or one exact --org-name",
                    ));
                }
                selected = Some((kind, value));
            }
            match current.subcommand() {
                Some((_, child)) => current = child,
                None => return Ok(()),
            }
        }
    }

    /// Clap validates conflicts before propagating globals from other depths.
    pub fn has_organization_selector_conflict(&self) -> bool {
        self.org_id.is_some() && self.org_name.is_some()
    }

    /// `cloud use --clear` takes no organization selector. The selectors are
    /// global, so clap cannot see a clash given at another depth.
    pub fn has_use_clear_selector_conflict(&self) -> bool {
        matches!(&self.command, CloudCommands::Use(args) if args.clear)
            && (self.org_id.is_some() || self.org_name.is_some())
    }

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

    /// The schema-discovery source's usage error, checked before credentials.
    pub fn clickpipe_schema_discover_validation_error(&self) -> Option<(&'static str, String)> {
        let CloudCommands::ClickPipe { command } = &self.command else {
            return None;
        };
        command.clickpipe_schema_discover_validation_error()
    }

    /// The create source's usage error, checked before credentials.
    pub fn clickpipe_create_validation_error(&self) -> Option<(&'static str, String)> {
        let CloudCommands::ClickPipe { command } = &self.command else {
            return None;
        };
        command.clickpipe_create_validation_error()
    }

    /// A `clickstack` usage error clap cannot detect, with the owning
    /// subcommand path below `clickstack`, checked before credentials.
    pub fn clickstack_validation_error(&self) -> Option<(&'static [&'static str], String)> {
        let CloudCommands::ClickStack { command } = &self.command else {
            return None;
        };
        command.validation_error()
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

// Resource command groups naturally differ in size; clap owns their concrete layout.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Pin the organization this project uses
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Like `git switch` for orgs: `use --org-name <NAME>` or `use --org-id <ID>` pins one for this project
    (per checkout, gitignored); later cloud commands use it unless --org-name/--org-id is passed.
  Bare `use` shows the pin; `use --clear` removes it. A stderr note says when the pin is used.
  A pin applies only to the API it was set against; another API ignores it with a warning.")]
    Use(UseArgs),

    /// Get write access: request a short-lived key that the user approves
    #[cfg(feature = "experimental-access-negotiation")]
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Write access: a short-lived API key the user approves in their browser.
  1. List the task's writes; each command's `--help` gives its `API key permission:` needs.
  2. `cloud access roles`: pick the role, or the few narrow roles together, that cover all of them with fewest extras.
  3. `cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`: one request per task.
  Key is saved in the project's `.clickhouse/` (nearest up the tree, else here);
    found from that directory and below, not above.
  `cloud auth status`: roles held, expiry. On FORBIDDEN the error lists what the key lacks: request a role covering it.
  Requires OAuth (`cloud auth login`). Several orgs: pass --org-id.")]
    Access {
        /// Organization ID (conflicts with --org-name)
        #[arg(long, global = true, value_name = "ID", conflicts_with = "org_name")]
        org: Option<String>,
        #[command(subcommand)]
        command: AccessCommands,
    },

    /// Manage organizations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `org list` is the source of the org IDs that other cloud commands take as --org-id.
  BYOC infrastructure IDs and state are shown by `cloud org get --org-id <org-id>`.
  Next: `cloud clickhouse list`, `cloud member list`.")]
    Org {
        #[command(subcommand)]
        command: OrgCommands,
    },

    /// Manage ClickHouse services
    #[command(name = "clickhouse", visible_alias = "ch", alias = "service")]
    #[cfg_attr(
        feature = "experimental-access-negotiation",
        command(after_help = "\
CONTEXT FOR AGENTS:
  Service ID: `clickhousectl cloud clickhouse list`.
  Reads: list, get, dotenv, profile list, settings list/get/schema, prometheus, query, client,
    query-endpoint get, private-endpoint get-config, backup-config get, snapshot list/get,
    snapshot config get, scaling-schedule get, upgrade-window get, replica-aware-routing get,
    jwt-provider list/get.
    Other commands are writes: on OAuth, run `cloud access request` first.
  Typical flow: `create --name X` -> `get <id>` until state is `running` -> `query <id> -q 'SELECT 1'`.")
    )]
    #[cfg_attr(
        not(feature = "experimental-access-negotiation"),
        command(after_help = "\
CONTEXT FOR AGENTS:
  Service ID: `clickhousectl cloud clickhouse list`.
  Reads: list, get, dotenv, profile list, settings list/get/schema, prometheus, query, client,
    query-endpoint get, private-endpoint get-config, backup-config get, snapshot list/get,
    snapshot config get, scaling-schedule get, upgrade-window get, replica-aware-routing get,
    jwt-provider list/get.
    Other commands need API key auth.
  Typical flow: `create --name X` -> `get <id>` until state is `running` -> `query <id> -q 'SELECT 1'`.")
    )]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Manage ClickHouse Cloud Postgres services
    #[command(
        visible_alias = "pg",
        after_help = "\
CONTEXT FOR AGENTS:
  Write commands need API key auth.
  Service IDs: `cloud postgres list`.
  Credentials come only from `create` and `reset-password`; treat `get` as never returning them.
  `get` shows host/user; connections use port 5432, database postgres and TLS.
  Role changes return after API acceptance; see each command's help for verification.
  Typical flow: `create` -> `get <id>` until running -> `certs get <id> --output ca.pem` -> connect."
    )]
    Postgres {
        #[command(subcommand)]
        command: crate::cloud::postgres::PostgresCommands,
    },

    /// Manage Query API endpoints
    #[command(after_help = "CONTEXT FOR AGENTS:
  Writes require API key authentication; list/get support OAuth.
  Service IDs: `cloud clickhouse list`; API key IDs: `cloud key list`.
  Endpoint IDs: `cloud query-api-endpoint list <service-id>`.
  User-owned endpoints can be read but cannot be updated or deleted.
  Typical flow: create -> get -> update -> delete.")]
    QueryApiEndpoint(QueryApiEndpointArgs),

    /// Manage saved queries
    #[command(after_help = "CONTEXT FOR AGENTS:
  Writes require API key authentication; list/get support OAuth.
  Service IDs: `cloud clickhouse list`.
  Saved query IDs: `cloud saved-query list <service-id>`; or select by exact --name.
  Typical flow: create -> get -> update -> delete.")]
    SavedQuery(SavedQueryArgs),

    /// Manage service backups and backup buckets
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Service IDs come from `cloud clickhouse list`; backup IDs from `cloud backup list <service-id>`.
  Backup list/get and bucket get support OAuth; bucket writes require API key auth.
  Bucket create/update read strict provider JSON from `--file`, with `-` for stdin.
  Restore a backup into a new service: `cloud clickhouse create --backup-id <backup-id>`.
  Change schedule or retention with `cloud clickhouse backup-config update`, not here.")]
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// Manage ClickPipes for data ingestion
    #[command(
        name = "clickpipe",
        after_help = "\
CONTEXT FOR AGENTS:
  Service ID: `clickhousectl cloud clickhouse list`. ClickPipe ID: `clickpipe list <SERVICE_ID>`.
  Everything except list/get is a write and needs API key auth, schema-discover included.
  Lifecycle requests return immediately; use `get` to observe the resulting transition.
  Metrics: `cloud clickhouse prometheus <SERVICE_ID>`; filter samples by `clickpipe_id`.
  Streaming/object-storage errors: query `system.clickpipes_log` with `cloud clickhouse query`.
  Typical flow: `clickpipe schema-discover <source> <service-id>` -> `clickpipe create <source>`
    -> `clickpipe get`."
    )]
    ClickPipe {
        #[command(subcommand)]
        command: Box<ClickPipeCommands>,
    },

    /// Manage ClickStack observability resources
    #[command(
        name = "clickstack",
        after_help = "\
CONTEXT FOR AGENTS:
  Service ID: `clickhousectl cloud clickhouse list`.
  Resource IDs come from their respective `list` commands.
  Everything except list/get is a write and needs API key auth.
  Create/update read complete JSON bodies from --file; `-` reads stdin.
  Updates replace the resource; supply the complete desired definition."
    )]
    ClickStack {
        #[command(subcommand)]
        command: ClickStackCommands,
    },

    /// Manage user-defined functions
    #[command(after_help = "CONTEXT FOR AGENTS:
  Writes require API key authentication; list/get support OAuth.
  Function names: `cloud udf list`; service IDs: `cloud clickhouse list`.
  Create uploads a ZIP archive and starts an asynchronous build.
  Typical flow: deploy NAME --service <id>, or create -> get until ready -> attach -> attachment get.")]
    Udf(UdfArgs),

    /// Manage API keys
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  `create` prints a generated key secret exactly once — capture stdout or create a new key.
  Role IDs come from `cloud org role list` and must be UUIDs here.
  `update` replaces --role-id and --ip-allow wholesale; omitted flags are left as-is.
  Next: `cloud auth login --api-key <id> --api-secret <secret>` to use a new key.")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// Manage organization members
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  User IDs come from `cloud member list`; role IDs from `cloud member list --json`
  or `cloud org role list`. `update` replaces the member's whole role set.
  `remove` takes effect immediately with no confirmation.
  Next: `cloud invitation create --email ...` to add someone who is not yet a member.")]
    Member {
        #[command(subcommand)]
        command: MemberCommands,
    },

    /// Manage organization invitations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Role IDs come from `cloud org role list`.
  The invitee must accept from the email.
  Next: `cloud member list` once the invitation is accepted.")]
    Invitation {
        #[command(subcommand)]
        command: InvitationCommands,
    },

    /// View the organization activity log
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Audit log of org and service changes. `activity list` without --from-date covers the last 7 days;
  pass an earlier --from-date for older entries.
  Activity IDs for `activity get` come from `activity list`.")]
    Activity {
        #[command(subcommand)]
        command: ActivityCommands,
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
            // Writes only the project's own pin file, never Cloud state.
            CloudCommands::Use(_) => false,
            #[cfg(feature = "experimental-access-negotiation")]
            CloudCommands::Access { command, .. } => command.is_write(),
            CloudCommands::Org { command } => command.is_write(),
            CloudCommands::Service { command } => command.is_write(),
            CloudCommands::Backup { command } => command.is_write(),
            CloudCommands::Member { command } => command.is_write(),
            CloudCommands::Invitation { command } => command.is_write(),
            CloudCommands::Key { command } => command.is_write(),
            CloudCommands::Udf(args) => args.is_write(),
            CloudCommands::QueryApiEndpoint(args) => args.is_write(),
            CloudCommands::SavedQuery(args) => args.is_write(),
            CloudCommands::Activity { command } => command.is_write(),
            CloudCommands::Postgres { command } => command.is_write(),
            CloudCommands::ClickPipe { command } => command.is_write(),
            CloudCommands::ClickStack { command } => command.is_write(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::cli::{Cli, Commands};
    use clap::{CommandFactory, Parser};

    /// Assert the cloud-wide selector while domain helpers return only their command.
    pub(crate) fn assert_org_selector(cloud: &super::CloudArgs, args: &[&str]) {
        let expected = args
            .windows(2)
            .filter(|pair| pair[0] == "--org-id")
            .map(|pair| pair[1])
            .next_back();
        assert_eq!(cloud.org_id.as_deref(), expected, "{args:?}");
    }

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
    fn url_override_visibility_matches_the_build_mode() {
        let mut command = Cli::command();
        command.build();
        let cloud = command.find_subcommand("cloud").unwrap();
        let url = cloud
            .get_arguments()
            .find(|arg| arg.get_id() == "url")
            .unwrap();

        assert_eq!(url.get_long(), Some("url"));
        assert!(url.is_global_set());
        assert_eq!(url.is_hide_set(), !cfg!(debug_assertions));
    }

    #[test]
    fn url_override_is_accepted_independently_of_help_visibility() {
        let cli = Cli::try_parse_from([
            "chctl",
            "cloud",
            "--url",
            "https://api.control-plane.example.com",
            "org",
            "get",
        ])
        .unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("cloud")
        };

        assert_eq!(
            cloud.url.as_deref(),
            Some("https://api.control-plane.example.com")
        );
    }

    #[test]
    fn org_id_is_global_and_visible_throughout_cloud_hierarchy() {
        fn walk(command: &clap::Command) {
            let selectors: Vec<_> = command
                .get_arguments()
                .filter(|arg| arg.get_id() == "org_id")
                .collect();
            assert_eq!(selectors.len(), 1, "{}", command.get_name());
            let selector = selectors[0];
            assert!(selector.is_global_set());
            assert!(!selector.is_hide_set());
            assert!(!selector.is_required_set());
            assert_eq!(selector.get_long(), Some("org-id"));
            for child in command
                .get_subcommands()
                .filter(|child| child.get_name() != "help")
            {
                walk(child);
            }
        }
        let mut command = Cli::command();
        command.build();
        assert!(!command.get_arguments().any(|arg| arg.get_id() == "org_id"));
        walk(command.find_subcommand("cloud").unwrap());
        assert!(
            Cli::try_parse_from(["chctl", "--org-id", "org-1", "cloud", "org", "get"]).is_err()
        );
    }

    #[test]
    fn every_cloud_positional_rejects_an_empty_value_before_any_request() {
        fn walk(command: &clap::Command, path: &str, checked: &mut usize) {
            // Arguments after `--` go to a native client, never into a
            // request, and an empty one can be meaningful there.
            for arg in command.get_positionals().filter(|arg| !arg.is_last_set()) {
                let index = arg.get_index().unwrap();
                let mut argv = vec![command.get_name().to_string()];
                argv.extend((1..index).map(|_| "x".to_string()));
                argv.push(String::new());
                let kind = command
                    .clone()
                    .try_get_matches_from(argv)
                    .map(|_| None)
                    .unwrap_or_else(|error| Some(error.kind()));
                assert!(
                    matches!(
                        kind,
                        Some(clap::error::ErrorKind::InvalidValue)
                            | Some(clap::error::ErrorKind::ValueValidation)
                    ),
                    "{path} <{}> accepted an empty value ({kind:?})",
                    arg.get_id()
                );
                *checked += 1;
            }
            for child in command
                .get_subcommands()
                .filter(|child| child.get_name() != "help")
            {
                walk(child, &format!("{path} {}", child.get_name()), checked);
            }
        }
        let mut command = Cli::command();
        command.build();
        let mut checked = 0;
        walk(
            command.find_subcommand("cloud").unwrap(),
            "cloud",
            &mut checked,
        );
        assert!(checked > 0);
    }

    #[test]
    fn org_id_reaches_cloud_args_at_every_command_and_positional_boundary() {
        let cases: &[&[&str]] = &[
            &["clickhouse", "get", "svc-1"],
            &["clickhouse", "backup-config", "get", "svc-1"],
            &["clickhouse", "settings", "get", "svc-1", "setting"],
            &["member", "get", "user-1"],
            &["invitation", "get", "invite-1"],
            &["key", "get", "key-1"],
            &["activity", "get", "activity-1"],
            &["backup", "get", "svc-1", "backup-1"],
            &["postgres", "config", "get", "pg-1"],
            &["postgres", "backup", "list", "pg-1"],
            &["clickstack", "dashboard", "get", "svc-1", "dashboard-1"],
            &[
                "clickpipe",
                "reverse-private-endpoint",
                "get",
                "svc-1",
                "endpoint-1",
            ],
            &["udf", "version", "list", "my_udf"],
            &["udf", "attachment", "get", "my_udf", "svc-1"],
            &["udf", "deploy", "my_udf", "--service", "svc-1"],
            &["query-api-endpoint", "get", "svc-1", "endpoint-1"],
            &["saved-query", "get", "svc-1", "query-1"],
            &["org", "get"],
            &["org", "update", "--new-name", "Renamed"],
            &[
                "org",
                "usage",
                "--from-date",
                "2026-09-01",
                "--to-date",
                "2026-09-15",
            ],
            &["org", "prometheus"],
            &["org", "prometheus", "discovery"],
            &["org", "role", "get", "role-1"],
            &["org", "list"],
            &["auth", "status"],
            &[
                "clickpipe",
                "schema-discover",
                "kafka",
                "svc-1",
                "--brokers",
                "broker:9092",
                "--topics",
                "events",
                "--format",
                "JSONEachRow",
                "--iam-role",
                "arn:role",
            ],
            &[
                "clickpipe",
                "schema-discover",
                "kinesis",
                "svc-1",
                "--stream-name",
                "events",
                "--region",
                "us-east-1",
                "--format",
                "JSONEachRow",
            ],
            &[
                "clickpipe",
                "schema-discover",
                "object-storage",
                "svc-1",
                "--source-url",
                "https://bucket.example/data",
                "--format",
                "JSONEachRow",
            ],
            &[
                "clickpipe",
                "schema-discover",
                "pubsub",
                "svc-1",
                "--project-id",
                "project",
                "--topic",
                "events",
                "--seek-type",
                "earliest",
                "--format",
                "JSONEachRow",
                "--service-account-file",
                "key.json",
            ],
        ];
        for case in cases {
            let first_flag = case
                .iter()
                .position(|arg| arg.starts_with("--"))
                .unwrap_or(case.len());
            for position in (0..=first_flag).chain(std::iter::once(case.len())) {
                let mut args = vec!["chctl", "cloud"];
                args.extend_from_slice(&case[..position]);
                args.extend(["--org-id", "org-shared"]);
                args.extend_from_slice(&case[position..]);
                let cli =
                    Cli::try_parse_from(&args).unwrap_or_else(|error| panic!("{args:?}: {error}"));
                let Commands::Cloud(cloud) = cli.command else {
                    panic!("cloud")
                };
                assert_eq!(cloud.org_id.as_deref(), Some("org-shared"), "{args:?}");
            }
        }
    }

    #[test]
    fn org_id_repetition_matches_other_cloud_global_value_flags() {
        for flag in ["--org-id", "--api-key", "--url"] {
            let error = Cli::try_parse_from([
                "chctl", "cloud", flag, "first", flag, "second", "org", "get",
            ])
            .err()
            .unwrap();
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
            let cli = Cli::try_parse_from([
                "chctl",
                "cloud",
                flag,
                "ancestor",
                "org",
                "get",
                flag,
                "descendant",
            ])
            .unwrap();
            let Commands::Cloud(cloud) = cli.command else {
                panic!("cloud")
            };
            let value = match flag {
                "--org-id" => cloud.org_id,
                "--api-key" => cloud.api_key,
                "--url" => cloud.url,
                _ => unreachable!(),
            };
            assert_eq!(value.as_deref(), Some("descendant"));
        }
    }

    /// The OAuth write gate (`cloud::run`) trusts this classification, so one
    /// table covers a read and a write in every domain; each domain's own test
    /// covers the rest of its commands.
    #[test]
    fn is_write_command_classifies_reads_and_writes_in_every_domain() {
        let cases: &[(&[&str], bool)] = &[
            (&["auth", "status"], false),
            (&["auth", "whoami"], false),
            (
                &["auth", "login", "--api-key", "k", "--api-secret", "s"],
                false,
            ),
            (&["use", "--org-name", "acme"], false),
            (&["use", "--clear"], false),
            (&["org", "list"], false),
            (&["org", "get"], false),
            (&["org", "balance"], false),
            (&["org", "notifications"], false),
            (&["org", "prometheus"], false),
            (&["org", "prometheus", "discovery"], false),
            (&["org", "quota", "list"], false),
            (&["org", "quota", "get", "q-1"], false),
            (&["org", "role", "list"], false),
            (&["org", "update", "--new-name", "renamed"], true),
            (&["org", "role", "delete", "role-1"], true),
            (&["org", "byoc", "delete", "byoc-1"], true),
            (&["clickhouse", "list"], false),
            (
                &["clickhouse", "query", "svc-1", "--query", "SELECT 1"],
                false,
            ),
            (&["clickhouse", "delete", "svc-1"], true),
            (
                &["clickhouse", "settings", "unset", "svc-1", "max_threads"],
                true,
            ),
            (&["backup", "list", "svc-1"], false),
            (&["backup", "get", "svc-1", "bk-1"], false),
            (&["backup", "bucket", "delete", "svc-1"], true),
            (&["member", "list"], false),
            (&["member", "update", "user-1", "--role-id", "role-1"], true),
            (&["invitation", "list"], false),
            (&["invitation", "delete", "inv-1"], true),
            (&["key", "list"], false),
            (&["key", "get", "key-1"], false),
            (&["key", "create", "--name", "k"], true),
            (&["key", "update", "key-1", "--new-name", "new"], true),
            (&["key", "delete", "key-1"], true),
            (&["udf", "list"], false),
            (&["udf", "delete", "my_udf"], true),
            (&["query-api-endpoint", "list", "svc-1"], false),
            (&["query-api-endpoint", "delete", "svc-1", "ep-1"], true),
            (&["saved-query", "list", "svc-1"], false),
            (&["saved-query", "delete", "svc-1", "q-1"], true),
            (&["activity", "list"], false),
            (&["activity", "get", "act-1"], false),
            (&["postgres", "list"], false),
            (&["postgres", "get", "pg-1"], false),
            (&["postgres", "certs", "get", "pg-1"], false),
            (&["postgres", "config", "get", "pg-1"], false),
            // Postgres has no exhaustive classification test of its own, so
            // every write is listed here.
            (
                &[
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
            ),
            (
                &["postgres", "update", "pg-1", "--size", "c6gd.large"],
                true,
            ),
            (&["postgres", "delete", "pg-1"], true),
            (
                &["postgres", "config", "replace", "pg-1", "--file", "c.json"],
                true,
            ),
            (
                &[
                    "postgres",
                    "config",
                    "patch",
                    "pg-1",
                    "--set",
                    "max_connections=500",
                ],
                true,
            ),
            (&["postgres", "reset-password", "pg-1", "--generate"], true),
            (
                &["postgres", "read-replica", "create", "pg-1", "--name", "r1"],
                true,
            ),
            (
                &[
                    "postgres",
                    "restore",
                    "pg-1",
                    "--name",
                    "r",
                    "--restore-target",
                    "2026-04-16T12:00:00Z",
                ],
                true,
            ),
            (&["postgres", "restart", "pg-1"], true),
            (&["postgres", "promote", "pg-1"], true),
            (&["postgres", "switchover", "pg-1"], true),
            (&["clickpipe", "list", "svc-1"], false),
            (
                &["clickpipe", "reverse-private-endpoint", "list", "svc-1"],
                false,
            ),
            (&["clickpipe", "delete", "svc-1", "pipe-1"], true),
            (
                &[
                    "clickpipe",
                    "reverse-private-endpoint",
                    "delete",
                    "svc-1",
                    "rpe-1",
                ],
                true,
            ),
            (&["clickstack", "source", "list", "svc-1"], false),
            (&["clickstack", "alert", "get", "svc-1", "alert-1"], false),
            (&["clickstack", "source", "delete", "svc-1", "src-1"], true),
            (
                &["clickstack", "webhook", "delete", "svc-1", "hook-1"],
                true,
            ),
        ];
        let mut domains = std::collections::BTreeSet::new();
        for (args, expected) in cases {
            let mut full = vec!["clickhousectl", "cloud"];
            full.extend_from_slice(args);
            assert_write(&full, *expected);
            domains.insert((args[0], *expected));
        }
        // Every top-level domain has at least one read; every domain that can
        // mutate has at least one write.
        let reads: std::collections::BTreeSet<_> = domains
            .iter()
            .filter(|(_, write)| !write)
            .map(|(domain, _)| *domain)
            .collect();
        let writes: std::collections::BTreeSet<_> = domains
            .iter()
            .filter(|(_, write)| *write)
            .map(|(domain, _)| *domain)
            .collect();
        let all = [
            "activity",
            "auth",
            "backup",
            "clickhouse",
            "clickpipe",
            "clickstack",
            "invitation",
            "key",
            "member",
            "org",
            "postgres",
            "query-api-endpoint",
            "saved-query",
            "udf",
            "use",
        ];
        assert_eq!(reads, all.into_iter().collect());
        assert_eq!(
            writes,
            all.into_iter()
                .filter(|domain| !matches!(*domain, "activity" | "auth" | "use"))
                .collect()
        );
    }

    fn cloud_args(args: &[&str]) -> super::CloudArgs {
        let argv: Vec<_> = ["clickhousectl", "cloud"]
            .into_iter()
            .chain(args.iter().copied())
            .collect();
        let Commands::Cloud(cloud) = Cli::try_parse_from(&argv).unwrap().command else {
            panic!("expected cloud command for {argv:?}");
        };
        *cloud
    }

    /// `--org-name` is the name selector at any depth, `cloud use` included,
    /// and conflicts with `--org-id`; there is no `--org` spelling (#1509).
    #[test]
    fn org_name_selects_by_name_and_conflicts_with_org_id() {
        let cloud = cloud_args(&["org", "get", "--org-name", "acme"]);
        assert_eq!(cloud.org_name.as_deref(), Some("acme"));
        let cloud = cloud_args(&["use", "--org-name", "acme"]);
        assert_eq!(cloud.org_name.as_deref(), Some("acme"));
        let error = Cli::try_parse_from([
            "chctl",
            "cloud",
            "org",
            "get",
            "--org-name",
            "acme",
            "--org-id",
            "org-1",
        ])
        .err()
        .unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        let error = Cli::try_parse_from(["chctl", "cloud", "use", "--org", "acme"])
            .err()
            .unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn use_clear_conflicts_with_an_organization_selector_at_any_depth() {
        assert!(!cloud_args(&["use", "--clear"]).has_use_clear_selector_conflict());
        assert!(!cloud_args(&["use", "--org-name", "acme"]).has_use_clear_selector_conflict());
        for args in [
            &["use", "--clear", "--org-name", "acme"][..],
            &["--org-id", "org-1", "use", "--clear"],
        ] {
            assert!(
                cloud_args(args).has_use_clear_selector_conflict(),
                "{args:?}"
            );
        }
    }

    fn cloud_command(args: &[&str]) -> super::CloudCommands {
        let argv: Vec<_> = ["clickhousectl", "cloud"]
            .into_iter()
            .chain(args.iter().copied())
            .collect();
        let Commands::Cloud(cloud) = Cli::try_parse_from(&argv).unwrap().command else {
            panic!("expected cloud command for {argv:?}");
        };
        cloud.command
    }

    /// Every spelling of `cloud clickhouse` (the name, the shown alias `ch`
    /// and the hidden compatibility alias `service`) and of `cloud postgres`
    /// (`pg`) parses to the same command, so it dispatches the same (#1499).
    #[test]
    fn clickhouse_and_postgres_spellings_parse_identically() {
        use super::CloudCommands;
        use crate::cloud::postgres::PostgresCommands;
        use crate::cloud::services::ServiceCommands;
        for spelling in ["clickhouse", "ch", "service"] {
            assert!(
                matches!(
                    cloud_command(&[spelling, "list"]),
                    CloudCommands::Service {
                        command: ServiceCommands::List { .. }
                    }
                ),
                "{spelling}"
            );
            assert!(
                matches!(
                    cloud_command(&[spelling, "get", "svc-1"]),
                    CloudCommands::Service {
                        command: ServiceCommands::Get { service_id }
                    } if service_id.id.as_deref() == Some("svc-1")
                ),
                "{spelling}"
            );
            assert!(
                matches!(
                    cloud_command(&[spelling, "delete", "svc-1", "--force"]),
                    CloudCommands::Service {
                        command: ServiceCommands::Delete { force: true, .. }
                    }
                ),
                "{spelling}"
            );
        }
        for spelling in ["postgres", "pg"] {
            assert!(
                matches!(
                    cloud_command(&[spelling, "list"]),
                    CloudCommands::Postgres {
                        command: PostgresCommands::List { .. }
                    }
                ),
                "{spelling}"
            );
        }
    }

    #[test]
    fn cloud_help_shows_clickhouse_ch_and_pg_but_hides_service() {
        let cli = Cli::command();
        let cloud = cli.find_subcommand("cloud").unwrap();
        let clickhouse = cloud.find_subcommand("service").unwrap();
        assert_eq!(clickhouse.get_name(), "clickhouse");
        assert_eq!(clickhouse.get_visible_aliases().collect::<Vec<_>>(), ["ch"]);
        assert!(clickhouse.get_all_aliases().any(|alias| alias == "service"));
        let postgres = cloud.find_subcommand("pg").unwrap();
        assert_eq!(postgres.get_name(), "postgres");
        assert_eq!(postgres.get_visible_aliases().collect::<Vec<_>>(), ["pg"]);

        let help = Cli::try_parse_from(["clickhousectl", "cloud", "--help"])
            .err()
            .expect("--help should stop parsing")
            .to_string();
        let names = crate::local::cli::tests::listed_command_names(&help);
        for shown in ["clickhouse", "ch", "postgres", "pg"] {
            assert!(names.iter().any(|name| name == shown), "{shown}: {names:?}");
        }
        assert!(!names.iter().any(|name| name == "service"), "{names:?}");
    }

    /// The hidden alias reaches the same decorated help, permission lines
    /// included, as the canonical name.
    #[test]
    fn service_alias_renders_the_clickhouse_help() {
        let help = |spelling: &str| {
            Cli::try_parse_from(["clickhousectl", "cloud", spelling, "get", "--help"])
                .err()
                .expect("--help should stop parsing")
                .to_string()
        };
        assert_eq!(help("service"), help("clickhouse"));
        assert_eq!(help("ch"), help("clickhouse"));
    }
}
