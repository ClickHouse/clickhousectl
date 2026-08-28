use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent};
use crate::cloud::shared::{parse_datetime, parse_serde_enum, parse_tags, resolve_org_id};
use clap::{ArgGroup, Subcommand};
use clickhouse_cloud_api::models::{
    ApiResponse, PgBouncerConfig, PgConfig, PgHaType, PgIdProperty, PgProvider, PgVersion,
    PostgresInstanceConfig, PostgresService, PostgresServiceListItem, PostgresServicePatchRequest,
    PostgresServicePostRequest, PostgresServiceReadReplicaRequest, PostgresServiceRestoreRequest,
    PostgresServiceSetPassword, PostgresServiceSetState, PostgresServiceSetStateCommand,
    ResourceTagsV1, ResourceTagsV1Response,
};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum PostgresCommands {
    /// List Postgres services in the organization
    List {
        #[arg(long)]
        org_id: Option<String>,
        /// Filter results by field (e.g. --filter state=running)
        #[arg(long)]
        filter: Vec<String>,
    },

    /// Get details for a single Postgres service
    Get {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create a new Postgres service
    Create {
        /// Service name
        #[arg(long)]
        name: String,
        /// Cloud region (e.g. us-east-1)
        #[arg(long)]
        region: String,
        /// Instance size (e.g. m7i.2xlarge). Server validates — accepts any value.
        #[arg(long)]
        size: String,
        /// Cloud provider
        #[arg(long, default_value = "aws")]
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
        /// Path to a JSON file with a PgBouncerConfig object
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update an existing Postgres service (metadata only)
    Update {
        postgres_id: String,
        #[arg(long)]
        size: Option<String>,
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(PgHaType::VALUES))]
        ha_type: Option<String>,
        /// Add a tag (repeatable), e.g. --add-tag env=prod
        #[arg(long)]
        add_tag: Vec<String>,
        /// Remove a tag by key (repeatable)
        #[arg(long)]
        remove_tag: Vec<String>,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete a Postgres service
    Delete {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage CA certificates
    #[command(subcommand)]
    Certs(CertsCommands),

    /// Manage Postgres runtime configuration
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Reset the Postgres service password
    #[command(
        group(ArgGroup::new("password_source").required(true).args(["password", "generate"]))
    )]
    ResetPassword {
        postgres_id: String,
        /// New password (min 12, must include upper, lower, digit)
        #[arg(long, conflicts_with = "generate")]
        password: Option<String>,
        /// Generate a random compliant password and print it
        #[arg(long, conflicts_with = "password")]
        generate: bool,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage read replicas
    #[command(name = "read-replica", subcommand)]
    ReadReplica(ReadReplicaCommands),

    /// Restore a Postgres service to a point in time
    Restore {
        /// Source Postgres service ID
        postgres_id: String,
        /// Name for the restored service
        #[arg(long)]
        name: String,
        /// Point-in-time target (ISO 8601 / RFC 3339, e.g. 2026-04-16T12:00:00Z)
        #[arg(long, value_parser = parse_datetime)]
        restore_target: String,
        #[arg(long)]
        tag: Vec<String>,
        #[arg(long)]
        pg_config_file: Option<PathBuf>,
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Restart a Postgres service
    Restart {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Promote a read replica to primary
    Promote {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Switch over between primary and replica
    Switchover {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CertsCommands {
    /// Get the CA certificate bundle (PEM) for a Postgres service
    Get {
        postgres_id: String,
        /// Write PEM to the given file (mode 0600 on unix) instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get current runtime configuration (pgConfig + pgBouncerConfig)
    Get {
        postgres_id: String,
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Replace the entire runtime configuration
    Replace {
        postgres_id: String,
        /// JSON file with a full PostgresInstanceConfig object
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Patch selected runtime configuration fields
    #[command(
        group(ArgGroup::new("patch_source").required(true).args(["sets", "file"]))
    )]
    Patch {
        postgres_id: String,
        /// Set a pgConfig field (repeatable), e.g. --set max_connections=500
        #[arg(long = "set", conflicts_with = "file")]
        sets: Vec<String>,
        /// JSON file with a partial PostgresInstanceConfig object
        #[arg(long, conflicts_with = "sets")]
        file: Option<PathBuf>,
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ReadReplicaCommands {
    /// Create a read replica of an existing Postgres service
    Create {
        /// Source Postgres service ID
        postgres_id: String,
        /// Name for the new replica
        #[arg(long)]
        name: String,
        #[arg(long)]
        tag: Vec<String>,
        #[arg(long)]
        pg_config_file: Option<PathBuf>,
        #[arg(long)]
        pg_bouncer_config_file: Option<PathBuf>,
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl PostgresCommands {
    pub fn is_write(&self) -> bool {
        match self {
            PostgresCommands::List { .. } | PostgresCommands::Get { .. } => false,
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

pub async fn run(client: &CloudClient, command: PostgresCommands, json: bool) -> CloudResult<()> {
    match command {
        PostgresCommands::List { org_id, filter } => {
            postgres_list(client, org_id.as_deref(), &filter, json).await
        }
        PostgresCommands::Get {
            postgres_id,
            org_id,
        } => postgres_get(client, &postgres_id, org_id.as_deref(), json).await,
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
            org_id,
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
                org_id: org_id.as_deref(),
            };
            postgres_create(client, opts, json).await
        }
        PostgresCommands::Update {
            postgres_id,
            size,
            ha_type,
            add_tag,
            remove_tag,
            org_id,
        } => {
            let opts = PostgresUpdateOptions {
                size: size.as_deref(),
                ha_type: ha_type.as_deref(),
                add_tag: &add_tag,
                remove_tag: &remove_tag,
                org_id: org_id.as_deref(),
            };
            postgres_update(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Delete {
            postgres_id,
            org_id,
        } => postgres_delete(client, &postgres_id, org_id.as_deref(), json).await,
        PostgresCommands::Certs(CertsCommands::Get {
            postgres_id,
            output,
            org_id,
        }) => {
            postgres_certs_get(
                client,
                &postgres_id,
                output.as_deref(),
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Config(ConfigCommands::Get {
            postgres_id,
            org_id,
        }) => postgres_config_get(client, &postgres_id, org_id.as_deref(), json).await,
        PostgresCommands::Config(ConfigCommands::Replace {
            postgres_id,
            file,
            org_id,
        }) => postgres_config_replace(client, &postgres_id, &file, org_id.as_deref(), json).await,
        PostgresCommands::Config(ConfigCommands::Patch {
            postgres_id,
            sets,
            file,
            org_id,
        }) => {
            postgres_config_patch(
                client,
                &postgres_id,
                &sets,
                file.as_deref(),
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::ResetPassword {
            postgres_id,
            password,
            generate,
            org_id,
        } => {
            postgres_reset_password(
                client,
                &postgres_id,
                password.as_deref(),
                generate,
                org_id.as_deref(),
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
            org_id,
        }) => {
            let opts = PostgresReadReplicaOptions {
                name: &name,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                org_id: org_id.as_deref(),
            };
            postgres_read_replica_create(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Restore {
            postgres_id,
            name,
            restore_target,
            tag,
            pg_config_file,
            pg_bouncer_config_file,
            org_id,
        } => {
            let opts = PostgresRestoreOptions {
                name: &name,
                restore_target: &restore_target,
                tags: &tag,
                pg_config_file: pg_config_file.as_deref(),
                pg_bouncer_config_file: pg_bouncer_config_file.as_deref(),
                org_id: org_id.as_deref(),
            };
            postgres_restore(client, &postgres_id, opts, json).await
        }
        PostgresCommands::Restart {
            postgres_id,
            org_id,
        } => {
            postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Restart,
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Promote {
            postgres_id,
            org_id,
        } => {
            postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Promote,
                org_id.as_deref(),
                json,
            )
            .await
        }
        PostgresCommands::Switchover {
            postgres_id,
            org_id,
        } => {
            postgres_state_change(
                client,
                &postgres_id,
                PostgresServiceSetStateCommand::Switchover,
                org_id.as_deref(),
                json,
            )
            .await
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unwrap_api<T>(resp: ApiResponse<T>) -> CloudResult<T> {
    resp.result
        .ok_or_else(|| CloudError::new("API response was missing a result body"))
}

fn parse_pg_size(value: &str) -> CloudResult<clickhouse_cloud_api::models::PgSize> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| CloudError::new(format!("invalid size '{}': {}", value, e)))
}

fn load_json_file<T: DeserializeOwned>(path: &Path) -> CloudResult<T> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| CloudError::new(format!("failed to read {}: {}", path.display(), e)))?;
    serde_json::from_str(&contents)
        .map_err(|e| CloudError::new(format!("failed to parse {} as JSON: {}", path.display(), e)))
}

/// Builds the `postgres config` write body from a user-supplied JSON document.
///
/// The document root must be a JSON object; anything else (a scalar, an array,
/// `null`) is rejected rather than read as "no sections", which would send an
/// empty body and reset the configuration.
///
/// The API rejects a body that omits either `pgConfig` or `pgBouncerConfig`, and
/// the request model is strict (no serde defaults), so an omitted key of an
/// object root resolves to an empty object here — explicitly, at the point of use.
fn instance_config_from_json(doc: &serde_json::Value) -> CloudResult<PostgresInstanceConfig> {
    let root = doc
        .as_object()
        .ok_or_else(|| CloudError::new("configuration document must be a JSON object"))?;
    let section = |key: &str| {
        root.get(key)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))
    };
    Ok(PostgresInstanceConfig {
        pg_config: serde_json::from_value(section("pgConfig"))
            .map_err(|e| CloudError::new(format!("invalid pgConfig: {}", e)))?,
        pg_bouncer_config: serde_json::from_value(section("pgBouncerConfig"))
            .map_err(|e| CloudError::new(format!("invalid pgBouncerConfig: {}", e)))?,
    })
}

/// Parse `--set key=value` overrides into a JSON object.
///
/// Each value is parsed as JSON first (so `max_connections=500` becomes a number),
/// falling back to a string if JSON parsing fails (`statement_timeout=5s`).
pub(super) fn parse_pg_config_overrides(
    sets: &[String],
) -> CloudResult<serde_json::Map<String, serde_json::Value>> {
    let mut out = serde_json::Map::new();
    for entry in sets {
        let (key, val) = entry.split_once('=').ok_or_else(|| {
            CloudError::new(format!("invalid --set '{}': expected key=value", entry))
        })?;
        let key = key.trim();
        if key.is_empty() {
            return Err(CloudError::new(format!(
                "invalid --set '{}': key cannot be empty",
                entry
            )));
        }
        let parsed = serde_json::from_str::<serde_json::Value>(val)
            .unwrap_or_else(|_| serde_json::Value::String(val.to_string()));
        out.insert(key.to_string(), parsed);
    }
    Ok(out)
}

fn generate_compliant_password() -> String {
    // Two UUIDv4s give 64 cryptographically-random hex chars (lowercase + digits).
    // Prefix "A1" ensures uppercase + digit presence; overall length 66, min-12 satisfied.
    let u1 = uuid::Uuid::new_v4().simple().to_string();
    let u2 = uuid::Uuid::new_v4().simple().to_string();
    format!("A1{}{}", u1, u2)
}

fn validate_password(pw: &str) -> CloudResult<()> {
    if pw.len() < 12 {
        return Err(CloudError::new("password must be at least 12 characters"));
    }
    let has_lower = pw.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = pw.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = pw.chars().any(|c| c.is_ascii_digit());
    if !(has_lower && has_upper && has_digit) {
        return Err(CloudError::new(
            "password must include at least one lowercase, one uppercase, and one digit",
        ));
    }
    Ok(())
}

fn write_pem_file(path: &Path, pem: &str) -> CloudResult<()> {
    use std::io::Write;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(pem.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        let mut f = std::fs::File::create(path)?;
        f.write_all(pem.as_bytes())?;
    }
    Ok(())
}

fn apply_filter(item: &PostgresServiceListItem, filters: &[String]) -> bool {
    for filter in filters {
        let Some((key, val)) = filter.split_once('=') else {
            continue;
        };
        // A response field the API omitted matches no filter value.
        let matches = match key.trim() {
            "state" => item
                .state
                .as_ref()
                .is_some_and(|s| format!("{:?}", s).eq_ignore_ascii_case(val)),
            "region" => item.region.as_deref() == Some(val),
            "name" => item.name.as_deref() == Some(val),
            "provider" => item
                .provider
                .as_ref()
                .is_some_and(|p| format!("{:?}", p).eq_ignore_ascii_case(val)),
            _ => true,
        };
        if !matches {
            return false;
        }
    }
    true
}

fn state_label(s: Option<&clickhouse_cloud_api::models::PgStateProperty>) -> String {
    match s {
        Some(s) => serde_json::to_value(s)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", s)),
        None => ABSENT.to_string(),
    }
}

fn enum_label<T: serde::Serialize>(v: Option<&T>) -> String {
    match v {
        Some(v) => serde_json::to_value(v)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default(),
        None => ABSENT.to_string(),
    }
}

fn render_postgres_service(svc: &PostgresService) {
    println!("  ID: {}", or_absent(svc.id.as_ref()));
    println!("  Name: {}", or_absent(svc.name.as_deref()));
    println!("  State: {}", state_label(svc.state.as_ref()));
    println!("  Provider: {}", enum_label(svc.provider.as_ref()));
    println!("  Region: {}", or_absent(svc.region.as_deref()));
    println!("  Size: {}", enum_label(svc.size.as_ref()));
    println!("  Storage (GB): {}", or_absent(svc.storage_size));
    println!(
        "  PG version: {}",
        enum_label(svc.postgres_version.as_ref())
    );
    println!("  HA type: {}", enum_label(svc.ha_type.as_ref()));
    println!("  Primary: {}", or_absent(svc.is_primary));
    println!("  Host: {}", or_absent(svc.hostname.as_deref()));
    println!("  Username: {}", or_absent(svc.username.as_deref()));
    println!(
        "  Created: {}",
        or_absent(svc.created_at.map(|c| c.to_rfc3339()))
    );
    if let Some(svc_tags) = svc.tags.as_ref().filter(|t| !t.is_empty()) {
        let tags: Vec<String> = svc_tags
            .iter()
            .map(|t| match (t.key.as_deref(), t.value.as_deref()) {
                (key, Some(value)) => format!("{}={}", or_absent(key), value),
                (key, None) => or_absent(key).to_string(),
            })
            .collect();
        println!("  Tags: {}", tags.join(", "));
    }
}

fn merge_tags(
    existing: &[ResourceTagsV1],
    add: &[ResourceTagsV1],
    remove_keys: &[String],
) -> Vec<ResourceTagsV1> {
    let remove: std::collections::HashSet<&str> = remove_keys.iter().map(|s| s.as_str()).collect();
    let add_keys: std::collections::HashSet<&str> = add.iter().map(|t| t.key.as_str()).collect();

    let mut merged: Vec<ResourceTagsV1> = existing
        .iter()
        .filter(|t| !remove.contains(t.key.as_str()) && !add_keys.contains(t.key.as_str()))
        .cloned()
        .collect();
    merged.extend(add.iter().cloned());
    merged
}

/// Merges `--add-tag`/`--remove-tag` against the tag list a GET returned.
///
/// An omitted `tags` in the response is indistinguishable from a field the API
/// dropped, so a read-modify-write must not proceed on it: `tags` is replaced
/// wholesale by the PATCH, so merging against an assumed empty set would delete
/// every tag the service still has. A returned tag without a key cannot be sent
/// back either, so say so rather than dropping it from the merged set.
fn merge_response_tags(
    current: Option<Vec<ResourceTagsV1Response>>,
    add: &[ResourceTagsV1],
    remove_keys: &[String],
) -> CloudResult<Vec<ResourceTagsV1>> {
    let current = current.ok_or_else(|| {
        CloudError::new(
            "the API response omitted the tags field, so --add-tag/--remove-tag cannot be merged \
             safely: an update replaces the tag set wholesale, and merging against an assumed empty \
             set would delete any tags the service already has",
        )
    })?;
    let existing = current
        .into_iter()
        .map(ResourceTagsV1::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CloudError::new(error.to_string()))?;
    Ok(merge_tags(&existing, add, remove_keys))
}

// ---------------------------------------------------------------------------
// Option structs (for commands with many args)
// ---------------------------------------------------------------------------

pub struct PostgresCreateOptions<'a> {
    pub name: &'a str,
    pub region: &'a str,
    pub size: &'a str,
    pub provider: &'a str,
    pub pg_version: Option<&'a str>,
    pub ha_type: Option<&'a str>,
    pub tags: &'a [String],
    pub pg_config_file: Option<&'a Path>,
    pub pg_bouncer_config_file: Option<&'a Path>,
    pub org_id: Option<&'a str>,
}

pub struct PostgresUpdateOptions<'a> {
    pub size: Option<&'a str>,
    pub ha_type: Option<&'a str>,
    pub add_tag: &'a [String],
    pub remove_tag: &'a [String],
    pub org_id: Option<&'a str>,
}

pub struct PostgresReadReplicaOptions<'a> {
    pub name: &'a str,
    pub tags: &'a [String],
    pub pg_config_file: Option<&'a Path>,
    pub pg_bouncer_config_file: Option<&'a Path>,
    pub org_id: Option<&'a str>,
}

pub struct PostgresRestoreOptions<'a> {
    pub name: &'a str,
    pub restore_target: &'a str,
    pub tags: &'a [String],
    pub pg_config_file: Option<&'a Path>,
    pub pg_bouncer_config_file: Option<&'a Path>,
    pub org_id: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn postgres_list(
    client: &CloudClient,
    org_id: Option<&str>,
    filters: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let resp = client
        .api()
        .postgres_service_get_list(&org_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let items = unwrap_api(resp)?;
    let filtered: Vec<PostgresServiceListItem> = items
        .into_iter()
        .filter(|i| apply_filter(i, filters))
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
        return Ok(());
    }

    if filtered.is_empty() {
        println!("No Postgres services found");
        return Ok(());
    }

    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "State")]
        state: String,
        #[tabled(rename = "Region")]
        region: String,
        #[tabled(rename = "Size")]
        size: String,
        #[tabled(rename = "PG")]
        pg: String,
        #[tabled(rename = "HA")]
        ha: String,
        #[tabled(rename = "Primary")]
        primary: String,
    }

    let rows: Vec<Row> = filtered
        .into_iter()
        .map(|i| Row {
            name: or_absent(i.name.as_deref()),
            id: or_absent(i.id.as_ref()),
            state: state_label(i.state.as_ref()),
            region: or_absent(i.region.as_deref()),
            size: enum_label(i.size.as_ref()),
            pg: enum_label(i.postgres_version.as_ref()),
            ha: enum_label(i.ha_type.as_ref()),
            primary: match i.is_primary {
                Some(true) => "yes".to_string(),
                Some(false) => "no".to_string(),
                None => ABSENT.to_string(),
            },
        })
        .collect();

    println!("{}", Table::new(rows).with(Style::markdown()));
    Ok(())
}

pub async fn postgres_get(
    client: &CloudClient,
    postgres_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let resp = client
        .api()
        .postgres_service_get(&org_id, postgres_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        // No connection string here: the get endpoint stops returning
        // credentials on July 31, 2026 — they are only available from
        // `postgres create` and `postgres reset-password`.
        render_postgres_service(&svc);
    }
    Ok(())
}

/// The post-create credentials block, or the warning that replaces it.
///
/// The password is returned once, so a placeholder in its place would be read
/// as the credential itself. An absent password therefore gets the omission
/// plus the command that mints a usable one; the create succeeded and the
/// password is recoverable, so this is a warning rather than an error. An empty
/// string is a password the API sent.
///
/// `connection_string` is the non-empty connection string the same response
/// carried, if any: the spec says it embeds the service password, so when it is
/// present the credential is not actually lost and telling the user to reset
/// would rotate a working password for nothing.
fn postgres_credentials_block(
    username: Option<&str>,
    password: Option<&str>,
    connection_string: Option<&str>,
    postgres_id: Option<&PgIdProperty>,
) -> String {
    match (password, connection_string, postgres_id) {
        (Some(password), _, _) => format!(
            "Credentials (save these — password shown only once):\n  Username: {}\n  Password: {}",
            or_absent(username),
            password
        ),
        (None, Some(_), _) => "WARNING: the API response omitted the `password` field, so the \
                               password cannot be shown on its own.\nThe connection string below \
                               embeds it, so no password reset is needed."
            .to_string(),
        (None, None, Some(id)) => format!(
            "WARNING: the API response omitted the one-time password, so it cannot be shown.\n\
             The service was created; reset the password to get a usable credential:\n  \
             clickhousectl cloud postgres reset-password {} --generate",
            id
        ),
        (None, None, None) => "WARNING: the API response omitted the one-time password, so it \
                               cannot be shown.\nThe service was created; once you have its id, \
                               reset the password with `clickhousectl cloud postgres \
                               reset-password <postgres-id> --generate` to get a usable credential."
            .to_string(),
    }
}

pub async fn postgres_create(
    client: &CloudClient,
    opts: PostgresCreateOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, opts.org_id).await?;

    let provider: PgProvider = parse_serde_enum(opts.provider, "provider", PgProvider::VALUES)?;
    let size = parse_pg_size(opts.size)?;
    let pg_version: Option<PgVersion> = opts
        .pg_version
        .map(|v| parse_serde_enum(v, "pg-version", PgVersion::VALUES))
        .transpose()?;
    let ha_type: Option<PgHaType> = opts
        .ha_type
        .map(|v| parse_serde_enum(v, "ha-type", PgHaType::VALUES))
        .transpose()?;
    let tags = parse_tags(opts.tags)?;
    let pg_config = opts
        .pg_config_file
        .map(load_json_file::<PgConfig>)
        .transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;

    let req = PostgresServicePostRequest {
        name: opts.name.to_string(),
        provider,
        region: opts.region.to_string(),
        size,
        postgres_version: pg_version,
        ha_type,
        tags,
        pg_config,
        pg_bouncer_config,
    };

    let resp = client
        .api()
        .postgres_service_create(&org_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Postgres service created");
        println!();
        render_postgres_service(&svc);
        println!();
        let connection_string = svc
            .connection_string
            .as_deref()
            .filter(|conn| !conn.is_empty());
        println!(
            "{}",
            postgres_credentials_block(
                svc.username.as_deref(),
                svc.password.as_deref(),
                connection_string,
                svc.id.as_ref()
            )
        );
        if let Some(conn) = connection_string {
            println!("  Connection string: {}", conn);
        }
    }
    Ok(())
}

pub async fn postgres_update(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresUpdateOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, opts.org_id).await?;

    let size = opts.size.map(parse_pg_size).transpose()?;
    let ha_type = opts
        .ha_type
        .map(|v| parse_serde_enum::<PgHaType>(v, "ha-type", PgHaType::VALUES))
        .transpose()?;

    // Merge tag add/remove against current tags if any tag changes requested.
    let tags = if !opts.add_tag.is_empty() || !opts.remove_tag.is_empty() {
        let current = client
            .api()
            .postgres_service_get(&org_id, postgres_id)
            .await
            .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
        let current = unwrap_api(current)?;
        let add = parse_tags(opts.add_tag)?.unwrap_or_default();
        Some(merge_response_tags(current.tags, &add, opts.remove_tag)?)
    } else {
        None
    };

    let req = PostgresServicePatchRequest {
        name: None,
        size,
        ha_type,
        tags,
    };

    let resp = client
        .api()
        .postgres_service_patch(&org_id, postgres_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Postgres service updated");
        println!();
        render_postgres_service(&svc);
    }
    Ok(())
}

pub async fn postgres_delete(
    client: &CloudClient,
    postgres_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;

    // The delete endpoint itself only ever returns the raw API envelope
    // (`ApiResponse<serde_json::Value>`, no resource in `result`), so fetch the
    // resource before deleting it and render that instead: `--json` output must
    // stay consistent with every other `cloud postgres` subcommand, which emits
    // the resource object rather than `{"status":...,"requestId":...}` (#614).
    let resp = client
        .api()
        .postgres_service_get(&org_id, postgres_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    client
        .api()
        .postgres_service_delete(&org_id, postgres_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Postgres service {} deletion initiated", postgres_id);
    }
    Ok(())
}

pub async fn postgres_certs_get(
    client: &CloudClient,
    postgres_id: &str,
    output: Option<&Path>,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let pem = client
        .api()
        .postgres_service_certs_get(&org_id, postgres_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;

    if let Some(path) = output {
        write_pem_file(path, &pem)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "path": path.display().to_string(),
                }))?
            );
        } else {
            println!("Wrote CA certificate to {}", path.display());
        }
        return Ok(());
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "certificate": pem }))?
        );
    } else {
        print!("{}", pem);
        if !pem.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

pub async fn postgres_config_get(
    client: &CloudClient,
    postgres_id: &str,
    org_id: Option<&str>,
    _json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let resp = client
        .api()
        .postgres_instance_config_get(&org_id, postgres_id)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let cfg = unwrap_api(resp)?;
    // Config is a flat 20+ field object — always emit as JSON (pretty).
    println!("{}", serde_json::to_string_pretty(&cfg)?);
    Ok(())
}

pub async fn postgres_config_replace(
    client: &CloudClient,
    postgres_id: &str,
    file: &Path,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let cfg = instance_config_from_json(&load_json_file::<serde_json::Value>(file)?)?;
    let resp = client
        .api()
        .postgres_instance_config_post(&org_id, postgres_id, &cfg)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let out = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Configuration replaced");
        if let Some(msg) = &out.message {
            println!("Note: {}", msg);
        }
    }
    Ok(())
}

pub async fn postgres_config_patch(
    client: &CloudClient,
    postgres_id: &str,
    sets: &[String],
    file: Option<&Path>,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;

    debug_assert!(
        !sets.is_empty() || file.is_some(),
        "clap ArgGroup(\"patch_source\") requires --set or --file"
    );

    let cfg = if let Some(path) = file {
        instance_config_from_json(&load_json_file::<serde_json::Value>(path)?)?
    } else {
        // Build the request body from --set entries: the overrides become the
        // pgConfig object, and pgBouncerConfig resolves to `{}`.
        let overrides = parse_pg_config_overrides(sets)?;
        instance_config_from_json(&serde_json::json!({
            "pgConfig": serde_json::Value::Object(overrides),
        }))
        .map_err(|e| CloudError::new(format!("failed to build config from --set entries: {}", e)))?
    };

    let resp = client
        .api()
        .postgres_instance_config_patch(&org_id, postgres_id, &cfg)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let out = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Configuration patched");
        if let Some(msg) = &out.message {
            println!("Note: {}", msg);
        }
    }
    Ok(())
}

pub async fn postgres_reset_password(
    client: &CloudClient,
    postgres_id: &str,
    password: Option<&str>,
    generate: bool,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;

    let pw = match (password, generate) {
        (Some(p), false) => {
            validate_password(p)?;
            p.to_string()
        }
        (None, true) => generate_compliant_password(),
        (None, false) => {
            unreachable!("clap ArgGroup(\"password_source\") requires --password or --generate")
        }
        (Some(_), true) => unreachable!("clap conflicts_with prevents this"),
    };

    let req = PostgresServiceSetPassword {
        password: pw.clone(),
    };
    let resp = client
        .api()
        .postgres_service_set_password(&org_id, postgres_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let out = unwrap_api(resp)?;
    // Emit what the user now needs to use: the API echoes the password back, but
    // fall back to the one we sent if the response omits it.
    let password = out.password.unwrap_or(pw);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "password": password,
            }))?
        );
    } else {
        println!("Password reset successfully");
        if generate {
            println!();
            println!("Generated password (save this — not recoverable):");
            println!("  {}", password);
        }
    }
    Ok(())
}

pub async fn postgres_read_replica_create(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresReadReplicaOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, opts.org_id).await?;
    let tags = parse_tags(opts.tags)?;
    let pg_config = opts
        .pg_config_file
        .map(load_json_file::<PgConfig>)
        .transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;

    let req = PostgresServiceReadReplicaRequest {
        name: opts.name.to_string(),
        tags,
        pg_config,
        pg_bouncer_config,
    };

    let resp = client
        .api()
        .postgres_instance_create_read_replica(&org_id, postgres_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Read replica created");
        println!();
        render_postgres_service(&svc);
    }
    Ok(())
}

pub async fn postgres_restore(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresRestoreOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, opts.org_id).await?;
    let tags = parse_tags(opts.tags)?;
    let pg_config = opts
        .pg_config_file
        .map(load_json_file::<PgConfig>)
        .transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;
    let restore_target = chrono::DateTime::parse_from_rfc3339(opts.restore_target)
        .map_err(|e| CloudError::new(format!("invalid restore-target: {}", e)))?
        .with_timezone(&chrono::Utc);

    let req = PostgresServiceRestoreRequest {
        name: opts.name.to_string(),
        restore_target,
        tags,
        pg_config,
        pg_bouncer_config,
    };

    let resp = client
        .api()
        .postgres_instance_restore(&org_id, postgres_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Postgres service restore initiated");
        println!();
        render_postgres_service(&svc);
    }
    Ok(())
}

pub async fn postgres_state_change(
    client: &CloudClient,
    postgres_id: &str,
    cmd: PostgresServiceSetStateCommand,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let req = PostgresServiceSetState { command: cmd };
    let resp = client
        .api()
        .postgres_service_patch_state(&org_id, postgres_id, &req)
        .await
        .map_err(|e| client.convert_error_for_organization(e, &org_id))?;
    let svc = unwrap_api(resp)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("State change accepted");
        println!();
        render_postgres_service(&svc);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests (CLI parsing + helpers)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    #[derive(Parser)]
    struct PostgresCli {
        #[command(subcommand)]
        command: PostgresCommands,
    }

    fn parse_postgres(args: &[&str]) -> PostgresCommands {
        assert_eq!(args.get(1), Some(&"cloud"));
        assert_eq!(args.get(2), Some(&"postgres"));
        PostgresCli::try_parse_from(std::iter::once(args[0]).chain(args.iter().skip(3).copied()))
            .expect("parse")
            .command
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
        ]);
        let PostgresCommands::List { filter, .. } = cmd else {
            panic!("expected list");
        };
        assert_eq!(filter, vec!["state=running", "region=us-east-1"]);
    }

    #[test]
    fn parses_postgres_get() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "get", "pg-1"]);
        let PostgresCommands::Get { postgres_id, .. } = cmd else {
            panic!("expected get");
        };
        assert_eq!(postgres_id, "pg-1");
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
            ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert_eq!(postgres_id, "pg-1");
        assert_eq!(size.as_deref(), Some("c6gd.large"));
        assert_eq!(add_tag, vec!["env=prod", "team=data"]);
        assert_eq!(remove_tag, vec!["old"]);
    }

    #[test]
    fn parses_postgres_update_no_fields() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "update", "pg-1"]);
        let PostgresCommands::Update {
            postgres_id, size, ..
        } = cmd
        else {
            panic!("expected update");
        };
        assert_eq!(postgres_id, "pg-1");
        assert!(size.is_none());
    }

    #[test]
    fn parses_postgres_delete() {
        let cmd = parse_postgres(&["clickhousectl", "cloud", "postgres", "delete", "pg-1"]);
        let PostgresCommands::Delete { postgres_id, .. } = cmd else {
            panic!("expected delete");
        };
        assert_eq!(postgres_id, "pg-1");
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
        let message = err.to_string();
        assert!(message.contains("--set <SETS>"), "{message}");
        assert!(message.contains("--file <FILE>"), "{message}");
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
        assert_eq!(postgres_id, "pg-1");
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

    // --- helper unit tests ---

    #[test]
    fn parse_pg_config_overrides_numeric_and_string() {
        let m = parse_pg_config_overrides(&[
            "max_connections=500".into(),
            "random_page_cost=1.1".into(),
            "statement_timeout=5s".into(),
        ])
        .unwrap();
        assert_eq!(m.get("max_connections"), Some(&serde_json::json!(500)));
        assert_eq!(m.get("random_page_cost"), Some(&serde_json::json!(1.1)));
        assert_eq!(
            m.get("statement_timeout"),
            Some(&serde_json::Value::String("5s".to_string()))
        );
    }

    #[test]
    fn parse_pg_config_overrides_rejects_malformed() {
        assert!(parse_pg_config_overrides(&["no_equals".into()]).is_err());
        assert!(parse_pg_config_overrides(&["=value".into()]).is_err());
    }

    #[test]
    fn parse_pg_config_overrides_last_wins_on_duplicates() {
        let m = parse_pg_config_overrides(&[
            "max_connections=100".into(),
            "max_connections=200".into(),
        ])
        .unwrap();
        assert_eq!(m.get("max_connections"), Some(&serde_json::json!(200)));
    }

    #[test]
    fn validate_password_rules() {
        assert!(validate_password("Short1").is_err());
        assert!(validate_password("alllowercase12345").is_err()); // no upper
        assert!(validate_password("ALLUPPERCASE12345").is_err()); // no lower
        assert!(validate_password("NoDigitsHereAtAll").is_err());
        assert!(validate_password("Valid1Password").is_ok());
    }

    #[test]
    fn generated_password_is_compliant() {
        let pw = generate_compliant_password();
        assert!(validate_password(&pw).is_ok());
    }

    #[test]
    fn merge_tags_adds_and_removes() {
        let existing = vec![
            ResourceTagsV1 {
                key: "env".into(),
                value: Some("dev".into()),
            },
            ResourceTagsV1 {
                key: "team".into(),
                value: Some("data".into()),
            },
        ];
        let add = vec![ResourceTagsV1 {
            key: "env".into(),
            value: Some("prod".into()),
        }];
        let remove = vec!["team".to_string()];
        let out = merge_tags(&existing, &add, &remove);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "env");
        assert_eq!(out[0].value.as_deref(), Some("prod"));
    }

    #[test]
    fn merge_response_tags_refuses_absent_tags() {
        let add = vec![ResourceTagsV1 {
            key: "env".into(),
            value: Some("prod".into()),
        }];
        let err = merge_response_tags(None, &add, &[])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("omitted the tags field"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn merge_response_tags_merges_an_empty_tag_list() {
        // `Some(vec![])` is the API saying "no tags", which is safe to merge on.
        let add = vec![ResourceTagsV1 {
            key: "env".into(),
            value: Some("prod".into()),
        }];
        let out = merge_response_tags(Some(vec![]), &add, &[]).unwrap();
        assert_eq!(out, add);
    }

    #[test]
    fn merge_response_tags_merges_returned_tags() {
        let current = vec![
            ResourceTagsV1Response {
                key: Some("env".into()),
                value: Some("dev".into()),
            },
            ResourceTagsV1Response {
                key: Some("team".into()),
                value: Some("data".into()),
            },
        ];
        let add = vec![ResourceTagsV1 {
            key: "env".into(),
            value: Some("prod".into()),
        }];
        let out = merge_response_tags(Some(current), &add, &["team".to_string()]).unwrap();
        assert_eq!(out, add);
    }

    #[test]
    fn merge_response_tags_refuses_a_returned_tag_without_a_key() {
        let current = vec![ResourceTagsV1Response {
            key: None,
            value: Some("dev".into()),
        }];
        let err = merge_response_tags(Some(current), &[], &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("key"), "unexpected error: {err}");
    }

    fn pg_test_id() -> PgIdProperty {
        PgIdProperty::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap()
    }

    #[test]
    fn postgres_credentials_block_shows_the_password_the_api_sent() {
        assert_eq!(
            postgres_credentials_block(Some("pg_user"), Some("s3cret"), None, Some(&pg_test_id())),
            "Credentials (save these — password shown only once):\n  Username: pg_user\n  \
             Password: s3cret"
        );
    }

    #[test]
    fn postgres_credentials_block_treats_an_empty_password_as_sent() {
        assert_eq!(
            postgres_credentials_block(None, Some(""), None, None),
            format!(
                "Credentials (save these — password shown only once):\n  Username: {ABSENT}\n  \
                 Password: "
            )
        );
    }

    #[test]
    fn postgres_credentials_block_points_at_the_connection_string_instead_of_a_reset() {
        // The connection string embeds the password, so the credential isn't
        // lost and a reset would rotate a working password for nothing.
        let block = postgres_credentials_block(
            Some("pg_user"),
            None,
            Some("postgresql://pg_user:s3cret@host:5432/postgres"),
            Some(&pg_test_id()),
        );
        assert!(
            !block.contains("reset-password"),
            "a recoverable password must not be reset: {block}"
        );
        assert!(
            block.contains("connection string below embeds it"),
            "the warning should point at the connection string: {block}"
        );
    }

    #[test]
    fn postgres_credentials_block_warns_with_the_reset_command_when_the_password_is_absent() {
        let block = postgres_credentials_block(Some("pg_user"), None, None, Some(&pg_test_id()));
        assert!(
            !block.contains(&format!("Password: {ABSENT}")),
            "an absent password must not render a placeholder credential: {block}"
        );
        assert!(block.starts_with("WARNING: the API response omitted the one-time password"));
        assert!(
            block.contains(
                "clickhousectl cloud postgres reset-password \
                 a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6 --generate"
            ),
            "the warning should name the exact recovery command: {block}"
        );
    }

    #[test]
    fn postgres_credentials_block_warns_generically_when_the_service_id_is_absent() {
        let block = postgres_credentials_block(Some("pg_user"), None, None, None);
        assert!(block.starts_with("WARNING: the API response omitted the one-time password"));
        assert!(
            block.contains("clickhousectl cloud postgres reset-password <postgres-id> --generate"),
            "without an id the warning should stay generic: {block}"
        );
    }

    #[test]
    fn instance_config_from_json_fills_omitted_sections() {
        // The request model is strict, so the handler resolves an omitted
        // section to `{}` — the minimal body the API accepts.
        let cfg = instance_config_from_json(&serde_json::json!({
            "pgConfig": { "max_connections": 500 },
        }))
        .unwrap();
        assert_eq!(cfg.pg_config.max_connections, Some(serde_json::json!(500)));
        assert_eq!(cfg.pg_bouncer_config, PgBouncerConfig::default());
        assert_eq!(
            serde_json::to_value(&cfg).unwrap(),
            serde_json::json!({ "pgConfig": { "max_connections": 500 }, "pgBouncerConfig": {} })
        );

        let empty = instance_config_from_json(&serde_json::json!({})).unwrap();
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": {} })
        );
    }

    #[test]
    fn instance_config_from_json_accepts_both_sections() {
        let cfg = instance_config_from_json(&serde_json::json!({
            "pgConfig": { "max_connections": 500, "work_mem": "64MB" },
            "pgBouncerConfig": {},
        }))
        .unwrap();
        assert_eq!(cfg.pg_config.max_connections, Some(serde_json::json!(500)));
        assert_eq!(cfg.pg_config.work_mem, Some(serde_json::json!("64MB")));
        assert_eq!(cfg.pg_bouncer_config, PgBouncerConfig::default());
    }

    #[test]
    fn instance_config_from_json_refuses_a_non_object_root() {
        // A non-object root must not read as "no sections": that would send
        // `{"pgConfig": {}, "pgBouncerConfig": {}}` and reset the config.
        for root in [
            serde_json::Value::Null,
            serde_json::json!([{ "pgConfig": {} }]),
            serde_json::json!("pgConfig"),
            serde_json::json!(7),
            serde_json::json!(true),
        ] {
            let err = instance_config_from_json(&root).unwrap_err().to_string();
            assert_eq!(err, "configuration document must be a JSON object");
        }
    }

    #[test]
    fn instance_config_from_json_reports_an_invalid_section() {
        let err = instance_config_from_json(&serde_json::json!({ "pgConfig": 7 }))
            .unwrap_err()
            .to_string();
        assert!(err.contains("invalid pgConfig"), "unexpected error: {err}");
    }

    #[test]
    fn absent_response_fields_render_as_a_dash() {
        let item = PostgresServiceListItem::default();
        assert_eq!(or_absent(item.name.as_deref()), ABSENT);
        assert_eq!(state_label(item.state.as_ref()), ABSENT);
        assert_eq!(enum_label(item.size.as_ref()), ABSENT);
    }

    #[test]
    fn apply_filter_does_not_match_absent_response_fields() {
        let absent = PostgresServiceListItem::default();
        assert!(!apply_filter(&absent, &["region=us-east-1".to_string()]));
        assert!(!apply_filter(&absent, &["state=running".to_string()]));
        // An unknown filter key stays permissive, as before.
        assert!(apply_filter(&absent, &["bogus=1".to_string()]));

        let present = PostgresServiceListItem {
            region: Some("us-east-1".to_string()),
            state: Some(clickhouse_cloud_api::models::PgStateProperty::Running),
            ..Default::default()
        };
        assert!(apply_filter(&present, &["region=us-east-1".to_string()]));
        assert!(apply_filter(&present, &["state=running".to_string()]));
    }
}
