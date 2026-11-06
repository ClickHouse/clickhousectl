use super::config::{load_json_file, load_pg_config_file};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_line};
use crate::cloud::shared::{parse_ip_or_cidr, parse_serde_enum, parse_tags, resolve_org_id};
use clickhouse_cloud_api::models::{
    PgBouncerConfig, PgHaType, PgIdProperty, PgProvider, PgSize, PgVersion,
    PostgresIpAccessListEntry, PostgresIpAccessListPatch, PostgresIpAccessListRuleReference,
    PostgresService, PostgresServiceListItem, PostgresServicePatchRequest,
    PostgresServicePostRequest, PostgresServiceSetPassword, PostgresServiceSetStateCommand,
    ResourceTagsV1, ResourceTagsV1Response,
};
use std::path::Path;
use tabled::{Table, Tabled, settings::Style};

fn parse_pg_size(value: &str) -> CloudResult<clickhouse_cloud_api::models::PgSize> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| CloudError::new(format!("invalid size '{}': {}", value, e)))
}

/// Parse an access rule in `IP_OR_CIDR:PORTS[=DESCRIPTION]` form.
///
/// Postgres access rules are IPv4 only, so the first `:` always ends the
/// source. The description follows the first `=` and is kept byte-for-byte,
/// including an explicitly empty value, matching the ClickHouse service
/// `IP_OR_CIDR[=DESCRIPTION]` parser: a trailing `=` sends `"description": ""`.
pub(super) fn parse_postgres_ip_access_entry(
    value: &str,
) -> Result<PostgresIpAccessListEntry, String> {
    let invalid = || {
        "expected IP_OR_CIDR:PORTS[=DESCRIPTION] with an IPv4 address or CIDR and a port \
         or inclusive range from 1 to 65535 (e.g. 203.0.113.0/24:5432 or 10.0.0.1:5432..6432)"
            .to_string()
    };
    let (rule, description) = value
        .split_once('=')
        .map_or((value, None), |(rule, description)| {
            (rule, Some(description.to_string()))
        });
    let (source, ports) = rule.split_once(':').ok_or_else(invalid)?;
    let source = source.trim();
    if !parse_ip_or_cidr(source).is_some_and(|address| address.is_ipv4()) {
        return Err(invalid());
    }
    let port_range = parse_postgres_port_range(ports).ok_or_else(invalid)?;
    Ok(PostgresIpAccessListEntry {
        description,
        id: None,
        port_range,
        source: source.to_string(),
    })
}

/// Validate `PORT` or `START..END` (inclusive, 1-65535) and render it in the
/// API's canonical form, so `+5432` or `05432` never reach the wire.
fn parse_postgres_port_range(ports: &str) -> Option<String> {
    let port = |value: &str| {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        value.parse::<u16>().ok().filter(|port| *port != 0)
    };
    match ports.split_once("..") {
        Some((start, end)) => {
            let (start, end) = (port(start)?, port(end)?);
            (start <= end).then(|| format!("{start}..{end}"))
        }
        None => port(ports).map(|port| port.to_string()),
    }
}

/// `--ip-allow` rules for create. Omitting both flags omits the field, which
/// allows all addresses; `--block-public-access` sends `[]`, which allows none.
pub(super) fn postgres_create_ip_access_list(
    ip_allow: &[PostgresIpAccessListEntry],
    block_public_access: bool,
) -> Option<Vec<PostgresIpAccessListEntry>> {
    (block_public_access || !ip_allow.is_empty()).then(|| ip_allow.to_vec())
}

/// `ipAccessList` patch for update, or `None` when no rule flag was passed.
/// The patch carries both lists, so the unused side is sent empty.
fn build_postgres_ip_access_list_patch(
    add: &[PostgresIpAccessListEntry],
    remove: &[String],
) -> Option<PostgresIpAccessListPatch> {
    (!add.is_empty() || !remove.is_empty()).then(|| PostgresIpAccessListPatch {
        add: add.to_vec(),
        remove: remove
            .iter()
            .map(|id| PostgresIpAccessListRuleReference { id: id.clone() })
            .collect(),
    })
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

/// A parsed, validated `cloud postgres list --filter KEY=VALUE` predicate.
///
/// `--filter` is applied client-side over the list response, so an unsupported
/// key cannot be rejected by the API. Parsing into this closed set at clap time
/// is what makes an unknown key a usage error (exit 2) instead of a silently
/// unfiltered full listing (issue #603).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostgresListFilter {
    /// Matched case-insensitively against the `state` wire value.
    State(String),
    /// Matched exactly against the region ID the API returned.
    Region(String),
    /// Matched exactly: service names are case-sensitive.
    Name(String),
    /// Matched case-insensitively against the `provider` wire value.
    Provider(String),
    /// Matched against the `isPrimary` boolean shown in the `Primary` column.
    IsPrimary(bool),
}

/// The supported filter keys, in the order they are listed to the user. Spelled
/// as the API's wire field names, which is what `--json` output shows.
const POSTGRES_LIST_FILTER_KEYS: [&str; 5] = ["state", "region", "name", "provider", "isPrimary"];

fn postgres_list_filter_keys() -> String {
    POSTGRES_LIST_FILTER_KEYS.join(", ")
}

/// Parses one `KEY=VALUE` filter. Every rejection is a clap `value_parser`
/// error, so the process exits 2 with the valid keys listed.
pub(super) fn parse_postgres_list_filter(
    raw: &str,
) -> std::result::Result<PostgresListFilter, String> {
    let Some((key, value)) = raw.split_once('=') else {
        return Err(format!(
            "invalid filter '{raw}': expected KEY=VALUE, where KEY is one of {}",
            postgres_list_filter_keys()
        ));
    };
    let key = key.trim();
    let value = value.trim();
    // Keys are matched case-insensitively so `isprimary` works alongside the
    // documented `isPrimary`; values keep their case for the per-key rules.
    let canonical = POSTGRES_LIST_FILTER_KEYS
        .into_iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(key))
        .ok_or_else(|| {
            format!(
                "unknown filter key '{key}': expected one of {}",
                postgres_list_filter_keys()
            )
        })?;
    if value.is_empty() {
        return Err(format!(
            "filter '{canonical}' requires a value: expected {canonical}=VALUE"
        ));
    }
    Ok(match canonical {
        "state" => PostgresListFilter::State(value.to_string()),
        "region" => PostgresListFilter::Region(value.to_string()),
        "name" => PostgresListFilter::Name(value.to_string()),
        "provider" => PostgresListFilter::Provider(value.to_string()),
        "isPrimary" => PostgresListFilter::IsPrimary(parse_filter_bool(canonical, value)?),
        other => unreachable!("unhandled filter key '{other}'"),
    })
}

/// `yes`/`no` are accepted because that is how the `Primary` column renders.
fn parse_filter_bool(key: &str, value: &str) -> std::result::Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err(format!(
            "invalid value '{value}' for filter '{key}': expected true or false"
        )),
    }
}

fn apply_filter(item: &PostgresServiceListItem, filters: &[PostgresListFilter]) -> bool {
    // A response field the API omitted matches no filter value.
    filters.iter().all(|filter| match filter {
        // Compared against the serde wire value (`Display`), not the Rust
        // Debug form: the latter never matches an `Unknown(..)` state.
        PostgresListFilter::State(want) => item
            .state
            .as_ref()
            .is_some_and(|state| state.to_string().eq_ignore_ascii_case(want)),
        PostgresListFilter::Region(want) => item.region.as_deref() == Some(want.as_str()),
        PostgresListFilter::Name(want) => item.name.as_deref() == Some(want.as_str()),
        PostgresListFilter::Provider(want) => item
            .provider
            .as_ref()
            .is_some_and(|provider| provider.to_string().eq_ignore_ascii_case(want)),
        PostgresListFilter::IsPrimary(want) => item.is_primary == Some(*want),
    })
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

// `print_line`, not `println!`: a closed stdout must not turn a completed
// operation into a panic (#598).
pub(super) fn render_postgres_service(svc: &PostgresService) {
    print_line(format!("  ID: {}", or_absent(svc.id.as_ref())));
    print_line(format!("  Name: {}", or_absent(svc.name.as_deref())));
    print_line(format!("  State: {}", state_label(svc.state.as_ref())));
    print_line(format!("  Provider: {}", enum_label(svc.provider.as_ref())));
    print_line(format!("  Region: {}", or_absent(svc.region.as_deref())));
    print_line(format!("  Size: {}", enum_label(svc.size.as_ref())));
    print_line(format!("  Storage (GB): {}", or_absent(svc.storage_size)));
    print_line(format!(
        "  PG version: {}",
        enum_label(svc.postgres_version.as_ref())
    ));
    print_line(format!("  HA type: {}", enum_label(svc.ha_type.as_ref())));
    print_line(format!("  Primary: {}", or_absent(svc.is_primary)));
    print_line(format!("  Host: {}", or_absent(svc.hostname.as_deref())));
    print_line(format!(
        "  Username: {}",
        or_absent(svc.username.as_deref())
    ));
    print_line(format!(
        "  Created: {}",
        or_absent(svc.created_at.map(|c| c.to_rfc3339()))
    ));
    if let Some(svc_tags) = svc.tags.as_ref().filter(|t| !t.is_empty()) {
        let tags: Vec<String> = svc_tags
            .iter()
            .map(|t| match (t.key.as_deref(), t.value.as_deref()) {
                (key, Some(value)) => format!("{}={}", or_absent(key), value),
                (key, None) => or_absent(key).to_string(),
            })
            .collect();
        print_line(format!("  Tags: {}", tags.join(", ")));
    }
    for line in postgres_restore_and_access_lines(svc) {
        print_line(line);
    }
}

/// The restore point and IPv4 access rules, each shown only when the response
/// carries it: the API omits the rules from most updates and the restore point
/// until a primary's first backup completes.
fn postgres_restore_and_access_lines(svc: &PostgresService) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(at) = svc.earliest_restore_time {
        lines.push(format!("  Earliest restore: {}", at.to_rfc3339()));
    }
    let Some(rules) = svc.ip_access_list.as_ref() else {
        return lines;
    };
    if rules.is_empty() {
        lines.push("  IPv4 access rules: none".to_string());
        return lines;
    }
    lines.push("  IPv4 access rules:".to_string());
    for rule in rules {
        let mut line = format!(
            "    {}: {} ports {}",
            or_absent(rule.id.as_deref()),
            or_absent(rule.source.as_deref()),
            or_absent(rule.port_range.as_deref())
        );
        if let Some(description) = rule.description.as_deref().filter(|d| !d.is_empty()) {
            line.push_str(&format!(" ({description})"));
        }
        lines.push(line);
    }
    lines
}

/// Print a short connection handoff without putting a password in argv or a URI.
///
/// The API does not expose port or database fields. They are stable service
/// defaults documented by ClickHouse Managed Postgres. Sparse asynchronous
/// responses can also omit host and username, so those fields retain explicit
/// placeholders until `get` supplies them.
pub(super) fn render_postgres_connection_guidance(svc: &PostgresService) {
    let postgres_id = svc
        .id
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "<postgres-id>".to_string());
    let host = svc
        .hostname
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("<host-from-get>");
    let username = svc
        .username
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("<username-from-get>");

    print_line("");
    print_line(format!(
        "Connection: host={host} port=5432 database=postgres user={username}"
    ));
    print_line(format!(
        "Next: clickhousectl cloud postgres get {postgres_id} (wait for state=running and fill any placeholders)"
    ));
    print_line(format!(
        "Verified TLS: clickhousectl cloud postgres certs get {postgres_id} --output ca.pem; see README for psql"
    ));
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

/// Preserves or merges the tag list a GET returned before every update.
///
/// An omitted `tags` in the response is indistinguishable from a field the API
/// dropped, so a read-modify-write must not proceed on it: `tags` is replaced
/// wholesale by the PATCH, so merging against an assumed empty set would delete
/// every tag the service still has. A returned tag without a key cannot be sent
/// back either, so say so rather than dropping it from the merged set.
pub(super) fn merge_response_tags(
    current: Option<Vec<ResourceTagsV1Response>>,
    add: &[ResourceTagsV1],
    remove_keys: &[String],
) -> CloudResult<Vec<ResourceTagsV1>> {
    let current = current.ok_or_else(|| {
        CloudError::new(
            "the API response omitted the tags field or returned null, so the update cannot preserve \
             tags safely: an update replaces the tag set wholesale, and assuming an empty set \
             would delete any tags the service already has",
        )
    })?;
    let existing = current
        .into_iter()
        .map(ResourceTagsV1::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CloudError::new(error.to_string()))?;
    Ok(merge_tags(&existing, add, remove_keys))
}

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
    pub ip_allow: &'a [PostgresIpAccessListEntry],
    pub block_public_access: bool,
}

pub struct PostgresUpdateOptions<'a> {
    pub name: Option<&'a str>,
    pub size: Option<&'a str>,
    pub ha_type: Option<&'a str>,
    pub add_tag: &'a [String],
    pub remove_tag: &'a [String],
    pub clear_tags: bool,
    pub add_ip_allow: &'a [PostgresIpAccessListEntry],
    pub remove_ip_allow: &'a [String],
}

pub async fn postgres_list(
    client: &CloudClient,
    filters: &[PostgresListFilter],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let items = client.list_postgres_services(&org_id).await?;
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
        #[tabled(rename = "Provider")]
        provider: String,
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
            provider: enum_label(i.provider.as_ref()),
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

pub async fn postgres_get(client: &CloudClient, postgres_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let svc = client.get_postgres_service(&org_id, postgres_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        // No connection string here: the get endpoint stops returning
        // credentials on July 31, 2026 — they are only available from
        // `postgres create` and `postgres reset-password`.
        render_postgres_service(&svc);
        render_postgres_connection_guidance(&svc);
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

/// Builds the `postgres create` POST body, reading any config files.
fn build_postgres_create_request(
    opts: &PostgresCreateOptions<'_>,
) -> CloudResult<PostgresServicePostRequest> {
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
    let pg_config = opts.pg_config_file.map(load_pg_config_file).transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;

    Ok(PostgresServicePostRequest {
        name: opts.name.to_string(),
        provider,
        region: opts.region.to_string(),
        size,
        postgres_version: pg_version,
        ha_type,
        tags,
        pg_config,
        pg_bouncer_config,
        ip_access_list: postgres_create_ip_access_list(opts.ip_allow, opts.block_public_access),
    })
}

pub async fn postgres_create(
    client: &CloudClient,
    opts: PostgresCreateOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let req = build_postgres_create_request(&opts)?;

    let org_id = resolve_org_id(client).await?;
    let svc = client.create_postgres_service(&org_id, &req).await?;

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

/// Builds the `postgres update` PATCH body from the already-parsed pieces
/// (name, size, HA type, merged tags, access-rule patch). Kept separate from
/// `postgres_update` so the shape sent on the wire is unit-testable without a
/// mock server.
pub(super) fn build_postgres_update_request(
    name: Option<&str>,
    size: Option<PgSize>,
    ha_type: Option<PgHaType>,
    tags: Option<Vec<ResourceTagsV1>>,
    ip_access_list: Option<PostgresIpAccessListPatch>,
) -> PostgresServicePatchRequest {
    PostgresServicePatchRequest {
        name: name.map(str::to_string),
        size,
        ha_type,
        tags,
        ip_access_list,
    }
}

pub async fn postgres_update(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresUpdateOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;

    let size = opts.size.map(parse_pg_size).transpose()?;
    let ha_type = opts
        .ha_type
        .map(|v| parse_serde_enum::<PgHaType>(v, "ha-type", PgHaType::VALUES))
        .transpose()?;

    // Clearing is already a complete replacement, so it never needs a GET.
    // The API clears tags when PATCH omits them, even for a rename or resize.
    // Every other update must preserve a complete current tag snapshot.
    let tags = if opts.clear_tags {
        Some(Vec::new())
    } else {
        let current = client.get_postgres_service(&org_id, postgres_id).await?;
        let add = parse_tags(opts.add_tag)?.unwrap_or_default();
        Some(merge_response_tags(current.tags, &add, opts.remove_tag)?)
    };

    let ip_access_list =
        build_postgres_ip_access_list_patch(opts.add_ip_allow, opts.remove_ip_allow);
    let req = build_postgres_update_request(opts.name, size, ha_type, tags, ip_access_list);

    let svc = client
        .patch_postgres_service(&org_id, postgres_id, &req)
        .await?;

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
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;

    // The delete endpoint itself only ever returns the raw API envelope
    // (`ApiResponse<serde_json::Value>`, no resource in `result`), so fetch the
    // resource before deleting it and render that instead: `--json` output must
    // stay consistent with every other `cloud postgres` subcommand, which emits
    // the resource object rather than `{"status":...,"requestId":...}` (#614).
    let svc = client.get_postgres_service(&org_id, postgres_id).await?;

    client.delete_postgres_service(&org_id, postgres_id).await?;

    // The service is already gone by here, so a closed stdout must not turn a
    // completed deletion into a panic — see `print_line` and #598.
    if json {
        print_line(serde_json::to_string_pretty(&svc)?);
    } else {
        print_line(format!(
            "Postgres service {} deletion initiated",
            postgres_id
        ));
    }
    Ok(())
}

pub async fn postgres_certs_get(
    client: &CloudClient,
    postgres_id: &str,
    output: Option<&Path>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let pem = client
        .get_postgres_service_certs(&org_id, postgres_id)
        .await?;

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

pub async fn postgres_reset_password(
    client: &CloudClient,
    postgres_id: &str,
    password: Option<&str>,
    generate: bool,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;

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
    let out = client
        .set_postgres_service_password(&org_id, postgres_id, &req)
        .await?;
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
        println!();
        println!("Connection details: clickhousectl cloud postgres get {postgres_id}");
    }
    Ok(())
}

/// Issues a state command that does not need operation-specific guidance.
pub async fn postgres_state_change(
    client: &CloudClient,
    postgres_id: &str,
    cmd: PostgresServiceSetStateCommand,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let svc = client
        .set_postgres_service_state(&org_id, postgres_id, cmd)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("State change accepted");
        println!();
        render_postgres_service(&svc);
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn parses_postgres_list_filter_keys_case_insensitively() {
        assert_eq!(
            parse_postgres_list_filter("ISPRIMARY=YES"),
            Ok(PostgresListFilter::IsPrimary(true))
        );
        assert_eq!(
            parse_postgres_list_filter(" state = running "),
            Ok(PostgresListFilter::State("running".to_string()))
        );
        // `no`/`false` both read as false, matching the `Primary` column.
        assert_eq!(
            parse_postgres_list_filter("isPrimary=no"),
            Ok(PostgresListFilter::IsPrimary(false))
        );
        assert_eq!(
            parse_postgres_list_filter("isPrimary=false"),
            Ok(PostgresListFilter::IsPrimary(false))
        );
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
    fn build_postgres_update_request_omits_name_when_not_given() {
        let req = build_postgres_update_request(None, None, None, None, None);
        assert!(req.name.is_none());
        assert!(req.size.is_none());
        assert!(req.ha_type.is_none());
        assert!(req.tags.is_none());
        assert!(req.ip_access_list.is_none());
        assert_eq!(serde_json::to_value(req).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn build_postgres_update_request_clears_tags_explicitly() {
        let req = build_postgres_update_request(None, None, None, Some(Vec::new()), None);

        assert_eq!(req.tags, Some(Vec::new()));
        assert_eq!(
            serde_json::to_value(req).unwrap(),
            serde_json::json!({"tags": []})
        );
    }

    #[test]
    fn build_postgres_update_request_sets_name_when_given() {
        let req = build_postgres_update_request(
            Some("renamed-pg"),
            Some(PgSize::C6gd_large),
            Some(PgHaType::Sync),
            Some(vec![ResourceTagsV1 {
                key: "env".into(),
                value: Some("prod".into()),
            }]),
            build_postgres_ip_access_list_patch(
                &[access_rule("203.0.113.0/24", "5432", Some("office"))],
                &["rule-1".to_string()],
            ),
        );
        assert_eq!(req.name.as_deref(), Some("renamed-pg"));
        assert!(req.size.is_some());
        assert_eq!(req.ha_type, Some(PgHaType::Sync));
        assert_eq!(req.tags.as_ref().unwrap().len(), 1);
        assert_eq!(
            serde_json::to_value(&req).unwrap()["ipAccessList"],
            serde_json::json!({
                "add": [{"source": "203.0.113.0/24", "portRange": "5432", "description": "office"}],
                "remove": [{"id": "rule-1"}]
            })
        );
    }

    pub(in crate::cloud::postgres) fn access_rule(
        source: &str,
        port_range: &str,
        description: Option<&str>,
    ) -> PostgresIpAccessListEntry {
        PostgresIpAccessListEntry {
            description: description.map(str::to_string),
            id: None,
            port_range: port_range.to_string(),
            source: source.to_string(),
        }
    }

    #[test]
    fn parses_postgres_ip_access_rules() {
        for (value, source, ports, description) in [
            ("203.0.113.7:5432", "203.0.113.7", "5432", None),
            (
                "10.0.0.0/8:6432=office",
                "10.0.0.0/8",
                "6432",
                Some("office"),
            ),
            ("0.0.0.0/0:5432..6432", "0.0.0.0/0", "5432..6432", None),
            ("192.0.2.1:1..65535=", "192.0.2.1", "1..65535", Some("")),
            ("192.0.2.1:05432", "192.0.2.1", "5432", None),
            ("192.0.2.1:5432..5432", "192.0.2.1", "5432..5432", None),
            (
                "192.0.2.0/24:5432=a=b: c \u{1f418}",
                "192.0.2.0/24",
                "5432",
                Some("a=b: c \u{1f418}"),
            ),
        ] {
            assert_eq!(
                parse_postgres_ip_access_entry(value).unwrap(),
                access_rule(source, ports, description),
                "{value}"
            );
        }
    }

    #[test]
    fn rejects_malformed_postgres_ip_access_rules() {
        for value in [
            "",
            ":5432",
            "203.0.113.7",
            "203.0.113.7=office",
            "203.0.113.7:",
            "203.0.113.7:0",
            "203.0.113.7:65536",
            "203.0.113.7:+5432",
            "203.0.113.7:-1",
            "203.0.113.7:5432-6432",
            "203.0.113.7:6432..5432",
            "203.0.113.7:5432..",
            "203.0.113.7:..6432",
            "203.0.113.7:5432..6432..7432",
            "203.0.113.7:postgres",
            "203.0.113.7: 5432",
            "10.0.0.0/33:5432",
            "10.0.0.0/8/9:5432",
            "not-an-ip:5432",
            "2001:db8::1:5432",
            "[2001:db8::1]:5432",
            "=office:5432",
        ] {
            let error = parse_postgres_ip_access_entry(value).unwrap_err();
            assert!(error.contains("IP_OR_CIDR:PORTS"), "{value}: {error}");
        }
    }

    #[test]
    fn postgres_create_ip_access_list_distinguishes_omitted_from_empty() {
        assert_eq!(postgres_create_ip_access_list(&[], false), None);
        assert_eq!(postgres_create_ip_access_list(&[], true), Some(Vec::new()));
        let rules = vec![
            access_rule("203.0.113.0/24", "5432", None),
            access_rule("203.0.113.0/24", "6432", Some("pooler")),
        ];
        assert_eq!(
            postgres_create_ip_access_list(&rules, false),
            Some(rules.clone())
        );
    }

    fn create_options<'a>(
        ip_allow: &'a [PostgresIpAccessListEntry],
        block_public_access: bool,
    ) -> PostgresCreateOptions<'a> {
        PostgresCreateOptions {
            name: "pg1",
            region: "us-east-1",
            size: "c6gd.large",
            provider: "aws",
            pg_version: None,
            ha_type: None,
            tags: &[],
            pg_config_file: None,
            pg_bouncer_config_file: None,
            ip_allow,
            block_public_access,
        }
    }

    #[test]
    fn build_postgres_create_request_minimal_omits_access_rules() {
        let req = build_postgres_create_request(&create_options(&[], false)).unwrap();
        assert_eq!(req.name, "pg1");
        assert_eq!(req.provider, PgProvider::Aws);
        assert_eq!(req.region, "us-east-1");
        assert_eq!(req.size, PgSize::C6gd_large);
        assert!(req.postgres_version.is_none());
        assert!(req.ha_type.is_none());
        assert!(req.tags.is_none());
        assert!(req.pg_config.is_none());
        assert!(req.pg_bouncer_config.is_none());
        assert!(req.ip_access_list.is_none());
        assert!(
            serde_json::to_value(&req)
                .unwrap()
                .get("ipAccessList")
                .is_none()
        );
    }

    #[test]
    fn build_postgres_create_request_sends_rules_or_an_empty_list() {
        let blocked = build_postgres_create_request(&create_options(&[], true)).unwrap();
        assert_eq!(
            serde_json::to_value(&blocked).unwrap()["ipAccessList"],
            serde_json::json!([])
        );

        let rules = vec![
            access_rule("203.0.113.0/24", "5432", Some("office")),
            access_rule("198.51.100.7", "5432..6432", None),
        ];
        let options = PostgresCreateOptions {
            pg_version: Some("17"),
            ha_type: Some("sync"),
            tags: &["env=prod".to_string()][..],
            ..create_options(&rules, false)
        };
        let req = build_postgres_create_request(&options).unwrap();
        assert_eq!(req.postgres_version, Some(PgVersion::_17));
        assert_eq!(req.ha_type, Some(PgHaType::Sync));
        assert_eq!(req.tags.as_ref().unwrap().len(), 1);
        assert_eq!(req.ip_access_list.as_ref(), Some(&rules));
        assert_eq!(
            serde_json::to_value(&req).unwrap()["ipAccessList"],
            serde_json::json!([
                {"source": "203.0.113.0/24", "portRange": "5432", "description": "office"},
                {"source": "198.51.100.7", "portRange": "5432..6432"}
            ])
        );
    }

    #[test]
    fn build_postgres_ip_access_list_patch_minimal_and_maximal() {
        assert_eq!(build_postgres_ip_access_list_patch(&[], &[]), None);

        let only_remove =
            build_postgres_ip_access_list_patch(&[], &["rule-1".to_string()]).unwrap();
        assert_eq!(
            serde_json::to_value(only_remove).unwrap(),
            serde_json::json!({"add": [], "remove": [{"id": "rule-1"}]})
        );

        let rules = vec![
            access_rule("203.0.113.0/24", "5432", Some("office")),
            access_rule("198.51.100.7", "6432", None),
        ];
        let patch = build_postgres_ip_access_list_patch(
            &rules,
            &["rule-1".to_string(), "rule-2".to_string()],
        )
        .unwrap();
        assert_eq!(patch.add, rules);
        assert!(patch.add.iter().all(|rule| rule.id.is_none()));
        assert_eq!(
            patch.remove,
            vec![
                PostgresIpAccessListRuleReference {
                    id: "rule-1".into()
                },
                PostgresIpAccessListRuleReference {
                    id: "rule-2".into()
                },
            ]
        );
    }

    #[test]
    fn postgres_service_view_shows_restore_point_and_access_rules_when_present() {
        assert!(postgres_restore_and_access_lines(&PostgresService::default()).is_empty());

        let blocked = PostgresService {
            ip_access_list: Some(Vec::new()),
            ..Default::default()
        };
        assert_eq!(
            postgres_restore_and_access_lines(&blocked),
            vec!["  IPv4 access rules: none"]
        );

        let svc = PostgresService {
            earliest_restore_time: Some("2026-10-01T12:00:00Z".parse().unwrap()),
            ip_access_list: Some(vec![
                clickhouse_cloud_api::models::PostgresIpAccessListEntryResponse {
                    description: Some("office".into()),
                    id: Some("rule-1".into()),
                    port_range: Some("5432..6432".into()),
                    source: Some("203.0.113.0/24".into()),
                },
                Default::default(),
            ]),
            ..Default::default()
        };
        let lines = postgres_restore_and_access_lines(&svc);
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!(lines[0].contains("2026-10-01T12:00:00"), "{lines:?}");
        for value in ["rule-1", "203.0.113.0/24", "5432..6432", "office"] {
            assert!(lines[2].contains(value), "{value}: {lines:?}");
        }
        assert_eq!(lines[3].matches(ABSENT).count(), 3, "{lines:?}");
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
    fn absent_response_fields_render_as_a_dash() {
        let item = PostgresServiceListItem::default();
        assert_eq!(or_absent(item.name.as_deref()), ABSENT);
        assert_eq!(state_label(item.state.as_ref()), ABSENT);
        assert_eq!(enum_label(item.size.as_ref()), ABSENT);
    }

    fn filters(raw: &[&str]) -> Vec<PostgresListFilter> {
        raw.iter()
            .map(|f| parse_postgres_list_filter(f).expect("valid filter"))
            .collect()
    }

    #[test]
    fn apply_filter_does_not_match_absent_response_fields() {
        let absent = PostgresServiceListItem::default();
        for filter in [
            "region=us-east-1",
            "state=running",
            "name=my-pg",
            "provider=aws",
            "isPrimary=true",
            "isPrimary=false",
        ] {
            assert!(
                !apply_filter(&absent, &filters(&[filter])),
                "absent field must not match {filter}"
            );
        }

        let present = PostgresServiceListItem {
            region: Some("us-east-1".to_string()),
            state: Some(clickhouse_cloud_api::models::PgStateProperty::Running),
            name: Some("my-pg".to_string()),
            provider: Some(clickhouse_cloud_api::models::PgProvider::Aws),
            is_primary: Some(true),
            ..Default::default()
        };
        assert!(apply_filter(
            &present,
            &filters(&[
                "region=us-east-1",
                "state=running",
                "name=my-pg",
                "provider=aws",
                "isPrimary=true",
            ])
        ));
        // Every filter must hold: one mismatch drops the item.
        assert!(!apply_filter(
            &present,
            &filters(&["region=us-east-1", "isPrimary=false"])
        ));
    }

    #[test]
    fn apply_filter_compares_states_by_wire_value_not_debug_form() {
        use clickhouse_cloud_api::models::PgStateProperty;

        // Multi-word wire values and the `Unknown` catch-all both matched
        // nothing while the comparison used `format!("{:?}")` (issue #603).
        let restoring = PostgresServiceListItem {
            state: Some(PgStateProperty::Restoring_backup),
            ..Default::default()
        };
        assert!(apply_filter(
            &restoring,
            &filters(&["state=restoring_backup"])
        ));
        assert!(apply_filter(
            &restoring,
            &filters(&["state=RESTORING_BACKUP"])
        ));
        assert!(!apply_filter(&restoring, &filters(&["state=running"])));

        let unknown = PostgresServiceListItem {
            state: Some(PgStateProperty::Unknown("hibernating".to_string())),
            ..Default::default()
        };
        assert!(apply_filter(&unknown, &filters(&["state=hibernating"])));
        assert!(!apply_filter(&unknown, &filters(&["state=running"])));
    }

    #[test]
    fn apply_filter_compares_providers_by_wire_value() {
        use clickhouse_cloud_api::models::PgProvider;

        let gcp = PostgresServiceListItem {
            provider: Some(PgProvider::Gcp),
            ..Default::default()
        };
        assert!(apply_filter(&gcp, &filters(&["provider=gcp"])));
        assert!(!apply_filter(&gcp, &filters(&["provider=aws"])));

        let unknown = PostgresServiceListItem {
            provider: Some(PgProvider::Unknown("azure".to_string())),
            ..Default::default()
        };
        assert!(apply_filter(&unknown, &filters(&["provider=azure"])));
    }

    #[test]
    fn apply_filter_with_no_filters_keeps_every_item() {
        assert!(apply_filter(&PostgresServiceListItem::default(), &[]));
    }
}
