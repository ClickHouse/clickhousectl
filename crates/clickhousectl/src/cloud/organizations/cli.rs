use super::org::parse_org_private_endpoint_remove_arg;
use crate::cloud::shared::{
    EmailSelector, NameSelector, parse_date_only, parse_datetime, parse_tag_filter,
};
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand, ValueEnum};

#[derive(Subcommand)]
pub enum OrgCommands {
    /// List organizations
    List,

    /// Get organization details
    Get,

    /// View organization quotas (Beta)
    Quota {
        #[command(subcommand)]
        command: QuotaCommands,
    },

    /// View active credit balances (Beta)
    Balance,

    /// List organization notifications (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns one page; JSON exposes result, limit, and nextCursor when provided.
  Pass nextCursor unchanged as --cursor until it is absent.
  Use `cloud org notification-types` to discover notification type names.")]
    Notifications(NotificationListArgs),

    /// List notification types (Beta)
    NotificationTypes,

    /// Manage BYOC infrastructure
    Byoc {
        #[command(subcommand)]
        command: ByocCommands,
    },

    /// Manage organization preview features (Beta)
    Preview {
        #[command(subcommand)]
        command: PreviewCommands,
    },

    /// Manage organization roles
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Role IDs from `list` can be used with member, invitation, and API key commands.
  Create/update read JSON request bodies from --file; `-` reads stdin.
  Only custom roles can be updated or deleted.")]
    Role {
        #[command(subcommand)]
        command: RoleCommands,
    },

    /// Update organization settings
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only the flags you pass change; everything else is left as-is.
  This can only remove private endpoints; add them with `cloud service update --add-private-endpoint-id`.")]
    Update {
        /// New organization name
        #[arg(long = "new-name", id = "new_name")]
        name: Option<String>,

        /// Remove a private endpoint from the org allow list (repeatable)
        ///
        /// Format: id[,description=TEXT],cloud-provider=aws|gcp|azure,region=REGION
        #[arg(
            long = "remove-private-endpoint",
            value_parser = parse_org_private_endpoint_remove_arg
        )]
        remove_private_endpoint: Vec<String>,

        /// Enable or disable core dump collection at the organization level
        #[arg(long)]
        enable_core_dumps: Option<bool>,
    },

    /// Get organization Prometheus configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  With no subcommand, prints raw metrics text from the legacy endpoint; --json is ignored.
  Use `discovery` for Prometheus HTTP service-discovery target groups.")]
    Prometheus {
        #[command(subcommand)]
        command: Option<PrometheusCommands>,

        /// Return the reduced (filtered) metric set
        #[arg(long, global = true)]
        filtered_metrics: Option<bool>,
    },

    /// Get organization usage/billing information
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The date range is inclusive and may span at most 31 days; longer ranges are rejected.
  Costs are in CHC (ClickHouse Credits), one row per entity per day plus a grand total.")]
    Usage {
        /// Report start date in UTC (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        from_date: String,

        /// Report end date in UTC, inclusive (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        to_date: String,

        /// Filter by resource tag: `tag:KEY=VALUE` or `tag:KEY` (repeatable)
        #[arg(long, value_parser = parse_tag_filter)]
        filter: Vec<String>,
    },
}

impl OrgCommands {
    pub fn is_write(&self) -> bool {
        match self {
            OrgCommands::List => false,
            OrgCommands::Get => false,
            OrgCommands::Quota { .. } => false,
            OrgCommands::Balance => false,
            OrgCommands::Notifications(_) => false,
            OrgCommands::NotificationTypes => false,
            OrgCommands::Byoc { command } => command.is_write(),
            OrgCommands::Preview { command } => command.is_write(),
            OrgCommands::Role { command } => command.is_write(),
            OrgCommands::Prometheus { .. } => false,
            OrgCommands::Usage { .. } => false,
            OrgCommands::Update { .. } => true,
        }
    }
}

#[derive(Args, Debug, Default)]
pub struct NotificationListArgs {
    /// Maximum notifications per page, 1-250; omit for 100
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..=250))]
    pub(super) limit: Option<i64>,

    /// Opaque nextCursor from the previous page
    #[arg(long, allow_hyphen_values = true)]
    pub(super) cursor: Option<String>,

    /// Notification severity
    #[arg(long)]
    pub(super) severity: Option<String>,

    /// Notification type from `cloud org notification-types`
    #[arg(long = "type")]
    pub(super) notification_type: Option<String>,

    /// Resource ID; service IDs also match their ClickPipes
    #[arg(long)]
    pub(super) resource_id: Option<String>,

    /// Resource kind, such as service, organization, clickpipe or postgres
    #[arg(long)]
    pub(super) resource_type: Option<String>,

    /// Earliest creation time, inclusive (RFC 3339)
    #[arg(long, value_parser = parse_datetime)]
    pub(super) from_date: Option<String>,

    /// Latest creation time, inclusive (RFC 3339)
    #[arg(long, value_parser = parse_datetime)]
    pub(super) to_date: Option<String>,
}

#[derive(Subcommand)]
pub enum RoleCommands {
    /// List organization roles
    List,

    /// Get organization role details
    Get {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,
    },

    /// Create a custom organization role
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Requires name, actors (IDs such as user/<uuid> or apiKey/<uuid>) and policies.
  Each policy needs allowDeny (ALLOW or DENY), permissions and resources (such as instance/<uuid>
  or instance/*); tags is optional.")]
    Create {
        /// JSON request body path, or `-` for stdin
        #[arg(
            long = "file",
            alias = "config-file",
            value_name = "PATH",
            required = true
        )]
        config_file: String,
    },

    /// Update a custom organization role
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only fields present in the JSON body change; actors and policies replace their whole lists.")]
    Update {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,

        /// JSON request body path, or `-` for stdin
        #[arg(
            long = "file",
            alias = "config-file",
            value_name = "PATH",
            required = true
        )]
        config_file: String,
    },

    /// Delete a custom organization role
    Delete {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,
    },
}

impl RoleCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::List | Self::Get { .. } => false,
            Self::Create { .. } | Self::Update { .. } | Self::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum ByocCommands {
    /// Get BYOC infrastructure details
    Get {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Get BYOC infrastructure provisioning progress (Beta)
    Progress {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Get BYOC private endpoint configuration (Beta)
    PrivateEndpointConfig {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Get BYOC infrastructure tags (Beta)
    Tags {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Validate cloud account readiness for BYOC (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Preflight only: simulates the cloud permissions ClickHouse needs and creates nothing.
  Requires API key auth, like `create`; OAuth is read-only.
  Takes the same flags as `cloud org byoc create`; a passing payload is accepted by create.
  Exits 1 after printing the result when any check is denied.
  `supported: false` means nothing was verified, not that everything passed.")]
    Validate {
        #[command(flatten)]
        infrastructure: Box<ByocInfrastructureArgs>,
    },

    /// Create BYOC infrastructure
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Preflight the same flags with `cloud org byoc validate` before creating.
  Watch provisioning with `cloud org byoc progress <id>`.
  Wait for `cloud org byoc get <id>` to show state `infra-ready` before creating a service.
  Discover profiles with `cloud service profile list --region <region> --byoc-id <id>`.")]
    Create {
        #[command(flatten)]
        infrastructure: Box<ByocInfrastructureArgs>,

        /// Human-readable infrastructure name
        #[arg(long)]
        display_name: Option<String>,
    },

    /// Update BYOC infrastructure
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only the flags you pass change; `--tag` replaces the whole tag set (see `cloud org byoc tags`).
  `--enable-private-link false` is rejected while any service has active private endpoints.
  The update is validated as a whole; rerunning a partly applied update is safe and converges.")]
    Update {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,

        #[command(flatten)]
        changes: ByocUpdateArgs,
    },

    /// Delete BYOC infrastructure
    Delete {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Manage ClickHouse personnel access to BYOC (Beta)
    Access {
        #[command(subcommand)]
        command: ByocAccessCommands,
    },
}

#[derive(Subcommand)]
pub enum ByocAccessCommands {
    /// Get the customer-authorized access setting
    Get {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Replace the customer-authorized access setting
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Replaces the whole setting; the previous mode and window are discarded.
  The output's `allowed` is the data plane's verdict at request time, not the stored mode.")]
    Update {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,

        #[command(flatten)]
        access: ByocAccessUpdateArgs,
    },
}

impl ByocAccessCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::Get { .. } => false,
            Self::Update { .. } => true,
        }
    }
}

/// When ClickHouse personnel may access BYOC infrastructure.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByocAccessMode {
    /// Allow access
    Always,
    /// Deny access
    Never,
    /// Allow access during one window
    Window,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ByocAccessUpdateArgs {
    /// When ClickHouse personnel may access the infrastructure
    #[arg(long, value_enum)]
    pub(super) mode: ByocAccessMode,

    /// Window start (RFC 3339); required with `--mode window`
    #[arg(
        long,
        value_parser = parse_access_window_start,
        required_if_eq("mode", "window")
    )]
    pub(super) start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Window length in seconds (1-172800); required with `--mode window`
    #[arg(
        long,
        value_parser = clap::value_parser!(i64).range(1..=172800),
        required_if_eq("mode", "window")
    )]
    pub(super) duration_seconds: Option<i64>,
}

fn parse_access_window_start(value: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    let value = parse_datetime(value)?;
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|start| start.with_timezone(&chrono::Utc))
        .map_err(|error| error.to_string())
}

/// The changes `cloud org byoc update` can send; at least one is required.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[group(id = "byoc_change", required = true, multiple = true)]
pub struct ByocUpdateArgs {
    /// New human-readable infrastructure name
    #[arg(long = "new-name", alias = "display-name")]
    pub(super) display_name: Option<String>,

    /// Turn private link connectivity on or off
    #[arg(long, value_name = "BOOL")]
    pub(super) enable_private_link: Option<bool>,

    /// Turn the private (internal) load balancer on or off
    #[arg(long, value_name = "BOOL")]
    pub(super) enable_private_load_balancer: Option<bool>,

    /// Turn the public (internet-facing) load balancer on or off
    #[arg(long, value_name = "BOOL")]
    pub(super) enable_public_load_balancer: Option<bool>,

    /// GCP BYO-VPC only: PSC NAT subnet name; only with `--enable-private-link true`
    #[arg(long, value_name = "NAME", requires = "enable_private_link")]
    pub(super) gcp_psc_subnet_id: Option<String>,

    /// Replacement tag for the infrastructure's cloud resources (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    pub(super) tags: Vec<String>,
}

/// Infrastructure flags shared by `cloud org byoc create` and `validate`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ByocInfrastructureArgs {
    /// Cloud region ID
    #[arg(long)]
    pub(super) region: String,

    /// Cloud account ID: AWS account, GCP project, or Azure subscription
    #[arg(long)]
    pub(super) account_id: String,

    /// Availability-zone suffix (repeatable)
    #[arg(long)]
    pub(super) availability_zone_suffix: Vec<String>,

    /// CIDR range for a ClickHouse-managed VPC; not with BYO-VPC flags
    #[arg(
        long,
        conflicts_with_all = ["vpc_id", "private_subnet_id", "public_subnet_id"]
    )]
    pub(super) vpc_cidr_range: Option<String>,

    /// BYO-VPC ID or network name (AWS, GCP); requires --private-subnet-id
    #[arg(long, requires = "private_subnet_id")]
    pub(super) vpc_id: Option<String>,

    /// BYO-VPC private subnet ID or name (repeatable; AWS 1-6, GCP 1)
    #[arg(long)]
    pub(super) private_subnet_id: Vec<String>,

    /// AWS BYO-VPC public subnet ID (repeatable; at most 6)
    #[arg(long)]
    pub(super) public_subnet_id: Vec<String>,

    /// AWS ExternalID in the ClickHouse management role trust policy
    #[arg(long)]
    pub(super) external_id: Option<String>,

    /// GCP BYO-VPC secondary range name for pod IPs (repeatable)
    #[arg(long)]
    pub(super) gcp_pod_cidr_range_name: Vec<String>,

    /// GCP Shared VPC host project, when it differs from --account-id
    #[arg(long)]
    pub(super) gcp_shared_vpc_host_project_id: Option<String>,

    /// Azure Entra tenant ID; required for Azure regions
    #[arg(long)]
    pub(super) tenant_id: Option<String>,

    /// Azure service principal client ID; required for Azure regions
    #[arg(long)]
    pub(super) service_principal_client_id: Option<String>,

    /// Tag for the infrastructure's cloud resources (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    pub(super) tags: Vec<String>,
}

impl ByocCommands {
    fn is_write(&self) -> bool {
        match self {
            ByocCommands::Get { .. } => false,
            ByocCommands::Progress { .. } => false,
            ByocCommands::PrivateEndpointConfig { .. } => false,
            ByocCommands::Tags { .. } => false,
            // A preflight that creates nothing, but the endpoint requires the
            // organization manage scope, which OAuth (read-only) never has.
            ByocCommands::Validate { .. } => true,
            ByocCommands::Create { .. } => true,
            ByocCommands::Update { .. } => true,
            ByocCommands::Delete { .. } => true,
            ByocCommands::Access { command } => command.is_write(),
        }
    }
}

#[derive(Subcommand)]
pub enum PreviewCommands {
    /// List preview features offered to the organization
    List,

    /// Switch a preview feature on or off
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only features listed with canToggle true can be switched by the caller.
  Switching on a feature not offered to the organization fails; switching off always works.")]
    Update {
        /// Preview feature ID (from `cloud org preview list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        preview_id: String,

        /// Turn the preview feature on or off
        #[arg(
            long,
            value_name = "BOOL",
            action = clap::ArgAction::Set,
            required = true
        )]
        enabled: bool,
    },
}

impl PreviewCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::List => false,
            Self::Update { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum PrometheusCommands {
    /// List Prometheus scrape targets
    Discovery,
}

#[derive(Subcommand)]
pub enum QuotaCommands {
    /// List organization quotas
    List,

    /// Get organization quota details
    Get {
        /// Quota code
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        quota_code: String,
    },
}

#[derive(Subcommand)]
pub enum MemberCommands {
    /// List organization members
    List,

    /// Get member details
    Get {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,
    },

    /// Update member roles
    Update {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,

        /// Role ID to assign (repeatable; conflicts with --clear-roles)
        #[arg(long, conflicts_with = "clear_roles")]
        role_id: Vec<String>,

        /// Remove all assigned roles; conflicts with --role-id
        #[arg(long, conflicts_with = "role_id")]
        clear_roles: bool,
    },

    /// Remove a member from the organization
    Remove {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,
    },

    /// Manage a member's notification settings (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Settings are the member's explicit choices; anything unset follows their role's platform default.
  Notification type names come from `cloud org notification-types`.")]
    NotificationSettings {
        #[command(subcommand)]
        command: MemberNotificationSettingsCommands,
    },
}

impl MemberCommands {
    pub fn is_write(&self) -> bool {
        match self {
            MemberCommands::List => false,
            MemberCommands::Get { .. } => false,
            MemberCommands::Update { .. } => true,
            MemberCommands::Remove { .. } => true,
            MemberCommands::NotificationSettings { command } => command.is_write(),
        }
    }
}

#[derive(Subcommand)]
pub enum MemberNotificationSettingsCommands {
    /// List a member's explicit notification settings
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns one page; JSON exposes result, limit, and nextCursor when provided.
  Pass nextCursor unchanged as --cursor until it is absent.
  An empty list means every notification follows the default, not that notifications are off.")]
    List(MemberNotificationSettingsListArgs),

    /// Apply one notification setting rule to a member
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Sets --effect for every --type, --resource-id and --channel combination; other settings are kept.
  `inherit` removes the setting so the default applies again. Rerunning the same update is safe.
  Resource IDs have no wildcards; organization-wide types such as billing take the organization ID.
  The update applies completely or not at all.")]
    Update(MemberNotificationSettingsUpdateArgs),
}

impl MemberNotificationSettingsCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::List(_) => false,
            Self::Update(_) => true,
        }
    }
}

#[derive(Args, Debug)]
pub struct MemberNotificationSettingsListArgs {
    /// User ID
    #[command(flatten)]
    pub(super) user_id: EmailSelector,

    /// Maximum settings per page, 1-250; omit for 100
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..=250))]
    pub(super) limit: Option<i64>,

    /// Opaque nextCursor from the previous page
    #[arg(long, allow_hyphen_values = true)]
    pub(super) cursor: Option<String>,

    /// Notification type from `cloud org notification-types`
    #[arg(long = "type")]
    pub(super) notification_type: Option<String>,

    /// Resource ID
    #[arg(long)]
    pub(super) resource_id: Option<String>,

    /// Resource kind, such as service, organization, clickpipe or postgres
    #[arg(long)]
    pub(super) resource_type: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberNotificationSettingsUpdateArgs {
    /// User ID
    #[command(flatten)]
    pub(super) user_id: EmailSelector,

    /// Setting to apply to every named combination
    #[arg(long, value_enum)]
    pub(super) effect: NotificationSettingEffect,

    /// Notification type from `cloud org notification-types` (repeatable)
    #[arg(
        long = "type",
        value_name = "TYPE",
        required = true,
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub(super) notification_types: Vec<String>,

    /// Resource ID the setting applies to (repeatable)
    #[arg(
        long = "resource-id",
        value_name = "ID",
        required = true,
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub(super) resource_ids: Vec<String>,

    /// Delivery channel (repeatable); omit for every channel
    #[arg(long = "channel", value_name = "CHANNEL", value_enum)]
    pub(super) channels: Vec<NotificationChannel>,
}

/// What a notification setting rule does to the settings it names.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSettingEffect {
    /// Receive the notification
    Enabled,
    /// Do not receive the notification
    Disabled,
    /// Remove the setting and follow the role's default
    Inherit,
}

/// A member notification delivery channel.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationChannel {
    /// Cloud console notifications
    Ui,
    /// Email
    Email,
}

#[derive(Subcommand)]
pub enum InvitationCommands {
    /// List pending invitations
    List,

    /// Create an invitation
    Create {
        /// Email address to invite (stored lowercased)
        #[arg(long)]
        email: String,

        /// Role ID to assign (repeatable)
        #[arg(long)]
        role_id: Vec<String>,
    },

    /// Get invitation details
    Get {
        /// Invitation ID
        #[command(flatten)]
        invitation_id: EmailSelector,
    },

    /// Delete an invitation
    Delete {
        /// Invitation ID
        #[command(flatten)]
        invitation_id: EmailSelector,
    },
}

impl InvitationCommands {
    pub fn is_write(&self) -> bool {
        match self {
            InvitationCommands::List => false,
            InvitationCommands::Get { .. } => false,
            InvitationCommands::Create { .. } => true,
            InvitationCommands::Delete { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "org", "role", "create"],
            "config_file",
            &["config-file"],
        );
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "org", "role", "update", "role-1"],
            "config_file",
            &["config-file"],
        );
    }

    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use crate::cloud::organizations::byoc::tests::{
        maximal_infrastructure_args, minimal_infrastructure_args,
    };
    use clap::Parser;

    fn parse_cloud_command(args: &[&str]) -> CloudCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud_args, args);
        cloud_args.command
    }

    fn assert_write(args: &[&str], expected: bool) {
        let command = parse_cloud_command(args);
        assert!(matches!(
            &command,
            CloudCommands::Org { .. }
                | CloudCommands::Member { .. }
                | CloudCommands::Invitation { .. }
        ));
        assert_eq!(
            command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    #[test]
    fn parses_organization_body_command_defaults() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };

        assert!(name.is_none());
        assert!(remove_private_endpoint.is_empty());
        assert!(enable_core_dumps.is_none());

        let CloudCommands::Member { command } =
            parse_cloud_command(&["clickhousectl", "cloud", "member", "update", "user-1"])
        else {
            panic!("expected member command");
        };
        let MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id.id.as_deref(), Some("user-1"));
        assert!(role_id.is_empty());
        assert!(!clear_roles);

        let CloudCommands::Invitation { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "invitation",
            "create",
            "--email",
            "user@example.com",
        ]) else {
            panic!("expected invitation command");
        };
        let InvitationCommands::Create { email, role_id } = command else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert!(role_id.is_empty());
    }

    #[test]
    fn parses_organization_body_command_maximal_and_repeatable_flags() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
            "--new-name",
            "Updated Org",
            "--remove-private-endpoint",
            "pe-1,description=old,cloud-provider=aws,region=us-east-1",
            "--remove-private-endpoint",
            "pe-2,description=legacy,cloud-provider=azure,region=eastus",
            "--enable-core-dumps",
            "false",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };

        assert_eq!(name.as_deref(), Some("Updated Org"));
        assert_eq!(
            remove_private_endpoint,
            vec![
                "pe-1,description=old,cloud-provider=aws,region=us-east-1",
                "pe-2,description=legacy,cloud-provider=azure,region=eastus",
            ]
        );
        assert_eq!(enable_core_dumps, Some(false));

        let CloudCommands::Member { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "member",
            "update",
            "user-1",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected member command");
        };
        let MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id.id.as_deref(), Some("user-1"));
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert!(!clear_roles);

        let CloudCommands::Invitation { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "invitation",
            "create",
            "--email",
            "user@example.com",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected invitation command");
        };
        let InvitationCommands::Create { email, role_id } = command else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
    }

    #[test]
    fn parses_minimal_private_endpoint_removal() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
            "--remove-private-endpoint",
            "pe-1,cloud-provider=aws,region=us-east-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            remove_private_endpoint,
            ..
        } = command
        else {
            panic!("expected org update");
        };

        assert_eq!(
            remove_private_endpoint,
            ["pe-1,cloud-provider=aws,region=us-east-1"]
        );
    }

    #[test]
    fn rejects_incomplete_private_endpoint_removals_during_clap_parsing() {
        for (value, missing) in [
            ("pe-1,region=us-east-1", &["cloud-provider"][..]),
            ("pe-1,cloud-provider=aws", &["region"][..]),
            ("pe-1,description=old", &["cloud-provider", "region"][..]),
            (
                "pe-1,cloud-provider=,region=us-east-1",
                &["cloud-provider"][..],
            ),
            ("pe-1,cloud-provider=aws,region= ", &["region"][..]),
        ] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "org",
                "update",
                "--org-id",
                "org-1",
                "--remove-private-endpoint",
                value,
            ])
            .err()
            .expect("incomplete endpoint removal should fail");

            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            // Clap wraps the parser's message, which quotes the value and
            // names the missing keys.
            let message = error.to_string();
            assert_eq!(
                super::super::org::tests::missing_endpoint_keys(
                    message.lines().next().unwrap(),
                    value
                ),
                missing,
                "{error}"
            );
        }
    }

    #[test]
    fn rejects_conflicting_member_role_changes() {
        for flags in [
            ["--role-id", "role-1", "--clear-roles"],
            ["--clear-roles", "--role-id", "role-1"],
        ] {
            let result = Cli::try_parse_from(
                ["clickhousectl", "cloud", "member", "update", "user-1"]
                    .into_iter()
                    .chain(flags),
            );
            let Err(error) = result else {
                panic!("set and clear flags must conflict");
            };
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn org_usage_tag_filters_preserve_api_grammar() {
        let base_args = [
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ];
        let filters = ["tag:env=prod", "tag:active", "tag:empty=", "tag:expr=a=b"];
        let cli = Cli::try_parse_from(
            base_args
                .into_iter()
                .chain(filters.iter().flat_map(|value| ["--filter", *value])),
        )
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org {
            command: OrgCommands::Usage { filter, .. },
        } = args.command
        else {
            panic!("expected org usage");
        };
        assert_eq!(filter, filters);
        for value in [
            "garbage",
            "state=running",
            "env=prod",
            "tag:",
            "tag:=x",
            "tag: =x",
        ] {
            let error = Cli::try_parse_from(base_args.into_iter().chain(["--filter", value]))
                .err()
                .expect("malformed filter must fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn parses_org_prometheus_discovery_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "prometheus",
            "discovery",
            "--org-id",
            "org-1",
            "--filtered-metrics",
            "false",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Prometheus {
            command: Some(PrometheusCommands::Discovery),
            filtered_metrics,
            ..
        } = command
        else {
            panic!("expected prometheus discovery");
        };

        assert_eq!(filtered_metrics, Some(false));
    }

    #[test]
    fn parses_org_notifications_filters_and_defaults() {
        let CloudCommands::Org {
            command: OrgCommands::Notifications(defaults),
        } = parse_cloud_command(&["clickhousectl", "cloud", "org", "notifications"])
        else {
            panic!("expected org notifications")
        };
        assert_eq!(defaults.limit, None);
        assert_eq!(defaults.cursor, None);
        assert_eq!(defaults.severity, None);
        assert_eq!(defaults.notification_type, None);
        assert_eq!(defaults.resource_id, None);
        assert_eq!(defaults.resource_type, None);
        assert_eq!(defaults.from_date, None);
        assert_eq!(defaults.to_date, None);
        let CloudCommands::Org {
            command: OrgCommands::Notifications(args),
        } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "notifications",
            "--limit",
            "250",
            "--cursor",
            "-opaque +/=&?",
            "--severity",
            "warning",
            "--type",
            "future.type",
            "--resource-id",
            "service-1",
            "--resource-type",
            "service",
            "--from-date",
            "2026-10-01T00:00:00+02:00",
            "--to-date",
            "2026-10-08T00:00:00Z",
        ])
        else {
            panic!("expected org notifications")
        };
        assert_eq!(args.limit, Some(250));
        assert_eq!(args.cursor.as_deref(), Some("-opaque +/=&?"));
        assert_eq!(args.severity.as_deref(), Some("warning"));
        assert_eq!(args.notification_type.as_deref(), Some("future.type"));
        assert_eq!(args.resource_id.as_deref(), Some("service-1"));
        assert_eq!(args.resource_type.as_deref(), Some("service"));
        assert_eq!(args.from_date.as_deref(), Some("2026-10-01T00:00:00+02:00"));
        assert_eq!(args.to_date.as_deref(), Some("2026-10-08T00:00:00Z"));
        assert_write(&["clickhousectl", "cloud", "org", "notifications"], false);
    }

    #[test]
    fn rejects_invalid_or_repeated_notification_flags() {
        for extra in [
            vec!["--limit", "0"],
            vec!["--limit", "251"],
            vec!["--from-date", "2026-10-01"],
            vec!["--to-date", "yesterday"],
            vec!["--limit", "1", "--limit", "2"],
        ] {
            assert!(
                Cli::try_parse_from(
                    ["clickhousectl", "cloud", "org", "notifications"]
                        .into_iter()
                        .chain(extra)
                )
                .is_err()
            );
        }
    }

    /// Both date flags go through `parse_date_only`: timestamps and impossible
    /// calendar dates are usage errors on either one.
    #[test]
    fn rejects_org_usage_timestamps_and_invalid_calendar_dates_on_either_flag() {
        for bad in ["2025-01-01T00:00:00Z", "2025-02-31"] {
            for (from, to) in [(bad, "2025-01-31"), ("2025-01-01", bad)] {
                let error = Cli::try_parse_from([
                    "clickhousectl",
                    "cloud",
                    "org",
                    "usage",
                    "--from-date",
                    from,
                    "--to-date",
                    to,
                ])
                .err()
                .unwrap_or_else(|| panic!("accepted {from} .. {to}"));
                assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
                assert!(error.to_string().contains("YYYY-MM-DD"), "{error}");
            }
        }
    }

    #[test]
    fn top_level_write_classification_covers_every_organization_access_command() {
        assert_write(&["clickhousectl", "cloud", "org", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "get", "--org-id", "org-1"],
            false,
        );
        assert_write(&["clickhousectl", "cloud", "org", "quota", "list"], false);
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "quota",
                "get",
                "services-per-organization",
            ],
            false,
        );
        assert_write(&["clickhousectl", "cloud", "org", "balance"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "notification-types"],
            false,
        );
        assert_write(&["clickhousectl", "cloud", "org", "prometheus"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "prometheus", "discovery"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "usage",
                "--from-date",
                "2025-01-01",
                "--to-date",
                "2025-01-31",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "update",
                "--org-id",
                "org-1",
            ],
            true,
        );

        assert_write(&["clickhousectl", "cloud", "member", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "member", "get", "user-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "member", "update", "user-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "member", "remove", "user-1"],
            true,
        );

        assert_write(&["clickhousectl", "cloud", "invitation", "list"], false);
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "get",
                "invitation-1",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "create",
                "--email",
                "user@example.com",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "delete",
                "invitation-1",
            ],
            true,
        );
    }

    fn parse_byoc_command(args: &[&str]) -> ByocCommands {
        let CloudCommands::Org { command } = parse_cloud_command(args) else {
            panic!("expected org command");
        };
        let OrgCommands::Byoc { command } = command else {
            panic!("expected BYOC command");
        };
        command
    }

    fn byoc_args(extra: &[&str]) -> Vec<String> {
        let mut args: Vec<String> = [
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
        ]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
        args.extend(extra.iter().map(|arg| arg.to_string()));
        args
    }

    #[test]
    fn parses_byoc_create_update_and_delete_commands() {
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
            "--availability-zone-suffix",
            "a",
            "--availability-zone-suffix",
            "b",
            "--vpc-cidr-range",
            "10.0.0.0/16",
            "--display-name",
            "production",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(infrastructure.region, "us-east-1");
        assert_eq!(infrastructure.account_id, "123456789012");
        assert_eq!(infrastructure.availability_zone_suffix, vec!["a", "b"]);
        assert_eq!(
            infrastructure.vpc_cidr_range.as_deref(),
            Some("10.0.0.0/16")
        );
        assert_eq!(display_name.as_deref(), Some("production"));

        let ByocCommands::Update { byoc_id, changes } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--display-name",
            "renamed",
        ]) else {
            panic!("expected BYOC update");
        };
        assert_eq!(byoc_id.id.as_deref(), Some("byoc-1"));
        assert_eq!(changes.display_name.as_deref(), Some("renamed"));
        assert_eq!(changes.enable_private_link, None);
        assert!(changes.tags.is_empty());

        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                "delete",
                "byoc-1",
                "--org-id",
                "org-1",
            ],
            true,
        );
    }

    #[test]
    fn byoc_get_and_progress_are_reads_selected_by_id_or_name() {
        for subcommand in ["get", "progress", "private-endpoint-config", "tags"] {
            let command = parse_byoc_command(&[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                subcommand,
                "byoc-1",
            ]);
            let byoc_id = match command {
                ByocCommands::Get { byoc_id }
                | ByocCommands::Progress { byoc_id }
                | ByocCommands::PrivateEndpointConfig { byoc_id }
                | ByocCommands::Tags { byoc_id } => byoc_id,
                _ => panic!("expected BYOC {subcommand}"),
            };
            assert_eq!(byoc_id.id.as_deref(), Some("byoc-1"));
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "org",
                    "byoc",
                    subcommand,
                    "byoc-1",
                ],
                false,
            );

            let command = parse_byoc_command(&[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                subcommand,
                "--name",
                "production",
            ]);
            let (ByocCommands::Get { byoc_id }
            | ByocCommands::Progress { byoc_id }
            | ByocCommands::PrivateEndpointConfig { byoc_id }
            | ByocCommands::Tags { byoc_id }) = command
            else {
                panic!("expected BYOC {subcommand}");
            };
            assert_eq!(byoc_id.name.as_deref(), Some("production"));

            let error = Cli::try_parse_from(["clickhousectl", "cloud", "org", "byoc", subcommand])
                .err()
                .expect("an ID or --name is required");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }

    #[test]
    fn byoc_validate_and_create_are_writes() {
        // Validate creates nothing, but its endpoint needs the organization
        // manage scope, which read-only OAuth never has.
        for subcommand in ["validate", "create"] {
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "org",
                    "byoc",
                    subcommand,
                    "--region",
                    "us-east-1",
                    "--account-id",
                    "123456789012",
                ],
                true,
            );
        }
    }

    #[test]
    fn byoc_create_needs_only_region_and_account() {
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
        ])
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(*infrastructure, minimal_infrastructure_args());
        assert_eq!(display_name, None);

        for missing in ["--region", "--account-id"] {
            let args: Vec<String> = byoc_args(&[]);
            let index = args.iter().position(|arg| arg == missing).unwrap();
            let mut args = args;
            args.drain(index..index + 2);
            let error = Cli::try_parse_from(&args).err().expect("must fail");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{missing}"
            );
        }
    }

    #[test]
    fn byoc_validate_shares_create_flags_and_rejects_display_name() {
        let flags = [
            "--region",
            "eastus",
            "--account-id",
            "subscription-1",
            "--availability-zone-suffix",
            "a",
            "--availability-zone-suffix",
            "b",
            "--availability-zone-suffix",
            "c",
            "--availability-zone-suffix",
            "d",
            "--availability-zone-suffix",
            "e",
            "--availability-zone-suffix",
            "f",
            "--vpc-id",
            "vpc-1",
            "--private-subnet-id",
            "subnet-a",
            "--private-subnet-id",
            "subnet-b",
            "--public-subnet-id",
            "subnet-pub",
            "--external-id",
            "external-1",
            "--gcp-pod-cidr-range-name",
            "pods-1",
            "--gcp-pod-cidr-range-name",
            "pods-2",
            "--gcp-shared-vpc-host-project-id",
            "host-project",
            "--tenant-id",
            "tenant-1",
            "--service-principal-client-id",
            "client-1",
            "--tag",
            "team=data",
            "--tag",
            "note=a=b",
            "--tag",
            "empty=",
        ];
        let mut validate = vec!["clickhousectl", "cloud", "org", "byoc", "validate"];
        validate.extend(flags);
        let ByocCommands::Validate { infrastructure } = parse_byoc_command(&validate) else {
            panic!("expected BYOC validate");
        };
        assert_eq!(*infrastructure, maximal_infrastructure_args());

        let mut create = vec!["clickhousectl", "cloud", "org", "byoc", "create"];
        create.extend(flags);
        create.extend(["--display-name", "production"]);
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&create)
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(*infrastructure, maximal_infrastructure_args());
        assert_eq!(display_name.as_deref(), Some("production"));

        let mut validate_with_name = validate.clone();
        validate_with_name.extend(["--display-name", "production"]);
        let error = Cli::try_parse_from(&validate_with_name)
            .err()
            .expect("validate has no --display-name");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn byoc_vpc_cidr_range_conflicts_with_byo_vpc_flags() {
        for byo_vpc in [
            ["--vpc-id", "vpc-1"],
            ["--private-subnet-id", "subnet-a"],
            ["--public-subnet-id", "subnet-pub"],
        ] {
            let mut extra = vec!["--vpc-cidr-range", "10.0.0.0/16"];
            extra.extend(byo_vpc);
            if byo_vpc[0] == "--vpc-id" {
                extra.extend(["--private-subnet-id", "subnet-a"]);
            }
            let error = Cli::try_parse_from(byoc_args(&extra))
                .err()
                .expect("managed and BYO VPC flags conflict");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{byo_vpc:?}"
            );
        }
    }

    #[test]
    fn byoc_vpc_id_requires_a_private_subnet() {
        let error = Cli::try_parse_from(byoc_args(&["--vpc-id", "vpc-1"]))
            .err()
            .expect("--vpc-id needs --private-subnet-id");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(
            Cli::try_parse_from(byoc_args(&[
                "--vpc-id",
                "vpc-1",
                "--private-subnet-id",
                "subnet-a"
            ]))
            .is_ok()
        );
    }

    #[test]
    fn parses_byoc_update_change_flags() {
        let ByocCommands::Update { byoc_id, changes } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "--name",
            "production",
            "--enable-private-link",
            "true",
            "--gcp-psc-subnet-id",
            "psc-nat",
            "--enable-private-load-balancer",
            "false",
            "--enable-public-load-balancer",
            "true",
            "--tag",
            "team=data",
            "--tag",
            "env=prod",
        ]) else {
            panic!("expected BYOC update");
        };
        assert_eq!(byoc_id.name.as_deref(), Some("production"));
        assert_eq!(
            changes,
            ByocUpdateArgs {
                display_name: None,
                enable_private_link: Some(true),
                enable_private_load_balancer: Some(false),
                enable_public_load_balancer: Some(true),
                gcp_psc_subnet_id: Some("psc-nat".into()),
                tags: vec!["team=data".into(), "env=prod".into()],
            }
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                "update",
                "byoc-1",
                "--enable-private-link",
                "false",
            ],
            true,
        );
    }

    #[test]
    fn byoc_update_requires_a_change_and_psc_subnet_requires_private_link() {
        let error =
            Cli::try_parse_from(["clickhousectl", "cloud", "org", "byoc", "update", "byoc-1"])
                .err()
                .expect("a change is required");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );

        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--gcp-psc-subnet-id",
            "psc-nat",
        ])
        .err()
        .expect("--gcp-psc-subnet-id requires --enable-private-link");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );

        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--enable-private-link",
            "yes",
        ])
        .err()
        .expect("booleans are true or false");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn parses_role_body_commands_and_classifies_access() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "role",
            "create",
            "--file",
            "role.json",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Role {
            command: RoleCommands::Create { config_file },
        } = command
        else {
            panic!("expected role create");
        };
        assert_eq!(config_file, "role.json");

        assert_write(&["clickhousectl", "cloud", "org", "role", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "role", "get", "role-1"],
            false,
        );
        for verb in ["create", "update", "delete"] {
            let mut args = vec!["clickhousectl", "cloud", "org", "role", verb];
            if verb != "create" {
                args.push("role-1");
            }
            if verb != "delete" {
                args.extend(["--file", "role.json"]);
            }
            assert_write(&args, true);
        }
    }

    fn access_update(extra: &[&str]) -> Result<ByocAccessUpdateArgs, clap::Error> {
        let mut args = vec![
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "access",
            "update",
            "byoc-1",
        ];
        args.extend(extra);
        let cli = Cli::try_parse_from(args)?;
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Org {
            command:
                OrgCommands::Byoc {
                    command:
                        ByocCommands::Access {
                            command: ByocAccessCommands::Update { byoc_id, access },
                        },
                },
        } = cloud_args.command
        else {
            panic!("expected BYOC access update");
        };
        assert_eq!(byoc_id.id.as_deref(), Some("byoc-1"));
        Ok(access)
    }

    #[test]
    fn byoc_access_get_is_a_read_and_update_is_a_write() {
        let ByocCommands::Access {
            command: ByocAccessCommands::Get { byoc_id },
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "access",
            "get",
            "--name",
            "production",
        ])
        else {
            panic!("expected BYOC access get");
        };
        assert_eq!(byoc_id.name.as_deref(), Some("production"));
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                "access",
                "get",
                "byoc-1",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                "access",
                "update",
                "byoc-1",
                "--mode",
                "never",
            ],
            true,
        );
    }

    #[test]
    fn byoc_access_update_parses_modes_and_window_bounds() {
        let never = access_update(&["--mode", "never"]).unwrap();
        assert_eq!(never.mode, ByocAccessMode::Never);
        assert_eq!(never.start_time, None);
        assert_eq!(never.duration_seconds, None);

        let window = access_update(&[
            "--mode",
            "window",
            "--start-time",
            "2026-11-07T10:00:00+02:00",
            "--duration-seconds",
            "172800",
        ])
        .unwrap();
        assert_eq!(window.mode, ByocAccessMode::Window);
        assert_eq!(
            window.start_time.map(|start| start.to_rfc3339()),
            Some("2026-11-07T08:00:00+00:00".to_string())
        );
        assert_eq!(window.duration_seconds, Some(172_800));

        for missing in [
            &["--mode", "window", "--duration-seconds", "60"][..],
            &["--mode", "window", "--start-time", "2026-11-07T08:00:00Z"][..],
            &[][..],
        ] {
            assert_eq!(
                access_update(missing).unwrap_err().kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{missing:?}"
            );
        }
        for invalid in [
            &[
                "--mode",
                "window",
                "--start-time",
                "2026-11-07",
                "--duration-seconds",
                "60",
            ][..],
            &[
                "--mode",
                "window",
                "--start-time",
                "2026-11-07T08:00:00Z",
                "--duration-seconds",
                "172801",
            ][..],
            &[
                "--mode",
                "window",
                "--start-time",
                "2026-11-07T08:00:00Z",
                "--duration-seconds",
                "0",
            ][..],
        ] {
            assert_eq!(
                access_update(invalid).unwrap_err().kind(),
                clap::error::ErrorKind::ValueValidation,
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn preview_list_is_a_read_and_update_requires_an_explicit_value() {
        assert_write(&["clickhousectl", "cloud", "org", "preview", "list"], false);
        for value in ["true", "false"] {
            let args = [
                "clickhousectl",
                "cloud",
                "org",
                "preview",
                "update",
                "preview-1",
                "--enabled",
                value,
            ];
            let CloudCommands::Org {
                command:
                    OrgCommands::Preview {
                        command:
                            PreviewCommands::Update {
                                preview_id,
                                enabled,
                            },
                    },
            } = parse_cloud_command(&args)
            else {
                panic!("expected preview update");
            };
            assert_eq!(preview_id, "preview-1");
            assert_eq!(enabled.to_string(), value);
            assert_write(&args, true);
        }

        for args in [
            &[
                "clickhousectl",
                "cloud",
                "org",
                "preview",
                "update",
                "preview-1",
            ][..],
            &[
                "clickhousectl",
                "cloud",
                "org",
                "preview",
                "update",
                "--enabled",
                "true",
            ][..],
        ] {
            assert_eq!(
                Cli::try_parse_from(args)
                    .err()
                    .expect("a required argument is missing")
                    .kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{args:?}"
            );
        }
        let bare_flag = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "preview",
            "update",
            "preview-1",
            "--enabled",
        ]);
        assert!(bare_flag.is_err(), "--enabled needs a value");
    }

    fn parse_member_notification_settings(args: &[&str]) -> MemberNotificationSettingsCommands {
        let mut full = vec!["clickhousectl", "cloud", "member", "notification-settings"];
        full.extend(args);
        let CloudCommands::Member {
            command: MemberCommands::NotificationSettings { command },
        } = parse_cloud_command(&full)
        else {
            panic!("expected member notification-settings");
        };
        command
    }

    #[test]
    fn member_notification_settings_list_is_a_read_with_page_and_filter_flags() {
        let MemberNotificationSettingsCommands::List(minimal) =
            parse_member_notification_settings(&["list", "user-1"])
        else {
            panic!("expected list");
        };
        assert_eq!(minimal.user_id.id.as_deref(), Some("user-1"));
        assert_eq!(minimal.limit, None);
        assert_eq!(minimal.cursor, None);
        assert_eq!(minimal.notification_type, None);
        assert_eq!(minimal.resource_id, None);
        assert_eq!(minimal.resource_type, None);

        let MemberNotificationSettingsCommands::List(maximal) =
            parse_member_notification_settings(&[
                "list",
                "--email",
                "dev@example.com",
                "--limit",
                "250",
                "--cursor",
                "-opaque",
                "--type",
                "service_backup_failed",
                "--resource-id",
                "svc-1",
                "--resource-type",
                "service",
            ])
        else {
            panic!("expected list");
        };
        assert_eq!(maximal.user_id.email.as_deref(), Some("dev@example.com"));
        assert_eq!(maximal.limit, Some(250));
        assert_eq!(maximal.cursor.as_deref(), Some("-opaque"));
        assert_eq!(
            maximal.notification_type.as_deref(),
            Some("service_backup_failed")
        );
        assert_eq!(maximal.resource_id.as_deref(), Some("svc-1"));
        assert_eq!(maximal.resource_type.as_deref(), Some("service"));

        for limit in ["0", "251"] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "member",
                "notification-settings",
                "list",
                "user-1",
                "--limit",
                limit,
            ])
            .err()
            .expect("out-of-range limit must fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "member",
                "notification-settings",
                "list",
                "user-1",
            ],
            false,
        );
    }

    #[test]
    fn member_notification_settings_update_is_a_write_with_repeatable_rule_flags() {
        let base = [
            "update",
            "user-1",
            "--effect",
            "disabled",
            "--type",
            "service_backup_failed",
            "--type",
            "billing_credit_low",
            "--resource-id",
            "svc-1",
            "--resource-id",
            "org-1",
            "--channel",
            "ui",
            "--channel",
            "email",
        ];
        let MemberNotificationSettingsCommands::Update(update) =
            parse_member_notification_settings(&base)
        else {
            panic!("expected update");
        };
        assert_eq!(update.effect, NotificationSettingEffect::Disabled);
        assert_eq!(
            update.notification_types,
            ["service_backup_failed", "billing_credit_low"]
        );
        assert_eq!(update.resource_ids, ["svc-1", "org-1"]);
        assert_eq!(
            update.channels,
            [NotificationChannel::Ui, NotificationChannel::Email]
        );

        let mut write = vec!["clickhousectl", "cloud", "member", "notification-settings"];
        write.extend(base);
        assert_write(&write, true);

        let without_channel = parse_member_notification_settings(&[
            "update",
            "user-1",
            "--effect",
            "inherit",
            "--type",
            "service_backup_failed",
            "--resource-id",
            "svc-1",
        ]);
        let MemberNotificationSettingsCommands::Update(without_channel) = without_channel else {
            panic!("expected update");
        };
        assert_eq!(without_channel.effect, NotificationSettingEffect::Inherit);
        assert!(without_channel.channels.is_empty());

        for missing in [
            &["--type", "service_backup_failed", "--resource-id", "svc-1"][..],
            &["--effect", "enabled", "--resource-id", "svc-1"][..],
            &["--effect", "enabled", "--type", "service_backup_failed"][..],
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "member",
                "notification-settings",
                "update",
                "user-1",
            ];
            args.extend(missing);
            let error = Cli::try_parse_from(args)
                .err()
                .expect("a required flag is missing");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{missing:?}"
            );
        }
        let empty_type = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "member",
            "notification-settings",
            "update",
            "user-1",
            "--effect",
            "enabled",
            "--type",
            "",
            "--resource-id",
            "svc-1",
        ]);
        assert!(empty_type.is_err(), "an empty --type must be rejected");
    }
}
