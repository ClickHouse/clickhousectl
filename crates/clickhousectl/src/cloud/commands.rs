use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, Credentials};
use clickhouse_cloud_api::models::{
    ApiKeyPatchRequest, ApiKeyPatchRequestState, ApiKeyPostRequest, ApiKeyPostRequestState,
    BackupConfigurationPatchRequest, InstancePrivateEndpointsPatch,
    InstanceServiceQueryApiEndpointsPostRequest, InstanceTagsPatch, IpAccessListEntry,
    IpAccessListPatch, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPatchRequest, OrganizationPrivateEndpointsPatch, ResourceTagsV1, Service,
    ServiceEndpointChange, ServiceEndpointChangeProtocol, ServiceEndpointProtocol,
    ServicePasswordPatchRequest, ServicePatchRequest, ServicePatchRequestReleasechannel,
    ServicePostRequest, ServicePostRequestCompliancetype, ServicePostRequestProfile,
    ServicePostRequestProvider, ServicePostRequestRegion, ServicePostRequestReleasechannel,
    ServiceReplicaScalingPatchRequest,
    ServiceStatePatchRequestCommand, ServicPrivateEndpointePostRequest,
};
use std::io::{IsTerminal, Write};
use tabled::{Table, Tabled, settings::Style};

/// Known provider values for client-side validation (from OpenAPI spec).
const KNOWN_PROVIDERS: &[&str] = &["aws", "gcp", "azure"];

/// Known region values for client-side validation (from OpenAPI spec).
const KNOWN_REGIONS: &[&str] = &[
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-south-1",
    "ap-southeast-1",
    "ap-southeast-2",
    "eu-central-1",
    "eu-west-1",
    "eu-west-2",
    "il-central-1",
    "us-east-1",
    "us-east-2",
    "us-west-2",
    "us-east1",
    "us-central1",
    "europe-west4",
    "asia-southeast1",
    "asia-northeast1",
    "eastus",
    "eastus2",
    "westus3",
    "germanywestcentral",
    "centralus",
];

/// Known release channel values for client-side validation (from OpenAPI spec).
const KNOWN_RELEASE_CHANNELS: &[&str] = &["slow", "default", "fast"];

/// Known compliance type values for client-side validation (from OpenAPI spec).
const KNOWN_COMPLIANCE_TYPES: &[&str] = &["hipaa", "pci"];

/// Known service profile values for client-side validation (from OpenAPI spec).
const KNOWN_PROFILES: &[&str] = &[
    "v1-default",
    "v1-highmem-xs",
    "v1-highmem-s",
    "v1-highmem-m",
    "v1-highmem-l",
    "v1-highmem-xl",
];

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
                .filter(|s| s.name == name)
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

fn parse_ip_access_list_patch(
    add: &[String],
    remove: &[String],
) -> Option<IpAccessListPatch> {
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
        remove: if remove.is_empty() { vec![] } else { remove.to_vec() },
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
        add: vec![],
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

fn parse_uuid_list(values: &[String], field: &str) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
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
                name: o.name.clone(),
                id: o.id.to_string(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        println!("Organization: {}", org.name);
        println!("  ID: {}", org.id);
        println!("  Created: {}", org.created_at.to_rfc3339());
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
            .map(|svc| {
                let endpoint = svc
                    .endpoints
                    .first()
                    .map(|e| {
                        format!("{}:{}", e.host, e.port)
                    })
                    .unwrap_or_else(|| "-".to_string());
                Row {
                    name: svc.name.clone(),
                    id: svc.id.to_string(),
                    state: svc.state.to_string(),
                    provider: svc.provider.to_string(),
                    region: svc.region.to_string(),
                    endpoint,
                }
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        println!("Service: {}", svc.name);
        println!("  ID: {}", svc.id);
        println!("  State: {}", svc.state);
        println!("  Provider: {}", svc.provider);
        println!("  Region: {}", svc.region);
        println!("  Tier: {}", svc.tier);
        println!("  Idle Scaling: {}", svc.idle_scaling);
        if !svc.endpoints.is_empty() {
            println!("  Endpoints:");
            for ep in &svc.endpoints {
                println!(
                    "    {} - {}:{}",
                    ep.protocol,
                    ep.host,
                    ep.port
                );
            }
        }
        if !svc.ip_access_list.is_empty() {
            println!("  IP Access List:");
            for ip in &svc.ip_access_list {
                let desc = ip.description.as_deref().unwrap_or("");
                println!("    {} {}", ip.source, desc);
            }
        }
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

    Ok(ServicePostRequest {
        name: opts.name.clone(),
        provider: parse_serde_enum::<ServicePostRequestProvider>(
            &opts.provider,
            "provider",
            KNOWN_PROVIDERS,
        )?,
        region: parse_serde_enum::<ServicePostRequestRegion>(
            &opts.region,
            "region",
            KNOWN_REGIONS,
        )?,
        ip_access_list,
        min_replica_memory_gb: opts.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: opts.max_replica_memory_gb.map(f64::from),
        num_replicas: opts.num_replicas.map(f64::from),
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
                KNOWN_RELEASE_CHANNELS,
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
                KNOWN_COMPLIANCE_TYPES,
            )?),
            None => None,
        },
        profile: match opts.profile.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestProfile>(
                value,
                "profile",
                KNOWN_PROFILES,
            )?),
            None => None,
        },
        private_preview_terms_checked: if opts.private_preview_terms_checked { Some(true) } else { None },
        endpoints: parse_service_endpoint_changes(&opts.enable_endpoints, &opts.disable_endpoints)?,
        enable_core_dumps: opts.enable_core_dumps,
        // Fields not exposed in CLI
        byoc_id: None,
        max_total_memory_gb: None,
        min_total_memory_gb: None,
        private_endpoint_ids: None,
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
                    KNOWN_RELEASE_CHANNELS,
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
        )?.unwrap_or_default(),
        roles: vec![],
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
        let svc = response.service;
        let password = response.password;

        println!("Service created successfully!");
        println!();
        println!("Service: {}", svc.name);
        println!("  ID: {}", svc.id);
        println!("  State: {}", svc.state);
        println!("  Provider: {}", svc.provider);
        println!("  Region: {}", svc.region);
        println!("  Replicas: {}", svc.num_replicas);
        println!("  Min Memory/Replica: {} GB", svc.min_replica_memory_gb);
        println!("  Max Memory/Replica: {} GB", svc.max_replica_memory_gb);
        if let Some(ep) = svc.endpoints.first() {
            println!("  Host: {}", ep.host);
            println!("  Port: {}", ep.port);
        }
        println!();
        println!("Credentials (save these, password shown only once):");
        println!("  Username: default");
        println!("  Password: {}", password);
    }
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

    if force {
        let svc = client.get_service(&org_id, service_id).await?;
        let state = svc.state.to_string();
        if matches!(state.as_str(), "running" | "idle" | "starting") {
            eprintln!("Stopping service {} before deletion...", service_id);
            client
                .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
                .await?;

            // Poll until the service is stopped
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let svc = client.get_service(&org_id, service_id).await?;
                let state = svc.state.to_string();
                eprintln!("  state: {}", state);
                if matches!(state.as_str(), "stopped" | "idle") {
                    break;
                }
                if matches!(state.as_str(), "terminated" | "failed" | "deleted") {
                    return Err(format!(
                        "service entered unexpected state '{}' while waiting for stop",
                        state
                    )
                    .into());
                }
            }
        }
    }

    let response = client.delete_service(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
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
            svc.name,
            svc.state
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
            svc.name,
            svc.state
        );
    }
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
                id: b.id.to_string(),
                status: b.status.to_string(),
                size: format_bytes(b.size_in_bytes),
                created: b.started_at.to_rfc3339(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        println!("Backup: {}", backup.id);
        println!("  Status: {}", backup.status);
        println!("  Created: {}", backup.started_at.to_rfc3339());
        println!("  Finished: {}", backup.finished_at.to_rfc3339());
        println!("  Size: {}", format_bytes(backup.size_in_bytes));
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

    let creds = Credentials {
        api_key,
        api_secret,
    };
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
        println!("Service {} updated", svc.name);
        println!("  ID: {}", svc.id);
        println!("  State: {}", svc.state);
    }
    Ok(())
}

pub struct ServiceScaleOptions {
    pub min_replica_memory_gb: Option<u32>,
    pub max_replica_memory_gb: Option<u32>,
    pub num_replicas: Option<u32>,
    pub idle_scaling: Option<bool>,
    pub idle_timeout_minutes: Option<u32>,
    pub org_id: Option<String>,
}

pub async fn service_scale(
    client: &CloudClient,
    service_id: &str,
    opts: ServiceScaleOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    let request = ServiceReplicaScalingPatchRequest {
        min_replica_memory_gb: opts.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: opts.max_replica_memory_gb.map(f64::from),
        num_replicas: opts.num_replicas.map(f64::from),
        idle_scaling: opts.idle_scaling,
        idle_timeout_minutes: opts.idle_timeout_minutes.map(f64::from),
    };

    let svc = client
        .update_replica_scaling(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} scaling updated", svc.name);
        println!("  Min Memory/Replica: {} GB", svc.min_replica_memory_gb);
        println!("  Max Memory/Replica: {} GB", svc.max_replica_memory_gb);
        println!("  Replicas: {}", svc.num_replicas);
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

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("Password reset for service {}", service_id);
        if resp.password.is_empty() {
            println!("  Password hash updated; no plaintext password returned");
        } else {
            println!("  New password: {}", resp.password);
        }
    }
    Ok(())
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
        println!("Query endpoint for service {}", service_id);
        println!("  ID: {}", ep.id);
        println!("  Roles: {}", ep.roles.join(", "));
        println!("  OpenAPI Keys: {}", ep.open_api_keys.join(", "));
        println!("  Allowed Origins: {}", ep.allowed_origins);
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
        println!("  ID: {}", ep.id);
        println!("  Roles: {}", ep.roles.join(", "));
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
        println!("  Endpoint ID: {}", ep.id);
        println!("  Description: {}", ep.description);
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
        println!("Private endpoint configuration for service {}", service_id);
        println!("  Endpoint Service ID: {}", config.endpoint_service_id);
        println!("  Private DNS Hostname: {}", config.private_dns_hostname);
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
            org.name,
            org.id
        );
    }
    Ok(())
}

pub async fn org_prometheus(
    client: &CloudClient,
    org_id: &str,
    filtered_metrics: Option<bool>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let prom = client.get_org_prometheus(org_id, filtered_metrics).await?;
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
    org_id: &str,
    from_date: &str,
    to_date: &str,
    filters: &[String],
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let usage = client
        .get_org_usage(org_id, from_date, to_date, filters)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&usage)?);
    } else {
        println!("Grand Total: {:.2} CHC", usage.grand_total_chc);
        if usage.costs.is_empty() {
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
        let rows: Vec<Row> = usage
            .costs
            .iter()
            .map(|cost| Row {
                entity: cost.entity_name.clone(),
                date: cost.date.clone(),
                total: format!("{:.2}", cost.total_chc),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
    }
    Ok(())
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
            #[tabled(rename = "Role")]
            role: String,
            #[tabled(rename = "Name")]
            name: String,
        }
        let rows: Vec<Row> = members
            .into_iter()
            .map(|m| Row {
                email: m.email.clone(),
                user_id: m.user_id.clone(),
                role: m.role.to_string(),
                name: m.name.clone(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        println!("Member: {}", member.email);
        println!("  User ID: {}", member.user_id);
        println!("  Role: {}", member.role);
        println!("  Name: {}", member.name);
        println!("  Joined: {}", member.joined_at.to_rfc3339());
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
        role: None,
    };

    let member = client.update_member(&org_id, user_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        println!("Member {} updated", member.email);
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
            #[tabled(rename = "Role")]
            role: String,
            #[tabled(rename = "Expires")]
            expires: String,
        }
        let rows: Vec<Row> = invitations
            .into_iter()
            .map(|inv| Row {
                email: inv.email.clone(),
                id: inv.id.to_string(),
                role: inv.role.to_string(),
                expires: inv.expire_at.to_rfc3339(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        role: clickhouse_cloud_api::models::InvitationPostRequestRole::default(),
    };

    let inv = client.create_invitation(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&inv)?);
    } else {
        println!(
            "Invitation sent to {} ({})",
            inv.email,
            inv.id
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
        println!("Invitation: {}", inv.id);
        println!("  Email: {}", inv.email);
        println!("  Role: {}", inv.role);
        println!("  Created: {}", inv.created_at.to_rfc3339());
        println!("  Expires: {}", inv.expire_at.to_rfc3339());
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
                name: k.name.clone(),
                id: k.id.to_string(),
                state: k.state.to_string(),
                expires: k
                    .expire_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "never".into()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
    }
    Ok(())
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

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("API key created!");
        println!("  Name: {}", resp.key.name);
        if !resp.key_id.is_empty() {
            println!("  Key ID: {}", resp.key_id);
        }
        if !resp.key_secret.is_empty() {
            println!("  Key Secret: {}", resp.key_secret);
        }
        if !resp.key_id.is_empty() || !resp.key_secret.is_empty() {
            println!();
            println!("Save the key secret now — it will not be shown again.");
        } else {
            println!("  Pre-hashed credentials accepted; no generated key material returned");
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
        println!("API Key: {}", key.name);
        println!("  ID: {}", key.id);
        println!("  State: {}", key.state);
        println!("  Roles: {}", key.roles.join(", "));
        println!("  Created: {}", key.created_at.to_rfc3339());
        if let Some(expires) = &key.expire_at {
            println!("  Expires: {}", expires.to_rfc3339());
        }
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
        println!("API key {} updated", key.name);
        println!("  ID: {}", key.id);
        println!("  State: {}", key.state);
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
                id: a.id.clone(),
                activity_type: a.r#type.to_string(),
                created: a.created_at.to_rfc3339(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::rounded()));
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
        println!("Activity: {}", activity.id);
        println!("  Type: {}", activity.r#type);
        println!("  Actor Type: {}", activity.actor_type);
        println!("  Actor ID: {}", activity.actor_id);
        println!("  Created: {}", activity.created_at.to_rfc3339());
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
        println!("Backup configuration for service {}", service_id);
        println!("  Backup period: {} hours", config.backup_period_in_hours);
        println!("  Retention: {} hours", config.backup_retention_period_in_hours);
        println!("  Start time: {}", config.backup_start_time);
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
        println!("  Backup period: {} hours", config.backup_period_in_hours);
        println!("  Retention: {} hours", config.backup_retention_period_in_hours);
        println!("  Start time: {}", config.backup_start_time);
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

pub struct ServiceClientOptions {
    pub name: Option<String>,
    pub id: Option<String>,
    pub query: Option<String>,
    pub queries_file: Option<String>,
    pub user: String,
    pub password: Option<String>,
    pub allow_mismatched_client_version: bool,
    pub generate_password: bool,
    pub org_id: Option<String>,
    pub args: Vec<String>,
}

pub async fn service_client(
    client: &CloudClient,
    opts: ServiceClientOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::{paths, version_manager};
    use std::os::unix::process::CommandExt;

    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    // Resolve the service
    let svc = resolve_service(client, &org_id, opts.name.as_deref(), opts.id.as_deref()).await?;

    // Find the nativesecure endpoint
    let endpoint = svc
        .endpoints
        .iter()
        .find(|e| e.protocol == ServiceEndpointProtocol::Nativesecure)
        .ok_or_else(|| {
            format!(
                "service '{}' has no nativesecure endpoint — is it running?",
                svc.name
            )
        })?;

    let host = &endpoint.host;
    let port = endpoint.port as u16;

    // Determine which client version to use
    let service_version = Some(svc.clickhouse_version.as_str());
    let version = if opts.allow_mismatched_client_version {
        // Try to use the local default version
        match version_manager::get_default_version() {
            Ok(local_ver) => {
                if let Some(svc_ver) = service_version
                    && svc_ver != local_ver.as_str()
                {
                    eprintln!(
                        "Warning: local client version ({}) does not match service version ({}). \
                         This may cause unsupported behavior.",
                        local_ver, svc_ver
                    );
                }
                local_ver
            }
            Err(_) => {
                // No local default — fall back to installing the service version
                eprintln!("No local default version set, falling back to service version.");
                ensure_version_installed(service_version).await?
            }
        }
    } else {
        ensure_version_installed(service_version).await?
    };

    let binary = paths::binary_path(&version)?;
    if !binary.exists() {
        return Err(format!("clickhouse binary not found at {}", binary.display()).into());
    }

    // Extract name/id as strings for use in messages and API calls
    let svc_name = &svc.name;
    let svc_id = svc.id.to_string();

    // Resolve password: --generate-password > --password > env var > TTY prompt
    let password = if opts.generate_password {
        eprintln!("Generating new password for service '{}'...", svc_name);
        let request = ServicePasswordPatchRequest::default();
        let resp = client.reset_password(&org_id, &svc_id, &request).await?;
        let new_password = resp.password;
        // Wait in case of any delay in password propagation
        eprintln!("Waiting for password to propagate...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        new_password
    } else if let Some(p) = opts.password {
        p
    } else if let Ok(p) = std::env::var("CLICKHOUSE_PASSWORD") {
        p
    } else if std::io::stdin().is_terminal() {
        eprint!("Password: ");
        rpassword::read_password()?
    } else {
        return Err(
            "no password provided. Use --password, CLICKHOUSE_PASSWORD env var, or --generate-password"
                .into(),
        );
    };

    // Build and exec the clickhouse-client command
    eprintln!("Connecting to {} ({}:{})...", svc_name, host, port);

    let mut cmd = std::process::Command::new(&binary);
    cmd.arg("client")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .arg("--secure")
        .arg("--user")
        .arg(&opts.user)
        .arg("--password")
        .arg(&password);

    if let Some(q) = &opts.query {
        cmd.arg("--query").arg(q);
    }

    if let Some(f) = &opts.queries_file {
        cmd.arg("--queries-file").arg(f);
    }

    cmd.args(&opts.args);
    let err = cmd.exec();
    Err(format!("failed to exec clickhouse-client: {}", err).into())
}

/// Ensure a ClickHouse version is installed locally, installing it if needed.
async fn ensure_version_installed(
    service_version: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::{paths, version_manager};

    let version_spec = service_version.ok_or("service has no clickhouse_version set")?;

    let spec = version_manager::parse_version_spec(version_spec)?;
    let platform = version_manager::platform::Platform::detect()?;
    let resolved = version_manager::resolve::resolve(&spec, &platform).await?;

    // If exact version is known, check if already installed
    if let Some(ref version) = resolved.exact_version {
        let version_dir = paths::version_dir(version)?;
        if version_dir.exists() {
            return Ok(version.clone());
        }
    }

    eprintln!(
        "Service requires ClickHouse {} — downloading...",
        resolved.display_version
    );
    let version = version_manager::install::install_resolved(&resolved, &platform, false).await?;
    Ok(version)
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
    fn build_create_service_request_supports_ga_optional_fields() {
        let opts = CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            num_replicas: Some(3),
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
}
