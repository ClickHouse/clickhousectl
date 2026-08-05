use crate::cloud::client::{CloudClient, CloudError};
use crate::cloud::credentials;
use crate::cloud::output::{ABSENT, or_absent, print_human};
use clickhouse_cloud_api::models::{
    ApiKeyPatchRequest, ApiKeyPatchRequestState, ApiKeyPostRequest, ApiKeyPostRequestState,
    AutoscalingMode, BackupConfigurationPatchRequest, InstancePrivateEndpointsPatch,
    InstanceServiceQueryApiEndpointsPostRequest, InstanceTagsPatch, IpAccessListEntry,
    IpAccessListPatch, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPatchRequest, OrganizationPrivateEndpointsPatch, ResourceTagsV1,
    ServicPrivateEndpointePostRequest, Service, ServiceEndpoint, ServiceEndpointChange,
    ServiceEndpointChangeProtocol, ServicePasswordPatchRequest, ServicePatchRequest,
    ServicePatchRequestReleasechannel, ServicePostRequest, ServicePostRequestCompliancetype,
    ServicePostRequestProfile, ServicePostRequestProvider, ServicePostRequestRegion,
    ServicePostRequestReleasechannel, ServiceReplicaScalingPatchRequest, ServiceState,
    ServiceStatePatchRequestCommand,
};
use std::io::{IsTerminal, Write};
use tabled::{Table, Tabled, settings::Style};

/// Comma-joins the rendered items of a response list.
///
/// An absent list renders as [`ABSENT`]; an absent field of an individual item
/// renders as [`ABSENT`] inside the join, so a partially-returned list stays
/// readable.
fn join_absent<T>(items: Option<&[T]>, render: impl Fn(&T) -> String) -> String {
    match items {
        Some(items) => items.iter().map(render).collect::<Vec<_>>().join(", "),
        None => ABSENT.to_string(),
    }
}

/// `host:port` of a service's first endpoint, for list tables.
///
/// Renders [`ABSENT`] when the API returned no endpoints, and keeps whichever
/// half of a partial endpoint it did return (`host` alone, or `-:port`).
fn first_endpoint(endpoints: Option<&[ServiceEndpoint]>) -> String {
    endpoints
        .and_then(|endpoints| endpoints.first())
        .map(|endpoint| match (endpoint.host.as_deref(), endpoint.port) {
            (Some(host), Some(port)) => format!("{host}:{port}"),
            (Some(host), None) => host.to_string(),
            (None, Some(port)) => format!("{ABSENT}:{port}"),
            (None, None) => ABSENT.to_string(),
        })
        .unwrap_or_else(|| ABSENT.to_string())
}

/// Resolve org ID from explicit arg or auto-detect
pub(super) async fn resolve_org_id(
    client: &CloudClient,
    org_id: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    match org_id {
        Some(id) => Ok(id.to_string()),
        None => Ok(client.get_default_org_id().await?),
    }
}

/// Resolve a service by name or ID within the given org.
/// Exactly one of `name` or `id` must be provided.
async fn resolve_service(
    client: &CloudClient,
    org_id: &str,
    name: Option<&str>,
    id: Option<&str>,
) -> Result<Service, Box<dyn std::error::Error>> {
    match (name, id) {
        (Some(name), None) => {
            let services = client.list_services(org_id).await?;
            let matches: Vec<_> = services
                .into_iter()
                .filter(|s| s.name.as_deref() == Some(name))
                .collect();
            match matches.len() {
                0 => Err(format!("no service found with name '{}'", name).into()),
                1 => Ok(matches.into_iter().next().unwrap()),
                n => Err(format!(
                    "found {} services named '{}' — use --id to disambiguate",
                    n, name
                )
                .into()),
            }
        }
        (None, Some(id)) => Ok(client.get_service(org_id, id).await?),
        (Some(_), Some(_)) => Err("specify either --name or --id, not both".into()),
        (None, None) => Err("specify --name or --id to identify the service".into()),
    }
}

/// Parse a string into a library enum via serde deserialization, with client-side
/// validation against a known-values list. Library enums have an `Unknown(String)`
/// catch-all that prevents serde from ever failing, so we validate first.
pub(super) fn parse_serde_enum<T: serde::de::DeserializeOwned>(
    value: &str,
    field: &str,
    known_values: &[&str],
) -> Result<T, Box<dyn std::error::Error>> {
    if !known_values.contains(&value) {
        return Err(format!(
            "invalid {}: unknown value '{}', expected one of: {}",
            field,
            value,
            known_values.join(", ")
        )
        .into());
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| format!("invalid {}: {}", field, e).into())
}

pub(super) fn parse_tag(value: &str) -> Result<ResourceTagsV1, Box<dyn std::error::Error>> {
    match value.split_once('=') {
        Some((key, tag_value)) => {
            let key = key.trim();
            if key.is_empty() {
                Err(format!("invalid tag '{}': tag key cannot be empty", value).into())
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
                Err(format!("invalid tag '{}': tag key cannot be empty", value).into())
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: None,
                })
            }
        }
    }
}

pub(super) fn parse_tags(
    values: &[String],
) -> Result<Option<Vec<ResourceTagsV1>>, Box<dyn std::error::Error>> {
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

fn parse_ip_access_entries(values: &[String]) -> Option<Vec<IpAccessListEntry>> {
    (!values.is_empty()).then(|| {
        values
            .iter()
            .map(|value| IpAccessListEntry {
                source: value.clone(),
                description: None,
            })
            .collect()
    })
}

fn parse_ip_access_list_patch(add: &[String], remove: &[String]) -> Option<IpAccessListPatch> {
    let patch = IpAccessListPatch {
        add: parse_ip_access_entries(add).unwrap_or_default(),
        remove: parse_ip_access_entries(remove).unwrap_or_default(),
    };

    (!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch)
}

fn parse_private_endpoint_ids_patch(
    add: &[String],
    remove: &[String],
) -> Option<InstancePrivateEndpointsPatch> {
    let patch = InstancePrivateEndpointsPatch {
        add: if add.is_empty() { vec![] } else { add.to_vec() },
        remove: if remove.is_empty() {
            vec![]
        } else {
            remove.to_vec()
        },
    };

    (!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch)
}

fn parse_service_endpoint_changes(
    enable: &[String],
    disable: &[String],
) -> Result<Option<Vec<ServiceEndpointChange>>, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    for protocol in enable {
        changes.push(ServiceEndpointChange {
            protocol: parse_serde_enum::<ServiceEndpointChangeProtocol>(
                protocol,
                "endpoint",
                &["mysql"],
            )?,
            enabled: true,
        });
    }

    for protocol in disable {
        changes.push(ServiceEndpointChange {
            protocol: parse_serde_enum::<ServiceEndpointChangeProtocol>(
                protocol,
                "endpoint",
                &["mysql"],
            )?,
            enabled: false,
        });
    }

    Ok((!changes.is_empty()).then_some(changes))
}

fn parse_instance_tags_patch(
    add: &[String],
    remove: &[String],
) -> Result<Option<InstanceTagsPatch>, Box<dyn std::error::Error>> {
    let patch = InstanceTagsPatch {
        add: parse_tags(add)?.unwrap_or_default(),
        remove: parse_tags(remove)?.unwrap_or_default(),
    };

    Ok((!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch))
}

fn parse_org_private_endpoint_remove(
    value: &str,
) -> Result<OrganizationPatchPrivateEndpoint, Box<dyn std::error::Error>> {
    let mut endpoint = OrganizationPatchPrivateEndpoint {
        id: String::new(),
        description: None,
        cloud_provider: OrganizationPatchPrivateEndpointCloudprovider::default(),
        region: OrganizationPatchPrivateEndpointRegion::default(),
    };

    for (index, part) in value.split(',').enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if index == 0 && !part.contains('=') {
            endpoint.id = part.to_string();
            continue;
        }

        let (key, raw_value) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid remove-private-endpoint segment '{}'", part))?;

        match key {
            "id" => endpoint.id = raw_value.to_string(),
            "description" => endpoint.description = Some(raw_value.to_string()),
            "cloud-provider" => {
                endpoint.cloud_provider =
                    serde_json::from_value::<OrganizationPatchPrivateEndpointCloudprovider>(
                        serde_json::Value::String(raw_value.to_string()),
                    )
                    .expect("enum with Unknown variant should always deserialize");
            }
            "region" => {
                endpoint.region =
                    serde_json::from_value::<OrganizationPatchPrivateEndpointRegion>(
                        serde_json::Value::String(raw_value.to_string()),
                    )
                    .expect("enum with Unknown variant should always deserialize");
            }
            _ => {
                return Err(format!(
                    "invalid remove-private-endpoint key '{}'; expected id, description, cloud-provider, or region",
                    key
                )
                .into())
            }
        }
    }

    Ok(endpoint)
}

fn parse_org_private_endpoints_patch(
    remove: &[String],
) -> Result<Option<OrganizationPrivateEndpointsPatch>, Box<dyn std::error::Error>> {
    if remove.is_empty() {
        return Ok(None);
    }

    let endpoints = remove
        .iter()
        .map(|value| parse_org_private_endpoint_remove(value))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(OrganizationPrivateEndpointsPatch {
        #[cfg(feature = "deprecated-fields")]
        add: None,
        remove: endpoints,
    }))
}

fn parse_api_key_hash_data(
    key_id_hash: Option<&str>,
    key_id_suffix: Option<&str>,
    key_secret_hash: Option<&str>,
) -> Result<Option<clickhouse_cloud_api::models::ApiKeyHashData>, Box<dyn std::error::Error>> {
    match (key_id_hash, key_id_suffix, key_secret_hash) {
        (None, None, None) => Ok(None),
        (Some(key_id_hash), Some(key_id_suffix), Some(key_secret_hash)) => {
            Ok(Some(clickhouse_cloud_api::models::ApiKeyHashData {
                key_id_hash: key_id_hash.to_string(),
                key_id_suffix: key_id_suffix.to_string(),
                key_secret_hash: key_secret_hash.to_string(),
            }))
        }
        _ => Err(
            "pre-hashed API key input requires --hash-key-id, --hash-key-id-suffix, and --hash-key-secret together"
                .into(),
        ),
    }
}

fn parse_ip_access_entries_lib(values: &[String]) -> Option<Vec<IpAccessListEntry>> {
    (!values.is_empty()).then(|| {
        values
            .iter()
            .map(|value| IpAccessListEntry {
                source: value.clone(),
                description: None,
            })
            .collect()
    })
}

fn parse_uuid_list(
    values: &[String],
    field: &str,
) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|s| {
            uuid::Uuid::parse_str(s)
                .map_err(|e| format!("invalid {} UUID '{}': {}", field, s, e).into())
        })
        .collect()
}

fn parse_api_key_state_post(
    value: &str,
) -> Result<ApiKeyPostRequestState, Box<dyn std::error::Error>> {
    match value {
        "enabled" => Ok(ApiKeyPostRequestState::Enabled),
        "disabled" => Ok(ApiKeyPostRequestState::Disabled),
        _ => Err(format!(
            "invalid state: unknown value '{}', expected one of: enabled, disabled",
            value
        )
        .into()),
    }
}

fn parse_api_key_state_patch(
    value: &str,
) -> Result<ApiKeyPatchRequestState, Box<dyn std::error::Error>> {
    match value {
        "enabled" => Ok(ApiKeyPatchRequestState::Enabled),
        "disabled" => Ok(ApiKeyPatchRequestState::Disabled),
        _ => Err(format!(
            "invalid state: unknown value '{}', expected one of: enabled, disabled",
            value
        )
        .into()),
    }
}

fn parse_expire_at(
    value: &str,
) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn std::error::Error>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| {
            format!(
                "invalid expire_at '{}': expected ISO 8601 / RFC 3339 format (e.g. 2025-12-31T23:59:59Z): {}",
                value, e
            )
            .into()
        })
}

pub async fn org_list(client: &CloudClient, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let orgs = client.list_organizations().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&orgs)?);
    } else {
        if orgs.is_empty() {
            println!("No organizations found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "ID")]
            id: String,
        }
        let rows: Vec<Row> = orgs
            .into_iter()
            .map(|o| Row {
                name: or_absent(o.name.as_deref()),
                id: or_absent(o.id),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn org_get(
    client: &CloudClient,
    org_id: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org = client.get_organization(org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&org)?);
    } else {
        print_human(&org)?;
    }
    Ok(())
}

pub async fn service_list(
    client: &CloudClient,
    org_id: Option<&str>,
    filters: &[String],
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let services = if filters.is_empty() {
        client.list_services(&org_id).await?
    } else {
        client.list_services_filtered(&org_id, filters).await?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&services)?);
    } else {
        if services.is_empty() {
            println!("No services found");
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
            #[tabled(rename = "Endpoint")]
            endpoint: String,
        }
        let rows: Vec<Row> = services
            .into_iter()
            .map(|svc| Row {
                name: or_absent(svc.name.as_deref()),
                id: or_absent(svc.id),
                state: or_absent(svc.state.as_ref()),
                provider: or_absent(svc.provider.as_ref()),
                region: or_absent(svc.region.as_ref()),
                endpoint: first_endpoint(svc.endpoints.as_deref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn service_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let svc = client.get_service(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        print_human(&svc)?;
    }
    Ok(())
}

/// Options for creating a service
#[derive(Default)]
pub struct CreateServiceOptions {
    pub name: String,
    pub provider: String,
    pub region: String,
    pub min_replica_memory_gb: Option<u32>,
    pub max_replica_memory_gb: Option<u32>,
    pub num_replicas: Option<u32>,
    pub min_replicas: Option<u32>,
    pub max_replicas: Option<u32>,
    pub autoscaling_mode: Option<String>,
    pub idle_scaling: Option<bool>,
    pub idle_timeout_minutes: Option<u32>,
    pub ip_allow: Vec<String>,
    pub backup_id: Option<String>,
    pub release_channel: Option<String>,
    pub data_warehouse_id: Option<String>,
    pub is_readonly: bool,
    pub encryption_key: Option<String>,
    pub encryption_role: Option<String>,
    pub enable_tde: bool,
    pub compliance_type: Option<String>,
    pub profile: Option<String>,
    pub tags: Vec<String>,
    pub enable_endpoints: Vec<String>,
    pub disable_endpoints: Vec<String>,
    pub private_preview_terms_checked: bool,
    pub enable_core_dumps: Option<bool>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct ServiceUpdateOptions {
    pub name: Option<String>,
    pub add_ip_allow: Vec<String>,
    pub remove_ip_allow: Vec<String>,
    pub add_private_endpoint_ids: Vec<String>,
    pub remove_private_endpoint_ids: Vec<String>,
    pub release_channel: Option<String>,
    pub enable_endpoints: Vec<String>,
    pub disable_endpoints: Vec<String>,
    pub transparent_data_encryption_key_id: Option<String>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
    pub enable_core_dumps: Option<bool>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct ServiceResetPasswordOptions {
    pub new_password_hash: Option<String>,
    pub new_double_sha1_hash: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct QueryEndpointCreateOptions {
    pub roles: Vec<String>,
    pub open_api_keys: Vec<String>,
    pub allowed_origins: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct OrgUpdateOptions {
    pub name: Option<String>,
    pub remove_private_endpoints: Vec<String>,
    pub enable_core_dumps: Option<bool>,
}

#[derive(Default)]
pub struct KeyCreateOptions {
    pub name: String,
    pub role_ids: Vec<String>,
    pub expires_at: Option<String>,
    pub state: Option<String>,
    pub ip_allow: Vec<String>,
    pub hash_key_id: Option<String>,
    pub hash_key_id_suffix: Option<String>,
    pub hash_key_secret: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct KeyUpdateOptions {
    pub name: Option<String>,
    pub role_ids: Vec<String>,
    pub expires_at: Option<String>,
    pub state: Option<String>,
    pub ip_allow: Vec<String>,
    pub org_id: Option<String>,
}

#[derive(Default)]
pub struct BackupConfigUpdateOptions {
    pub backup_period_hours: Option<u32>,
    pub backup_retention_period_hours: Option<u32>,
    pub backup_start_time: Option<String>,
    pub org_id: Option<String>,
}

/// Resolved horizontal-autoscaling fields for a service create/scale request.
struct HorizontalAutoscaling {
    autoscaling_mode: Option<AutoscalingMode>,
    min_replicas: Option<i64>,
    max_replicas: Option<i64>,
}

/// Resolve the horizontal-autoscaling fields shared by `service create` and
/// `service scale`.
///
/// The mode is sent only when `--autoscaling-mode` is given explicitly. The
/// API resolves an omitted mode itself, and a min/max band with the mode
/// omitted and min == max is accepted as a vertical fixed replica count that
/// needs no horizontal entitlement — inferring `horizontal` here would change
/// those semantics.
///
/// Rejects `--min-replicas` without `--max-replicas` (and vice versa) with a
/// clear error before any network call. clap already rejects mixing the
/// horizontal pair with `--num-replicas`; the memory flags and
/// `--autoscaling-mode` combine freely with either set because a single
/// request can switch modes (e.g. `--autoscaling-mode vertical
/// --num-replicas 3`, or `--autoscaling-mode horizontal` with the equal
/// memory bounds horizontal requires). Remaining combination rules are the
/// API's to enforce.
fn resolve_horizontal_autoscaling(
    autoscaling_mode: Option<&str>,
    min_replicas: Option<u32>,
    max_replicas: Option<u32>,
) -> Result<HorizontalAutoscaling, Box<dyn std::error::Error>> {
    if min_replicas.is_some() != max_replicas.is_some() {
        return Err("--min-replicas and --max-replicas must be specified together".into());
    }
    let autoscaling_mode = autoscaling_mode
        .map(|value| {
            parse_serde_enum::<AutoscalingMode>(value, "autoscaling_mode", AutoscalingMode::VALUES)
        })
        .transpose()?;
    Ok(HorizontalAutoscaling {
        autoscaling_mode,
        min_replicas: min_replicas.map(i64::from),
        max_replicas: max_replicas.map(i64::from),
    })
}

fn build_create_service_request(
    opts: &CreateServiceOptions,
) -> Result<ServicePostRequest, Box<dyn std::error::Error>> {
    let ip_access_list = if opts.ip_allow.is_empty() {
        vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Allow all (created by clickhousectl)".to_string()),
        }]
    } else {
        parse_ip_access_entries(&opts.ip_allow).unwrap_or_default()
    };

    let horizontal = resolve_horizontal_autoscaling(
        opts.autoscaling_mode.as_deref(),
        opts.min_replicas,
        opts.max_replicas,
    )?;

    Ok(ServicePostRequest {
        name: opts.name.clone(),
        provider: parse_serde_enum::<ServicePostRequestProvider>(
            &opts.provider,
            "provider",
            ServicePostRequestProvider::VALUES,
        )?,
        region: parse_serde_enum::<ServicePostRequestRegion>(
            &opts.region,
            "region",
            ServicePostRequestRegion::VALUES,
        )?,
        ip_access_list,
        min_replica_memory_gb: opts.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: opts.max_replica_memory_gb.map(f64::from),
        num_replicas: opts.num_replicas.map(i64::from),
        idle_scaling: opts.idle_scaling,
        idle_timeout_minutes: opts.idle_timeout_minutes.map(f64::from),
        backup_id: opts
            .backup_id
            .as_deref()
            .map(uuid::Uuid::parse_str)
            .transpose()
            .map_err(|e| format!("invalid backup_id: {}", e))?,
        release_channel: match opts.release_channel.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestReleasechannel>(
                value,
                "release_channel",
                ServicePostRequestReleasechannel::VALUES,
            )?),
            None => None,
        },
        tags: parse_tags(&opts.tags)?,
        data_warehouse_id: opts.data_warehouse_id.clone(),
        is_readonly: if opts.is_readonly { Some(true) } else { None },
        encryption_key: opts.encryption_key.clone(),
        encryption_assumed_role_identifier: opts.encryption_role.clone(),
        has_transparent_data_encryption: if opts.enable_tde { Some(true) } else { None },
        compliance_type: match opts.compliance_type.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestCompliancetype>(
                value,
                "compliance_type",
                ServicePostRequestCompliancetype::VALUES,
            )?),
            None => None,
        },
        profile: match opts.profile.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestProfile>(
                value,
                "profile",
                ServicePostRequestProfile::VALUES,
            )?),
            None => None,
        },
        private_preview_terms_checked: if opts.private_preview_terms_checked {
            Some(true)
        } else {
            None
        },
        endpoints: parse_service_endpoint_changes(&opts.enable_endpoints, &opts.disable_endpoints)?,
        enable_core_dumps: opts.enable_core_dumps,
        // Fields not exposed in CLI
        autoscaling_mode: horizontal.autoscaling_mode,
        byoc_id: None,
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
        // Deprecated fields — only exist (and stay None) under the
        // `deprecated-fields` feature; gated out of the struct otherwise.
        #[cfg(feature = "deprecated-fields")]
        max_total_memory_gb: None,
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: None,
        #[cfg(feature = "deprecated-fields")]
        private_endpoint_ids: None,
        #[cfg(feature = "deprecated-fields")]
        tier: None,
    })
}

fn build_update_service_request(
    opts: &ServiceUpdateOptions,
) -> Result<ServicePatchRequest, Box<dyn std::error::Error>> {
    Ok(ServicePatchRequest {
        name: opts.name.clone(),
        ip_access_list: parse_ip_access_list_patch(&opts.add_ip_allow, &opts.remove_ip_allow),
        private_endpoint_ids: parse_private_endpoint_ids_patch(
            &opts.add_private_endpoint_ids,
            &opts.remove_private_endpoint_ids,
        ),
        release_channel: opts
            .release_channel
            .as_deref()
            .map(|value| {
                parse_serde_enum::<ServicePatchRequestReleasechannel>(
                    value,
                    "release_channel",
                    ServicePatchRequestReleasechannel::VALUES,
                )
            })
            .transpose()?,
        endpoints: parse_service_endpoint_changes(&opts.enable_endpoints, &opts.disable_endpoints)?,
        transparent_data_encryption_key_id: opts.transparent_data_encryption_key_id.clone(),
        tags: parse_instance_tags_patch(&opts.add_tags, &opts.remove_tags)?,
        enable_core_dumps: opts.enable_core_dumps,
    })
}

fn build_service_password_patch_request(
    opts: &ServiceResetPasswordOptions,
) -> ServicePasswordPatchRequest {
    ServicePasswordPatchRequest {
        new_password_hash: opts.new_password_hash.clone(),
        new_double_sha1_hash: opts.new_double_sha1_hash.clone(),
    }
}

fn build_query_endpoint_create_request(
    opts: &QueryEndpointCreateOptions,
) -> InstanceServiceQueryApiEndpointsPostRequest {
    InstanceServiceQueryApiEndpointsPostRequest {
        roles: opts.roles.clone(),
        open_api_keys: opts.open_api_keys.clone(),
        allowed_origins: opts.allowed_origins.clone().unwrap_or_default(),
    }
}

fn build_org_update_request(
    opts: &OrgUpdateOptions,
) -> Result<OrganizationPatchRequest, Box<dyn std::error::Error>> {
    Ok(OrganizationPatchRequest {
        name: opts.name.clone(),
        private_endpoints: parse_org_private_endpoints_patch(&opts.remove_private_endpoints)?,
        enable_core_dumps: opts.enable_core_dumps,
    })
}

fn build_api_key_create_request(
    opts: &KeyCreateOptions,
) -> Result<ApiKeyPostRequest, Box<dyn std::error::Error>> {
    Ok(ApiKeyPostRequest {
        name: opts.name.clone(),
        expire_at: opts
            .expires_at
            .as_deref()
            .map(parse_expire_at)
            .transpose()?,
        state: match opts.state.as_deref() {
            Some(value) => parse_api_key_state_post(value)?,
            None => ApiKeyPostRequestState::default(),
        },
        assigned_role_ids: parse_uuid_list(&opts.role_ids, "role_id")?,
        ip_access_list: parse_ip_access_entries_lib(&opts.ip_allow).unwrap_or_default(),
        hash_data: parse_api_key_hash_data(
            opts.hash_key_id.as_deref(),
            opts.hash_key_id_suffix.as_deref(),
            opts.hash_key_secret.as_deref(),
        )?,
        #[cfg(feature = "deprecated-fields")]
        roles: None,
    })
}

fn build_api_key_update_request(
    opts: &KeyUpdateOptions,
) -> Result<ApiKeyPatchRequest, Box<dyn std::error::Error>> {
    Ok(ApiKeyPatchRequest {
        name: opts.name.clone(),
        assigned_role_ids: if opts.role_ids.is_empty() {
            None
        } else {
            Some(parse_uuid_list(&opts.role_ids, "role_id")?)
        },
        expire_at: opts
            .expires_at
            .as_deref()
            .map(parse_expire_at)
            .transpose()?,
        state: opts
            .state
            .as_deref()
            .map(parse_api_key_state_patch)
            .transpose()?,
        ip_access_list: parse_ip_access_entries_lib(&opts.ip_allow),
        #[cfg(feature = "deprecated-fields")]
        roles: None,
    })
}

fn build_backup_config_update_request(
    opts: &BackupConfigUpdateOptions,
) -> BackupConfigurationPatchRequest {
    BackupConfigurationPatchRequest {
        backup_period_in_hours: opts.backup_period_hours.map(f64::from),
        backup_retention_period_in_hours: opts.backup_retention_period_hours.map(f64::from),
        backup_start_time: opts.backup_start_time.clone(),
    }
}

/// The post-create hint showing how to query the new service.
///
/// The hint is only useful with a real service id: an absent id would render a
/// command line the user cannot run, so the hint is dropped rather than printed
/// with a placeholder id in it.
fn service_query_hint(service_id: Option<uuid::Uuid>) -> Option<String> {
    service_id.map(|id| {
        format!(
            "Run SQL with: clickhousectl cloud service query --id {} --query \"SELECT 1\"\n\
             (the Query API endpoint is provisioned automatically on first use)",
            id
        )
    })
}

/// The post-create credentials block, or the warning that replaces it.
///
/// The generated password is returned once, so a placeholder in its place would
/// be read as the credential itself. An absent password therefore gets the
/// omission plus the command that mints a usable one; the create succeeded and
/// the password is recoverable, so this is a warning rather than an error. An
/// empty string is a password the API sent.
fn service_credentials_block(password: Option<&str>, service_id: Option<uuid::Uuid>) -> String {
    match (password, service_id) {
        (Some(password), _) => format!(
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: {}",
            password
        ),
        (None, Some(id)) => format!(
            "WARNING: the API response omitted the one-time password, so it cannot be shown.\n\
             The service was created; reset the password to get a usable credential:\n  \
             clickhousectl cloud service reset-password {}",
            id
        ),
        (None, None) => "WARNING: the API response omitted the one-time password, so it cannot be \
                         shown.\nThe service was created; once you have its id, reset the password \
                         with `clickhousectl cloud service reset-password <service-id>` to get a \
                         usable credential."
            .to_string(),
    }
}

pub async fn service_create(
    client: &CloudClient,
    opts: CreateServiceOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input before any network call so typos like --provider awss
    // fail locally instead of on the /organizations lookup.
    let request = build_create_service_request(&opts)?;
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let response = client.create_service(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let service_id = response.service.as_ref().and_then(|svc| svc.id);
        println!("Service created successfully!");
        println!();
        if let Some(service) = &response.service {
            print_human(service)?;
        }
        println!();
        println!(
            "{}",
            service_credentials_block(response.password.as_deref(), service_id)
        );
        if let Some(hint) = service_query_hint(service_id) {
            println!();
            println!("{}", hint);
        }
    }
    Ok(())
}

/// Classifies one poll of a service's state while waiting for a stop to land.
///
/// Returns `true` once the service has stopped. An absent state cannot be
/// classified and the loop has no other exit, so treating it as "not stopped
/// yet" would poll forever: fail instead of waiting on a state the API is not
/// reporting.
fn classify_stop_poll_state(
    state: Option<&ServiceState>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let state = state
        .ok_or(
            "the API response omitted the service state while waiting for the service to stop, \
             so the stop cannot be confirmed",
        )?
        .to_string();
    if matches!(state.as_str(), "stopped" | "idle") {
        return Ok(true);
    }
    if matches!(state.as_str(), "terminated" | "failed" | "deleted") {
        return Err(format!(
            "service entered unexpected state '{}' while waiting for stop",
            state
        )
        .into());
    }
    Ok(false)
}

#[derive(Default)]
struct StopPollProgress {
    previous_state: Option<String>,
}

impl StopPollProgress {
    fn render(&mut self, state: &str, verbose: bool) -> Option<String> {
        let changed = self.previous_state.as_deref() != Some(state);
        self.previous_state = Some(state.to_string());
        (verbose || changed).then(|| format!("  state: {state}"))
    }
}

/// Replace the API's running-service conflict with the CLI remedy.
///
/// Keep other conflicts intact: the API can reject deletion for reasons that
/// `--force` cannot fix, and a forced deletion must not suggest the flag the
/// user already passed.
fn service_delete_error(error: CloudError, force: bool, service_id: &str) -> CloudError {
    if !force
        && error.message.starts_with("CONFLICT:")
        && error.message.contains("Current state: 'running'")
    {
        CloudError::new(format!(
            "service is running and cannot be deleted. Use --force to stop it first, or \
             `clickhousectl cloud service stop {service_id}`."
        ))
    } else {
        error
    }
}

/// Return a query API key only when its exact resource and organization IDs
/// were saved during provisioning. The boolean indicates that partial cleanup
/// metadata must remain on disk because discarding it would lose the key ID.
fn service_query_key_cleanup(
    org_id: &str,
    service_id: &str,
) -> Result<(Option<String>, bool), Box<dyn std::error::Error>> {
    let Some(key) = credentials::try_get_service_query_key(service_id)? else {
        return Ok((None, false));
    };
    let Some(api_key_id) = key.api_key_id else {
        eprintln!(
            "Warning: the stored query key for service {service_id} predates exact management \
             API key IDs; service deletion will continue without unsafe cloud key cleanup."
        );
        return Ok((None, false));
    };
    let Some(key_org_id) = key.organization_id else {
        eprintln!(
            "Warning: the stored query key for service {service_id} has a management API key ID \
             but no provisioning organization; cloud key cleanup was skipped and the local \
             record was retained."
        );
        return Ok((None, true));
    };
    if key_org_id != org_id {
        return Err(format!(
            "the stored query key for service {service_id} belongs to organization {key_org_id}, \
             not {org_id}; refusing to delete either resource"
        )
        .into());
    }
    Ok((Some(api_key_id), false))
}

async fn cleanup_service_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(api_key_id) = api_key_id else {
        return Ok(());
    };

    client
        .delete_api_key_if_exists(org_id, api_key_id)
        .await
        .map_err(|mut error| {
            error.message = format!(
                "failed to delete the auto-provisioned query API key for service \
                 {service_id}: {}",
                error.message
            );
            error
        })?;
    Ok(())
}

pub async fn service_delete(
    client: &CloudClient,
    service_id: &str,
    force: bool,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let (query_key_id, retain_query_key) = service_query_key_cleanup(&org_id, service_id)?;

    if force {
        let svc = client.get_service_if_exists(&org_id, service_id).await?;
        // An absent state matches nothing: skip the stop and let the delete
        // call decide, rather than guessing the service is running.
        let state = svc
            .as_ref()
            .map(|service| or_absent(service.state.as_ref()))
            .unwrap_or_default();
        if matches!(state.as_str(), "running" | "idle" | "starting") {
            eprintln!("Stopping service {} before deletion...", service_id);
            client
                .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
                .await?;

            // Poll until the service is stopped
            let verbose_polling =
                std::io::stderr().is_terminal() && !json && std::env::var_os("CI").is_none();
            let mut progress = StopPollProgress::default();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let svc = client.get_service(&org_id, service_id).await?;
                let state = or_absent(svc.state.as_ref());
                if let Some(line) = progress.render(&state, verbose_polling) {
                    eprintln!("{line}");
                }
                if classify_stop_poll_state(svc.state.as_ref())? {
                    break;
                }
            }
        }
    }

    let response = client
        .delete_service_if_exists(&org_id, service_id)
        .await
        .map_err(|error| service_delete_error(error, force, service_id))?;
    // Delete the key only after the service is gone. If cleanup fails, retain
    // its exact IDs locally so repeating service delete can retry safely.
    cleanup_service_query_key(client, &org_id, service_id, query_key_id.as_deref()).await?;
    if !retain_query_key {
        credentials::remove_service_query_key(service_id)?;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else if response.is_none() {
        println!("Service {} is already absent", service_id);
    } else {
        println!("Service {} deletion initiated", service_id);
    }
    Ok(())
}

pub async fn service_start(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let svc = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Start)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!(
            "Service {} starting (state: {})",
            or_absent(svc.name.as_deref()),
            or_absent(svc.state.as_ref())
        );
    }
    Ok(())
}

pub async fn service_stop(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let svc = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!(
            "Service {} stopping (state: {})",
            or_absent(svc.name.as_deref()),
            or_absent(svc.state.as_ref())
        );
    }
    Ok(())
}

pub async fn clickpipe_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipes = client.list_clickpipes(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipes)?);
    } else if clickpipes.is_empty() {
        println!("No ClickPipes found");
    } else {
        println!("ClickPipes:");
        for cp in &clickpipes {
            println!(
                "  {} ({}) - {}",
                or_absent(cp.name.as_deref()),
                or_absent(cp.id.as_ref()),
                or_absent(cp.state.as_ref())
            );
        }
    }
    Ok(())
}

pub async fn clickpipe_create_s3(
    client: &CloudClient,
    args: &crate::cloud::cli::ObjectStorageCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipePostObjectStorageSource, ClickPipePostObjectStorageSourceAuthentication,
        ClickPipePostRequest, ClickPipePostSource, MskIamUser,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let parsed_columns = parse_columns(&args.columns)?;

    let (authentication, iam_role_val, access_key) = match (
        args.iam_role.as_deref(),
        args.access_key_id.as_deref(),
        args.secret_key.as_deref(),
    ) {
        (Some(role), _, _) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE),
            Some(role.to_string()),
            None,
        ),
        (_, Some(key_id), Some(secret)) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER),
            None,
            Some(MskIamUser {
                access_key_id: key_id.to_string(),
                secret_key: secret.to_string(),
            }),
        ),
        _ => (None, None, None),
    };
    let authentication = authentication
        .or_else(|| {
            args.connection_string
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::CONNECTION_STRING)
        })
        .or_else(|| {
            args.service_account_file
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT)
        });

    let service_account_key = match args.service_account_file.as_deref() {
        Some(path) => Some(read_gcp_service_account_file(path)?),
        None => None,
    };

    let source = ClickPipePostObjectStorageSource {
        r#type: parse_enum(&args.storage_type)?,
        format: parse_enum(&args.format)?,
        url: args.source_url.clone(),
        compression: Some(parse_enum(&args.compression)?),
        is_continuous: if args.continuous { Some(true) } else { None },
        queue_url: args.queue_url.clone(),
        delimiter: args.delimiter.clone(),
        authentication,
        iam_role: iam_role_val,
        access_key,
        connection_string: args.connection_string.clone(),
        azure_container_name: args.azure_container_name.clone(),
        path: args.path.clone(),
        service_account_key,
        skip_initial_load: if args.skip_initial_load {
            Some(true)
        } else {
            None
        },
        start_after: args.start_after.clone(),
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            object_storage: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Build the Kafka `credentials` JSON body, whose shape is a `oneOf` determined
/// by the auth mode (see the `ClickPipePostKafkaSource.credentials` schema).
/// IAM_ROLE sends a null body — the role ARN flows through the separate
/// top-level `iamRole` field on the source, not through credentials.
///
/// `mtls_contents` is the pre-read (certificate, privateKey) PEM bundle used
/// only for MUTUAL_TLS; the caller reads these from disk so this function
/// stays pure and testable.
fn build_kafka_credentials(
    authentication: &clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication,
    args: &crate::cloud::cli::KafkaSourceFields,
    mtls_contents: Option<(String, String)>,
) -> Result<serde_json::Value, String> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    match authentication {
        Auth::PLAIN | Auth::SCRAM_SHA_256 | Auth::SCRAM_SHA_512 => {
            match (args.username.as_deref(), args.password.as_deref()) {
                (Some(u), Some(p)) => Ok(serde_json::json!({ "username": u, "password": p })),
                _ => Err(format!(
                    "{} requires --username and --password",
                    args.auth.as_deref().unwrap_or("PLAIN")
                )),
            }
        }
        Auth::IAM_USER => match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
            (Some(k), Some(s)) => Ok(serde_json::json!({ "accessKeyId": k, "secretKey": s })),
            _ => Err("IAM_USER requires --access-key-id and --secret-key".into()),
        },
        Auth::IAM_ROLE => {
            if args.iam_role.is_none() {
                Err("IAM_ROLE requires --iam-role".into())
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Auth::MUTUAL_TLS => match mtls_contents {
            Some((cert, key)) => Ok(serde_json::json!({ "certificate": cert, "privateKey": key })),
            None => Err("MUTUAL_TLS requires --client-certificate and --client-key".into()),
        },
        Auth::Unknown(_) => Ok(serde_json::Value::Null),
    }
}

/// Build a `ClickPipePostKafkaSource` from the CLI args, performing all
/// authentication/credential/schema-registry/CA validation up front so bad
/// invocations fail fast before any network call. Shared by the
/// `clickpipe create kafka` and `clickpipe schema-discover <SERVICE_ID> kafka`
/// handlers.
fn build_kafka_source(
    args: &crate::cloud::cli::KafkaSourceFields,
) -> Result<clickhouse_cloud_api::models::ClickPipePostKafkaSource, Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeKafkaOffset, ClickPipeKafkaSchemaRegistryCredentials,
        ClickPipeMutateKafkaSchemaRegistry, ClickPipePostKafkaSource,
        ClickPipePostKafkaSourceAuthentication,
    };

    let authentication: ClickPipePostKafkaSourceAuthentication = match args.auth.as_deref() {
        Some(a) => parse_enum(a)?,
        None => ClickPipePostKafkaSourceAuthentication::default(),
    };

    let mtls_cert_contents = match (
        &authentication,
        args.client_certificate.as_deref(),
        args.client_key.as_deref(),
    ) {
        (ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS, Some(cert_path), Some(key_path)) => {
            Some((
                std::fs::read_to_string(cert_path)?,
                std::fs::read_to_string(key_path)?,
            ))
        }
        _ => None,
    };
    let credentials = build_kafka_credentials(&authentication, args, mtls_cert_contents)?;

    let schema_registry = args
        .schema_registry_url
        .as_ref()
        .map(|url| -> Result<_, Box<dyn std::error::Error>> {
            let creds = match (
                args.schema_registry_username.as_deref(),
                args.schema_registry_password.as_deref(),
            ) {
                (Some(u), Some(p)) => ClickPipeKafkaSchemaRegistryCredentials {
                    username: u.to_string(),
                    password: p.to_string(),
                },
                _ => ClickPipeKafkaSchemaRegistryCredentials::default(),
            };
            let ca_cert = match args.schema_registry_ca_certificate.as_deref() {
                Some(path) => Some(std::fs::read_to_string(path)?),
                None => None,
            };
            Ok(ClickPipeMutateKafkaSchemaRegistry {
                url: url.clone(),
                authentication: Default::default(),
                credentials: creds,
                ca_certificate: ca_cert,
            })
        })
        .transpose()?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    Ok(ClickPipePostKafkaSource {
        r#type: parse_enum(&args.kafka_type)?,
        format: parse_enum(&args.format)?,
        brokers: args.brokers.clone(),
        topics: args.topics.clone(),
        consumer_group: args.consumer_group.clone(),
        exactly_once: None,
        authentication,
        credentials,
        iam_role: args.iam_role.clone(),
        offset: Some(ClickPipeKafkaOffset {
            strategy: parse_enum(&args.offset)?,
            timestamp: args.offset_timestamp.clone(),
        }),
        schema_registry,
        ca_certificate: ca_cert_contents,
        reverse_private_endpoint_ids: args.reverse_private_endpoint_ids.clone(),
    })
}

/// Build a `ClickPipePostKinesisSource` from the CLI args. Shared by the
/// `clickpipe create kinesis` and `clickpipe schema-discover <SERVICE_ID> kinesis`
/// handlers.
fn build_kinesis_source(
    args: &crate::cloud::cli::KinesisSourceFields,
) -> Result<clickhouse_cloud_api::models::ClickPipePostKinesisSource, Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostKinesisSource, MskIamUser};

    let access_key = match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
        (Some(k), Some(s)) => Some(MskIamUser {
            access_key_id: k.to_string(),
            secret_key: s.to_string(),
        }),
        _ => None,
    };

    Ok(ClickPipePostKinesisSource {
        format: parse_enum(&args.format)?,
        stream_name: args.stream_name.clone(),
        region: args.region.clone(),
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        access_key,
        use_enhanced_fan_out: if args.enhanced_fan_out {
            Some(true)
        } else {
            None
        },
        iterator_type: parse_enum(&args.iterator_type)?,
        timestamp: args
            .iterator_timestamp
            .map(|t| {
                i64::try_from(t).map_err(|_| format!("--iterator-timestamp {t} is out of range"))
            })
            .transpose()?,
    })
}

pub async fn clickpipe_create_kafka(
    client: &CloudClient,
    args: &crate::cloud::cli::KafkaCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kafka_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kafka: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_kinesis(
    client: &CloudClient,
    args: &crate::cloud::cli::KinesisCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kinesis_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kinesis: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Discover the inferred schema for a Kafka or Kinesis source without creating
/// a ClickPipe (Beta). Side-effect-free, but the API gateway rejects
/// OAuth/Bearer on this POST endpoint, so it is classified as a write command
/// and requires API key auth.
pub async fn clickpipe_schema_discover(
    client: &CloudClient,
    service_id: &str,
    command: &crate::cloud::cli::ClickPipeSchemaDiscoverCommands,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    let source = match command {
        crate::cloud::cli::ClickPipeSchemaDiscoverCommands::Kafka(args) => {
            ClickPipeSchemaDiscoverySource {
                kafka: Some(build_kafka_source(args)?),
                kinesis: None,
            }
        }
        crate::cloud::cli::ClickPipeSchemaDiscoverCommands::Kinesis(args) => {
            ClickPipeSchemaDiscoverySource {
                kafka: None,
                kinesis: Some(build_kinesis_source(args)?),
            }
        }
    };

    let request = ClickPipeSchemaDiscoveryRequest { source };
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client
        .click_pipe_schema_discovery(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Type")]
            r#type: String,
            #[tabled(rename = "Optional")]
            optional: String,
        }
        let rows: Vec<Row> = response
            .fields
            .unwrap_or_default()
            .into_iter()
            .map(|f| Row {
                name: or_absent(f.name),
                r#type: or_absent(f.r#type),
                optional: match f.optional {
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                    None => "".to_string(),
                },
            })
            .collect();
        if rows.is_empty() {
            println!("No fields discovered");
        } else {
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
    }
    Ok(())
}

pub async fn clickpipe_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        print_human(&clickpipe)?;
    }
    Ok(())
}

pub async fn clickpipe_delete(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    client
        .delete_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::json!({ "deleted": clickpipe_id }));
    } else {
        println!("ClickPipe {} deleted", clickpipe_id);
    }
    Ok(())
}

pub async fn clickpipe_state(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    command: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand;
    let cmd = match command {
        "start" => ClickPipeStatePatchRequestCommand::Start,
        "stop" => ClickPipeStatePatchRequestCommand::Stop,
        "resync" => ClickPipeStatePatchRequestCommand::Resync,
        other => return Err(format!("Unknown state command: {}", other).into()),
    };
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client
        .change_clickpipe_state(&org_id, service_id, clickpipe_id, cmd)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!(
            "ClickPipe {} {} (state: {})",
            or_absent(clickpipe.name.as_deref()),
            command,
            or_absent(clickpipe.state.as_ref())
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn clickpipe_scale(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    replicas: Option<u32>,
    cpu_millicores: Option<u32>,
    memory_gb: Option<f64>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = clickhouse_cloud_api::models::ClickPipeScalingPatchRequest {
        replicas: replicas.map(i64::from),
        replica_cpu_millicores: cpu_millicores.map(i64::from),
        replica_memory_gb: memory_gb,
        #[cfg(feature = "deprecated-fields")]
        concurrency: None,
    };
    let clickpipe = client
        .update_clickpipe_scaling(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        let scaling = clickpipe.scaling.unwrap_or_default();
        println!(
            "ClickPipe {} scaling updated",
            or_absent(clickpipe.name.as_deref())
        );
        println!("  Replicas: {}", or_absent(scaling.replicas));
        println!("  CPU: {}m", or_absent(scaling.replica_cpu_millicores));
        println!("  Memory: {} GB", or_absent(scaling.replica_memory_gb));
    }
    Ok(())
}

pub async fn clickpipe_settings_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let settings = client
        .get_clickpipe_settings(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        print_human(&settings)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn clickpipe_settings_update(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    streaming_max_insert_wait_ms: Option<u32>,
    object_storage_concurrency: Option<u32>,
    object_storage_polling_interval_ms: Option<u32>,
    object_storage_max_insert_bytes: Option<u64>,
    object_storage_max_file_count: Option<u32>,
    clickhouse_max_threads: Option<u32>,
    clickhouse_max_insert_threads: Option<u32>,
    object_storage_use_cluster_function: Option<bool>,
    clickhouse_parallel_view_processing: Option<bool>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
        streaming_max_insert_wait_ms: streaming_max_insert_wait_ms.map(i64::from),
        object_storage_concurrency: object_storage_concurrency.map(i64::from),
        object_storage_polling_interval_ms: object_storage_polling_interval_ms.map(i64::from),
        object_storage_max_insert_bytes: object_storage_max_insert_bytes.map(|v| v as i64),
        object_storage_max_file_count: object_storage_max_file_count.map(i64::from),
        clickhouse_max_threads: clickhouse_max_threads.map(i64::from),
        clickhouse_max_insert_threads: clickhouse_max_insert_threads.map(i64::from),
        object_storage_use_cluster_function,
        clickhouse_parallel_view_processing,
        clickhouse_max_download_threads: None,
        clickhouse_min_insert_block_size_bytes: None,
        clickhouse_parallel_distributed_insert_select: None,
    };
    let settings = client
        .update_clickpipe_settings(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        println!("ClickPipe settings updated");
        let value = serde_json::to_value(&settings)?;
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                if !val.is_null() {
                    println!("  {}: {}", key, val);
                }
            }
        }
    }
    Ok(())
}

/// Parse a CLI string into a library enum. Library enums have a
/// `#[serde(untagged)] Unknown(String)` variant so unknown inputs are
/// forwarded to the API (which returns the canonical validation error).
fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| format!("invalid value '{}': {}", s, e))
}

/// Parse `name:type` column specifications into library destination columns.
fn parse_columns(
    columns: &[String],
) -> Result<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>, String> {
    columns
        .iter()
        .map(|col| {
            let (name, col_type) = col
                .split_once(':')
                .ok_or_else(|| format!("Invalid column format '{}': expected name:type", col))?;
            Ok(clickhouse_cloud_api::models::ClickPipeDestinationColumn {
                name: name.to_string(),
                r#type: col_type.to_string(),
            })
        })
        .collect()
}

/// Build a managed-table destination with the default MergeTree engine.
fn build_destination(
    database: &str,
    table: &str,
    columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
) -> clickhouse_cloud_api::models::ClickPipeMutateDestination {
    // Database pipes (Postgres/MySQL/BigQuery) carry the destination table on
    // the per-mapping `targetTable` and reject any of {table, managedTable,
    // tableDefinition, columns} at the top level. Detect that case via empty
    // `table` and emit a destination with only `database` populated.
    if table.is_empty() {
        return clickhouse_cloud_api::models::ClickPipeMutateDestination {
            database: database.to_string(),
            ..Default::default()
        };
    }
    clickhouse_cloud_api::models::ClickPipeMutateDestination {
        database: database.to_string(),
        table: Some(table.to_string()),
        columns,
        managed_table: Some(true),
        roles: None,
        table_definition: Some(
            clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition::default(),
        ),
    }
}

/// Read a GCP service-account JSON key file from disk and return the
/// base64-encoded contents. Used by both the object-storage and BigQuery
/// `create` handlers — the upstream API wants the encoded blob regardless
/// of which source it ends up on.
fn read_gcp_service_account_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        contents.as_bytes(),
    ))
}

/// Print the standard "created" confirmation for any create_* handler.
fn print_created(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", or_absent(clickpipe.name.as_deref()));
        println!("  ID: {}", or_absent(clickpipe.id.as_ref()));
        println!("  State: {}", or_absent(clickpipe.state.as_ref()));
    }
    Ok(())
}

/// Parse `schema.table:target_table` mappings into (schema, table, target) tuples.
/// Source-specific handlers map these into their own TableMapping struct.
fn parse_db_table_mappings(mappings: &[String]) -> Result<Vec<(String, String, String)>, String> {
    mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected schema.table:target_table",
                    m
                )
            })?;
            let (schema, table) = source
                .split_once('.')
                .ok_or_else(|| format!("Invalid source '{}': expected schema.table", source))?;
            Ok((schema.to_string(), table.to_string(), target.to_string()))
        })
        .collect()
}

pub async fn clickpipe_create_postgres(
    client: &CloudClient,
    args: &crate::cloud::cli::PostgresCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutatePostgresSource, ClickPipePostRequest, ClickPipePostSource,
        ClickPipePostgresPipeSettings, ClickPipePostgresPipeTableMapping, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let mappings = parse_db_table_mappings(&args.table_mappings)?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let pg_mappings = mappings
        .into_iter()
        .map(|(schema, t, target)| ClickPipePostgresPipeTableMapping {
            source_schema_name: schema,
            source_table: t,
            target_table: target,
            ..Default::default()
        })
        .collect();

    let source = ClickPipeMutatePostgresSource {
        r#type: Some(parse_enum(&args.postgres_type)?),
        credentials: PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        },
        host: args.host.clone(),
        port: i64::from(args.port),
        database: args.pg_database.clone(),
        disable_tls: false,
        skip_cert_verification: false,
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        settings: ClickPipePostgresPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            publication_name: args.publication_name.clone(),
            replication_slot_name: args.replication_slot_name.clone(),
            ..Default::default()
        },
        table_mappings: pg_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            postgres: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_mysql(
    client: &CloudClient,
    args: &crate::cloud::cli::MySqlCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutateMySQLSource, ClickPipeMySQLPipeSettings, ClickPipeMySQLPipeTableMapping,
        ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let mappings = parse_db_table_mappings(&args.table_mappings)?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let mysql_mappings = mappings
        .into_iter()
        .map(|(schema, t, target)| ClickPipeMySQLPipeTableMapping {
            source_schema_name: schema,
            source_table: t,
            target_table: target,
            ..Default::default()
        })
        .collect();

    let source = ClickPipeMutateMySQLSource {
        r#type: Some(parse_enum(&args.mysql_type)?),
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        host: args.host.clone(),
        port: i64::from(args.port),
        authentication: Some(parse_enum(&args.auth)?),
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: if args.skip_cert_verification {
            Some(true)
        } else {
            None
        },
        server_id: args.server_id.map(|v| v as i64),
        settings: ClickPipeMySQLPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            replication_mechanism: Some(parse_enum(&args.replication_mechanism)?),
            ..Default::default()
        },
        table_mappings: mysql_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mysql: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_mongodb(
    client: &CloudClient,
    args: &crate::cloud::cli::MongoDbCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMongoDBPipeSettings, ClickPipeMongoDBPipeTableMapping,
        ClickPipeMutateMongoDBSource, ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;

    // MongoDB uses `database.collection:target_table` format.
    let mongo_mappings: Vec<ClickPipeMongoDBPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected database.collection:target_table",
                    m
                )
            })?;
            let (db, collection) = source.split_once('.').ok_or_else(|| {
                format!("Invalid source '{}': expected database.collection", source)
            })?;
            Ok(ClickPipeMongoDBPipeTableMapping {
                source_database_name: db.to_string(),
                source_collection: collection.to_string(),
                target_table: target.to_string(),
                table_engine: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let source = ClickPipeMutateMongoDBSource {
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        uri: args.uri.clone(),
        read_preference: parse_enum(&args.read_preference)?,
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: None,
        settings: ClickPipeMongoDBPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            ..Default::default()
        },
        table_mappings: mongo_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mongodb: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_bigquery(
    client: &CloudClient,
    args: &crate::cloud::cli::BigQueryCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeBigQueryPipeSettings, ClickPipeBigQueryPipeTableMapping,
        ClickPipeMutateBigQuerySource, ClickPipePostRequest, ClickPipePostSource, ServiceAccount,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let sa_b64 = read_gcp_service_account_file(&args.service_account_file)?;

    // BigQuery uses `dataset.table:target_table` format.
    let bq_mappings: Vec<ClickPipeBigQueryPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected dataset.table:target_table",
                    m
                )
            })?;
            let (dataset, t) = source
                .split_once('.')
                .ok_or_else(|| format!("Invalid source '{}': expected dataset.table", source))?;
            Ok(ClickPipeBigQueryPipeTableMapping {
                source_dataset_name: dataset.to_string(),
                source_table: t.to_string(),
                target_table: target.to_string(),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let source = ClickPipeMutateBigQuerySource {
        credentials: ServiceAccount {
            service_account_file: sa_b64,
        },
        snapshot_staging_path: args.staging_path.clone(),
        settings: ClickPipeBigQueryPipeSettings {
            replication_mode: parse_enum("snapshot")?,
            ..Default::default()
        },
        table_mappings: bq_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            bigquery: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn backup_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let backups = client.list_backups(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backups)?);
    } else {
        if backups.is_empty() {
            println!("No backups found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Status")]
            status: String,
            #[tabled(rename = "Size")]
            size: String,
            #[tabled(rename = "Created")]
            created: String,
        }
        let rows: Vec<Row> = backups
            .into_iter()
            .map(|b| Row {
                id: or_absent(b.id),
                status: or_absent(b.status.as_ref()),
                size: or_absent(b.size_in_bytes.map(format_bytes)),
                created: or_absent(b.started_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn backup_get(
    client: &CloudClient,
    service_id: &str,
    backup_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let backup = client.get_backup(&org_id, service_id, backup_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backup)?);
    } else {
        print_human(&backup)?;
    }
    Ok(())
}

pub fn auth_interactive() -> Result<(), Box<dyn std::error::Error>> {
    print!("API Key: ");
    std::io::stdout().flush()?;
    let mut api_key = String::new();
    std::io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        return Err("API key cannot be empty".into());
    }

    print!("API Secret: ");
    std::io::stdout().flush()?;
    let api_secret = rpassword::read_password()?;

    if api_secret.is_empty() {
        return Err("API secret cannot be empty".into());
    }

    let mut creds = credentials::load_credentials().unwrap_or_default();
    creds.api_key = Some(api_key);
    creds.api_secret = Some(api_secret);
    credentials::save_credentials(&creds)?;

    println!(
        "Credentials saved to {}",
        credentials::credentials_path().display()
    );
    Ok(())
}

pub async fn service_update(
    client: &CloudClient,
    service_id: &str,
    opts: ServiceUpdateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input before any network call so typos like --release-channel turbo
    // fail locally instead of on the /organizations lookup.
    let request = build_update_service_request(&opts)?;
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let svc = client.update_service(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} updated", or_absent(svc.name.as_deref()));
        println!("  ID: {}", or_absent(svc.id));
        println!("  State: {}", or_absent(svc.state.as_ref()));
    }
    Ok(())
}

#[derive(Default)]
pub struct ServiceScaleOptions {
    pub min_replica_memory_gb: Option<u32>,
    pub max_replica_memory_gb: Option<u32>,
    pub num_replicas: Option<u32>,
    pub min_replicas: Option<u32>,
    pub max_replicas: Option<u32>,
    pub autoscaling_mode: Option<String>,
    pub idle_scaling: Option<bool>,
    pub idle_timeout_minutes: Option<u32>,
    pub org_id: Option<String>,
}

fn build_service_scale_request(
    opts: &ServiceScaleOptions,
) -> Result<ServiceReplicaScalingPatchRequest, Box<dyn std::error::Error>> {
    let horizontal = resolve_horizontal_autoscaling(
        opts.autoscaling_mode.as_deref(),
        opts.min_replicas,
        opts.max_replicas,
    )?;

    Ok(ServiceReplicaScalingPatchRequest {
        autoscaling_mode: horizontal.autoscaling_mode,
        min_replica_memory_gb: opts.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: opts.max_replica_memory_gb.map(f64::from),
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
        num_replicas: opts.num_replicas.map(i64::from),
        idle_scaling: opts.idle_scaling,
        idle_timeout_minutes: opts.idle_timeout_minutes.map(f64::from),
    })
}

pub async fn service_scale(
    client: &CloudClient,
    service_id: &str,
    opts: ServiceScaleOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = build_service_scale_request(&opts)?;
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let svc = client
        .update_replica_scaling(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} scaling updated", or_absent(svc.name.as_deref()));
        println!(
            "  Autoscaling Mode: {}",
            or_absent(svc.autoscaling_mode.as_ref())
        );
        match svc.autoscaling_mode {
            Some(AutoscalingMode::Horizontal) => {
                println!("  Min Replicas: {}", or_absent(svc.min_replicas));
                println!("  Max Replicas: {}", or_absent(svc.max_replicas));
                println!("  Memory/Replica: {} GB", or_absent(svc.replica_memory_gb));
            }
            Some(AutoscalingMode::Vertical) => {
                println!(
                    "  Min Memory/Replica: {} GB",
                    or_absent(svc.min_replica_memory_gb)
                );
                println!(
                    "  Max Memory/Replica: {} GB",
                    or_absent(svc.max_replica_memory_gb)
                );
                println!("  Replicas: {}", or_absent(svc.num_replicas));
            }
            // A mode this CLI version doesn't know, or one the API did not
            // return; don't guess which fields apply.
            _ => {}
        }
    }
    Ok(())
}

pub async fn service_reset_password(
    client: &CloudClient,
    service_id: &str,
    opts: ServiceResetPasswordOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_service_password_patch_request(&opts);
    let resp = client.reset_password(&org_id, service_id, &request).await?;

    // Resolve before either output branch, so --json cannot report success
    // over a response that dropped the one-time generated password.
    let outcome =
        resolve_reset_password_outcome(generation_requested(&request), resp.password.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("Password reset for service {}", service_id);
        match outcome {
            ResetPasswordOutcome::Generated(password) => {
                println!("  New password: {}", password)
            }
            ResetPasswordOutcome::HashUpdated => {
                println!("  Password hash updated; no plaintext password returned")
            }
        }
    }
    Ok(())
}

/// What a `service reset-password` response says about the new credential.
#[derive(Debug)]
enum ResetPasswordOutcome<'a> {
    /// The API generated the password; it is returned once and never again.
    Generated(&'a str),
    /// The caller supplied a hash, so the API generates no plaintext password.
    HashUpdated,
}

/// Whether the PATCH body asks the API to generate a password.
///
/// `newPasswordHash` alone decides it: the spec says `newDoubleSha1Hash` "will
/// be ignored and the generated password will be used" when `newPasswordHash`
/// is absent, and that the response carries a password "only if there was no
/// 'newPasswordHash' in the request". So a double-SHA1-only body is still a
/// generation request, and treating it as a hash update would discard the
/// generated password the API rotated to.
fn generation_requested(request: &ServicePasswordPatchRequest) -> bool {
    request.new_password_hash.is_none()
}

/// Resolves what to report from the request mode and the response.
///
/// The mode is read from the request rather than inferred from what came back:
/// only a hash request explains a response without a password, and an absent
/// one on a generation request means the new credential is lost.
fn resolve_reset_password_outcome(
    generation_requested: bool,
    password: Option<&str>,
) -> Result<ResetPasswordOutcome<'_>, Box<dyn std::error::Error>> {
    if !generation_requested {
        return Ok(ResetPasswordOutcome::HashUpdated);
    }
    match password {
        Some(password) => Ok(ResetPasswordOutcome::Generated(password)),
        None => Err(
            "the API response omitted the generated password, so it cannot be shown: the \
                     service password may already have been rotated — run the reset again to get \
                     a password you can use"
                .into(),
        ),
    }
}

pub async fn query_endpoint_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let ep = client.get_query_endpoint(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ep)?);
    } else {
        print_human(&ep)?;
    }
    Ok(())
}

pub async fn query_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    opts: QueryEndpointCreateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_query_endpoint_create_request(&opts);

    let ep = client
        .create_query_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ep)?);
    } else {
        println!("Query endpoint created for service {}", service_id);
        println!("  ID: {}", or_absent(ep.id.as_deref()));
        println!(
            "  Roles: {}",
            or_absent(ep.roles.as_ref().map(|roles| roles.join(", ")))
        );
    }
    Ok(())
}

pub async fn query_endpoint_delete(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let response = client.delete_query_endpoint(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Query endpoint deleted for service {}", service_id);
    }
    Ok(())
}

pub struct ServiceQueryOptions {
    pub name: Option<String>,
    pub id: Option<String>,
    pub query: Option<String>,
    pub queries_file: Option<String>,
    pub database: Option<String>,
    pub format: Option<String>,
    pub json: bool,
    pub org_id: Option<String>,
    pub no_auto_enable: bool,
}

pub async fn service_query(
    client: &CloudClient,
    opts: ServiceQueryOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let sql = read_query_sql(opts.query.as_deref(), opts.queries_file.as_deref())?;

    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let service =
        resolve_service(client, &org_id, opts.name.as_deref(), opts.id.as_deref()).await?;
    // The whole query path is keyed on the service id: prefer the one the API
    // echoed, and fall back to the one the user passed if the response omitted
    // it.
    let service_id = match service.id {
        Some(id) => id.to_string(),
        None => opts
            .id
            .clone()
            .ok_or("the API response is missing the service id")?,
    };
    let service_name = or_absent(service.name.as_deref());

    // An explicit format always wins over agent-triggered JSON mode. Clap
    // rejects an explicit --json together with --format.
    let format = opts.format.unwrap_or_else(|| {
        if opts.json {
            "JSONEachRow".to_string()
        } else {
            default_query_format()
        }
    });

    let response = if client.is_bearer_auth() {
        // OAuth: the Query API authenticates the user's bearer token
        // directly, SQL-console style — the query runs as the user's own
        // cloud identity, with no per-service Query API key and no
        // query-endpoint configuration needed on the service.
        // `--no-auto-enable` is a no-op here since nothing is ever
        // provisioned.
        let run = |wake: bool| {
            client.api().run_query_bearer(
                &service_id,
                &sql,
                opts.database.as_deref(),
                &format,
                wake,
            )
        };
        let result = match run(false).await {
            Err(clickhouse_cloud_api::Error::ServiceIdle) => {
                eprint_waking_service(&service_name);
                run(true).await
            }
            other => other,
        };
        result.map_err(|e| convert_query_error(client, e, &service_name))?
    } else {
        let key = match credentials::get_service_query_key(&service_id) {
            Some(k) => k,
            None if opts.no_auto_enable => {
                return Err(format!(
                    "no stored Query API key for service {service_id}; rerun without --no-auto-enable to auto-provision"
                )
                .into());
            }
            None => {
                eprintln!(
                    "Provisioning Query API endpoint + key for service '{}'...",
                    service_name
                );
                crate::cloud::service_query::ensure_service_query_setup(
                    client,
                    &org_id,
                    &service_id,
                    &service_name,
                )
                .await?
            }
        };

        // The query host normally wakes an idled service on its own for
        // Query API key auth, but handle the wake confirmation here too so
        // both auth paths behave the same if it ever asks.
        let run = |wake: bool| {
            client.api().run_query(
                &service_id,
                &key.key_id,
                &key.key_secret,
                &sql,
                opts.database.as_deref(),
                &format,
                wake,
            )
        };
        let result = match run(false).await {
            Err(clickhouse_cloud_api::Error::ServiceIdle) => {
                eprint_waking_service(&service_name);
                run(true).await
            }
            other => other,
        };
        result.map_err(|e| convert_query_error(client, e, &service_name))?
    };

    use futures_util::StreamExt;
    use std::io::Write as _;
    let mut stream = response.bytes_stream();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut byte_count = 0;
    let mut last_byte = None;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| -> Box<dyn std::error::Error> {
            format!("Failed to read query response: {e}").into()
        })?;
        handle.write_all(&bytes)?;
        byte_count += bytes.len();
        if let Some(last) = bytes.last() {
            last_byte = Some(*last);
        }
    }
    match query_output_completion(&format, byte_count, last_byte) {
        QueryOutputCompletion::None => {}
        QueryOutputCompletion::Newline => handle.write_all(b"\n")?,
        QueryOutputCompletion::Acknowledge => eprintln!("OK"),
    }
    handle.flush()?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum QueryOutputCompletion {
    None,
    Newline,
    Acknowledge,
}

fn query_output_completion(
    format: &str,
    byte_count: usize,
    last_byte: Option<u8>,
) -> QueryOutputCompletion {
    if byte_count == 0 {
        // Keep an empty SELECT or DDL response out of the row stream while
        // still making successful no-output statements visible to the user.
        return QueryOutputCompletion::Acknowledge;
    }
    if query_format_is_binary(format) || last_byte == Some(b'\n') {
        QueryOutputCompletion::None
    } else {
        QueryOutputCompletion::Newline
    }
}

fn query_format_is_binary(format: &str) -> bool {
    matches!(
        format.to_ascii_lowercase().as_str(),
        "arrow"
            | "arrowstream"
            | "avro"
            | "avroconfluent"
            | "bson"
            | "capnproto"
            | "messagepack"
            | "msgpack"
            | "native"
            | "npy"
            | "orc"
            | "parquet"
            | "parquetmetadata"
            | "protobuf"
            | "protobuflist"
            | "protobufsingle"
            | "rawblob"
            | "rowbinary"
            | "rowbinarywithdefaults"
            | "rowbinarywithnames"
            | "rowbinarywithnamesandtypes"
    )
}

/// Stderr notice shown when the query host reports the service is idled and
/// the CLI resends the query with the wake confirmation.
fn eprint_waking_service(service_name: &str) {
    eprintln!("Service '{service_name}' is idle; waking it (this may take a minute)...");
}

/// Map Query API errors to user-facing messages: a stopped service gets a
/// hint to start it (the query host never wakes a stopped service), the
/// rest go through the standard cloud error conversion.
fn convert_query_error(
    client: &CloudClient,
    err: clickhouse_cloud_api::Error,
    service_name: &str,
) -> Box<dyn std::error::Error> {
    match err {
        clickhouse_cloud_api::Error::ServiceStopped => format!(
            "service '{service_name}' is stopped; start it with `clickhousectl cloud service start` and retry"
        )
        .into(),
        other => client.convert_error(other).into(),
    }
}

fn read_query_sql(
    inline: Option<&str>,
    queries_file: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read as _;

    if let Some(q) = inline {
        let trimmed = q.trim();
        if trimmed.is_empty() {
            return Err("--query was empty".into());
        }
        return Ok(q.to_string());
    }

    if let Some(path) = queries_file {
        let mut content = String::new();
        if path == "-" {
            std::io::stdin().read_to_string(&mut content)?;
        } else {
            content = std::fs::read_to_string(path)?;
        }
        if content.trim().is_empty() {
            return Err("queries file was empty".into());
        }
        return Ok(content);
    }

    if std::io::stdin().is_terminal() {
        return Err("no SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.".into());
    }

    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;
    if content.trim().is_empty() {
        return Err("no SQL received on stdin".into());
    }
    Ok(content)
}

fn default_query_format() -> String {
    if std::io::stdout().is_terminal() {
        "PrettyCompact".to_string()
    } else {
        "TabSeparated".to_string()
    }
}

pub async fn private_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    description: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = ServicPrivateEndpointePostRequest {
        id: endpoint_id.to_string(),
        description: description.map(String::from).unwrap_or_default(),
    };

    let ep = client
        .create_private_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ep)?);
    } else {
        println!("Private endpoint created for service {}", service_id);
        println!("  Endpoint ID: {}", or_absent(ep.id.as_deref()));
        println!("  Description: {}", or_absent(ep.description.as_deref()));
    }
    Ok(())
}

pub async fn private_endpoint_get_config(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let config = client
        .get_service_private_endpoint_config(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        print_human(&config)?;
    }
    Ok(())
}

// =============================================================================
// Phase 3 — Org command handlers
// =============================================================================

pub async fn org_update(
    client: &CloudClient,
    org_id: &str,
    opts: OrgUpdateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = build_org_update_request(&opts)?;

    let org = client.update_organization(org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&org)?);
    } else {
        println!(
            "Organization updated: {} ({})",
            or_absent(org.name.as_deref()),
            or_absent(org.id)
        );
    }
    Ok(())
}

pub async fn org_prometheus(
    client: &CloudClient,
    org_id: Option<&str>,
    filtered_metrics: Option<bool>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let prom = client.get_org_prometheus(&org_id, filtered_metrics).await?;
    println!("{}", prom);
    Ok(())
}

pub async fn service_prometheus(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    filtered_metrics: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let prom = client
        .get_service_prometheus(&org_id, service_id, filtered_metrics)
        .await?;
    println!("{}", prom);
    Ok(())
}

pub async fn org_usage(
    client: &CloudClient,
    org_id: Option<&str>,
    from_date: &str,
    to_date: &str,
    filters: &[String],
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let usage = client
        .get_org_usage(&org_id, from_date, to_date, filters)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&usage)?);
    } else {
        println!(
            "Grand Total: {} CHC",
            or_absent(usage.grand_total_chc.map(|total| format!("{total:.2}")))
        );
        let costs = usage.costs.unwrap_or_default();
        if costs.is_empty() {
            println!("No usage cost records found");
            return Ok(());
        }

        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Entity")]
            entity: String,
            #[tabled(rename = "Date")]
            date: String,
            #[tabled(rename = "Total (CHC)")]
            total: String,
        }
        let rows: Vec<Row> = costs
            .iter()
            .map(|cost| Row {
                entity: usage_entity_label(cost.entity_name.as_deref(), cost.entity_id),
                date: or_absent(cost.date.as_deref()),
                total: or_absent(cost.total_chc.map(|total| format!("{total:.2}"))),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

fn usage_entity_label(name: Option<&str>, id: Option<uuid::Uuid>) -> String {
    match (name.filter(|name| !name.is_empty()), id) {
        (Some(name), _) => name.to_string(),
        (None, Some(id)) => format!("{id} (unknown)"),
        (None, None) => ABSENT.to_string(),
    }
}

// =============================================================================
// Phase 4 — Member command handlers
// =============================================================================

pub async fn member_list(
    client: &CloudClient,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let members = client.list_members(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&members)?);
    } else {
        if members.is_empty() {
            println!("No members found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Email")]
            email: String,
            #[tabled(rename = "User ID")]
            user_id: String,
            #[tabled(rename = "Roles")]
            roles: String,
            #[tabled(rename = "Name")]
            name: String,
        }
        let rows: Vec<Row> = members
            .into_iter()
            .map(|m| Row {
                email: or_absent(m.email.as_deref()),
                user_id: or_absent(m.user_id.as_deref()),
                roles: join_absent(m.assigned_roles.as_deref(), |r| {
                    or_absent(r.role_name.as_deref())
                }),
                name: or_absent(m.name.as_deref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn member_get(
    client: &CloudClient,
    user_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let member = client.get_member(&org_id, user_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        print_human(&member)?;
    }
    Ok(())
}

pub async fn member_update(
    client: &CloudClient,
    user_id: &str,
    role_ids: &[String],
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = clickhouse_cloud_api::models::MemberPatchRequest {
        assigned_role_ids: if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
        #[cfg(feature = "deprecated-fields")]
        role: None,
    };

    let member = client.update_member(&org_id, user_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        println!("Member {} updated", or_absent(member.email.as_deref()));
    }
    Ok(())
}

pub async fn member_remove(
    client: &CloudClient,
    user_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let response = client.delete_member(&org_id, user_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Member {} removed", user_id);
    }
    Ok(())
}

// =============================================================================
// Phase 4 — Invitation command handlers
// =============================================================================

pub async fn invitation_list(
    client: &CloudClient,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let invitations = client.list_invitations(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&invitations)?);
    } else {
        if invitations.is_empty() {
            println!("No invitations found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Email")]
            email: String,
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Roles")]
            roles: String,
            #[tabled(rename = "Expires")]
            expires: String,
        }
        let rows: Vec<Row> = invitations
            .into_iter()
            .map(|inv| Row {
                email: or_absent(inv.email.as_deref()),
                id: or_absent(inv.id),
                roles: join_absent(inv.assigned_roles.as_deref(), |r| {
                    or_absent(r.role_name.as_deref())
                }),
                expires: or_absent(inv.expire_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn invitation_create(
    client: &CloudClient,
    email: &str,
    role_ids: &[String],
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = clickhouse_cloud_api::models::InvitationPostRequest {
        email: email.to_string(),
        assigned_role_ids: role_ids.iter().map(|s| s.to_string()).collect(),
        #[cfg(feature = "deprecated-fields")]
        role: None,
    };

    let inv = client.create_invitation(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&inv)?);
    } else {
        println!(
            "Invitation sent to {} ({})",
            or_absent(inv.email.as_deref()),
            or_absent(inv.id)
        );
    }
    Ok(())
}

pub async fn invitation_get(
    client: &CloudClient,
    invitation_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let inv = client.get_invitation(&org_id, invitation_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&inv)?);
    } else {
        print_human(&inv)?;
    }
    Ok(())
}

pub async fn invitation_delete(
    client: &CloudClient,
    invitation_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let response = client.delete_invitation(&org_id, invitation_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Invitation {} deleted", invitation_id);
    }
    Ok(())
}

// =============================================================================
// Phase 5 — API Key command handlers
// =============================================================================

pub async fn key_list(
    client: &CloudClient,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let keys = client.list_api_keys(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&keys)?);
    } else {
        if keys.is_empty() {
            println!("No API keys found");
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
            #[tabled(rename = "Expires")]
            expires: String,
        }
        let rows: Vec<Row> = keys
            .into_iter()
            .map(|k| Row {
                name: or_absent(k.name.as_deref()),
                id: or_absent(k.id),
                state: or_absent(k.state.as_ref()),
                expires: k
                    .expire_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "never".into()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

/// What a `key create` response says about the key's credentials.
#[derive(Debug)]
enum KeyCreateMaterial<'a> {
    /// The API returned the generated pair; it is shown once and never again.
    Generated {
        key_id: &'a str,
        key_secret: &'a str,
    },
    /// The caller supplied pre-hashed credentials, so no pair is generated.
    PreHashed,
}

/// Resolves the credentials to print from the request mode and the response.
///
/// Only a pre-hashed request explains a response without key material, so the
/// mode is read from the request rather than inferred from what came back: an
/// absent `keyId`/`keySecret` on a generated-key request means the one-time
/// secret is lost, which is an error, not a "no key material returned" notice.
fn resolve_key_create_material<'a>(
    pre_hashed: bool,
    key_id: Option<&'a str>,
    key_secret: Option<&'a str>,
    key_name: Option<&str>,
) -> Result<KeyCreateMaterial<'a>, Box<dyn std::error::Error>> {
    if pre_hashed {
        return Ok(KeyCreateMaterial::PreHashed);
    }
    match (key_id, key_secret) {
        (Some(key_id), Some(key_secret)) => Ok(KeyCreateMaterial::Generated { key_id, key_secret }),
        _ => {
            // Name the key when the response did return it, so the user knows
            // which one to look for.
            let named = match key_name {
                Some(name) => format!(" '{}'", name),
                None => String::new(),
            };
            Err(format!(
                "the API response omitted the generated key material, so the one-time key secret \
                 cannot be shown: the key{} may still have been created — list the organization's \
                 keys and delete it if so",
                named
            )
            .into())
        }
    }
}

pub async fn key_create(
    client: &CloudClient,
    opts: KeyCreateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input before any network call so typos like --state broken
    // fail locally instead of on the /organizations lookup.
    let request = build_api_key_create_request(&opts)?;
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let resp = client.create_api_key(&org_id, &request).await?;

    let name = resp.key.as_ref().and_then(|key| key.name.as_deref());
    // Resolve before either output branch, so --json cannot report success
    // over a response that dropped the one-time key material.
    let material = resolve_key_create_material(
        request.hash_data.is_some(),
        resp.key_id.as_deref(),
        resp.key_secret.as_deref(),
        name,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("API key created!");
        println!("  Name: {}", or_absent(name));
        match material {
            KeyCreateMaterial::Generated { key_id, key_secret } => {
                println!("  Key ID: {}", key_id);
                println!("  Key Secret: {}", key_secret);
                println!();
                println!("Save the key secret now — it will not be shown again.");
            }
            KeyCreateMaterial::PreHashed => {
                println!("  Pre-hashed credentials accepted; no generated key material returned");
            }
        }
    }
    Ok(())
}

pub async fn key_get(
    client: &CloudClient,
    key_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let key = client.get_api_key(&org_id, key_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&key)?);
    } else {
        print_human(&key)?;
    }
    Ok(())
}

pub async fn key_update(
    client: &CloudClient,
    key_id: &str,
    opts: KeyUpdateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input before any network call so typos like --state broken
    // fail locally instead of on the /organizations lookup.
    let request = build_api_key_update_request(&opts)?;
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let key = client.update_api_key(&org_id, key_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&key)?);
    } else {
        println!("API key {} updated", or_absent(key.name.as_deref()));
        println!("  ID: {}", or_absent(key.id));
        println!("  State: {}", or_absent(key.state.as_ref()));
    }
    Ok(())
}

pub async fn key_delete(
    client: &CloudClient,
    key_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let response = client.delete_api_key(&org_id, key_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("API key {} deleted", key_id);
    }
    Ok(())
}

// =============================================================================
// Phase 6 — Activity command handlers
// =============================================================================

pub async fn activity_list(
    client: &CloudClient,
    org_id: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let activities = client.list_activities(&org_id, from_date, to_date).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&activities)?);
    } else {
        if activities.is_empty() {
            println!("No activities found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Type")]
            activity_type: String,
            #[tabled(rename = "Created")]
            created: String,
        }
        let rows: Vec<Row> = activities
            .into_iter()
            .map(|a| Row {
                id: or_absent(a.id.as_deref()),
                activity_type: or_absent(a.r#type.as_ref()),
                created: or_absent(a.created_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn activity_get(
    client: &CloudClient,
    activity_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let activity = client.get_activity(&org_id, activity_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&activity)?);
    } else {
        print_human(&activity)?;
    }
    Ok(())
}

// =============================================================================
// Phase 6 — Backup Config command handlers
// =============================================================================

pub async fn backup_config_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let config = client.get_backup_config(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        print_human(&config)?;
    }
    Ok(())
}

pub async fn backup_config_update(
    client: &CloudClient,
    service_id: &str,
    opts: BackupConfigUpdateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_backup_config_update_request(&opts);

    let config = client
        .update_backup_config(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("Backup configuration updated for service {}", service_id);
        println!(
            "  Backup period: {} hours",
            or_absent(config.backup_period_in_hours)
        );
        println!(
            "  Retention: {} hours",
            or_absent(config.backup_retention_period_in_hours)
        );
        println!(
            "  Start time: {}",
            or_absent(config.backup_start_time.as_deref())
        );
    }
    Ok(())
}

fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_endpoint_keeps_the_present_half_of_a_partial_endpoint() {
        let endpoint = |host: Option<&str>, port: Option<i64>| ServiceEndpoint {
            host: host.map(str::to_string),
            port,
            ..Default::default()
        };

        assert_eq!(first_endpoint(None), ABSENT);
        assert_eq!(first_endpoint(Some(&[])), ABSENT);
        assert_eq!(
            first_endpoint(Some(&[endpoint(Some("host"), Some(9440))])),
            "host:9440"
        );
        assert_eq!(
            first_endpoint(Some(&[endpoint(Some("host"), None)])),
            "host"
        );
        assert_eq!(
            first_endpoint(Some(&[endpoint(None, Some(9440))])),
            format!("{ABSENT}:9440")
        );
        assert_eq!(first_endpoint(Some(&[endpoint(None, None)])), ABSENT);
    }

    #[test]
    fn service_query_hint_is_dropped_when_the_id_is_absent() {
        assert_eq!(service_query_hint(None), None);

        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        let hint = service_query_hint(Some(id)).unwrap();
        assert!(
            hint.contains("--id a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"),
            "hint should name the service: {hint}"
        );
        assert!(hint.contains("provisioned automatically on first use"));
    }

    #[test]
    fn service_credentials_block_shows_the_password_the_api_sent() {
        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        assert_eq!(
            service_credentials_block(Some("s3cret"), Some(id)),
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: s3cret"
        );
    }

    #[test]
    fn service_credentials_block_treats_an_empty_password_as_sent() {
        assert_eq!(
            service_credentials_block(Some(""), None),
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: "
        );
    }

    #[test]
    fn service_credentials_block_warns_with_the_reset_command_when_the_password_is_absent() {
        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        let block = service_credentials_block(None, Some(id));
        assert!(
            !block.contains(&format!("Password: {ABSENT}")),
            "an absent password must not render a placeholder credential: {block}"
        );
        assert!(block.starts_with("WARNING: the API response omitted the one-time password"));
        assert!(
            block.contains(
                "clickhousectl cloud service reset-password a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"
            ),
            "the warning should name the exact recovery command: {block}"
        );
    }

    #[test]
    fn service_credentials_block_warns_generically_when_the_service_id_is_absent() {
        let block = service_credentials_block(None, None);
        assert!(block.starts_with("WARNING: the API response omitted the one-time password"));
        assert!(
            block.contains("clickhousectl cloud service reset-password <service-id>"),
            "without an id the warning should stay generic: {block}"
        );
    }

    #[test]
    fn classify_stop_poll_state_fails_on_an_absent_state() {
        let err = classify_stop_poll_state(None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the API response omitted the service state while waiting for the service to stop, \
             so the stop cannot be confirmed"
        );
    }

    #[test]
    fn classify_stop_poll_state_separates_stopped_waiting_and_failed() {
        assert!(classify_stop_poll_state(Some(&ServiceState::Stopped)).unwrap());
        assert!(classify_stop_poll_state(Some(&ServiceState::Idle)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Stopping)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Running)).unwrap());
        // An unrecognized state keeps the loop polling rather than failing.
        assert!(
            !classify_stop_poll_state(Some(&ServiceState::Unknown("hibernating".into()))).unwrap()
        );

        let err = classify_stop_poll_state(Some(&ServiceState::Failed)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "service entered unexpected state 'failed' while waiting for stop"
        );
        // "deleted" is not a typed variant, so it arrives through the catch-all.
        let err =
            classify_stop_poll_state(Some(&ServiceState::Unknown("deleted".into()))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "service entered unexpected state 'deleted' while waiting for stop"
        );
    }

    #[test]
    fn stop_poll_progress_collapses_repeats_but_keeps_transitions() {
        let mut progress = StopPollProgress::default();
        assert_eq!(
            progress.render("stopping", false).as_deref(),
            Some("  state: stopping")
        );
        assert_eq!(progress.render("stopping", false), None);
        assert_eq!(
            progress.render("running", false).as_deref(),
            Some("  state: running")
        );
        assert_eq!(
            progress.render("stopped", false).as_deref(),
            Some("  state: stopped")
        );
    }

    #[test]
    fn stop_poll_progress_keeps_repeats_in_verbose_mode() {
        let mut progress = StopPollProgress::default();
        assert!(progress.render("stopping", true).is_some());
        assert!(progress.render("stopping", true).is_some());
    }

    #[test]
    fn query_output_completion_acknowledges_empty_responses() {
        assert_eq!(
            query_output_completion("TabSeparated", 0, None),
            QueryOutputCompletion::Acknowledge
        );
        assert_eq!(
            query_output_completion("RowBinary", 0, None),
            QueryOutputCompletion::Acknowledge
        );
    }

    #[test]
    fn query_output_completion_adds_only_a_missing_text_newline() {
        assert_eq!(
            query_output_completion("TabSeparated", 2, Some(b'K')),
            QueryOutputCompletion::Newline
        );
        assert_eq!(
            query_output_completion("JSONEachRow", 2, Some(b'\n')),
            QueryOutputCompletion::None
        );
    }

    #[test]
    fn query_output_completion_never_changes_binary_bodies() {
        for format in ["RowBinary", "Native", "Parquet", "ArrowStream", "MsgPack"] {
            assert_eq!(
                query_output_completion(format, 3, Some(0)),
                QueryOutputCompletion::None,
                "{format} output must stay byte-for-byte intact"
            );
        }
    }

    #[test]
    fn usage_entity_label_distinguishes_named_unknown_and_absent_entities() {
        let id = uuid::Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(
            usage_entity_label(Some("production"), Some(id)),
            "production"
        );
        assert_eq!(
            usage_entity_label(None, Some(id)),
            "11111111-2222-3333-4444-555555555555 (unknown)"
        );
        assert_eq!(
            usage_entity_label(Some(""), Some(id)),
            format!("{id} (unknown)")
        );
        assert_eq!(usage_entity_label(None, None), ABSENT);
    }

    #[test]
    fn service_delete_error_suggests_force_for_a_running_service() {
        let error = CloudError::new(
            "CONFLICT: Only instance in one of the following states can be terminated. \
             Current state: 'running'",
        );

        let error = service_delete_error(error, false, "svc-1");

        assert_eq!(
            error.message,
            "service is running and cannot be deleted. Use --force to stop it first, or \
             `clickhousectl cloud service stop svc-1`."
        );
    }

    #[test]
    fn service_delete_error_preserves_unrelated_and_forced_failures() {
        let unrelated = CloudError::new("CONFLICT: service has dependent resources");
        assert_eq!(
            service_delete_error(unrelated, false, "svc-1").message,
            "CONFLICT: service has dependent resources"
        );

        let forced = CloudError::new("CONFLICT: Current state: 'running'");
        assert_eq!(
            service_delete_error(forced, true, "svc-1").message,
            "CONFLICT: Current state: 'running'"
        );
    }

    #[test]
    fn resolve_key_create_material_returns_the_generated_pair() {
        let material =
            resolve_key_create_material(false, Some("key-id"), Some("key-secret"), Some("ci"))
                .unwrap();
        match material {
            KeyCreateMaterial::Generated { key_id, key_secret } => {
                assert_eq!(key_id, "key-id");
                assert_eq!(key_secret, "key-secret");
            }
            KeyCreateMaterial::PreHashed => panic!("expected the generated pair"),
        }
    }

    #[test]
    fn resolve_key_create_material_reports_pre_hashed_regardless_of_response() {
        assert!(matches!(
            resolve_key_create_material(true, None, None, Some("ci")).unwrap(),
            KeyCreateMaterial::PreHashed
        ));
        // The mode comes from the request, so echoed material does not change it.
        assert!(matches!(
            resolve_key_create_material(true, Some("key-id"), None, None).unwrap(),
            KeyCreateMaterial::PreHashed
        ));
    }

    #[test]
    fn resolve_key_create_material_fails_when_generated_material_is_absent() {
        let err = resolve_key_create_material(false, None, Some("key-secret"), Some("ci"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("omitted the generated key material"),
            "unexpected error: {err}"
        );
        assert!(err.contains("the key 'ci' may still have been created"));

        // An empty string is material the API did send, so it is not absent.
        let material = resolve_key_create_material(false, Some(""), Some(""), Some("ci")).unwrap();
        assert!(matches!(material, KeyCreateMaterial::Generated { .. }));

        // Without a name the message stays grammatical.
        let err = resolve_key_create_material(false, Some("key-id"), None, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("the key may still have been created"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn generation_is_requested_unless_a_password_hash_is_sent() {
        let request = |new_password_hash: Option<&str>, new_double_sha1_hash: Option<&str>| {
            ServicePasswordPatchRequest {
                new_password_hash: new_password_hash.map(str::to_string),
                new_double_sha1_hash: new_double_sha1_hash.map(str::to_string),
            }
        };
        assert!(generation_requested(&request(None, None)));
        // The API ignores `newDoubleSha1Hash` without `newPasswordHash` and
        // generates a password anyway, so this is still a generation request:
        // reporting "hash updated" would discard the new credential.
        assert!(generation_requested(&request(None, Some("sha1"))));
        assert!(!generation_requested(&request(Some("sha256"), None)));
        assert!(!generation_requested(&request(
            Some("sha256"),
            Some("sha1")
        )));
    }

    #[test]
    fn resolve_reset_password_outcome_returns_the_generated_password() {
        match resolve_reset_password_outcome(true, Some("s3cret")).unwrap() {
            ResetPasswordOutcome::Generated(password) => assert_eq!(password, "s3cret"),
            ResetPasswordOutcome::HashUpdated => panic!("expected the generated password"),
        }

        // An empty string is a password the API did send, so it is not absent.
        assert!(matches!(
            resolve_reset_password_outcome(true, Some("")).unwrap(),
            ResetPasswordOutcome::Generated("")
        ));
    }

    #[test]
    fn resolve_reset_password_outcome_reports_hash_updated_regardless_of_response() {
        assert!(matches!(
            resolve_reset_password_outcome(false, None).unwrap(),
            ResetPasswordOutcome::HashUpdated
        ));
        // The mode comes from the request, so an echoed password does not
        // turn a hash reset into a generated one.
        assert!(matches!(
            resolve_reset_password_outcome(false, Some("s3cret")).unwrap(),
            ResetPasswordOutcome::HashUpdated
        ));
    }

    #[test]
    fn resolve_reset_password_outcome_fails_when_the_generated_password_is_absent() {
        let err = resolve_reset_password_outcome(true, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("omitted the generated password"),
            "unexpected error: {err}"
        );
        assert!(
            err.contains("may already have been rotated") && err.contains("run the reset again"),
            "unexpected error: {err}"
        );
    }

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
    fn build_create_service_request_supports_ga_optional_fields() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            num_replicas: Some(3),
            min_replicas: None,
            max_replicas: None,
            autoscaling_mode: None,
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(10),
            ip_allow: vec!["10.0.0.0/8".to_string()],
            backup_id: Some("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6".to_string()),
            release_channel: Some("fast".to_string()),
            data_warehouse_id: Some("dw-1".to_string()),
            is_readonly: true,
            encryption_key: Some("key-1".to_string()),
            encryption_role: Some("role-1".to_string()),
            enable_tde: true,
            compliance_type: Some("hipaa".to_string()),
            profile: Some("v1-default".to_string()),
            tags: vec!["env=prod".to_string()],
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec![],
            private_preview_terms_checked: true,
            enable_core_dumps: Some(true),
            org_id: None,
        };

        let request = build_create_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["tags"][0]["key"], "env");
        assert_eq!(json["endpoints"][0]["protocol"], "mysql");
        assert_eq!(json["privatePreviewTermsChecked"], true);
        assert_eq!(json["enableCoreDumps"], true);
        // Fields not exposed in CLI are omitted from the JSON
        assert!(json.get("byocId").is_none());
    }

    #[test]
    fn build_create_service_request_trims_tag_keys() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replica_memory_gb: None,
            max_replica_memory_gb: None,
            num_replicas: None,
            min_replicas: None,
            max_replicas: None,
            autoscaling_mode: None,
            idle_scaling: None,
            idle_timeout_minutes: None,
            ip_allow: vec![],
            backup_id: None,
            release_channel: None,
            data_warehouse_id: None,
            is_readonly: false,
            encryption_key: None,
            encryption_role: None,
            enable_tde: false,
            compliance_type: None,
            profile: None,
            tags: vec![" env =prod".to_string()],
            enable_endpoints: vec![],
            disable_endpoints: vec![],
            private_preview_terms_checked: false,
            enable_core_dumps: None,
            org_id: None,
        };

        let request = build_create_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["tags"][0]["key"], "env");
        assert_eq!(json["tags"][0]["value"], "prod");
    }

    #[test]
    fn build_create_service_request_rejects_empty_tag_keys() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replica_memory_gb: None,
            max_replica_memory_gb: None,
            num_replicas: None,
            min_replicas: None,
            max_replicas: None,
            autoscaling_mode: None,
            idle_scaling: None,
            idle_timeout_minutes: None,
            ip_allow: vec![],
            backup_id: None,
            release_channel: None,
            data_warehouse_id: None,
            is_readonly: false,
            encryption_key: None,
            encryption_role: None,
            enable_tde: false,
            compliance_type: None,
            profile: None,
            tags: vec!["=prod".to_string()],
            enable_endpoints: vec![],
            disable_endpoints: vec![],
            private_preview_terms_checked: false,
            enable_core_dumps: None,
            org_id: None,
        };

        let err = build_create_service_request(&opts).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag '=prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_update_service_request_supports_patch_fields() {
        let opts = ServiceUpdateOptions {
            name: Some("updated".to_string()),
            add_ip_allow: vec!["10.0.0.0/8".to_string()],
            remove_ip_allow: vec!["0.0.0.0/0".to_string()],
            add_private_endpoint_ids: vec!["pe-1".to_string()],
            remove_private_endpoint_ids: vec!["pe-2".to_string()],
            release_channel: Some("default".to_string()),
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec![],
            transparent_data_encryption_key_id: Some("tde-1".to_string()),
            add_tags: vec!["env=staging".to_string()],
            remove_tags: vec!["old=tag".to_string()],
            enable_core_dumps: Some(false),
            org_id: None,
        };

        let request = build_update_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["ipAccessList"]["add"][0]["source"], "10.0.0.0/8");
        assert_eq!(json["ipAccessList"]["remove"][0]["source"], "0.0.0.0/0");
        assert_eq!(json["privateEndpointIds"]["add"][0], "pe-1");
        assert_eq!(json["privateEndpointIds"]["remove"][0], "pe-2");
        assert!(json["tags"].is_object());
        assert_eq!(json["tags"]["add"][0]["key"], "env");
        assert_eq!(json["tags"]["remove"][0]["key"], "old");
        assert_eq!(json["transparentDataEncryptionKeyId"], "tde-1");
        assert_eq!(json["enableCoreDumps"], false);
    }

    #[test]
    fn build_update_service_request_rejects_empty_tag_keys() {
        let opts = ServiceUpdateOptions {
            name: None,
            add_ip_allow: vec![],
            remove_ip_allow: vec![],
            add_private_endpoint_ids: vec![],
            remove_private_endpoint_ids: vec![],
            release_channel: None,
            enable_endpoints: vec![],
            disable_endpoints: vec![],
            transparent_data_encryption_key_id: None,
            add_tags: vec![" =prod".to_string()],
            remove_tags: vec![],
            enable_core_dumps: None,
            org_id: None,
        };

        let err = build_update_service_request(&opts).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag ' =prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_api_key_requests_support_hashes_and_ip_allowlists() {
        let create_opts = KeyCreateOptions {
            name: "ci-key".to_string(),
            role_ids: vec!["a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6".to_string()],
            expires_at: Some("2025-12-31T23:59:59Z".to_string()),
            state: Some("enabled".to_string()),
            ip_allow: vec!["10.0.0.0/8".to_string()],
            hash_key_id: Some("id-hash".to_string()),
            hash_key_id_suffix: Some("abcd".to_string()),
            hash_key_secret: Some("secret-hash".to_string()),
            org_id: None,
        };
        let create_request = build_api_key_create_request(&create_opts).unwrap();
        let create_json = serde_json::to_value(&create_request).unwrap();
        assert_eq!(create_json["hashData"]["keyIdHash"], "id-hash");
        assert_eq!(create_json["ipAccessList"][0]["source"], "10.0.0.0/8");
        assert_eq!(
            create_json["assignedRoleIds"][0],
            "a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"
        );

        let update_opts = KeyUpdateOptions {
            name: Some("renamed".to_string()),
            role_ids: vec!["a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6".to_string()],
            expires_at: Some("2025-01-01T00:00:00Z".to_string()),
            state: Some("disabled".to_string()),
            ip_allow: vec!["0.0.0.0/0".to_string()],
            org_id: None,
        };
        let update_request = build_api_key_update_request(&update_opts).unwrap();
        let update_json = serde_json::to_value(&update_request).unwrap();
        assert_eq!(update_json["expireAt"], "2025-01-01T00:00:00Z");
        assert_eq!(update_json["state"], "disabled");
        assert_eq!(update_json["ipAccessList"][0]["source"], "0.0.0.0/0");
    }

    #[test]
    fn build_api_key_create_request_rejects_invalid_uuid() {
        let opts = KeyCreateOptions {
            name: "ci-key".to_string(),
            role_ids: vec!["not-a-uuid".to_string()],
            ..Default::default()
        };
        let err = build_api_key_create_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("not-a-uuid"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_api_key_create_request_rejects_invalid_expire_at() {
        let opts = KeyCreateOptions {
            name: "ci-key".to_string(),
            expires_at: Some("next-tuesday".to_string()),
            ..Default::default()
        };
        let err = build_api_key_create_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("next-tuesday"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_org_and_backup_config_requests_match_tested_shapes() {
        let org_opts = OrgUpdateOptions {
            name: Some("Updated Org".to_string()),
            remove_private_endpoints: vec![
                "pe-1,description=old,cloud-provider=aws,region=us-east-1".to_string(),
            ],
            enable_core_dumps: Some(false),
        };
        let org_request = build_org_update_request(&org_opts).unwrap();
        let org_json = serde_json::to_value(&org_request).unwrap();
        assert_eq!(org_json["privateEndpoints"]["remove"][0]["id"], "pe-1");
        assert_eq!(
            org_json["privateEndpoints"]["remove"][0]["cloudProvider"],
            "aws"
        );
        assert_eq!(org_json["enableCoreDumps"], false);

        let backup_opts = BackupConfigUpdateOptions {
            backup_period_hours: Some(12),
            backup_retention_period_hours: Some(336),
            backup_start_time: Some("03:00".to_string()),
            org_id: None,
        };
        let backup_request = build_backup_config_update_request(&backup_opts);
        let backup_json = serde_json::to_value(&backup_request).unwrap();
        assert_eq!(backup_json["backupPeriodInHours"], 12.0);
        assert_eq!(backup_json["backupRetentionPeriodInHours"], 336.0);
        assert_eq!(backup_json["backupStartTime"], "03:00");
    }

    // Regression tests: invalid enum values must be rejected by build_* functions
    // before any network call (resolve_org_id). See issue #101.

    #[test]
    fn build_create_service_request_rejects_invalid_provider() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "awss".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("awss"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_create_service_request_rejects_invalid_region() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-99".to_string(),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("us-east-99"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_create_service_request_rejects_invalid_release_channel() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            release_channel: Some("turbo".to_string()),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("turbo"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_create_service_request_horizontal_autoscaling_on_wire() {
        // Maximal: explicit --autoscaling-mode horizontal + min/max replicas.
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..Default::default()
        };
        let request = build_create_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["autoscalingMode"], "horizontal");
        assert_eq!(json["minReplicas"], 2);
        assert_eq!(json["maxReplicas"], 8);
        assert!(json["minReplicas"].is_i64());
        assert!(json["maxReplicas"].is_i64());
        // Vertical fields stay absent.
        assert!(json.get("numReplicas").is_none());
        assert!(json.get("minReplicaMemoryGb").is_none());
        assert!(json.get("maxReplicaMemoryGb").is_none());
    }

    #[test]
    fn build_create_service_request_replica_pair_without_mode_omits_mode() {
        // No explicit --autoscaling-mode: the replica pair passes through with
        // the mode absent. The API resolves an omitted mode itself — an equal
        // band is accepted as a vertical fixed replica count without the
        // horizontal entitlement, so the CLI must not inject "horizontal".
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replicas: Some(1),
            max_replicas: Some(4),
            ..Default::default()
        };
        let request = build_create_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("autoscalingMode").is_none());
        assert_eq!(json["minReplicas"], 1);
        assert_eq!(json["maxReplicas"], 4);
    }

    #[test]
    fn build_create_service_request_vertical_omits_horizontal_fields() {
        // Minimal: vertical-only usage leaves autoscalingMode/replicas absent.
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            num_replicas: Some(3),
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            ..Default::default()
        };
        let request = build_create_service_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("autoscalingMode").is_none());
        assert!(json.get("minReplicas").is_none());
        assert!(json.get("maxReplicas").is_none());
        assert_eq!(json["numReplicas"], 3);
        assert!(json["numReplicas"].is_i64());
    }

    #[test]
    fn build_create_service_request_rejects_min_without_max_replicas() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replicas: Some(2),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("--min-replicas"),
            "error should guide the user: {}",
            err
        );

        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            max_replicas: Some(8),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("--min-replicas"),
            "error should guide the user: {}",
            err
        );
    }

    #[test]
    fn build_create_service_request_rejects_invalid_autoscaling_mode() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("turbo".to_string()),
            ..Default::default()
        };
        let err = build_create_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("turbo"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn resolve_horizontal_autoscaling_explicit_vertical_with_no_replicas() {
        // Explicit --autoscaling-mode vertical with no replica pair → mode set,
        // replicas absent (lets the server apply vertical defaults).
        let resolved = resolve_horizontal_autoscaling(Some("vertical"), None, None).unwrap();
        assert_eq!(resolved.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert!(resolved.min_replicas.is_none());
        assert!(resolved.max_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_horizontal_autoscaling_on_wire() {
        let opts = ServiceScaleOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..Default::default()
        };
        let request = build_service_scale_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["autoscalingMode"], "horizontal");
        assert_eq!(json["minReplicas"], 2);
        assert_eq!(json["maxReplicas"], 8);
        assert!(json.get("numReplicas").is_none());
        assert!(json.get("minReplicaMemoryGb").is_none());
        assert!(json.get("maxReplicaMemoryGb").is_none());
    }

    #[test]
    fn build_service_scale_request_switch_to_vertical_on_wire() {
        // Switching a horizontal service back to vertical sends the mode and
        // the vertical fields in one request.
        let opts = ServiceScaleOptions {
            autoscaling_mode: Some("vertical".to_string()),
            num_replicas: Some(3),
            min_replica_memory_gb: Some(8),
            max_replica_memory_gb: Some(32),
            ..Default::default()
        };
        let request = build_service_scale_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["autoscalingMode"], "vertical");
        assert_eq!(json["numReplicas"], 3);
        assert_eq!(json["minReplicaMemoryGb"], 8.0);
        assert_eq!(json["maxReplicaMemoryGb"], 32.0);
        assert!(json.get("minReplicas").is_none());
        assert!(json.get("maxReplicas").is_none());
    }

    #[test]
    fn build_service_scale_request_switch_to_horizontal_with_memory_on_wire() {
        // Switching to horizontal pins the equal per-replica memory the mode
        // requires in the same request.
        let opts = ServiceScaleOptions {
            autoscaling_mode: Some("horizontal".to_string()),
            min_replicas: Some(2),
            max_replicas: Some(8),
            min_replica_memory_gb: Some(16),
            max_replica_memory_gb: Some(16),
            ..Default::default()
        };
        let request = build_service_scale_request(&opts).unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["autoscalingMode"], "horizontal");
        assert_eq!(json["minReplicas"], 2);
        assert_eq!(json["maxReplicas"], 8);
        assert_eq!(json["minReplicaMemoryGb"], 16.0);
        assert_eq!(json["maxReplicaMemoryGb"], 16.0);
        assert!(json.get("numReplicas").is_none());
    }

    #[test]
    fn build_service_scale_request_rejects_min_without_max_replicas() {
        let opts = ServiceScaleOptions {
            max_replicas: Some(8),
            ..Default::default()
        };
        let err = build_service_scale_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("--min-replicas"),
            "error should guide the user: {}",
            err
        );
    }

    #[test]
    fn build_kinesis_source_rejects_out_of_range_iterator_timestamp() {
        let args = crate::cloud::cli::KinesisSourceFields {
            stream_name: "stream".to_string(),
            region: "us-east-1".to_string(),
            format: "JSONEachRow".to_string(),
            auth: "IAM_ROLE".to_string(),
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            iterator_type: "AT_TIMESTAMP".to_string(),
            iterator_timestamp: Some(u64::MAX),
            enhanced_fan_out: false,
        };
        let err = build_kinesis_source(&args).unwrap_err();
        assert!(
            err.to_string().contains("out of range"),
            "error should mention the range: {}",
            err
        );

        let args = crate::cloud::cli::KinesisSourceFields {
            iterator_timestamp: Some(1_750_000_000),
            ..args
        };
        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.timestamp, Some(1_750_000_000));
    }

    #[test]
    fn build_update_service_request_rejects_invalid_release_channel() {
        let opts = ServiceUpdateOptions {
            release_channel: Some("turbo".to_string()),
            ..Default::default()
        };
        let err = build_update_service_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("turbo"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_api_key_create_request_rejects_invalid_state() {
        let opts = KeyCreateOptions {
            name: "ci-key".to_string(),
            state: Some("broken".to_string()),
            ..Default::default()
        };
        let err = build_api_key_create_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("broken"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_api_key_update_request_rejects_invalid_state() {
        let opts = KeyUpdateOptions {
            state: Some("broken".to_string()),
            ..Default::default()
        };
        let err = build_api_key_update_request(&opts).unwrap_err();
        assert!(
            err.to_string().contains("broken"),
            "error should mention the bad value: {}",
            err
        );
    }

    #[test]
    fn build_password_and_query_endpoint_requests_use_new_fields() {
        let password_request = build_service_password_patch_request(&ServiceResetPasswordOptions {
            new_password_hash: Some("sha256".to_string()),
            new_double_sha1_hash: Some("sha1".to_string()),
            org_id: None,
        });
        let password_json = serde_json::to_value(&password_request).unwrap();
        assert_eq!(password_json["newPasswordHash"], "sha256");
        assert_eq!(password_json["newDoubleSha1Hash"], "sha1");

        let query_request = build_query_endpoint_create_request(&QueryEndpointCreateOptions {
            roles: vec!["admin".to_string()],
            open_api_keys: vec!["key-1".to_string()],
            allowed_origins: Some("https://example.com".to_string()),
            org_id: None,
        });
        let query_json = serde_json::to_value(&query_request).unwrap();
        assert_eq!(query_json["roles"][0], "admin");
        assert_eq!(query_json["openApiKeys"][0], "key-1");
        assert_eq!(query_json["allowedOrigins"], "https://example.com");
    }

    #[test]
    fn parse_db_table_mappings_valid() {
        let mappings = vec![
            "public.users:public_users".to_string(),
            "schema1.orders:schema1_orders".to_string(),
        ];
        let result = super::parse_db_table_mappings(&mappings).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ("public".into(), "users".into(), "public_users".into())
        );
        assert_eq!(
            result[1],
            ("schema1".into(), "orders".into(), "schema1_orders".into())
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_colon() {
        let mappings = vec!["public.users".to_string()];
        let result = super::parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("expected schema.table:target_table")
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_dot() {
        let mappings = vec!["users:target".to_string()];
        let result = super::parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected schema.table"));
    }

    #[test]
    fn parse_db_table_mappings_empty() {
        let mappings: Vec<String> = vec![];
        let result = super::parse_db_table_mappings(&mappings).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_enum_known_variant() {
        use clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceFormat;
        let format: ClickPipePostObjectStorageSourceFormat =
            super::parse_enum("JSONEachRow").unwrap();
        assert_eq!(format, ClickPipePostObjectStorageSourceFormat::JSONEachRow);
    }

    #[test]
    fn parse_enum_unknown_falls_through() {
        // Unknown values map to the catch-all Unknown(String) variant —
        // forwarded to the API which returns the canonical validation error.
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceType;
        let kafka_type: ClickPipePostKafkaSourceType =
            super::parse_enum("not-a-real-type").unwrap();
        assert_eq!(
            kafka_type,
            ClickPipePostKafkaSourceType::Unknown("not-a-real-type".to_string())
        );
    }

    #[test]
    fn parse_enum_preserves_rename_spellings() {
        // Enums use `#[serde(rename = "s3")]` etc. — wire format is authoritative.
        use clickhouse_cloud_api::models::{
            ClickPipePostKafkaSourceAuthentication, ClickPipePostObjectStorageSourceType,
        };
        let ty: ClickPipePostObjectStorageSourceType = super::parse_enum("s3").unwrap();
        assert_eq!(ty, ClickPipePostObjectStorageSourceType::S3);
        let auth: ClickPipePostKafkaSourceAuthentication =
            super::parse_enum("SCRAM-SHA-256").unwrap();
        assert_eq!(auth, ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_256);
    }

    #[test]
    fn parse_columns_valid() {
        let cols = vec!["id:Int64".to_string(), "name:String".to_string()];
        let parsed = super::parse_columns(&cols).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "id");
        assert_eq!(parsed[0].r#type, "Int64");
        assert_eq!(parsed[1].name, "name");
        assert_eq!(parsed[1].r#type, "String");
    }

    #[test]
    fn parse_columns_missing_colon_errors() {
        let cols = vec!["id_without_type".to_string()];
        let err = super::parse_columns(&cols).unwrap_err();
        assert!(err.contains("expected name:type"));
    }

    #[test]
    fn build_destination_uses_defaults_for_table_definition() {
        let dest = super::build_destination("mydb", "events", vec![]);
        assert_eq!(dest.database, "mydb");
        assert_eq!(dest.table.as_deref(), Some("events"));
        assert_eq!(dest.managed_table, Some(true));
        // Default table engine is MergeTree, not something else.
        assert_eq!(
            dest.table_definition
                .as_ref()
                .expect("non-database pipe gets a tableDefinition")
                .engine
                .r#type,
            clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType::MergeTree
        );
    }

    // `build_kafka_credentials` tests — lock the wire shape for each auth mode.
    // Authoritative source: `ClickPipePostKafkaSource.credentials` in
    // `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json`.

    fn kafka_args() -> crate::cloud::cli::KafkaCreateArgs {
        crate::cloud::cli::KafkaCreateArgs {
            service_id: "svc".into(),
            name: "pipe".into(),
            source: crate::cloud::cli::KafkaSourceFields {
                brokers: "b:9092".into(),
                topics: "t".into(),
                format: "JSONEachRow".into(),
                kafka_type: "kafka".into(),
                consumer_group: None,
                auth: None,
                username: None,
                password: None,
                iam_role: None,
                access_key_id: None,
                secret_key: None,
                offset: "from_beginning".into(),
                offset_timestamp: None,
                schema_registry_url: None,
                schema_registry_username: None,
                schema_registry_password: None,
                ca_certificate: None,
                client_certificate: None,
                client_key: None,
                schema_registry_ca_certificate: None,
                reverse_private_endpoint_ids: vec![],
            },
            database: "d".into(),
            table: "t".into(),
            columns: vec![],
            org_id: None,
        }
    }

    #[test]
    fn kafka_credentials_plain_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("PLAIN".into());
        args.source.username = Some("u".into());
        args.source.password = Some("p".into());
        let creds = super::build_kafka_credentials(&Auth::PLAIN, &args.source, None).unwrap();
        assert_eq!(creds["username"], "u");
        assert_eq!(creds["password"], "p");
    }

    #[test]
    fn kafka_credentials_iam_user_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_USER".into());
        args.source.access_key_id = Some("AKIA".into());
        args.source.secret_key = Some("secret".into());
        let creds = super::build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap();
        // MskIamUser wire shape is {accessKeyId, secretKey} — NOT snake_case.
        assert_eq!(creds["accessKeyId"], "AKIA");
        assert_eq!(creds["secretKey"], "secret");
        assert!(creds.get("access_key_id").is_none());
    }

    #[test]
    fn kafka_credentials_iam_role_is_null() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        args.source.iam_role = Some("arn:aws:iam::123:role/Foo".into());
        // IAM_ROLE sends credentials=null; the role ARN flows through the
        // top-level `iamRole` field on the Kafka source, not credentials.
        let creds = super::build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap();
        assert!(creds.is_null());
    }

    #[test]
    fn kafka_credentials_mutual_tls_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let contents = Some(("CERT_PEM".into(), "KEY_PEM".into()));
        let creds =
            super::build_kafka_credentials(&Auth::MUTUAL_TLS, &args.source, contents).unwrap();
        assert_eq!(creds["certificate"], "CERT_PEM");
        assert_eq!(creds["privateKey"], "KEY_PEM");
    }

    #[test]
    fn kafka_credentials_iam_user_missing_args_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let err = super::build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap_err();
        assert!(err.contains("--access-key-id"));
    }

    #[test]
    fn kafka_credentials_iam_role_missing_arn_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        let err = super::build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap_err();
        assert!(err.contains("--iam-role"));
    }
}
