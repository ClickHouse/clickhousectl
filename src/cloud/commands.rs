use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, Credentials};
use crate::cloud::types::*;
use std::io::Write;

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
    pub org_id: Option<String>,
}

pub async fn service_create(
    client: &CloudClient,
    opts: CreateServiceOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, opts.org_id.as_deref()).await?;

    // Build IP access list
    let ip_access_list = if opts.ip_allow.is_empty() {
        // Default to allow all if not specified
        Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Allow all (created by clickhousectl)".to_string()),
        }])
    } else {
        Some(
            opts.ip_allow
                .iter()
                .map(|ip| IpAccessEntry {
                    source: ip.clone(),
                    description: None,
                })
                .collect(),
        )
    };

    let request = CreateServiceRequest {
        name: opts.name,
        provider: opts.provider,
        region: opts.region,
        ip_access_list,
        min_replica_memory_gb: opts.min_replica_memory_gb,
        max_replica_memory_gb: opts.max_replica_memory_gb,
        num_replicas: opts.num_replicas,
        idle_scaling: opts.idle_scaling,
        idle_timeout_minutes: opts.idle_timeout_minutes,
        backup_id: opts.backup_id,
        release_channel: opts.release_channel,
        data_warehouse_id: opts.data_warehouse_id,
        is_readonly: if opts.is_readonly { Some(true) } else { None },
        encryption_key: opts.encryption_key,
        encryption_assumed_role_identifier: opts.encryption_role,
        has_transparent_data_encryption: if opts.enable_tde { Some(true) } else { None },
        compliance_type: opts.compliance_type,
        profile: opts.profile,
        ..Default::default()
    };

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
        if let Some(endpoints) = &response.service.endpoints {
            if let Some(ep) = endpoints.first() {
                println!("  Host: {}", ep.host);
                println!("  Port: {}", ep.port);
            }
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
    org_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    client.delete_service(&org_id, service_id).await?;
    println!("Service {} deletion initiated", service_id);
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
        .change_service_state(&org_id, service_id, "start")
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
        .change_service_state(&org_id, service_id, "stop")
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Service {} stopping (state: {})", svc.name, svc.state);
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
                .map(|s| format_bytes(s))
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
    name: Option<&str>,
    ip_allow: &[String],
    clear_ip_allow: bool,
    idle_scaling: Option<bool>,
    idle_timeout_minutes: Option<u32>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let ip_access_list = if clear_ip_allow {
        // To clear, remove all by passing empty remove list — actual clear logic TBD in follow-up
        Some(IpAccessListPatch::default())
    } else if ip_allow.is_empty() {
        None
    } else {
        Some(IpAccessListPatch {
            add: Some(
                ip_allow
                    .iter()
                    .map(|ip| IpAccessEntry {
                        source: ip.clone(),
                        description: None,
                    })
                    .collect(),
            ),
            remove: None,
        })
    };

    let request = UpdateServiceRequest {
        name: name.map(String::from),
        ip_access_list,
        ..Default::default()
    };

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

pub async fn service_scale(
    client: &CloudClient,
    service_id: &str,
    min_replica_memory_gb: Option<u32>,
    max_replica_memory_gb: Option<u32>,
    num_replicas: Option<u32>,
    idle_scaling: Option<bool>,
    idle_timeout_minutes: Option<u32>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = ReplicaScalingRequest {
        min_replica_memory_gb,
        max_replica_memory_gb,
        num_replicas,
        idle_scaling,
        idle_timeout_minutes,
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
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let resp = client.reset_password(&org_id, service_id).await?;

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
    roles: &[String],
    open_api: Option<bool>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let _ = open_api; // TODO: open_api_enabled removed, use open_api_keys in follow-up
    let request = CreateQueryEndpointRequest {
        roles: if roles.is_empty() {
            None
        } else {
            Some(roles.to_vec())
        },
        open_api_keys: None,
        allowed_origins: None,
    };

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

// =============================================================================
// Phase 3 — Org command handlers
// =============================================================================

pub async fn org_update(
    client: &CloudClient,
    org_id: &str,
    name: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = UpdateOrgRequest {
        name: name.map(String::from),
        ..Default::default()
    };

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
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let prom = client.get_org_prometheus(org_id).await?;

    // Org prometheus returns text/plain, so we just print the JSON value
    println!("{}", serde_json::to_string_pretty(&prom)?);
    Ok(())
}

pub async fn service_prometheus(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let prom = client.get_service_prometheus(&org_id, service_id).await?;
    println!("{}", prom);
    Ok(())
}

pub async fn org_usage(
    client: &CloudClient,
    org_id: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let usage = client.get_org_usage(org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&usage)?);
    } else {
        if let Some(total) = usage.grand_total_chc {
            println!("Grand Total: {:.2} CHC", total);
        }
        if let Some(cost) = &usage.costs {
            let entity = cost.entity_name.as_deref().unwrap_or("-");
            let date = cost.date.as_deref().unwrap_or("-");
            let total = cost
                .total_chc
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "-".to_string());
            println!("Usage cost:");
            println!("  {} - {} ({} CHC)", entity, date, total);
        } else {
            println!("No usage cost record found");
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
            println!("  {} ({}) - {} [{}]", m.email, m.user_id, m.role, name);
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
        println!("  Role: {}", member.role);
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
            println!("  {} ({}) - {} [expires: {}]", inv.email, inv.id, inv.role, expires);
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
        println!("  Role: {}", inv.role);
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
            println!("  {} ({}) - {} [expires: {}]", key.name, key.id, key.state, expires);
        }
    }
    Ok(())
}

pub async fn key_create(
    client: &CloudClient,
    name: &str,
    role_ids: &[String],
    expires_at: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = CreateApiKeyRequest {
        name: name.to_string(),
        expire_at: expires_at.map(String::from),
        state: None,
        assigned_role_ids: if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
        ip_access_list: None,
        hash_data: None,
    };

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
    name: Option<&str>,
    role_ids: &[String],
    state: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    let request = UpdateApiKeyRequest {
        name: name.map(String::from),
        assigned_role_ids: if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
        state: state.map(String::from),
        expire_at: None,
        ip_access_list: None,
    };

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
    schedule: Option<&str>,
    retention_period_days: Option<u32>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;

    // TODO: CLI args need updating in follow-up to match new field names
    let request = UpdateBackupConfigRequest {
        backup_period_in_hours: None,
        backup_retention_period_in_hours: retention_period_days.map(|d| d * 24),
        backup_start_time: schedule.map(String::from),
    };

    let config = client.update_backup_config(&org_id, service_id, &request).await?;

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

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
