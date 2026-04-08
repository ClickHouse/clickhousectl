use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, Credentials};
use crate::cloud::types::*;
use std::io::Write;
use std::str::FromStr;

/// Resolve org ID from explicit arg or auto-detect
async fn resolve_org_id(
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
            let matches: Vec<_> = services.into_iter().filter(|s| s.name == name).collect();
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

fn parse_enum<T>(value: &str, field: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: FromStr<Err = String>,
{
    T::from_str(value).map_err(|err| format!("invalid {} '{}': {}", field, value, err).into())
}

fn parse_tag(value: &str) -> Result<ResourceTag, Box<dyn std::error::Error>> {
    match value.split_once('=') {
        Some((key, tag_value)) => {
            let key = key.trim();
            if key.is_empty() {
                Err(format!("invalid tag '{}': tag key cannot be empty", value).into())
            } else {
                Ok(ResourceTag {
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
                Ok(ResourceTag {
                    key: key.to_string(),
                    value: None,
                })
            }
        }
    }
}

fn parse_tags(values: &[String]) -> Result<Option<Vec<ResourceTag>>, Box<dyn std::error::Error>> {
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

fn parse_ip_access_entries(values: &[String]) -> Option<Vec<IpAccessEntry>> {
    (!values.is_empty()).then(|| {
        values
            .iter()
            .map(|value| IpAccessEntry {
                source: value.clone(),
                description: None,
            })
            .collect()
    })
}

fn parse_ip_access_list_patch(add: &[String], remove: &[String]) -> Option<IpAccessListPatch> {
    let patch = IpAccessListPatch {
        add: parse_ip_access_entries(add),
        remove: parse_ip_access_entries(remove),
    };

    (patch.add.is_some() || patch.remove.is_some()).then_some(patch)
}

fn parse_private_endpoint_ids_patch(
    add: &[String],
    remove: &[String],
) -> Option<InstancePrivateEndpointsPatch> {
    let patch = InstancePrivateEndpointsPatch {
        add: (!add.is_empty()).then(|| add.to_vec()),
        remove: (!remove.is_empty()).then(|| remove.to_vec()),
    };

    (patch.add.is_some() || patch.remove.is_some()).then_some(patch)
}

fn parse_service_endpoint_changes(
    enable: &[String],
    disable: &[String],
) -> Result<Option<Vec<ServiceEndpointChange>>, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    for protocol in enable {
        changes.push(ServiceEndpointChange {
            protocol: parse_enum(protocol, "endpoint")?,
            enabled: true,
        });
    }

    for protocol in disable {
        changes.push(ServiceEndpointChange {
            protocol: parse_enum(protocol, "endpoint")?,
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
        add: parse_tags(add)?,
        remove: parse_tags(remove)?,
    };

    Ok((patch.add.is_some() || patch.remove.is_some()).then_some(patch))
}

fn parse_org_private_endpoint_remove(
    value: &str,
) -> Result<OrganizationPatchPrivateEndpoint, Box<dyn std::error::Error>> {
    let mut endpoint = OrganizationPatchPrivateEndpoint {
        id: None,
        description: None,
        cloud_provider: None,
        region: None,
    };

    for (index, part) in value.split(',').enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if index == 0 && !part.contains('=') {
            endpoint.id = Some(part.to_string());
            continue;
        }

        let (key, raw_value) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid remove-private-endpoint segment '{}'", part))?;

        match key {
            "id" => endpoint.id = Some(raw_value.to_string()),
            "description" => endpoint.description = Some(raw_value.to_string()),
            "cloud-provider" => {
                endpoint.cloud_provider = Some(parse_enum(raw_value, "cloud_provider")?)
            }
            "region" => endpoint.region = Some(parse_enum(raw_value, "region")?),
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
        remove: Some(endpoints),
    }))
}

fn parse_api_key_hash_data(
    key_id_hash: Option<&str>,
    key_id_suffix: Option<&str>,
    key_secret_hash: Option<&str>,
) -> Result<Option<ApiKeyHashData>, Box<dyn std::error::Error>> {
    match (key_id_hash, key_id_suffix, key_secret_hash) {
        (None, None, None) => Ok(None),
        (Some(key_id_hash), Some(key_id_suffix), Some(key_secret_hash)) => Ok(Some(ApiKeyHashData {
            key_id_hash: key_id_hash.to_string(),
            key_id_suffix: key_id_suffix.to_string(),
            key_secret_hash: key_secret_hash.to_string(),
        })),
        _ => Err(
            "pre-hashed API key input requires --hash-key-id, --hash-key-id-suffix, and --hash-key-secret together"
                .into(),
        ),
    }
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
        println!("Organizations:");
        for org in orgs {
            println!("  {} ({})", org.name, org.id);
        }
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
        if let Some(created) = org.created_at {
            println!("  Created: {}", created);
        }
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
        println!("Services:");
        for svc in services {
            let endpoint = svc
                .endpoints
                .as_ref()
                .and_then(|eps| eps.first())
                .map(|e| format!("{}:{}", e.host, e.port))
                .unwrap_or_else(|| "-".to_string());
            println!(
                "  {} ({}) - {} [{}/{}] {}",
                svc.name, svc.id, svc.state, svc.provider, svc.region, endpoint
            );
        }
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
        if let Some(tier) = &svc.tier {
            println!("  Tier: {}", tier);
        }
        if let Some(idle) = svc.idle_scaling {
            println!("  Idle Scaling: {}", idle);
        }
        if let Some(endpoints) = &svc.endpoints {
            println!("  Endpoints:");
            for ep in endpoints {
                println!("    {} - {}:{}", ep.protocol, ep.host, ep.port);
            }
        }
        if let Some(ip_list) = &svc.ip_access_list {
            println!("  IP Access List:");
            for ip in ip_list {
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
) -> Result<CreateServiceRequest, Box<dyn std::error::Error>> {
    let ip_access_list = if opts.ip_allow.is_empty() {
        Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Allow all (created by clickhousectl)".to_string()),
        }])
    } else {
        parse_ip_access_entries(&opts.ip_allow)
    };

    Ok(CreateServiceRequest {
        name: opts.name.clone(),
        provider: parse_enum(&opts.provider, "provider")?,
        region: parse_enum(&opts.region, "region")?,
        ip_access_list,
        min_replica_memory_gb: opts.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: opts.max_replica_memory_gb.map(f64::from),
        num_replicas: opts.num_replicas.map(f64::from),
        idle_scaling: opts.idle_scaling,
        idle_timeout_minutes: opts.idle_timeout_minutes.map(f64::from),
        backup_id: opts.backup_id.clone(),
        release_channel: opts
            .release_channel
            .as_deref()
            .map(|value| parse_enum(value, "release_channel"))
            .transpose()?,
        tags: parse_tags(&opts.tags)?,
        data_warehouse_id: opts.data_warehouse_id.clone(),
        is_readonly: opts.is_readonly.then_some(true),
        encryption_key: opts.encryption_key.clone(),
        encryption_assumed_role_identifier: opts.encryption_role.clone(),
        has_transparent_data_encryption: opts.enable_tde.then_some(true),
        compliance_type: opts
            .compliance_type
            .as_deref()
            .map(|value| parse_enum(value, "compliance_type"))
            .transpose()?,
        profile: opts
            .profile
            .as_deref()
            .map(|value| parse_enum(value, "profile"))
            .transpose()?,
        private_preview_terms_checked: opts.private_preview_terms_checked.then_some(true),
        endpoints: parse_service_endpoint_changes(&opts.enable_endpoints, &opts.disable_endpoints)?,
        enable_core_dumps: opts.enable_core_dumps,
    })
}

fn build_update_service_request(
    opts: &ServiceUpdateOptions,
) -> Result<UpdateServiceRequest, Box<dyn std::error::Error>> {
    Ok(UpdateServiceRequest {
        name: opts.name.clone(),
        ip_access_list: parse_ip_access_list_patch(&opts.add_ip_allow, &opts.remove_ip_allow),
        private_endpoint_ids: parse_private_endpoint_ids_patch(
            &opts.add_private_endpoint_ids,
            &opts.remove_private_endpoint_ids,
        ),
        release_channel: opts
            .release_channel
            .as_deref()
            .map(|value| parse_enum(value, "release_channel"))
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
) -> CreateQueryEndpointRequest {
    CreateQueryEndpointRequest {
        roles: (!opts.roles.is_empty()).then(|| opts.roles.clone()),
        open_api_keys: (!opts.open_api_keys.is_empty()).then(|| opts.open_api_keys.clone()),
        allowed_origins: opts.allowed_origins.clone(),
    }
}

fn build_org_update_request(
    opts: &OrgUpdateOptions,
) -> Result<UpdateOrgRequest, Box<dyn std::error::Error>> {
    Ok(UpdateOrgRequest {
        name: opts.name.clone(),
        private_endpoints: parse_org_private_endpoints_patch(&opts.remove_private_endpoints)?,
        enable_core_dumps: opts.enable_core_dumps,
    })
}

fn build_api_key_create_request(
    opts: &KeyCreateOptions,
) -> Result<CreateApiKeyRequest, Box<dyn std::error::Error>> {
    Ok(CreateApiKeyRequest {
        name: opts.name.clone(),
        expire_at: opts.expires_at.clone(),
        state: opts
            .state
            .as_deref()
            .map(|value| parse_enum(value, "state"))
            .transpose()?,
        assigned_role_ids: (!opts.role_ids.is_empty()).then(|| opts.role_ids.clone()),
        ip_access_list: parse_ip_access_entries(&opts.ip_allow),
        hash_data: parse_api_key_hash_data(
            opts.hash_key_id.as_deref(),
            opts.hash_key_id_suffix.as_deref(),
            opts.hash_key_secret.as_deref(),
        )?,
    })
}

fn build_api_key_update_request(
    opts: &KeyUpdateOptions,
) -> Result<UpdateApiKeyRequest, Box<dyn std::error::Error>> {
    Ok(UpdateApiKeyRequest {
        name: opts.name.clone(),
        assigned_role_ids: (!opts.role_ids.is_empty()).then(|| opts.role_ids.clone()),
        expire_at: opts.expires_at.clone(),
        state: opts
            .state
            .as_deref()
            .map(|value| parse_enum(value, "state"))
            .transpose()?,
        ip_access_list: parse_ip_access_entries(&opts.ip_allow),
    })
}

fn build_backup_config_update_request(
    opts: &BackupConfigUpdateOptions,
) -> UpdateBackupConfigRequest {
    UpdateBackupConfigRequest {
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
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_create_service_request(&opts)?;

    let response = client.create_service(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Service created successfully!");
        println!();
        println!("Service: {}", response.service.name);
        println!("  ID: {}", response.service.id);
        println!("  State: {}", response.service.state);
        println!("  Provider: {}", response.service.provider);
        println!("  Region: {}", response.service.region);
        if let Some(replicas) = response.service.num_replicas {
            println!("  Replicas: {}", replicas);
        }
        if let Some(min_mem) = response.service.min_replica_memory_gb {
            println!("  Min Memory/Replica: {} GB", min_mem);
        }
        if let Some(max_mem) = response.service.max_replica_memory_gb {
            println!("  Max Memory/Replica: {} GB", max_mem);
        }
        if let Some(endpoints) = &response.service.endpoints
            && let Some(ep) = endpoints.first()
        {
            println!("  Host: {}", ep.host);
            println!("  Port: {}", ep.port);
        }
        println!();
        println!("Credentials (save these, password shown only once):");
        println!("  Username: default");
        println!("  Password: {}", response.password);
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
                .change_service_state(&org_id, service_id, ServiceStateCommand::Stop)
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
        .change_service_state(&org_id, service_id, ServiceStateCommand::Start)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} starting (state: {})", svc.name, svc.state);
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
        .change_service_state(&org_id, service_id, ServiceStateCommand::Stop)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} stopping (state: {})", svc.name, svc.state);
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
    } else {
        if clickpipes.is_empty() {
            println!("No ClickPipes found");
            return Ok(());
        }
        println!("ClickPipes:");
        for cp in clickpipes {
            println!("  {} ({}) - {}", cp.name, cp.id, cp.state);
        }
    }
    Ok(())
}

pub async fn clickpipe_create_s3(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    url: &str,
    format: &str,
    database: &str,
    table: &str,
    columns: &[String],
    storage_type: &str,
    compression: &str,
    continuous: bool,
    queue_url: Option<&str>,
    delimiter: Option<&str>,
    iam_role: Option<&str>,
    access_key_id: Option<&str>,
    secret_key: Option<&str>,
    connection_string: Option<&str>,
    azure_container_name: Option<&str>,
    path: Option<&str>,
    service_account_key: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let parsed_columns: Vec<crate::cloud::types::ClickPipeDestinationColumn> = columns
        .iter()
        .map(|col| {
            let (name, col_type) = col.split_once(':').ok_or_else(|| {
                format!("Invalid column format '{}': expected name:type", col)
            })?;
            Ok(crate::cloud::types::ClickPipeDestinationColumn {
                name: name.to_string(),
                column_type: col_type.to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let (authentication, iam_role_val, access_key) = match (iam_role, access_key_id, secret_key) {
        (Some(role), _, _) => (Some("IAM_ROLE".to_string()), Some(role.to_string()), None),
        (_, Some(key_id), Some(secret)) => (
            Some("IAM_USER".to_string()),
            None,
            Some(crate::cloud::types::ObjectStorageAccessKey {
                access_key_id: key_id.to_string(),
                secret_key: secret.to_string(),
            }),
        ),
        _ => (None, None, None),
    };

    let authentication = authentication
        .or_else(|| connection_string.map(|_| "CONNECTION_STRING".to_string()))
        .or_else(|| service_account_key.map(|_| "SERVICE_ACCOUNT".to_string()));

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source: crate::cloud::types::CreateClickPipeSource {
            kafka: None,
            kinesis: None,
            postgres: None,
            mysql: None,
            mongodb: None,
            bigquery: None,
            object_storage: Some(crate::cloud::types::ObjectStorageSource {
                storage_type: storage_type.to_string(),
                format: format.to_string(),
                url: url.to_string(),
                compression: compression.to_string(),
                is_continuous: if continuous { Some(true) } else { None },
                queue_url: queue_url.map(|s| s.to_string()),
                delimiter: delimiter.map(|s| s.to_string()),
                authentication,
                iam_role: iam_role_val,
                access_key,
                connection_string: connection_string.map(|s| s.to_string()),
                azure_container_name: azure_container_name.map(|s| s.to_string()),
                path: path.map(|s| s.to_string()),
                service_account_key: service_account_key.map(|s| s.to_string()),
            }),
        },
        destination: crate::cloud::types::ClickPipeDestination {
            database: database.to_string(),
            table: table.to_string(),
            managed_table: true,
            table_definition: Some(crate::cloud::types::ClickPipeTableDefinition {
                engine: crate::cloud::types::ClickPipeTableEngine {
                    engine_type: "MergeTree".to_string(),
                },
                sorting_key: None,
                partition_by: None,
                primary_key: None,
            }),
            columns: if parsed_columns.is_empty() { None } else { Some(parsed_columns) },
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
    }
    Ok(())
}

pub async fn clickpipe_create_kafka(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    brokers: &str,
    topics: &str,
    format: &str,
    database: &str,
    table: &str,
    columns: &[String],
    kafka_type: &str,
    consumer_group: Option<&str>,
    auth: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    iam_role: Option<&str>,
    access_key_id: Option<&str>,
    secret_key: Option<&str>,
    offset: &str,
    offset_timestamp: Option<&str>,
    schema_registry_url: Option<&str>,
    schema_registry_username: Option<&str>,
    schema_registry_password: Option<&str>,
    ca_certificate: Option<&str>,
    client_certificate: Option<&str>,
    client_key: Option<&str>,
    schema_registry_ca_certificate: Option<&str>,
    reverse_private_endpoint_ids: &[String],
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let parsed_columns: Vec<crate::cloud::types::ClickPipeDestinationColumn> = columns
        .iter()
        .map(|col| {
            let (name, col_type) = col.split_once(':').ok_or_else(|| {
                format!("Invalid column format '{}': expected name:type", col)
            })?;
            Ok(crate::cloud::types::ClickPipeDestinationColumn {
                name: name.to_string(),
                column_type: col_type.to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let credentials = match (username, password) {
        (Some(u), Some(p)) => Some(crate::cloud::types::KafkaCredentials {
            username: u.to_string(),
            password: p.to_string(),
        }),
        _ => None,
    };

    let access_key = match (access_key_id, secret_key) {
        (Some(key_id), Some(secret)) => Some(crate::cloud::types::ObjectStorageAccessKey {
            access_key_id: key_id.to_string(),
            secret_key: secret.to_string(),
        }),
        _ => None,
    };

    let schema_registry = schema_registry_url.map(|url| {
        let sr_credentials = match (schema_registry_username, schema_registry_password) {
            (Some(u), Some(p)) => Some(crate::cloud::types::KafkaCredentials {
                username: u.to_string(),
                password: p.to_string(),
            }),
            _ => None,
        };
        let sr_ca_cert = match schema_registry_ca_certificate {
            Some(path) => Some(std::fs::read_to_string(path).unwrap_or_default()),
            None => None,
        };
        crate::cloud::types::KafkaSchemaRegistry {
            url: url.to_string(),
            authentication: if sr_credentials.is_some() {
                Some("PLAIN".to_string())
            } else {
                None
            },
            credentials: sr_credentials,
            ca_certificate: sr_ca_cert,
        }
    });

    let ca_cert_contents = match ca_certificate {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let client_cert_contents = match client_certificate {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let client_key_contents = match client_key {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source: crate::cloud::types::CreateClickPipeSource {
            object_storage: None,
            kinesis: None,
            postgres: None,
            mysql: None,
            mongodb: None,
            bigquery: None,
            kafka: Some(crate::cloud::types::KafkaSource {
                kafka_type: kafka_type.to_string(),
                format: format.to_string(),
                brokers: brokers.to_string(),
                topics: topics.to_string(),
                consumer_group: consumer_group.map(|s| s.to_string()),
                authentication: auth.map(|s| s.to_string()),
                credentials,
                iam_role: iam_role.map(|s| s.to_string()),
                access_key,
                offset: Some(crate::cloud::types::KafkaOffset {
                    strategy: offset.to_string(),
                    timestamp: offset_timestamp.map(|s| s.to_string()),
                }),
                schema_registry,
                ca_certificate: ca_cert_contents,
                certificate: client_cert_contents,
                private_key: client_key_contents,
                reverse_private_endpoint_ids: reverse_private_endpoint_ids.to_vec(),
            }),
        },
        destination: crate::cloud::types::ClickPipeDestination {
            database: database.to_string(),
            table: table.to_string(),
            managed_table: true,
            table_definition: Some(crate::cloud::types::ClickPipeTableDefinition {
                engine: crate::cloud::types::ClickPipeTableEngine {
                    engine_type: "MergeTree".to_string(),
                },
                sorting_key: None,
                partition_by: None,
                primary_key: None,
            }),
            columns: if parsed_columns.is_empty() { None } else { Some(parsed_columns) },
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
    }
    Ok(())
}

pub async fn clickpipe_create_kinesis(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    stream_name: &str,
    region: &str,
    format: &str,
    database: &str,
    table: &str,
    columns: &[String],
    auth: &str,
    iam_role: Option<&str>,
    access_key_id: Option<&str>,
    secret_key: Option<&str>,
    iterator_type: &str,
    iterator_timestamp: Option<u64>,
    enhanced_fan_out: bool,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let parsed_columns: Vec<crate::cloud::types::ClickPipeDestinationColumn> = columns
        .iter()
        .map(|col| {
            let (name, col_type) = col.split_once(':').ok_or_else(|| {
                format!("Invalid column format '{}': expected name:type", col)
            })?;
            Ok(crate::cloud::types::ClickPipeDestinationColumn {
                name: name.to_string(),
                column_type: col_type.to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let access_key = match (access_key_id, secret_key) {
        (Some(key_id), Some(secret)) => Some(crate::cloud::types::ObjectStorageAccessKey {
            access_key_id: key_id.to_string(),
            secret_key: secret.to_string(),
        }),
        _ => None,
    };

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source: crate::cloud::types::CreateClickPipeSource {
            object_storage: None,
            kafka: None,
            postgres: None,
            mysql: None,
            mongodb: None,
            bigquery: None,
            kinesis: Some(crate::cloud::types::KinesisSource {
                format: format.to_string(),
                stream_name: stream_name.to_string(),
                region: region.to_string(),
                authentication: auth.to_string(),
                iam_role: iam_role.map(|s| s.to_string()),
                access_key,
                use_enhanced_fan_out: if enhanced_fan_out { Some(true) } else { None },
                iterator_type: Some(iterator_type.to_string()),
                timestamp: iterator_timestamp,
            }),
        },
        destination: crate::cloud::types::ClickPipeDestination {
            database: database.to_string(),
            table: table.to_string(),
            managed_table: true,
            table_definition: Some(crate::cloud::types::ClickPipeTableDefinition {
                engine: crate::cloud::types::ClickPipeTableEngine {
                    engine_type: "MergeTree".to_string(),
                },
                sorting_key: None,
                partition_by: None,
                primary_key: None,
            }),
            columns: if parsed_columns.is_empty() { None } else { Some(parsed_columns) },
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
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
    let clickpipe = client.get_clickpipe(&org_id, service_id, clickpipe_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
        if let Some(sid) = &clickpipe.service_id {
            println!("  Service ID: {}", sid);
        }
        if let Some(created) = &clickpipe.created_at {
            println!("  Created: {}", created);
        }
        if let Some(updated) = &clickpipe.updated_at {
            println!("  Updated: {}", updated);
        }
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
    client.delete_clickpipe(&org_id, service_id, clickpipe_id).await?;

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
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client.change_clickpipe_state(&org_id, service_id, clickpipe_id, command).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe {} {} (state: {})", clickpipe.name, command, clickpipe.state);
    }
    Ok(())
}

fn parse_db_table_mappings(
    mappings: &[String],
) -> Result<Vec<crate::cloud::types::DbTableMapping>, String> {
    mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!("Invalid table mapping '{}': expected schema.table:target_table", m)
            })?;
            let (schema, table) = source.split_once('.').ok_or_else(|| {
                format!("Invalid source '{}': expected schema.table", source)
            })?;
            Ok(crate::cloud::types::DbTableMapping {
                source_schema_name: schema.to_string(),
                source_table: table.to_string(),
                target_table: target.to_string(),
                table_engine: Some("ReplacingMergeTree".to_string()),
            })
        })
        .collect()
}

fn empty_source() -> crate::cloud::types::CreateClickPipeSource {
    crate::cloud::types::CreateClickPipeSource {
        object_storage: None,
        kafka: None,
        kinesis: None,
        postgres: None,
        mysql: None,
        mongodb: None,
        bigquery: None,
    }
}

pub async fn clickpipe_create_postgres(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    host: &str,
    port: u16,
    pg_database: &str,
    username: &str,
    password: &str,
    table_mappings: &[String],
    postgres_type: &str,
    replication_mode: &str,
    auth: &str,
    iam_role: Option<&str>,
    tls_host: Option<&str>,
    ca_certificate: Option<&str>,
    publication_name: Option<&str>,
    replication_slot_name: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let mappings = parse_db_table_mappings(table_mappings)?;

    let ca_cert_contents = match ca_certificate {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let mut source = empty_source();
    source.postgres = Some(crate::cloud::types::PostgresSource {
        postgres_type: postgres_type.to_string(),
        credentials: crate::cloud::types::DbCredentials {
            username: username.to_string(),
            password: password.to_string(),
        },
        host: host.to_string(),
        port,
        database: pg_database.to_string(),
        authentication: auth.to_string(),
        iam_role: iam_role.map(|s| s.to_string()),
        tls_host: tls_host.map(|s| s.to_string()),
        ca_certificate: ca_cert_contents,
        settings: crate::cloud::types::PostgresSettings {
            replication_mode: replication_mode.to_string(),
            publication_name: publication_name.map(|s| s.to_string()),
            replication_slot_name: replication_slot_name.map(|s| s.to_string()),
        },
        table_mappings: mappings,
    });

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source,
        destination: crate::cloud::types::ClickPipeDestination {
            database: "default".to_string(),
            table: "".to_string(),
            managed_table: false,
            table_definition: None,
            columns: None,
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
    }
    Ok(())
}

pub async fn clickpipe_create_mysql(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    table_mappings: &[String],
    mysql_type: &str,
    replication_mode: &str,
    replication_mechanism: &str,
    auth: &str,
    iam_role: Option<&str>,
    tls_host: Option<&str>,
    ca_certificate: Option<&str>,
    disable_tls: bool,
    skip_cert_verification: bool,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let mappings = parse_db_table_mappings(table_mappings)?;

    let ca_cert_contents = match ca_certificate {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let mut source = empty_source();
    source.mysql = Some(crate::cloud::types::MySQLSource {
        mysql_type: mysql_type.to_string(),
        credentials: crate::cloud::types::DbCredentials {
            username: username.to_string(),
            password: password.to_string(),
        },
        host: host.to_string(),
        port,
        authentication: auth.to_string(),
        iam_role: iam_role.map(|s| s.to_string()),
        tls_host: tls_host.map(|s| s.to_string()),
        ca_certificate: ca_cert_contents,
        disable_tls: if disable_tls { Some(true) } else { None },
        skip_cert_verification: if skip_cert_verification { Some(true) } else { None },
        settings: crate::cloud::types::MySQLSettings {
            replication_mode: replication_mode.to_string(),
            replication_mechanism: replication_mechanism.to_string(),
        },
        table_mappings: mappings,
    });

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source,
        destination: crate::cloud::types::ClickPipeDestination {
            database: "default".to_string(),
            table: "".to_string(),
            managed_table: false,
            table_definition: None,
            columns: None,
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
    }
    Ok(())
}

pub async fn clickpipe_create_mongodb(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    uri: &str,
    username: &str,
    password: &str,
    table_mappings: &[String],
    replication_mode: &str,
    read_preference: &str,
    tls_host: Option<&str>,
    ca_certificate: Option<&str>,
    disable_tls: bool,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let mongo_mappings: Vec<crate::cloud::types::MongoDBTableMapping> = table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!("Invalid table mapping '{}': expected database.collection:target_table", m)
            })?;
            let (db, collection) = source.split_once('.').ok_or_else(|| {
                format!("Invalid source '{}': expected database.collection", source)
            })?;
            Ok(crate::cloud::types::MongoDBTableMapping {
                source_database_name: db.to_string(),
                source_collection: collection.to_string(),
                target_table: target.to_string(),
                table_engine: Some("ReplacingMergeTree".to_string()),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let ca_cert_contents = match ca_certificate {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let mut source = empty_source();
    source.mongodb = Some(crate::cloud::types::MongoDBSource {
        credentials: crate::cloud::types::DbCredentials {
            username: username.to_string(),
            password: password.to_string(),
        },
        uri: uri.to_string(),
        read_preference: Some(read_preference.to_string()),
        tls_host: tls_host.map(|s| s.to_string()),
        ca_certificate: ca_cert_contents,
        disable_tls: if disable_tls { Some(true) } else { None },
        settings: crate::cloud::types::MongoDBSettings {
            replication_mode: replication_mode.to_string(),
        },
        table_mappings: mongo_mappings,
    });

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source,
        destination: crate::cloud::types::ClickPipeDestination {
            database: "default".to_string(),
            table: "".to_string(),
            managed_table: false,
            table_definition: None,
            columns: None,
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
    }
    Ok(())
}

pub async fn clickpipe_create_bigquery(
    client: &CloudClient,
    service_id: &str,
    name: &str,
    service_account_file: &str,
    staging_path: &str,
    table_mappings: &[String],
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let sa_contents = std::fs::read_to_string(service_account_file)?;
    let sa_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        sa_contents.as_bytes(),
    );

    let bq_mappings: Vec<crate::cloud::types::BigQueryTableMapping> = table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!("Invalid table mapping '{}': expected dataset.table:target_table", m)
            })?;
            let (dataset, table) = source.split_once('.').ok_or_else(|| {
                format!("Invalid source '{}': expected dataset.table", source)
            })?;
            Ok(crate::cloud::types::BigQueryTableMapping {
                source_dataset_name: dataset.to_string(),
                source_table: table.to_string(),
                target_table: target.to_string(),
                table_engine: Some("ReplacingMergeTree".to_string()),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut source = empty_source();
    source.bigquery = Some(crate::cloud::types::BigQuerySource {
        credentials: crate::cloud::types::BigQueryCredentials {
            service_account_file: sa_b64,
        },
        snapshot_staging_path: staging_path.to_string(),
        settings: crate::cloud::types::BigQuerySettings {
            replication_mode: "snapshot".to_string(),
        },
        table_mappings: bq_mappings,
    });

    let request = crate::cloud::types::CreateClickPipeRequest {
        name: name.to_string(),
        source,
        destination: crate::cloud::types::ClickPipeDestination {
            database: "default".to_string(),
            table: "".to_string(),
            managed_table: false,
            table_definition: None,
            columns: None,
        },
    };

    let clickpipe = client.create_clickpipe(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", clickpipe.name);
        println!("  ID: {}", clickpipe.id);
        println!("  State: {}", clickpipe.state);
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
        println!("Backups:");
        for backup in backups {
            let size = backup
                .size_in_bytes
                .map(format_bytes)
                .unwrap_or_else(|| "-".to_string());
            let created = backup.started_at.as_deref().unwrap_or("-");
            println!("  {} - {} ({}) {}", backup.id, backup.status, size, created);
        }
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
        if let Some(created) = &backup.started_at {
            println!("  Created: {}", created);
        }
        if let Some(finished) = &backup.finished_at {
            println!("  Finished: {}", finished);
        }
        if let Some(size) = backup.size_in_bytes {
            println!("  Size: {}", format_bytes(size));
        }
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
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_update_service_request(&opts)?;

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

    let request = ReplicaScalingRequest {
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
        if let Some(min) = svc.min_replica_memory_gb {
            println!("  Min Memory/Replica: {} GB", min);
        }
        if let Some(max) = svc.max_replica_memory_gb {
            println!("  Max Memory/Replica: {} GB", max);
        }
        if let Some(n) = svc.num_replicas {
            println!("  Replicas: {}", n);
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

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("Password reset for service {}", service_id);
        if let Some(password) = resp.password {
            println!("  New password: {}", password);
        } else {
            println!("  Password hash updated; no plaintext password returned");
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
        if let Some(id) = &ep.id {
            println!("  ID: {}", id);
        }
        if let Some(roles) = &ep.roles {
            println!("  Roles: {}", roles.join(", "));
        }
        if let Some(keys) = &ep.open_api_keys {
            println!("  OpenAPI Keys: {}", keys.join(", "));
        }
        if let Some(origins) = &ep.allowed_origins {
            println!("  Allowed Origins: {}", origins);
        }
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
        if let Some(id) = &ep.id {
            println!("  ID: {}", id);
        }
        if let Some(roles) = &ep.roles {
            println!("  Roles: {}", roles.join(", "));
        }
    }
    Ok(())
}

pub async fn query_endpoint_delete(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    client.delete_query_endpoint(&org_id, service_id).await?;
    println!("Query endpoint deleted for service {}", service_id);
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

    let request = CreatePrivateEndpointRequest {
        id: endpoint_id.to_string(),
        description: description.map(String::from),
    };

    let ep = client
        .create_private_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ep)?);
    } else {
        println!("Private endpoint created for service {}", service_id);
        if let Some(id) = &ep.id {
            println!("  Endpoint ID: {}", id);
        }
        if let Some(desc) = &ep.description {
            println!("  Description: {}", desc);
        }
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
        println!("Organization updated: {} ({})", org.name, org.id);
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
        if let Some(total) = usage.grand_total_chc {
            println!("Grand Total: {:.2} CHC", total);
        }
        if let Some(costs) = &usage.costs {
            if costs.is_empty() {
                println!("No usage cost records found");
                return Ok(());
            }

            println!("Usage costs:");
            for cost in costs {
                let entity = cost.entity_name.as_deref().unwrap_or("-");
                let date = cost.date.as_deref().unwrap_or("-");
                let total = cost
                    .total_chc
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string());
                println!("  {} - {} ({} CHC)", entity, date, total);
            }
        } else {
            println!("No usage cost records found");
        }
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
        println!("Members:");
        for m in members {
            let name = m.name.as_deref().unwrap_or("");
            let role = m.role.as_deref().unwrap_or("-");
            println!("  {} ({}) - {} [{}]", m.email, m.user_id, role, name);
        }
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
        println!("  Role: {}", member.role.as_deref().unwrap_or("-"));
        if let Some(name) = &member.name {
            println!("  Name: {}", name);
        }
        if let Some(joined) = &member.joined_at {
            println!("  Joined: {}", joined);
        }
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

    let request = UpdateMemberRequest {
        assigned_role_ids: if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
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
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    client.delete_member(&org_id, user_id).await?;
    println!("Member {} removed", user_id);
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
        println!("Invitations:");
        for inv in invitations {
            let expires = inv.expire_at.as_deref().unwrap_or("-");
            let role = inv.role.as_deref().unwrap_or("-");
            println!(
                "  {} ({}) - {} [expires: {}]",
                inv.email, inv.id, role, expires
            );
        }
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

    let request = CreateInvitationRequest {
        email: email.to_string(),
        assigned_role_ids: if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
    };

    let inv = client.create_invitation(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&inv)?);
    } else {
        println!("Invitation sent to {} ({})", inv.email, inv.id);
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
        println!("  Role: {}", inv.role.as_deref().unwrap_or("-"));
        if let Some(created) = &inv.created_at {
            println!("  Created: {}", created);
        }
        if let Some(expires) = &inv.expire_at {
            println!("  Expires: {}", expires);
        }
    }
    Ok(())
}

pub async fn invitation_delete(
    client: &CloudClient,
    invitation_id: &str,
    org_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    client.delete_invitation(&org_id, invitation_id).await?;
    println!("Invitation {} deleted", invitation_id);
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
        println!("API Keys:");
        for key in keys {
            let expires = key.expire_at.as_deref().unwrap_or("never");
            println!(
                "  {} ({}) - {} [expires: {}]",
                key.name, key.id, key.state, expires
            );
        }
    }
    Ok(())
}

pub async fn key_create(
    client: &CloudClient,
    opts: KeyCreateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_api_key_create_request(&opts)?;

    let resp = client.create_api_key(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("API key created!");
        println!("  Name: {}", resp.key.name);
        if let Some(key_id) = &resp.key_id {
            println!("  Key ID: {}", key_id);
        }
        if let Some(key_secret) = &resp.key_secret {
            println!("  Key Secret: {}", key_secret);
        }
        if resp.key_id.is_some() || resp.key_secret.is_some() {
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
        if let Some(roles) = &key.roles {
            println!("  Roles: {}", roles.join(", "));
        }
        if let Some(created) = &key.created_at {
            println!("  Created: {}", created);
        }
        if let Some(expires) = &key.expire_at {
            println!("  Expires: {}", expires);
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
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;
    let request = build_api_key_update_request(&opts)?;

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
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    client.delete_api_key(&org_id, key_id).await?;
    println!("API key {} deleted", key_id);
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
        println!("Activities:");
        for a in activities {
            let created = a.created_at.as_deref().unwrap_or("-");
            println!("  {} - {} {}", a.id, a.activity_type, created);
        }
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
        println!("  Type: {}", activity.activity_type);
        if let Some(actor_type) = &activity.actor_type {
            println!("  Actor Type: {}", actor_type);
        }
        if let Some(actor_id) = &activity.actor_id {
            println!("  Actor ID: {}", actor_id);
        }
        if let Some(created) = &activity.created_at {
            println!("  Created: {}", created);
        }
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
        if let Some(hours) = config.backup_period_in_hours {
            println!("  Backup period: {} hours", hours);
        }
        if let Some(hours) = config.backup_retention_period_in_hours {
            println!("  Retention: {} hours", hours);
        }
        if let Some(time) = &config.backup_start_time {
            println!("  Start time: {}", time);
        }
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
        if let Some(hours) = config.backup_period_in_hours {
            println!("  Backup period: {} hours", hours);
        }
        if let Some(hours) = config.backup_retention_period_in_hours {
            println!("  Retention: {} hours", hours);
        }
        if let Some(time) = &config.backup_start_time {
            println!("  Start time: {}", time);
        }
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
        .as_ref()
        .and_then(|eps| {
            eps.iter()
                .find(|e| e.protocol == ServiceEndpointProtocol::NativeSecure)
        })
        .ok_or_else(|| {
            format!(
                "service '{}' has no nativesecure endpoint — is it running?",
                svc.name
            )
        })?;

    let host = &endpoint.host;
    let port = endpoint.port as u16;

    // Determine which client version to use
    let service_version = svc.clickhouse_version.as_deref();
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

    // Resolve password: --generate-password > --password > env var > TTY prompt
    let password = if opts.generate_password {
        eprintln!("Generating new password for service '{}'...", svc.name);
        let request = ServicePasswordPatchRequest::default();
        let resp = client.reset_password(&org_id, &svc.id, &request).await?;
        let new_password = resp.password.ok_or("API did not return a password")?;
        // Wait in case of any delay in password propagation
        eprintln!("Waiting for password to propagate...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        new_password
    } else if let Some(p) = opts.password {
        p
    } else if let Ok(p) = std::env::var("CLICKHOUSE_PASSWORD") {
        p
    } else if atty::is(atty::Stream::Stdin) {
        eprint!("Password: ");
        rpassword::read_password()?
    } else {
        return Err(
            "no password provided. Use --password, CLICKHOUSE_PASSWORD env var, or --generate-password"
                .into(),
        );
    };

    // Build and exec the clickhouse-client command
    eprintln!("Connecting to {} ({}:{})...", svc.name, host, port);

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
            backup_id: Some("backup-1".to_string()),
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
            role_ids: vec!["role-1".to_string()],
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

        let update_opts = KeyUpdateOptions {
            name: Some("renamed".to_string()),
            role_ids: vec!["role-1".to_string()],
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
