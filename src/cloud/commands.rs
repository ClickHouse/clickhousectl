use crate::cloud::client::CloudClient;
use crate::cloud::credentials::{self, Credentials};
use crate::cloud::types::*;
use std::io::Write;

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
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

    let services = client.list_services(&org_id).await?;

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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
    pub byoc_id: Option<String>,
    pub compliance_type: Option<String>,
    pub profile: Option<String>,
    pub org_id: Option<String>,
}

pub async fn service_create(
    client: &CloudClient,
    opts: CreateServiceOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = match opts.org_id.as_deref() {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
        byoc_id: opts.byoc_id,
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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

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
            let created = backup.created_at.as_deref().unwrap_or("-");
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
    let org_id = match org_id {
        Some(id) => id.to_string(),
        None => client.get_default_org_id().await?,
    };

    let backup = client.get_backup(&org_id, service_id, backup_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backup)?);
    } else {
        println!("Backup: {}", backup.id);
        println!("  Status: {}", backup.status);
        if let Some(created) = &backup.created_at {
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
