use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use chrono::{DateTime, FixedOffset, NaiveDate};
use clickhouse_cloud_api::models::{IpAccessListEntry, ResourceTagsV1};
use std::net::IpAddr;

/// Resolve the shared cloud organization scope lazily.
pub(super) async fn resolve_org_id(client: &CloudClient) -> CloudResult<String> {
    client.resolve_organization_id().await
}

/// An existing resource is selected by its opaque positional ID or exact name.
#[derive(clap::Args, Debug, Clone)]
#[group(skip)]
pub struct NameSelector {
    /// Existing resource ID
    #[arg(
        id = "resource_id",
        value_name = "ID",
        required_unless_present = "name",
        conflicts_with = "name"
    )]
    pub id: Option<String>,
    /// Exact resource name within the selected scope
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
#[group(id = "source_selector", required = true, multiple = false)]
pub struct SourceSelector {
    /// Source Postgres service ID
    #[arg(value_name = "POSTGRES_ID")]
    pub id: Option<String>,
    /// Exact source Postgres service name
    #[arg(long = "source-name")]
    pub source_name: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
#[group(id = "email_selector", required = true, multiple = false)]
pub struct EmailSelector {
    /// Existing resource ID
    #[arg(value_name = "ID")]
    pub id: Option<String>,
    /// Exact stored email address (case-sensitive, without normalization)
    #[arg(long)]
    pub email: Option<String>,
}

/// Resolve only after the caller has loaded every relevant page. Missing names
/// could hide a duplicate, so an incomplete collection cannot prove uniqueness.
pub(super) fn select_named_id<'a, I: std::fmt::Display>(
    kind: &'static str,
    name: &str,
    records: impl IntoIterator<Item = (Option<&'a str>, Option<I>)>,
) -> CloudResult<String> {
    let mut matches = Vec::new();
    for (candidate, id) in records {
        let candidate = candidate.ok_or_else(|| {
            CloudError::new(format!(
                "cannot resolve {kind}: list contains an entry without a name or email"
            ))
        })?;
        if candidate == name {
            let id = id
                .map(|id| id.to_string())
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| CloudError::new(format!("matching {kind} has no usable ID")))?;
            matches.push(id);
        }
    }
    match matches.len() {
        0 => Err(CloudError::new(format!(
            "no {kind} found matching '{name}'"
        ))),
        1 => Ok(matches.remove(0)),
        count => Err(CloudError::new(format!(
            "found {count} {kind} resources matching '{name}'; supply a positional ID"
        ))),
    }
}

#[derive(Clone, Copy)]
pub(super) enum NamedResource {
    Service,
    Postgres,
    Key,
    Role,
    Byoc,
}

impl NameSelector {
    pub(super) async fn resolve(
        &self,
        client: &CloudClient,
        resource: NamedResource,
    ) -> CloudResult<String> {
        match (&self.id, &self.name) {
            (Some(id), None) => return Ok(id.clone()),
            (None, Some(_)) => {}
            _ => {
                return Err(CloudError::new(
                    "supply exactly one positional ID or --name",
                ));
            }
        }
        let name = self.name.as_deref().expect("selector checked above");
        let org = resolve_org_id(client).await?;
        match resource {
            NamedResource::Service => {
                let rows = client.list_services(&org).await?;
                select_named_id(
                    "service",
                    name,
                    rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
                )
            }
            NamedResource::Postgres => {
                let rows = client.list_postgres_services(&org).await?;
                select_named_id(
                    "Postgres service",
                    name,
                    rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
                )
            }
            NamedResource::Key => {
                let rows = client.list_api_keys(&org).await?;
                select_named_id(
                    "API key",
                    name,
                    rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
                )
            }
            NamedResource::Role => {
                let rows = client.list_organization_roles(&org).await?;
                select_named_id(
                    "organization role",
                    name,
                    rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
                )
            }
            NamedResource::Byoc => {
                let org = client.get_organization(&org).await?;
                let rows = org.byoc_config.ok_or_else(|| {
                    CloudError::new("organization response is missing byocConfig")
                })?;
                select_named_id(
                    "BYOC infrastructure",
                    name,
                    rows.iter()
                        .map(|r| (r.display_name.as_deref(), r.id.as_ref())),
                )
            }
        }
    }
}

impl SourceSelector {
    pub(super) async fn resolve(&self, client: &CloudClient) -> CloudResult<String> {
        NameSelector {
            id: self.id.clone(),
            name: self.source_name.clone(),
        }
        .resolve(client, NamedResource::Postgres)
        .await
    }
}

impl EmailSelector {
    pub(super) async fn resolve_member(&self, client: &CloudClient) -> CloudResult<String> {
        let email = match (&self.id, &self.email) {
            (Some(id), None) => return Ok(id.clone()),
            (None, Some(email)) => email,
            _ => {
                return Err(CloudError::new(
                    "supply exactly one positional ID or --email",
                ));
            }
        };
        let org = resolve_org_id(client).await?;
        let rows = client.list_members(&org).await?;
        select_named_id(
            "member",
            email,
            rows.iter()
                .map(|r| (r.email.as_deref(), r.user_id.as_ref())),
        )
    }

    pub(super) async fn resolve_invitation(&self, client: &CloudClient) -> CloudResult<String> {
        let email = match (&self.id, &self.email) {
            (Some(id), None) => return Ok(id.clone()),
            (None, Some(email)) => email,
            _ => {
                return Err(CloudError::new(
                    "supply exactly one positional ID or --email",
                ));
            }
        };
        let org = resolve_org_id(client).await?;
        let rows = client.list_invitations(&org).await?;
        select_named_id(
            "invitation",
            email,
            rows.iter().map(|r| (r.email.as_deref(), r.id.as_ref())),
        )
    }
}

/// Parse a string into a library enum after validating its known wire values.
pub(super) fn parse_serde_enum<T: serde::de::DeserializeOwned>(
    value: &str,
    field: &str,
    known_values: &[&str],
) -> CloudResult<T> {
    if !known_values.contains(&value) {
        return Err(CloudError::new(format!(
            "invalid {}: unknown value '{}', expected one of: {}",
            field,
            value,
            known_values.join(", ")
        )));
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| CloudError::new(format!("invalid {}: {}", field, e)))
}

pub(super) fn parse_tag(value: &str) -> CloudResult<ResourceTagsV1> {
    match value.split_once('=') {
        Some((key, tag_value)) => {
            let key = key.trim();
            if key.is_empty() {
                Err(CloudError::new(format!(
                    "invalid tag '{}': tag key cannot be empty",
                    value
                )))
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: Some(tag_value.to_string()),
                })
            }
        }
        None => {
            let key = value.trim();
            if key.is_empty() {
                Err(CloudError::new(format!(
                    "invalid tag '{}': tag key cannot be empty",
                    value
                )))
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: None,
                })
            }
        }
    }
}

pub(super) fn parse_tags(values: &[String]) -> CloudResult<Option<Vec<ResourceTagsV1>>> {
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            values
                .iter()
                .map(|value| parse_tag(value))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// Validate the API's equality and existence tag filters without changing the
/// key or value sent on the wire. Tag values may be empty or contain `=`.
pub(super) fn parse_tag_filter(value: &str) -> Result<String, String> {
    let key = value
        .strip_prefix("tag:")
        .map(|tag| tag.split_once('=').map_or(tag, |(key, _)| key));
    if key.is_none_or(|key| key.trim().is_empty()) {
        return Err(
            "expected tag:KEY=VALUE or tag:KEY with a nonempty key (e.g. tag:env=production)"
                .to_string(),
        );
    }
    Ok(value.to_string())
}

/// Parse an IP allowlist argument in `SOURCE[=DESCRIPTION]` form.
///
/// `=` keeps the description delimiter unambiguous for IPv6 sources. The
/// description is kept byte-for-byte, including an explicitly empty value.
fn parse_ip_access_entry(value: &str) -> CloudResult<IpAccessListEntry> {
    let (source, description) = value
        .split_once('=')
        .map_or((value, None), |(source, description)| {
            (source, Some(description.to_string()))
        });
    let source = source.trim();

    let (address, prefix) = match source.split_once('/') {
        Some((address, prefix)) if !prefix.contains('/') => (address, Some(prefix)),
        Some(_) => return Err(invalid_ip_access_entry(value)),
        None => (source, None),
    };
    let address = address
        .parse::<IpAddr>()
        .map_err(|_| invalid_ip_access_entry(value))?;
    if let Some(prefix) = prefix {
        let prefix = prefix
            .parse::<u8>()
            .map_err(|_| invalid_ip_access_entry(value))?;
        let max_prefix = if address.is_ipv4() { 32 } else { 128 };
        if prefix > max_prefix {
            return Err(invalid_ip_access_entry(value));
        }
    }

    Ok(IpAccessListEntry {
        source: source.to_string(),
        description,
    })
}

fn invalid_ip_access_entry(value: &str) -> CloudError {
    CloudError::new(format!(
        "invalid IP allowlist entry '{}': expected IP_OR_CIDR[=DESCRIPTION]",
        value
    ))
}

pub(super) fn parse_ip_access_entries(
    values: &[String],
) -> CloudResult<Option<Vec<IpAccessListEntry>>> {
    if values.is_empty() {
        Ok(None)
    } else {
        values
            .iter()
            .map(|value| parse_ip_access_entry(value))
            .collect::<CloudResult<Vec<_>>>()
            .map(Some)
    }
}

pub(super) fn parse_date_only(value: &str) -> Result<String, String> {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(format!("invalid date '{}': expected YYYY-MM-DD", value));
    }

    Ok(value.to_string())
}

pub(super) fn parse_datetime(value: &str) -> Result<String, String> {
    if DateTime::<FixedOffset>::parse_from_rfc3339(value).is_err() {
        return Err(format!(
            "invalid datetime '{}': expected ISO 8601 / RFC 3339",
            value
        ));
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_rejects_empty_keys() {
        let err = parse_tag("=value").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag '=value': tag key cannot be empty"
        );

        let err = parse_tag("   ").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag '   ': tag key cannot be empty"
        );
    }

    #[test]
    fn parse_ip_access_entries_supports_bare_ipv4_ipv6_and_descriptions() {
        let values = vec![
            "192.0.2.7".to_string(),
            "10.0.0.0/8=office".to_string(),
            "2001:db8::/32=\u{6771}\u{4eac} \u{1f5fc}".to_string(),
            "2001:db8::1=".to_string(),
        ];
        let entries = parse_ip_access_entries(&values).unwrap().unwrap();

        assert_eq!(entries[0].source, "192.0.2.7");
        assert!(entries[0].description.is_none());
        assert_eq!(entries[1].source, "10.0.0.0/8");
        assert_eq!(entries[1].description.as_deref(), Some("office"));
        assert_eq!(entries[2].source, "2001:db8::/32");
        assert_eq!(
            entries[2].description.as_deref(),
            Some("\u{6771}\u{4eac} \u{1f5fc}")
        );
        assert_eq!(entries[3].source, "2001:db8::1");
        assert_eq!(entries[3].description.as_deref(), Some(""));
    }

    #[test]
    fn parse_ip_access_entries_rejects_invalid_sources() {
        for value in [
            "",
            "=office",
            "not-an-ip",
            "10.0.0.0/nope",
            "10.0.0.0/33",
            "2001:db8::/129",
            "10.0.0.0/8/9",
        ] {
            let error = parse_ip_access_entries(&[value.to_string()]).unwrap_err();
            assert!(error.to_string().contains(value), "{error}");
        }
    }
}

#[cfg(test)]
mod selector_tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use crate::cloud::services::{ServiceCommands, ServiceSettingsCommands};
    use clap::{CommandFactory, Parser, error::ErrorKind};

    #[test]
    fn named_target_operations_require_one_selector() {
        // Each row is a leaf with an existing named target. Required options
        // follow the selector; child parent IDs are part of the prefix.
        let cases: &[(&[&str], &[&str])] = &[
            (&["service", "get"], &[]),
            (&["service", "delete"], &[]),
            (&["service", "start"], &[]),
            (&["service", "wake"], &[]),
            (&["service", "stop"], &[]),
            (&["service", "update"], &["--new-name", "replacement"]),
            (&["service", "scale"], &[]),
            (&["service", "reset-password"], &[]),
            (&["service", "prometheus"], &[]),
            (&["service", "repair-query-key"], &[]),
            (&["service", "settings", "list"], &[]),
            (&["service", "settings", "schema"], &[]),
            (
                &["service", "settings", "set"],
                &["--setting", "max_threads=4"],
            ),
            (&["service", "scaling-schedule", "get"], &[]),
            (
                &["service", "scaling-schedule", "set"],
                &["--file", "schedule.json"],
            ),
            (&["service", "scaling-schedule", "delete"], &[]),
            (&["service", "query-endpoint", "get"], &[]),
            (
                &["service", "query-endpoint", "create"],
                &["--role", "sql_console_read_only"],
            ),
            (&["service", "query-endpoint", "delete"], &[]),
            (&["service", "private-endpoint", "get-config"], &[]),
            (
                &["service", "private-endpoint", "create"],
                &["--endpoint-id", "endpoint"],
            ),
            (&["service", "backup-config", "get"], &[]),
            (&["service", "backup-config", "update"], &[]),
            (&["service", "upgrade-window", "get"], &[]),
            (
                &["service", "upgrade-window", "set"],
                &["--weekday", "1", "--start-hour", "0"],
            ),
            (&["service", "upgrade-window", "delete"], &[]),
            (&["postgres", "get"], &[]),
            (
                &["postgres", "logs"],
                &[
                    "--from-date",
                    "2026-01-01T00:00:00Z",
                    "--to-date",
                    "2026-01-02T00:00:00Z",
                ],
            ),
            (&["postgres", "update"], &["--new-name", "replacement"]),
            (&["postgres", "delete"], &[]),
            (&["postgres", "certs", "get"], &[]),
            (&["postgres", "config", "get"], &[]),
            (
                &["postgres", "config", "replace"],
                &["--file", "config.json"],
            ),
            (
                &["postgres", "config", "patch"],
                &["--set", "max_connections=200"],
            ),
            (&["postgres", "reset-password"], &["--generate"]),
            (
                &["postgres", "metrics"],
                &[
                    "--from-date",
                    "2026-01-01T00:00:00Z",
                    "--to-date",
                    "2026-01-02T00:00:00Z",
                ],
            ),
            (&["postgres", "prometheus", "service"], &[]),
            (&["postgres", "restart"], &[]),
            (&["postgres", "promote"], &[]),
            (&["postgres", "switchover"], &[]),
            (&["key", "get"], &[]),
            (&["key", "update"], &["--new-name", "replacement"]),
            (&["key", "delete"], &[]),
            (&["org", "role", "get"], &[]),
            (&["org", "role", "update"], &["--file", "role.json"]),
            (&["org", "role", "delete"], &[]),
            (
                &["org", "byoc", "update"],
                &["--display-name", "replacement"],
            ),
            (&["org", "byoc", "delete"], &[]),
            (&["clickpipe", "get", "parent"], &[]),
            (&["clickpipe", "update", "parent"], &["--file", "pipe.json"]),
            (&["clickpipe", "delete", "parent"], &[]),
            (&["clickpipe", "start", "parent"], &[]),
            (&["clickpipe", "stop", "parent"], &[]),
            (&["clickpipe", "resync", "parent"], &[]),
            (&["clickpipe", "scale", "parent"], &["--replicas", "2"]),
            (&["clickpipe", "settings", "get", "parent"], &[]),
            (
                &["clickpipe", "settings", "update", "parent"],
                &["--streaming-max-insert-wait-ms", "1000"],
            ),
            (&["query-api-endpoint", "get", "parent"], &[]),
            (
                &["query-api-endpoint", "update", "parent"],
                &["--file", "endpoint.json"],
            ),
            (&["query-api-endpoint", "delete", "parent"], &[]),
        ];
        for (prefix, suffix) in cases {
            let parse = |selector: &[&str]| {
                Cli::try_parse_from(
                    ["chctl", "cloud"]
                        .into_iter()
                        .chain(prefix.iter().copied())
                        .chain(selector.iter().copied())
                        .chain(suffix.iter().copied()),
                )
            };
            assert!(
                parse(&["opaque-not-a-uuid"]).is_ok(),
                "ID {prefix:?}: {:?}",
                parse(&["opaque-not-a-uuid"]).err()
            );
            assert!(
                parse(&["--name", "Exact Name"]).is_ok(),
                "name {prefix:?}: {:?}",
                parse(&["--name", "Exact Name"]).err()
            );
            assert_eq!(
                parse(&[]).err().unwrap().kind(),
                ErrorKind::MissingRequiredArgument,
                "{prefix:?}"
            );
            assert_eq!(
                parse(&["id", "--name", "Exact Name"]).err().unwrap().kind(),
                ErrorKind::ArgumentConflict,
                "{prefix:?}"
            );
        }
    }

    #[test]
    fn setting_natural_key_cannot_be_consumed_as_service_id() {
        for operation in ["get", "unset"] {
            let prefix = ["chctl", "cloud", "service", "settings", operation];
            let cli = Cli::try_parse_from(prefix.into_iter().chain([
                "--name",
                "analytics",
                "max_threads",
            ]))
            .unwrap();
            let Commands::Cloud(args) = cli.command else {
                panic!()
            };
            let CloudCommands::Service {
                command: ServiceCommands::Settings { command },
            } = args.command
            else {
                panic!()
            };
            let (selector, key) = match command {
                ServiceSettingsCommands::Get { target }
                | ServiceSettingsCommands::Unset { target } => {
                    let (selector, key) = target.parts();
                    (selector, key.to_string())
                }
                _ => panic!(),
            };
            assert_eq!(selector.name.as_deref(), Some("analytics"));
            assert!(selector.id.is_none());
            assert_eq!(key, "max_threads");
            assert!(Cli::try_parse_from(prefix.into_iter().chain(["id", "max_threads"])).is_ok());
            for suffix in [
                vec!["max_threads"],
                vec!["--name", "analytics"],
                vec!["id", "max_threads", "--name", "analytics"],
            ] {
                assert!(Cli::try_parse_from(prefix.into_iter().chain(suffix)).is_err());
            }
        }
    }

    #[test]
    fn query_positional_id_and_legacy_id_are_exclusive_and_legacy_is_hidden() {
        let prefix = ["chctl", "cloud", "service", "query"];
        for selector in [
            vec!["opaque-id"],
            vec!["--id", "opaque-id"],
            vec!["--name", "analytics"],
        ] {
            assert!(Cli::try_parse_from(prefix.into_iter().chain(selector)).is_ok());
        }
        for selector in [
            vec![],
            vec!["id", "--id", "id"],
            vec!["id", "--name", "name"],
            vec!["--id", "id", "--name", "name"],
        ] {
            assert!(Cli::try_parse_from(prefix.into_iter().chain(selector)).is_err());
        }
        let mut root = Cli::command();
        root.build();
        let query = root
            .find_subcommand("cloud")
            .unwrap()
            .find_subcommand("service")
            .unwrap()
            .find_subcommand("query")
            .unwrap();
        assert!(
            query
                .get_arguments()
                .find(|arg| arg.get_id() == "id")
                .unwrap()
                .is_hide_set()
        );
    }

    #[test]
    fn exact_resolution_rejects_incomplete_or_ambiguous_collections() {
        let rows = [
            (Some("Analytics"), Some("id-1")),
            (Some("analytics"), Some("opaque-id")),
        ];
        assert_eq!(
            select_named_id("service", "analytics", rows).unwrap(),
            "opaque-id"
        );
        for rows in [
            vec![],
            vec![(Some("other"), Some("id"))],
            vec![(Some("analytics"), None)],
            vec![(Some("analytics"), Some(" "))],
            vec![(None, Some("id"))],
            vec![
                (Some("analytics"), Some("one")),
                (Some("analytics"), Some("two")),
            ],
        ] {
            assert!(select_named_id("service", "analytics", rows).is_err());
        }
    }
}

#[cfg(test)]
mod scoped_selector_tests {
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use crate::cloud::postgres::PostgresCommands;
    use clap::{Parser, error::ErrorKind};

    #[test]
    fn organization_selector_conflicts_across_every_command_depth() {
        let path = ["service", "settings", "get", "service-id", "setting"];
        for org_id_at in 0..=path.len() {
            for org_name_at in 0..=path.len() {
                let mut args = vec!["chctl", "cloud"];
                for i in 0..=path.len() {
                    if i == org_id_at {
                        args.extend(["--org-id", "org-id"]);
                    }
                    if i == org_name_at {
                        args.extend(["--org-name", "production"]);
                    }
                    if let Some(token) = path.get(i) {
                        args.push(token);
                    }
                }
                let mut command = <Cli as clap::CommandFactory>::command();
                let error = match Cli::try_parse_from(&args) {
                    Err(error) => error,
                    Ok(cli) => crate::validate_post_parse(&cli, &mut command)
                        .expect_err("global selectors must conflict after propagation"),
                };
                assert_eq!(error.kind(), ErrorKind::ArgumentConflict, "{args:?}");
            }
        }
        for position in 0..=path.len() {
            let mut args = vec!["chctl", "cloud"];
            args.extend_from_slice(&path[..position]);
            args.extend(["--org-name", "production"]);
            args.extend_from_slice(&path[position..]);
            let Commands::Cloud(cloud) = Cli::try_parse_from(&args).unwrap().command else {
                panic!()
            };
            assert_eq!(cloud.org_name.as_deref(), Some("production"));
        }
    }

    #[test]
    fn source_and_creation_names_are_independent_and_creation_name_stays_required() {
        for prefix in [
            vec!["postgres", "restore"],
            vec!["postgres", "read-replica", "create"],
        ] {
            let extra = if prefix.contains(&"restore") {
                vec!["--restore-target", "2026-09-01T12:00:00Z"]
            } else {
                vec![]
            };
            let parse = |selection: &[&str]| {
                Cli::try_parse_from(
                    ["chctl", "cloud"]
                        .into_iter()
                        .chain(prefix.iter().copied())
                        .chain(selection.iter().copied())
                        .chain(extra.iter().copied()),
                )
            };
            for selection in [
                vec!["source-id", "--name", "new"],
                vec!["--source-name", "source", "--name", "new"],
            ] {
                assert!(parse(&selection).is_ok());
            }
            for selection in [
                vec!["--name", "new"],
                vec!["source-id"],
                vec!["--source-name", "source"],
                vec!["source-id", "--source-name", "source", "--name", "new"],
            ] {
                assert!(parse(&selection).is_err());
            }
        }
        let cli = Cli::try_parse_from([
            "chctl",
            "cloud",
            "postgres",
            "restore",
            "--source-name",
            "source",
            "--name",
            "new",
            "--restore-target",
            "2026-09-01T12:00:00Z",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!()
        };
        let CloudCommands::Postgres {
            command: PostgresCommands::Restore {
                postgres_id, name, ..
            },
        } = args.command
        else {
            panic!()
        };
        assert_eq!(postgres_id.source_name.as_deref(), Some("source"));
        assert!(postgres_id.id.is_none());
        assert_eq!(name, "new");
    }

    #[test]
    fn email_selectors_are_required_exclusive_and_never_use_display_names() {
        for prefix in [
            vec!["member", "get"],
            vec!["member", "update"],
            vec!["member", "remove"],
            vec!["invitation", "get"],
            vec!["invitation", "delete"],
        ] {
            let parse = |selection: &[&str]| {
                Cli::try_parse_from(
                    ["chctl", "cloud"]
                        .into_iter()
                        .chain(prefix.iter().copied())
                        .chain(selection.iter().copied()),
                )
            };
            assert!(parse(&["opaque-id"]).is_ok());
            assert!(parse(&["--email", "Person@example.com"]).is_ok());
            for selection in [
                vec![],
                vec!["id", "--email", "Person@example.com"],
                vec!["--name", "Person"],
            ] {
                assert!(parse(&selection).is_err());
            }
        }
    }

    #[test]
    fn settings_keep_both_keys_across_options_and_reject_incomplete_forms() {
        for operation in ["get", "unset"] {
            let prefix = ["chctl", "cloud", "service", "settings", operation];
            for args in [
                vec!["id", "--org-id", "org", "key"],
                vec!["id", "--json", "key"],
                vec!["--name", "analytics", "key"],
                vec!["key", "--name", "analytics"],
                vec!["id", "--", "-key"],
            ] {
                assert!(
                    Cli::try_parse_from(prefix.into_iter().chain(args.iter().copied())).is_ok(),
                    "{operation} {args:?}"
                );
            }
            for args in [
                vec![],
                vec!["id"],
                vec!["--name", "analytics"],
                vec!["id", "key", "extra"],
                vec!["analytics", "key", "--name", "analytics"],
                vec!["--name", "analytics", "-key"],
            ] {
                assert!(
                    Cli::try_parse_from(prefix.into_iter().chain(args.iter().copied())).is_err(),
                    "{operation} {args:?}"
                );
            }
        }
    }
}

#[cfg(test)]
mod selector_usage_tests {
    use crate::cli::Cli;
    use clap::CommandFactory;

    #[test]
    fn scoped_child_usage_and_argument_indices_put_service_before_target() {
        let mut root = Cli::command();
        root.build();
        for path in [
            vec!["cloud", "clickpipe", "get"],
            vec!["cloud", "clickpipe", "settings", "get"],
            vec!["cloud", "query-api-endpoint", "get"],
            vec!["cloud", "clickstack", "alert", "delete"],
            vec!["cloud", "clickstack", "webhook", "delete"],
            vec!["cloud", "clickstack", "source", "get"],
            vec!["cloud", "clickstack", "role", "get"],
            vec!["cloud", "clickstack", "dashboard", "get"],
            vec!["cloud", "clickstack", "saved-search", "get"],
        ] {
            let mut command = &root;
            for part in path {
                command = command.find_subcommand(part).unwrap();
            }
            let service = command
                .get_arguments()
                .find(|arg| arg.get_id() == "service_id")
                .unwrap();
            let target = command
                .get_arguments()
                .find(|arg| arg.get_id() == "resource_id")
                .unwrap();
            assert_eq!(service.get_index(), Some(1));
            assert!(service.is_required_set());
            assert_eq!(target.get_index(), Some(2));
            let usage = command.clone().render_usage().to_string();
            assert!(
                usage.find("<SERVICE_ID>").unwrap() < usage.find("[ID]").unwrap(),
                "{usage}"
            );
        }
    }
}
