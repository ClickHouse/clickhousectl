use crate::cloud::api_keys::{cleanup_service_query_key, service_query_key_cleanup};
use crate::cloud::client::{
    CloudClient, CloudError, ResourceKind, ResourceLookup, Result as CloudResult,
};
use crate::cloud::credentials;
use crate::cloud::output::{ABSENT, eprint_line, or_absent, print_human, print_line};
use crate::cloud::shared::PollProgress;
use crate::cloud::shared::{parse_ip_access_entries, parse_serde_enum, parse_tags, resolve_org_id};
use clickhouse_cloud_api::models::{
    AutoscalingMode, BackupEncryptionConfig, InstancePrivateEndpointsPatch, InstanceTagsPatch,
    IpAccessListEntry, IpAccessListPatch, Service, ServiceEndpoint, ServiceEndpointChange,
    ServiceEndpointChangeProtocol, ServiceEndpointProtocol, ServicePasswordPatchRequest,
    ServicePatchRequest, ServicePatchRequestReleasechannel, ServicePostRequest,
    ServicePostRequestCompliancetype, ServicePostRequestProfile, ServicePostRequestProvider,
    ServicePostRequestRegion, ServicePostRequestReleasechannel, ServicePostResponse,
    ServiceReplicaScalingPatchRequest, ServiceState, ServiceStatePatchRequest,
    ServiceStatePatchRequestCommand,
};
#[cfg(test)]
use clickhouse_cloud_api::models::{IpAccessListEntryResponse, ResourceTagsV1Response};
use std::{collections::HashSet, io::IsTerminal, path::Path, time::Duration};
use tabled::{Table, Tabled, settings::Style};

/// Gap between `GET service` calls while `service delete --force` waits for a
/// stop to finish. A real stop takes minutes, so the loop can emit dozens of
/// progress lines — see [`crate::cloud::output::eprint_line`] for why none of
/// them may panic.
const STOP_POLL_INTERVAL: Duration = Duration::from_secs(5);

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

fn parse_ip_access_list_patch(
    add: &[String],
    remove: &[String],
) -> CloudResult<Option<IpAccessListPatch>> {
    let patch = IpAccessListPatch {
        add: parse_ip_access_entries(add)?.unwrap_or_default(),
        remove: parse_ip_access_entries(remove)?.unwrap_or_default(),
    };

    Ok((!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch))
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
) -> CloudResult<Option<Vec<ServiceEndpointChange>>> {
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
) -> CloudResult<Option<InstanceTagsPatch>> {
    let patch = InstanceTagsPatch {
        add: parse_tags(add)?.unwrap_or_default(),
        remove: parse_tags(remove)?.unwrap_or_default(),
    };

    Ok((!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch))
}

pub(super) async fn service_list(
    client: &CloudClient,
    filters: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;

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
            .map(|service| Row {
                name: or_absent(service.name.as_deref()),
                id: or_absent(service.id),
                state: or_absent(service.state.as_ref()),
                provider: or_absent(service.provider.as_ref()),
                region: or_absent(service.region.as_ref()),
                endpoint: first_endpoint(service.endpoints.as_deref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub(super) async fn service_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let service = client.get_service(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        print_human(&service)?;
    }
    Ok(())
}

/// The `.env` connection settings for a service, with the same meanings as
/// `local clickhouse dotenv`: `CLICKHOUSE_PORT` is the native (TLS) port. Keys the
/// API response can't supply are left out rather than guessed.
fn service_dotenv_vars(
    service: Option<&Service>,
    password: Option<&str>,
) -> Vec<(&'static str, String)> {
    let endpoint = |protocol: ServiceEndpointProtocol| {
        service
            .and_then(|service| service.endpoints.as_deref())
            .unwrap_or_default()
            .iter()
            .find(|endpoint| endpoint.protocol.as_ref() == Some(&protocol))
    };
    let native = endpoint(ServiceEndpointProtocol::Nativesecure);
    let https = endpoint(ServiceEndpointProtocol::Https);

    let mut vars = Vec::new();
    if let Some(host) = native
        .and_then(|endpoint| endpoint.host.clone())
        .or_else(|| https.and_then(|endpoint| endpoint.host.clone()))
    {
        vars.push(("CLICKHOUSE_HOST", host));
    }
    if let Some(port) = native.and_then(|endpoint| endpoint.port) {
        vars.push(("CLICKHOUSE_PORT", port.to_string()));
    }
    if let Some(port) = https.and_then(|endpoint| endpoint.port) {
        vars.push(("CLICKHOUSE_HTTP_PORT", port.to_string()));
    }
    vars.push(("CLICKHOUSE_USER", "default".to_string()));
    vars.push(("CLICKHOUSE_SECURE", "true".to_string()));
    if let Some(password) = password {
        vars.push(("CLICKHOUSE_PASSWORD", password.to_string()));
    }
    vars
}

#[derive(serde::Serialize)]
struct ServiceDotenvOutput {
    file: &'static str,
    service: String,
    vars: Vec<ServiceDotenvVar>,
}

#[derive(serde::Serialize)]
struct ServiceDotenvVar {
    key: &'static str,
    value: String,
}

pub(super) async fn service_dotenv(
    client: &CloudClient,
    service_id: &str,
    use_local: bool,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let service = client.get_service(&org_id, service_id).await?;
    let vars = service_dotenv_vars(Some(&service), None);
    if !vars.iter().any(|(key, _)| *key == "CLICKHOUSE_HOST") {
        return Err(CloudError::new(format!(
            "the API returned no endpoints for service {service_id}, so there is no host to \
             write; check `clickhousectl cloud service get {service_id}`"
        )));
    }

    let file = if use_local { ".env.local" } else { ".env" };
    crate::dotenv::write_vars(Path::new(file), "CLICKHOUSE_", &vars)?;

    let output = ServiceDotenvOutput {
        file,
        service: service
            .name
            .clone()
            .unwrap_or_else(|| service_id.to_string()),
        vars: vars
            .into_iter()
            .map(|(key, value)| ServiceDotenvVar { key, value })
            .collect(),
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Wrote to {} (service '{}')", output.file, output.service);
        for var in &output.vars {
            println!("  {}={}", var.key, var.value);
        }
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct CreateServiceOptions {
    pub(super) name: String,
    pub(super) provider: String,
    pub(super) region: String,
    pub(super) min_replica_memory_gb: Option<u32>,
    pub(super) max_replica_memory_gb: Option<u32>,
    pub(super) num_replicas: Option<u32>,
    pub(super) min_replicas: Option<u32>,
    pub(super) max_replicas: Option<u32>,
    pub(super) autoscaling_mode: Option<String>,
    pub(super) idle_scaling: Option<bool>,
    pub(super) idle_timeout_minutes: Option<u32>,
    pub(super) ip_allow: Vec<String>,
    pub(super) backup_id: Option<String>,
    pub(super) backup_encryption_config: Option<BackupEncryptionConfig>,
    pub(super) release_channel: Option<String>,
    pub(super) data_warehouse_id: Option<String>,
    pub(super) is_readonly: bool,
    pub(super) enable_tde: bool,
    pub(super) compliance_type: Option<String>,
    pub(super) profile: Option<String>,
    pub(super) byoc_id: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) enable_endpoints: Vec<String>,
    pub(super) disable_endpoints: Vec<String>,
    pub(super) private_preview_terms_checked: bool,
    pub(super) enable_core_dumps: Option<bool>,
}

#[derive(Default)]
pub(super) struct ServiceUpdateOptions {
    pub(super) name: Option<String>,
    pub(super) add_ip_allow: Vec<String>,
    pub(super) remove_ip_allow: Vec<String>,
    pub(super) add_private_endpoint_ids: Vec<String>,
    pub(super) remove_private_endpoint_ids: Vec<String>,
    pub(super) release_channel: Option<String>,
    pub(super) enable_endpoints: Vec<String>,
    pub(super) disable_endpoints: Vec<String>,
    pub(super) transparent_data_encryption_key_id: Option<String>,
    pub(super) add_tags: Vec<String>,
    pub(super) remove_tags: Vec<String>,
    pub(super) enable_core_dumps: Option<bool>,
}

#[derive(Default)]
pub(super) struct ServiceResetPasswordOptions {
    pub(super) new_password_hash: Option<String>,
    pub(super) new_double_sha1_hash: Option<String>,
}

struct HorizontalAutoscaling {
    autoscaling_mode: Option<AutoscalingMode>,
    min_replicas: Option<i64>,
    max_replicas: Option<i64>,
}

fn resolve_horizontal_autoscaling(
    autoscaling_mode: Option<&str>,
    min_replicas: Option<u32>,
    max_replicas: Option<u32>,
) -> CloudResult<HorizontalAutoscaling> {
    if min_replicas.is_some() != max_replicas.is_some() {
        return Err(CloudError::new(
            "--min-replicas and --max-replicas must be specified together",
        ));
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

/// Read a backup's `encryption_config.json` from a file or stdin (`-`), passing
/// the object through unchanged. Every failure is a usage error raised before
/// any request is sent.
pub(super) fn read_backup_encryption_config(path: &str) -> CloudResult<BackupEncryptionConfig> {
    use std::io::Read as _;

    let (raw, source) = if path == "-" {
        let mut raw = String::new();
        std::io::stdin().read_to_string(&mut raw).map_err(|error| {
            CloudError::usage(format!(
                "failed to read --backup-encryption-config from stdin: {error}"
            ))
        })?;
        (raw, "stdin".to_string())
    } else {
        (
            std::fs::read_to_string(path).map_err(|error| {
                CloudError::usage(format!(
                    "failed to read --backup-encryption-config file '{path}': {error}"
                ))
            })?,
            format!("file '{path}'"),
        )
    };
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(serde_json::Value::Object(object)) => Ok(object.into_iter().collect()),
        Ok(_) => Err(CloudError::usage(format!(
            "--backup-encryption-config {source} must contain a JSON object"
        ))),
        Err(error) => Err(CloudError::usage(format!(
            "--backup-encryption-config {source} is not valid JSON: {error}"
        ))),
    }
}

fn build_create_service_request(options: &CreateServiceOptions) -> CloudResult<ServicePostRequest> {
    let ip_access_list = if options.ip_allow.is_empty() {
        vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Allow all (created by clickhousectl)".to_string()),
        }]
    } else {
        parse_ip_access_entries(&options.ip_allow)?.unwrap_or_default()
    };

    let horizontal = resolve_horizontal_autoscaling(
        options.autoscaling_mode.as_deref(),
        options.min_replicas,
        options.max_replicas,
    )?;

    if options.byoc_id.as_deref().is_some_and(str::is_empty) {
        return Err(CloudError::new("--byoc-id cannot be empty"));
    }
    if options.byoc_id.is_some()
        && (options.min_replica_memory_gb.is_none() || options.max_replica_memory_gb.is_none())
    {
        return Err(CloudError::new(
            "--byoc-id requires --min-replica-memory-gb and --max-replica-memory-gb",
        ));
    }

    let profile = options
        .profile
        .as_deref()
        .map(|value| parse_create_service_profile(value, options.byoc_id.as_deref()))
        .transpose()?;
    if matches!(profile, Some(ServicePostRequestProfile::Unknown(_)))
        && options.min_replica_memory_gb != options.max_replica_memory_gb
    {
        return Err(CloudError::new(
            "a dynamic BYOC --profile requires equal --min-replica-memory-gb and --max-replica-memory-gb",
        ));
    }

    Ok(ServicePostRequest {
        name: options.name.clone(),
        provider: parse_serde_enum::<ServicePostRequestProvider>(
            &options.provider,
            "provider",
            ServicePostRequestProvider::VALUES,
        )?,
        region: parse_serde_enum::<ServicePostRequestRegion>(
            &options.region,
            "region",
            ServicePostRequestRegion::VALUES,
        )?,
        ip_access_list,
        min_replica_memory_gb: options.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: options.max_replica_memory_gb.map(f64::from),
        num_replicas: options.num_replicas.map(i64::from),
        idle_scaling: options.idle_scaling,
        idle_timeout_minutes: options.idle_timeout_minutes.map(f64::from),
        backup_id: options
            .backup_id
            .as_deref()
            .map(uuid::Uuid::parse_str)
            .transpose()
            .map_err(|error| CloudError::new(format!("invalid backup_id: {}", error)))?,
        release_channel: match options.release_channel.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestReleasechannel>(
                value,
                "release_channel",
                ServicePostRequestReleasechannel::VALUES,
            )?),
            None => None,
        },
        tags: parse_tags(&options.tags)?,
        data_warehouse_id: options.data_warehouse_id.clone(),
        is_readonly: if options.is_readonly {
            Some(true)
        } else {
            None
        },
        has_transparent_data_encryption: if options.enable_tde { Some(true) } else { None },
        compliance_type: match options.compliance_type.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestCompliancetype>(
                value,
                "compliance_type",
                ServicePostRequestCompliancetype::VALUES,
            )?),
            None => None,
        },
        profile,
        private_preview_terms_checked: if options.private_preview_terms_checked {
            Some(true)
        } else {
            None
        },
        endpoints: parse_service_endpoint_changes(
            &options.enable_endpoints,
            &options.disable_endpoints,
        )?,
        enable_core_dumps: options.enable_core_dumps,
        autoscaling_mode: horizontal.autoscaling_mode,
        byoc_id: options.byoc_id.clone(),
        backup_encryption_config: options.backup_encryption_config.clone(),
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
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

fn parse_create_service_profile(
    value: &str,
    byoc_id: Option<&str>,
) -> CloudResult<ServicePostRequestProfile> {
    if ServicePostRequestProfile::VALUES.contains(&value) {
        return parse_serde_enum::<ServicePostRequestProfile>(
            value,
            "profile",
            ServicePostRequestProfile::VALUES,
        );
    }
    if byoc_id.is_none() {
        return Err(CloudError::new(format!(
            "invalid profile: unknown standard profile '{value}'; dynamic profiles require --byoc-id"
        )));
    }
    if value.is_empty() {
        return Err(CloudError::new("--profile cannot be empty"));
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|error| CloudError::new(format!("invalid profile: {error}")))
}

fn build_update_service_request(
    options: &ServiceUpdateOptions,
) -> CloudResult<ServicePatchRequest> {
    Ok(ServicePatchRequest {
        name: options.name.clone(),
        ip_access_list: parse_ip_access_list_patch(
            &options.add_ip_allow,
            &options.remove_ip_allow,
        )?,
        private_endpoint_ids: parse_private_endpoint_ids_patch(
            &options.add_private_endpoint_ids,
            &options.remove_private_endpoint_ids,
        ),
        release_channel: options
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
        endpoints: parse_service_endpoint_changes(
            &options.enable_endpoints,
            &options.disable_endpoints,
        )?,
        transparent_data_encryption_key_id: options.transparent_data_encryption_key_id.clone(),
        tags: parse_instance_tags_patch(&options.add_tags, &options.remove_tags)?,
        enable_core_dumps: options.enable_core_dumps,
    })
}

/// Whether `options` requests removing anything from a service, so callers
/// can decide whether a pre-update `GET` is needed to check the removals
/// against the current state.
fn has_removals(options: &ServiceUpdateOptions) -> bool {
    !options.remove_ip_allow.is_empty()
        || !options.remove_private_endpoint_ids.is_empty()
        || !options.remove_tags.is_empty()
}

/// Requested values that do not match anything in `current` (by exact string
/// equality). The service update API silently no-ops a removal that matches
/// nothing, which is idempotent but can hide a typo, so the caller surfaces
/// each unmatched entry as a warning instead of failing.
fn unmatched<'a>(requested: &'a [String], current: &HashSet<&str>) -> Vec<&'a str> {
    requested
        .iter()
        .map(String::as_str)
        .filter(|value| !current.contains(value))
        .collect()
}

/// Human-readable warnings for every `--remove-*` value that does not match
/// anything in `current`. IP entries come from the validated request so the
/// comparison uses the same parsed source that the API receives.
fn unmatched_removal_warnings(
    options: &ServiceUpdateOptions,
    validated_request: &ServicePatchRequest,
    current: &Service,
) -> Vec<String> {
    let mut warnings = Vec::new();

    let current_ip_allow: HashSet<&str> = current
        .ip_access_list
        .iter()
        .flatten()
        .filter_map(|entry| entry.source.as_deref())
        .collect();
    let remove_ip_allow = validated_request
        .ip_access_list
        .as_ref()
        .map(|patch| patch.remove.as_slice())
        .unwrap_or_default();
    for entry in remove_ip_allow {
        if current_ip_allow.contains(entry.source.as_str()) {
            continue;
        }
        let value = &entry.source;
        warnings.push(format!(
            "--remove-ip-allow {value} did not match any entry in the service's current IP allow list; no entry was removed"
        ));
    }

    let current_private_endpoint_ids: HashSet<&str> = current
        .private_endpoint_ids
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    for value in unmatched(
        &options.remove_private_endpoint_ids,
        &current_private_endpoint_ids,
    ) {
        warnings.push(format!(
            "--remove-private-endpoint-id {value} did not match any private endpoint on the service; nothing was removed"
        ));
    }

    let current_tags: HashSet<&str> = current
        .tags
        .iter()
        .flatten()
        .filter_map(|tag| tag.key.as_deref())
        .collect();
    let remove_tag_keys: Vec<String> = options
        .remove_tags
        .iter()
        .map(|value| tag_key(value).to_string())
        .collect();
    for value in unmatched(&remove_tag_keys, &current_tags) {
        warnings.push(format!(
            "--remove-tag {value} did not match any tag on the service; nothing was removed"
        ));
    }

    warnings
}

/// The key portion of a `key` or `key=value` tag argument, matching how
/// [`crate::cloud::shared::parse_tag`] splits removal/addition arguments.
fn tag_key(value: &str) -> &str {
    value.split_once('=').map_or(value, |(key, _)| key).trim()
}

fn build_service_password_patch_request(
    options: &ServiceResetPasswordOptions,
) -> ServicePasswordPatchRequest {
    ServicePasswordPatchRequest {
        new_password_hash: options.new_password_hash.clone(),
        new_double_sha1_hash: options.new_double_sha1_hash.clone(),
    }
}

pub(super) fn build_service_state_patch_request(
    command: ServiceStatePatchRequestCommand,
) -> ServiceStatePatchRequest {
    ServiceStatePatchRequest {
        command: Some(command),
    }
}

fn service_query_hint(service_id: Option<uuid::Uuid>) -> Option<String> {
    service_id.map(|id| {
        format!(
            "Run SQL with: clickhousectl cloud service query {} --query \"SELECT 1\"\n\
             (the Query API endpoint is provisioned automatically on first use)",
            id
        )
    })
}

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

pub(super) async fn service_create(
    client: &CloudClient,
    options: CreateServiceOptions,
    dotenv: Option<&Path>,
    json: bool,
) -> CloudResult<()> {
    // The password is returned only once, so find an unwritable file before
    // the service exists rather than after.
    if let Some(path) = dotenv {
        check_dotenv_target(path)?;
    }
    let request = build_create_service_request(&options)?;
    let org_id = resolve_org_id(client).await?;
    validate_dynamic_byoc_profile(client, &org_id, &request).await?;
    let response = client.create_service(&org_id, &request).await?;
    let service_id = response.service.as_ref().and_then(|service| service.id);

    let written = match dotenv {
        None => None,
        Some(path) => {
            let vars = service_dotenv_vars(response.service.as_ref(), response.password.as_deref());
            if let Err(error) = crate::dotenv::write_vars(path, "CLICKHOUSE_", &vars) {
                // Print the password as if --dotenv were absent, so the
                // credential is never lost, then fail the command.
                print_created_service(&response, service_id, json)?;
                let message = format!(
                    "the service was created, but writing {} failed: {error}. The one-time \
                     password is in the output above.",
                    path.display()
                );
                return Err(CloudError {
                    message,
                    ..CloudError::from(error)
                });
            }
            if !vars.iter().any(|(key, _)| *key == "CLICKHOUSE_HOST") {
                eprint_line(format!(
                    "Warning: the API response had no endpoints, so {} has no host or ports. \
                     Add them later with `clickhousectl cloud service dotenv {}`.",
                    path.display(),
                    service_id.map_or_else(|| "<service-id>".to_string(), |id| id.to_string())
                ));
            }
            Some(DotenvWritten {
                file: path.display().to_string(),
                keys: vars.into_iter().map(|(key, _)| key).collect(),
            })
        }
    };

    match written {
        None => print_created_service(&response, service_id, json)?,
        Some(written) if json => {
            let mut value = serde_json::to_value(&response)?;
            if let Some(object) = value.as_object_mut() {
                object.remove("password");
                object.insert("dotenv".to_string(), serde_json::to_value(&written)?);
            }
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Some(written) => {
            println!("Service created successfully!");
            println!();
            if let Some(service) = &response.service {
                print_human(service)?;
            }
            println!();
            if response.password.is_some() {
                println!(
                    "Credentials: username default; the one-time password was written to {} as \
                     CLICKHOUSE_PASSWORD (not shown).",
                    written.file
                );
            } else {
                println!("{}", service_credentials_block(None, service_id));
            }
            println!("Wrote {} to {}", written.keys.join(", "), written.file);
            if let Some(hint) = service_query_hint(service_id) {
                println!();
                println!("{}", hint);
            }
        }
    }
    Ok(())
}

/// The create output without `--dotenv`, which shows the one-time password.
fn print_created_service(
    response: &ServicePostResponse,
    service_id: Option<uuid::Uuid>,
    json: bool,
) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(response)?);
    } else {
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

/// What `create --dotenv` wrote, reported in place of the password.
#[derive(serde::Serialize)]
struct DotenvWritten {
    file: String,
    keys: Vec<&'static str>,
}

/// Reject a `--dotenv` target that cannot be written, before the service is
/// created. Opening for append checks write access without changing the file.
fn check_dotenv_target(path: &Path) -> CloudResult<()> {
    if path.exists() {
        if !path.is_file() {
            return Err(CloudError::usage(format!(
                "--dotenv {} is not a regular file",
                path.display()
            )));
        }
        let with_path = |error: std::io::Error| CloudError {
            message: format!("--dotenv {}: {error}", path.display()),
            ..CloudError::from(error)
        };
        std::fs::read_to_string(path).map_err(with_path)?;
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(with_path)?;
        return Ok(());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err(CloudError::usage(format!(
            "--dotenv {}: directory {} does not exist",
            path.display(),
            parent.display()
        )));
    }
    Ok(())
}

async fn validate_dynamic_byoc_profile(
    client: &CloudClient,
    org_id: &str,
    request: &ServicePostRequest,
) -> CloudResult<()> {
    let Some(ServicePostRequestProfile::Unknown(profile_name)) = request.profile.as_ref() else {
        return Ok(());
    };
    let byoc_id = request
        .byoc_id
        .as_deref()
        .ok_or_else(|| CloudError::new("dynamic profiles require --byoc-id"))?;
    let region = request.region.to_string();
    let profiles = client
        .list_service_profiles(org_id, Some(&region), Some(byoc_id))
        .await?;
    let profile = profiles
        .iter()
        .find(|candidate| candidate.profile.as_deref() == Some(profile_name.as_str()))
        .ok_or_else(|| {
            CloudError::new(format!(
                "profile '{profile_name}' is not available for BYOC infrastructure '{byoc_id}' in region '{region}'"
            ))
        })?;
    let memory_gi = profile.memory_gi.ok_or_else(|| {
        CloudError::new(format!(
            "profile '{profile_name}' did not include its memory size; cannot validate service memory bounds"
        ))
    })?;
    let (Some(min_memory), Some(max_memory)) =
        (request.min_replica_memory_gb, request.max_replica_memory_gb)
    else {
        return Err(CloudError::new(
            "dynamic BYOC profiles require both replica memory bounds",
        ));
    };
    if min_memory != memory_gi || max_memory != memory_gi {
        return Err(CloudError::new(format!(
            "profile '{profile_name}' requires both replica memory bounds to equal {memory_gi} GiB"
        )));
    }
    Ok(())
}

fn classify_stop_poll_state(state: Option<&ServiceState>) -> CloudResult<bool> {
    let state = state
        .ok_or_else(|| {
            CloudError::new(
                "the API response omitted the service state while waiting for the service to stop, \
             so the stop cannot be confirmed",
            )
        })?
        .to_string();
    if matches!(state.as_str(), "stopped" | "idle") {
        return Ok(true);
    }
    if matches!(state.as_str(), "terminated" | "failed" | "deleted") {
        return Err(CloudError::new(format!(
            "service entered unexpected state '{}' while waiting for stop",
            state
        )));
    }
    Ok(false)
}

fn service_delete_conflict(error: CloudError, service_id: &str) -> CloudError {
    let api_message = error.message.clone();
    let message = format!(
        "service could not be deleted because of a conflict. If it is running, use --force to \
         stop it first, or `clickhousectl cloud service stop {service_id}`. API response: \
         {api_message}"
    );
    CloudError { message, ..error }
}

pub(super) fn service_delete_error(
    client: &CloudClient,
    error: clickhouse_cloud_api::Error,
    force: bool,
    service_id: &str,
    org_id: &str,
) -> CloudError {
    let is_conflict = matches!(&error, clickhouse_cloud_api::Error::Api { status: 409, .. });
    let error = client.convert_error_for_lookup(
        error,
        ResourceLookup::in_org(ResourceKind::Service, service_id, org_id),
    );
    if is_conflict && !force {
        service_delete_conflict(error, service_id)
    } else {
        error
    }
}

pub(super) async fn service_delete(
    client: &CloudClient,
    service_id: &str,
    force: bool,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let (query_key_ids, retain_query_key) = service_query_key_cleanup(&org_id, service_id)?;

    if force {
        let service = client.get_service_if_exists(&org_id, service_id).await?;
        let state = service
            .as_ref()
            .map(|service| or_absent(service.state.as_ref()))
            .unwrap_or_default();
        if matches!(state.as_str(), "running" | "idle" | "starting") {
            // Every write below goes through `eprint_line`/`print_line`, never
            // `eprintln!`/`println!`: this loop streams progress for as long as
            // the stop takes (minutes), so it routinely outlives whatever was
            // reading its output, and a panicking write turned a successful
            // delete into exit 101 (#598).
            eprint_line(format!(
                "Stopping service {} before deletion...",
                service_id
            ));
            client
                .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
                .await?;

            let verbose_polling =
                std::io::stderr().is_terminal() && !json && std::env::var_os("CI").is_none();
            let mut progress = PollProgress::default();
            loop {
                tokio::time::sleep(STOP_POLL_INTERVAL).await;
                let service = client.get_service(&org_id, service_id).await?;
                let state = or_absent(service.state.as_ref());
                if let Some(line) = progress.render(&state, verbose_polling) {
                    eprint_line(line);
                }
                if classify_stop_poll_state(service.state.as_ref())? {
                    break;
                }
            }
        }
    }

    let response = client.delete_service(&org_id, service_id, force).await?;
    cleanup_service_query_key(client, &org_id, service_id, &query_key_ids).await?;
    if !retain_query_key {
        credentials::remove_service_query_key(service_id)?;
    }
    // The service is already gone by here, so a closed stdout must not turn a
    // completed deletion into a panic — see `print_line` and #598.
    if json {
        print_line(serde_json::to_string_pretty(&response)?);
    } else {
        print_line(format!("Service {} deletion initiated", service_id));
    }
    Ok(())
}

pub(super) async fn service_start(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let service = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Start)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} start request accepted (returned state: {})",
            or_absent(service.name.as_deref()),
            or_absent(service.state.as_ref())
        );
    }
    Ok(())
}

pub(super) async fn service_stop(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let service = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} stop request accepted (returned state: {})",
            or_absent(service.name.as_deref()),
            or_absent(service.state.as_ref())
        );
    }
    Ok(())
}

pub(super) async fn service_wake(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let service = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Awake)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} waking (state: {})",
            or_absent(service.name.as_deref()),
            or_absent(service.state.as_ref())
        );
    }
    Ok(())
}

pub(super) async fn service_update(
    client: &CloudClient,
    service_id: &str,
    options: ServiceUpdateOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_update_service_request(&options)?;
    let org_id = resolve_org_id(client).await?;

    if has_removals(&options) {
        let current = client.get_service(&org_id, service_id).await?;
        for warning in unmatched_removal_warnings(&options, &request, &current) {
            eprint_line(format!("Warning: {warning}"));
        }
    }

    let service = client.update_service(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!("Service {} updated", or_absent(service.name.as_deref()));
        println!("  ID: {}", or_absent(service.id));
        println!("  State: {}", or_absent(service.state.as_ref()));
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct ServiceScaleOptions {
    pub(super) min_replica_memory_gb: Option<u32>,
    pub(super) max_replica_memory_gb: Option<u32>,
    pub(super) num_replicas: Option<u32>,
    pub(super) min_replicas: Option<u32>,
    pub(super) max_replicas: Option<u32>,
    pub(super) autoscaling_mode: Option<String>,
    pub(super) idle_scaling: Option<bool>,
    pub(super) idle_timeout_minutes: Option<u32>,
}

fn build_service_scale_request(
    options: &ServiceScaleOptions,
) -> CloudResult<ServiceReplicaScalingPatchRequest> {
    let horizontal = resolve_horizontal_autoscaling(
        options.autoscaling_mode.as_deref(),
        options.min_replicas,
        options.max_replicas,
    )?;

    Ok(ServiceReplicaScalingPatchRequest {
        autoscaling_mode: horizontal.autoscaling_mode,
        min_replica_memory_gb: options.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: options.max_replica_memory_gb.map(f64::from),
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
        num_replicas: options.num_replicas.map(i64::from),
        idle_scaling: options.idle_scaling,
        idle_timeout_minutes: options.idle_timeout_minutes.map(f64::from),
    })
}

pub(super) async fn service_scale(
    client: &CloudClient,
    service_id: &str,
    options: ServiceScaleOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_service_scale_request(&options)?;
    let org_id = resolve_org_id(client).await?;
    let service = client
        .update_replica_scaling(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} scaling updated",
            or_absent(service.name.as_deref())
        );
        println!(
            "  Autoscaling Mode: {}",
            or_absent(service.autoscaling_mode.as_ref())
        );
        match service.autoscaling_mode {
            Some(AutoscalingMode::Horizontal) => {
                println!("  Min Replicas: {}", or_absent(service.min_replicas));
                println!("  Max Replicas: {}", or_absent(service.max_replicas));
                println!(
                    "  Memory/Replica: {} GB",
                    or_absent(service.replica_memory_gb)
                );
            }
            Some(AutoscalingMode::Vertical) => {
                println!(
                    "  Min Memory/Replica: {} GB",
                    or_absent(service.min_replica_memory_gb)
                );
                println!(
                    "  Max Memory/Replica: {} GB",
                    or_absent(service.max_replica_memory_gb)
                );
                println!("  Replicas: {}", or_absent(service.num_replicas));
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) async fn service_reset_password(
    client: &CloudClient,
    service_id: &str,
    options: ServiceResetPasswordOptions,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let request = build_service_password_patch_request(&options);
    let response = client.reset_password(&org_id, service_id, &request).await?;
    let outcome = resolve_reset_password_outcome(
        generation_requested(&request),
        response.password.as_deref(),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
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

#[derive(Debug)]
enum ResetPasswordOutcome<'a> {
    Generated(&'a str),
    HashUpdated,
}

fn generation_requested(request: &ServicePasswordPatchRequest) -> bool {
    request.new_password_hash.is_none()
}

fn resolve_reset_password_outcome(
    generation_requested: bool,
    password: Option<&str>,
) -> CloudResult<ResetPasswordOutcome<'_>> {
    if !generation_requested {
        return Ok(ResetPasswordOutcome::HashUpdated);
    }
    match password {
        Some(password) => Ok(ResetPasswordOutcome::Generated(password)),
        None => Err(CloudError::new(
            "the API response omitted the generated password, so it cannot be shown: the \
             service password may already have been rotated — run the reset again to get \
             a password you can use",
        )),
    }
}

pub(super) async fn service_prometheus(
    client: &CloudClient,
    service_id: &str,
    filtered_metrics: Option<bool>,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let prometheus = client
        .get_service_prometheus(&org_id, service_id, filtered_metrics)
        .await?;
    println!("{}", prometheus);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::output::{CloudErrorCode, CloudErrorDetail};
    use crate::failure::{ApiFailure, FailureKind};

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
        assert!(hint.contains("query a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"));
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
        assert!(!block.contains(&format!("Password: {ABSENT}")));
        assert!(block.contains(
            "clickhousectl cloud service reset-password a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"
        ));
    }

    #[test]
    fn service_credentials_block_warns_generically_when_the_service_id_is_absent() {
        assert!(
            service_credentials_block(None, None)
                .contains("clickhousectl cloud service reset-password <service-id>")
        );
    }

    #[test]
    fn classify_stop_poll_state_fails_on_an_absent_state() {
        let error = classify_stop_poll_state(None).unwrap_err();
        assert!(error.to_string().contains("omitted the service state"));
    }

    #[test]
    fn classify_stop_poll_state_separates_stopped_waiting_and_failed() {
        assert!(classify_stop_poll_state(Some(&ServiceState::Stopped)).unwrap());
        assert!(classify_stop_poll_state(Some(&ServiceState::Idle)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Stopping)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Running)).unwrap());
        assert!(
            !classify_stop_poll_state(Some(&ServiceState::Unknown("hibernating".into()))).unwrap()
        );
        assert!(classify_stop_poll_state(Some(&ServiceState::Failed)).is_err());
        assert!(classify_stop_poll_state(Some(&ServiceState::Unknown("deleted".into()))).is_err());
    }

    #[test]
    fn service_delete_error_uses_the_api_status_and_preserves_classification() {
        let client = CloudClient::new(
            Some("test-key"),
            Some("test-secret"),
            Some("https://api.example.com/v1"),
        )
        .unwrap();
        let error = clickhouse_cloud_api::Error::Api {
            status: 409,
            message: "opaque conflict response".into(),
        };
        let rewritten = service_delete_error(&client, error, false, "svc-1", "org-1");

        assert_eq!(
            rewritten.message,
            "service could not be deleted because of a conflict. If it is running, use --force \
             to stop it first, or `clickhousectl cloud service stop svc-1`. API response: opaque \
             conflict response"
        );
        assert_eq!(
            rewritten.kind,
            crate::cloud::client::CloudErrorKind::Generic
        );
        assert_eq!(
            rewritten.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 409))
        );
    }

    #[test]
    fn service_delete_error_does_not_classify_from_message_text() {
        let client = CloudClient::new(
            Some("test-key"),
            Some("test-secret"),
            Some("https://api.example.com/v1"),
        )
        .unwrap();
        let running_prose = || clickhouse_cloud_api::Error::Api {
            status: 400,
            message: "CONFLICT: Current state: 'running'".into(),
        };

        let ordinary = service_delete_error(&client, running_prose(), false, "svc-1", "org-1");
        assert_eq!(ordinary.message, "CONFLICT: Current state: 'running'");
        assert_eq!(
            ordinary.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 400))
        );

        let forced = service_delete_error(
            &client,
            clickhouse_cloud_api::Error::Api {
                status: 409,
                message: "opaque conflict response".into(),
            },
            true,
            "svc-1",
            "org-1",
        );
        assert_eq!(forced.message, "opaque conflict response");
        assert_eq!(
            forced.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 409))
        );
    }

    #[test]
    fn service_delete_conflict_preserves_converted_error_metadata() {
        let detail = CloudErrorDetail {
            code: CloudErrorCode::ResourceNotFound,
            message: "structured error".into(),
            host: None,
            port: None,
            command: Some("cloud service list".into()),
            api_key_id: None,
            ip_access_list: None,
        };
        let error = CloudError::auth("opaque conflict response")
            .with_failure(ApiFailure::with_status(FailureKind::Http4xx, 409))
            .with_details(detail);

        let rewritten = service_delete_conflict(error, "svc-1");

        assert_eq!(rewritten.kind, crate::cloud::client::CloudErrorKind::Auth);
        assert_eq!(
            rewritten.failure,
            Some(ApiFailure::with_status(FailureKind::Http4xx, 409))
        );
        let detail = rewritten.details.as_deref().expect("structured details");
        assert_eq!(detail.code, CloudErrorCode::ResourceNotFound);
        assert_eq!(detail.message, "structured error");
        assert_eq!(detail.command.as_deref(), Some("cloud service list"));
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
        assert!(generation_requested(&request(None, Some("sha1"))));
        assert!(!generation_requested(&request(Some("sha256"), None)));
        assert!(!generation_requested(&request(
            Some("sha256"),
            Some("sha1")
        )));
    }

    #[test]
    fn resolve_reset_password_outcome_returns_the_generated_password() {
        assert!(matches!(
            resolve_reset_password_outcome(true, Some("s3cret")).unwrap(),
            ResetPasswordOutcome::Generated("s3cret")
        ));
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
        assert!(matches!(
            resolve_reset_password_outcome(false, Some("s3cret")).unwrap(),
            ResetPasswordOutcome::HashUpdated
        ));
    }

    #[test]
    fn resolve_reset_password_outcome_fails_when_the_generated_password_is_absent() {
        let error = resolve_reset_password_outcome(true, None).unwrap_err();
        assert!(error.to_string().contains("omitted the generated password"));
    }

    fn minimal_create_options() -> CreateServiceOptions {
        CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_create_service_request_supports_minimal_fields() {
        let request = build_create_service_request(&minimal_create_options()).unwrap();

        assert_eq!(request.name, "svc");
        assert_eq!(request.provider, ServicePostRequestProvider::Aws);
        assert_eq!(request.region, ServicePostRequestRegion::Us_east_1);
        assert_eq!(request.ip_access_list.len(), 1);
        assert_eq!(request.ip_access_list[0].source, "0.0.0.0/0");
        assert_eq!(
            request.ip_access_list[0].description.as_deref(),
            Some("Allow all (created by clickhousectl)")
        );
        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert!(request.idle_scaling.is_none());
        assert!(request.idle_timeout_minutes.is_none());
        assert!(request.backup_id.is_none());
        assert!(request.backup_encryption_config.is_none());
        assert!(request.release_channel.is_none());
        assert!(request.tags.is_none());
        assert!(request.data_warehouse_id.is_none());
        assert!(request.is_readonly.is_none());
        assert!(request.has_transparent_data_encryption.is_none());
        assert!(request.compliance_type.is_none());
        assert!(request.profile.is_none());
        assert!(request.private_preview_terms_checked.is_none());
        assert!(request.endpoints.is_none());
        assert!(request.enable_core_dumps.is_none());
        assert!(request.byoc_id.is_none());
    }

    #[test]
    fn build_create_service_request_supports_maximal_fields() {
        let options = CreateServiceOptions {
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            num_replicas: Some(3),
            autoscaling_mode: Some("vertical".to_string()),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(10),
            ip_allow: vec!["2001:db8::/32=office".to_string()],
            backup_id: Some("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6".to_string()),
            backup_encryption_config: Some(BackupEncryptionConfig::from([
                ("schema_version".to_string(), serde_json::json!(1)),
                (
                    "restore_key_pairs".to_string(),
                    serde_json::json!([{ "encrypted_dek": "wrapped" }]),
                ),
            ])),
            release_channel: Some("fast".to_string()),
            data_warehouse_id: Some("dw-1".to_string()),
            is_readonly: true,
            enable_tde: true,
            compliance_type: Some("hipaa".to_string()),
            profile: Some("v1-default".to_string()),
            tags: vec!["env=prod".to_string()],
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec!["mysql".to_string()],
            private_preview_terms_checked: true,
            enable_core_dumps: Some(true),
            ..minimal_create_options()
        };
        let request = build_create_service_request(&options).unwrap();

        assert_eq!(request.name, "svc");
        assert_eq!(request.provider, ServicePostRequestProvider::Aws);
        assert_eq!(request.region, ServicePostRequestRegion::Us_east_1);
        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert_eq!(request.min_replica_memory_gb, Some(24.0));
        assert_eq!(request.max_replica_memory_gb, Some(48.0));
        assert_eq!(request.num_replicas, Some(3));
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.idle_scaling, Some(true));
        assert_eq!(request.idle_timeout_minutes, Some(10.0));
        assert_eq!(request.ip_access_list.len(), 1);
        assert_eq!(request.ip_access_list[0].source, "2001:db8::/32");
        assert_eq!(
            request.ip_access_list[0].description.as_deref(),
            Some("office")
        );
        assert_eq!(
            request.backup_id,
            Some(uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap())
        );
        assert_eq!(
            serde_json::to_value(request.backup_encryption_config.as_ref().unwrap()).unwrap(),
            serde_json::json!({
                "schema_version": 1,
                "restore_key_pairs": [{ "encrypted_dek": "wrapped" }]
            })
        );
        assert_eq!(
            request.release_channel,
            Some(ServicePostRequestReleasechannel::Fast)
        );
        assert_eq!(request.data_warehouse_id.as_deref(), Some("dw-1"));
        assert_eq!(request.is_readonly, Some(true));
        assert_eq!(request.has_transparent_data_encryption, Some(true));
        assert_eq!(
            request.compliance_type,
            Some(ServicePostRequestCompliancetype::Hipaa)
        );
        assert_eq!(request.profile, Some(ServicePostRequestProfile::V1_default));
        let tags = request.tags.as_ref().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key, "env");
        assert_eq!(tags[0].value.as_deref(), Some("prod"));
        let endpoints = request.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(endpoints[0].enabled);
        assert_eq!(endpoints[1].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(!endpoints[1].enabled);
        assert_eq!(request.private_preview_terms_checked, Some(true));
        assert_eq!(request.enable_core_dumps, Some(true));
        assert!(request.byoc_id.is_none());
    }

    #[test]
    fn read_backup_encryption_config_passes_a_json_object_through_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("encryption_config.json");
        let document = serde_json::json!({
            "schema_version": 1,
            "restore_key_pairs": [{
                "customer_managed_encryption_key": { "aws_kms_key_arn": "arn:aws:kms:k" },
                "encrypted_dek": "d2VsbA==",
                "future_field": null
            }]
        });
        std::fs::write(&path, document.to_string()).unwrap();

        let config = read_backup_encryption_config(path.to_str().unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&config).unwrap(), document);
    }

    #[test]
    fn read_backup_encryption_config_rejects_bad_input_as_usage_errors() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.json");
        let array = directory.path().join("array.json");
        let invalid = directory.path().join("invalid.json");
        std::fs::write(&array, "[1, 2]").unwrap();
        std::fs::write(&invalid, "{not json").unwrap();

        for path in [&missing, &array, &invalid] {
            let error = read_backup_encryption_config(path.to_str().unwrap()).unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{path:?}: {error:?}"
            );
        }
    }

    #[test]
    fn build_create_service_request_accepts_a_dynamic_byoc_profile() {
        let request = build_create_service_request(&CreateServiceOptions {
            min_replica_memory_gb: Some(48),
            max_replica_memory_gb: Some(48),
            profile: Some("v1-standard-byoc-4".to_string()),
            byoc_id: Some("byoc-1".to_string()),
            ..minimal_create_options()
        })
        .unwrap();

        assert_eq!(request.byoc_id.as_deref(), Some("byoc-1"));
        assert_eq!(
            request.profile,
            Some(ServicePostRequestProfile::Unknown(
                "v1-standard-byoc-4".to_string()
            ))
        );
        assert_eq!(request.min_replica_memory_gb, Some(48.0));
        assert_eq!(request.max_replica_memory_gb, Some(48.0));
    }

    #[test]
    fn build_create_service_request_preserves_standard_profile_validation() {
        let standard = build_create_service_request(&CreateServiceOptions {
            profile: Some("v1-highmem-xs".to_string()),
            ..minimal_create_options()
        })
        .unwrap();
        assert_eq!(
            standard.profile,
            Some(ServicePostRequestProfile::V1_highmem_xs)
        );

        let error = build_create_service_request(&CreateServiceOptions {
            profile: Some("future-standard-profile".to_string()),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("dynamic profiles require --byoc-id")
        );
    }

    #[test]
    fn build_create_service_request_enforces_byoc_memory_constraints() {
        for options in [
            CreateServiceOptions {
                byoc_id: Some("byoc-1".to_string()),
                ..minimal_create_options()
            },
            CreateServiceOptions {
                byoc_id: Some("byoc-1".to_string()),
                min_replica_memory_gb: Some(48),
                ..minimal_create_options()
            },
            CreateServiceOptions {
                byoc_id: Some("byoc-1".to_string()),
                min_replica_memory_gb: Some(48),
                max_replica_memory_gb: Some(116),
                profile: Some("v1-standard-byoc-4".to_string()),
                ..minimal_create_options()
            },
        ] {
            assert!(build_create_service_request(&options).is_err());
        }
    }

    #[test]
    fn build_create_service_request_trims_tag_keys() {
        let request = build_create_service_request(&CreateServiceOptions {
            tags: vec![" env =prod".to_string()],
            ..minimal_create_options()
        })
        .unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["tags"][0]["key"], "env");
        assert_eq!(json["tags"][0]["value"], "prod");
    }

    #[test]
    fn build_create_service_request_rejects_empty_tag_keys() {
        let error = build_create_service_request(&CreateServiceOptions {
            tags: vec!["=prod".to_string()],
            ..minimal_create_options()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid tag '=prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_update_service_request_supports_minimal_fields() {
        let request = build_update_service_request(&ServiceUpdateOptions::default()).unwrap();

        assert!(request.name.is_none());
        assert!(request.ip_access_list.is_none());
        assert!(request.private_endpoint_ids.is_none());
        assert!(request.release_channel.is_none());
        assert!(request.endpoints.is_none());
        assert!(request.transparent_data_encryption_key_id.is_none());
        assert!(request.tags.is_none());
        assert!(request.enable_core_dumps.is_none());
    }

    #[test]
    fn build_update_service_request_supports_maximal_fields() {
        let options = ServiceUpdateOptions {
            name: Some("updated".to_string()),
            add_ip_allow: vec!["10.0.0.0/8=office".to_string()],
            remove_ip_allow: vec!["0.0.0.0/0".to_string()],
            add_private_endpoint_ids: vec!["pe-1".to_string()],
            remove_private_endpoint_ids: vec!["pe-2".to_string()],
            release_channel: Some("default".to_string()),
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec!["mysql".to_string()],
            transparent_data_encryption_key_id: Some("tde-1".to_string()),
            add_tags: vec!["env=staging".to_string()],
            remove_tags: vec!["old=tag".to_string()],
            enable_core_dumps: Some(false),
        };
        let request = build_update_service_request(&options).unwrap();

        assert_eq!(request.name.as_deref(), Some("updated"));
        let ip_access_list = request.ip_access_list.as_ref().unwrap();
        assert_eq!(ip_access_list.add.len(), 1);
        assert_eq!(ip_access_list.add[0].source, "10.0.0.0/8");
        assert_eq!(ip_access_list.add[0].description.as_deref(), Some("office"));
        assert_eq!(ip_access_list.remove.len(), 1);
        assert_eq!(ip_access_list.remove[0].source, "0.0.0.0/0");
        assert!(ip_access_list.remove[0].description.is_none());
        let private_endpoint_ids = request.private_endpoint_ids.as_ref().unwrap();
        assert_eq!(private_endpoint_ids.add, vec!["pe-1"]);
        assert_eq!(private_endpoint_ids.remove, vec!["pe-2"]);
        assert_eq!(
            request.release_channel,
            Some(ServicePatchRequestReleasechannel::Default)
        );
        let endpoints = request.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(endpoints[0].enabled);
        assert_eq!(endpoints[1].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(!endpoints[1].enabled);
        assert_eq!(
            request.transparent_data_encryption_key_id.as_deref(),
            Some("tde-1")
        );
        let tags = request.tags.as_ref().unwrap();
        assert_eq!(tags.add.len(), 1);
        assert_eq!(tags.add[0].key, "env");
        assert_eq!(tags.add[0].value.as_deref(), Some("staging"));
        assert_eq!(tags.remove.len(), 1);
        assert_eq!(tags.remove[0].key, "old");
        assert_eq!(tags.remove[0].value.as_deref(), Some("tag"));
        assert_eq!(request.enable_core_dumps, Some(false));
    }

    #[test]
    fn build_update_service_request_rejects_empty_tag_keys() {
        let error = build_update_service_request(&ServiceUpdateOptions {
            add_tags: vec![" =prod".to_string()],
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid tag ' =prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_update_service_request_rejects_invalid_release_channel() {
        let error = build_update_service_request(&ServiceUpdateOptions {
            release_channel: Some("turbo".to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn has_removals_is_false_when_no_remove_flags_set() {
        assert!(!has_removals(&ServiceUpdateOptions::default()));
        assert!(!has_removals(&ServiceUpdateOptions {
            add_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        }));
    }

    #[test]
    fn has_removals_is_true_when_any_remove_flag_set() {
        assert!(has_removals(&ServiceUpdateOptions {
            remove_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        }));
        assert!(has_removals(&ServiceUpdateOptions {
            remove_private_endpoint_ids: vec!["pe-1".to_string()],
            ..Default::default()
        }));
        assert!(has_removals(&ServiceUpdateOptions {
            remove_tags: vec!["env".to_string()],
            ..Default::default()
        }));
    }

    #[test]
    fn unmatched_returns_only_values_absent_from_current() {
        let current: HashSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(
            unmatched(&["a".to_string(), "c".to_string()], &current),
            vec!["c"]
        );
        assert!(unmatched(&["a".to_string(), "b".to_string()], &current).is_empty());
    }

    #[test]
    fn tag_key_strips_value_and_trims() {
        assert_eq!(tag_key("env"), "env");
        assert_eq!(tag_key("env=prod"), "env");
        assert_eq!(tag_key(" env = prod"), "env");
    }

    fn service_with_ip_allow_endpoints_and_tags() -> Service {
        Service {
            ip_access_list: Some(vec![IpAccessListEntryResponse {
                source: Some("10.0.0.0/8".to_string()),
                description: None,
            }]),
            private_endpoint_ids: Some(vec!["pe-1".to_string()]),
            tags: Some(vec![ResourceTagsV1Response {
                key: Some("env".to_string()),
                value: Some("prod".to_string()),
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn unmatched_removal_warnings_is_empty_when_everything_matches() {
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec![" 10.0.0.0/8 =retired office".to_string()],
            remove_private_endpoint_ids: vec!["pe-1".to_string()],
            remove_tags: vec!["env".to_string()],
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();
        assert!(unmatched_removal_warnings(&options, &request, &current).is_empty());
    }

    #[test]
    fn unmatched_removal_warnings_match_parsed_ipv4_and_ipv6_sources() {
        let current = Service {
            ip_access_list: Some(vec![
                IpAccessListEntryResponse {
                    source: Some("192.0.2.0/24".to_string()),
                    description: None,
                },
                IpAccessListEntryResponse {
                    source: Some("2001:db8::/32".to_string()),
                    description: None,
                },
            ]),
            ..Default::default()
        };
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec![
                " 192.0.2.0/24 =old office".to_string(),
                " 2001:db8::/32 =old ipv6 range".to_string(),
            ],
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();

        assert!(unmatched_removal_warnings(&options, &request, &current).is_empty());
    }

    #[test]
    fn unmatched_removal_warnings_names_every_unmatched_entry() {
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec![" 10.99.99.99/32 =old office".to_string()],
            remove_private_endpoint_ids: vec!["pe-missing".to_string()],
            remove_tags: vec!["missing=value".to_string()],
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();
        let warnings = unmatched_removal_warnings(&options, &request, &current);
        assert_eq!(warnings.len(), 3);
        assert!(
            warnings[0].contains("--remove-ip-allow 10.99.99.99/32")
                && warnings[0].contains("did not match"),
            "{warnings:?}"
        );
        assert!(
            warnings[1].contains("--remove-private-endpoint-id pe-missing"),
            "{warnings:?}"
        );
        assert!(warnings[2].contains("--remove-tag missing"), "{warnings:?}");
    }

    #[test]
    fn unmatched_removal_warnings_matches_tag_removal_by_key_only() {
        // Removing "env=staging" should still match a current "env=prod" tag:
        // the key is what the server matches on for removal.
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_tags: vec!["env=staging".to_string()],
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();
        assert!(unmatched_removal_warnings(&options, &request, &current).is_empty());
    }

    #[test]
    fn unmatched_removal_warnings_handles_empty_current_lists() {
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();
        let warnings = unmatched_removal_warnings(&options, &request, &Service::default());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("--remove-ip-allow 10.0.0.0/8"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_provider() {
        let error = build_create_service_request(&CreateServiceOptions {
            provider: "awss".to_string(),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("awss"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_region() {
        let error = build_create_service_request(&CreateServiceOptions {
            region: "us-east-99".to_string(),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("us-east-99"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_release_channel() {
        let error = build_create_service_request(&CreateServiceOptions {
            release_channel: Some("turbo".to_string()),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_autoscaling_mode() {
        let error = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("turbo".to_string()),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn build_create_service_request_supports_horizontal_fields() {
        let request = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..minimal_create_options()
        })
        .unwrap();
        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert!(request.num_replicas.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
    }

    #[test]
    fn build_create_service_request_replica_pair_without_mode_omits_mode() {
        let request = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(1),
            max_replicas: Some(4),
            ..minimal_create_options()
        })
        .unwrap();
        assert!(request.autoscaling_mode.is_none());
        assert_eq!(request.min_replicas, Some(1));
        assert_eq!(request.max_replicas, Some(4));
    }

    #[test]
    fn build_create_service_request_vertical_omits_horizontal_fields() {
        let request = build_create_service_request(&CreateServiceOptions {
            num_replicas: Some(3),
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            ..minimal_create_options()
        })
        .unwrap();
        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.num_replicas, Some(3));
        assert_eq!(request.min_replica_memory_gb, Some(24.0));
        assert_eq!(request.max_replica_memory_gb, Some(48.0));
    }

    #[test]
    fn build_create_service_request_rejects_min_without_max_replicas() {
        for options in [
            CreateServiceOptions {
                min_replicas: Some(2),
                ..minimal_create_options()
            },
            CreateServiceOptions {
                max_replicas: Some(8),
                ..minimal_create_options()
            },
        ] {
            let error = build_create_service_request(&options).unwrap_err();
            assert!(error.to_string().contains("--min-replicas"));
        }
    }

    #[test]
    fn resolve_horizontal_autoscaling_explicit_vertical_with_no_replicas() {
        let resolved = resolve_horizontal_autoscaling(Some("vertical"), None, None).unwrap();
        assert_eq!(resolved.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert!(resolved.min_replicas.is_none());
        assert!(resolved.max_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_minimal_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions::default()).unwrap();

        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert!(request.idle_scaling.is_none());
        assert!(request.idle_timeout_minutes.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_horizontal_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_maximal_vertical_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            autoscaling_mode: Some("vertical".to_string()),
            num_replicas: Some(3),
            min_replica_memory_gb: Some(8),
            max_replica_memory_gb: Some(32),
            idle_scaling: Some(false),
            idle_timeout_minutes: Some(15),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert_eq!(request.num_replicas, Some(3));
        assert_eq!(request.min_replica_memory_gb, Some(8.0));
        assert_eq!(request.max_replica_memory_gb, Some(32.0));
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.idle_scaling, Some(false));
        assert_eq!(request.idle_timeout_minutes, Some(15.0));
    }

    #[test]
    fn build_service_scale_request_supports_horizontal_memory_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            autoscaling_mode: Some("horizontal".to_string()),
            min_replicas: Some(2),
            max_replicas: Some(8),
            min_replica_memory_gb: Some(16),
            max_replica_memory_gb: Some(16),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert_eq!(request.min_replica_memory_gb, Some(16.0));
        assert_eq!(request.max_replica_memory_gb, Some(16.0));
        assert!(request.num_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_rejects_min_without_max_replicas() {
        let error = build_service_scale_request(&ServiceScaleOptions {
            max_replicas: Some(8),
            ..Default::default()
        })
        .unwrap_err();
        assert!(error.to_string().contains("--min-replicas"));
    }

    #[test]
    fn build_service_password_patch_request_supports_minimal_fields() {
        let request = build_service_password_patch_request(&ServiceResetPasswordOptions::default());

        assert!(request.new_password_hash.is_none());
        assert!(request.new_double_sha1_hash.is_none());
    }

    #[test]
    fn build_service_password_patch_request_supports_maximal_fields() {
        let request = build_service_password_patch_request(&ServiceResetPasswordOptions {
            new_password_hash: Some("sha256".to_string()),
            new_double_sha1_hash: Some("sha1".to_string()),
        });

        assert_eq!(request.new_password_hash.as_deref(), Some("sha256"));
        assert_eq!(request.new_double_sha1_hash.as_deref(), Some("sha1"));
    }

    #[test]
    fn build_service_state_patch_request_preserves_every_command() {
        let start = build_service_state_patch_request(ServiceStatePatchRequestCommand::Start);
        let stop = build_service_state_patch_request(ServiceStatePatchRequestCommand::Stop);
        let awake = build_service_state_patch_request(ServiceStatePatchRequestCommand::Awake);

        assert_eq!(start.command, Some(ServiceStatePatchRequestCommand::Start));
        assert_eq!(stop.command, Some(ServiceStatePatchRequestCommand::Stop));
        assert_eq!(awake.command, Some(ServiceStatePatchRequestCommand::Awake));
    }

    fn dotenv_test_service(endpoints: serde_json::Value) -> Service {
        serde_json::from_value(serde_json::json!({ "endpoints": endpoints })).unwrap()
    }

    #[test]
    fn service_dotenv_vars_reads_native_and_https_endpoints() {
        let service = dotenv_test_service(serde_json::json!([
            {"protocol": "https", "host": "abc.example.clickhouse.cloud", "port": 8443},
            {"protocol": "mysql", "host": "mysql.example", "port": 3306},
            {"protocol": "nativesecure", "host": "abc.example.clickhouse.cloud", "port": 9440}
        ]));
        assert_eq!(
            service_dotenv_vars(Some(&service), Some("secret")),
            vec![
                (
                    "CLICKHOUSE_HOST",
                    "abc.example.clickhouse.cloud".to_string()
                ),
                ("CLICKHOUSE_PORT", "9440".to_string()),
                ("CLICKHOUSE_HTTP_PORT", "8443".to_string()),
                ("CLICKHOUSE_USER", "default".to_string()),
                ("CLICKHOUSE_SECURE", "true".to_string()),
                ("CLICKHOUSE_PASSWORD", "secret".to_string()),
            ]
        );
    }

    #[test]
    fn service_dotenv_vars_leaves_out_what_the_response_lacks() {
        let https_only = dotenv_test_service(serde_json::json!([
            {"protocol": "https", "host": "abc.example", "port": 8443},
            {"protocol": "nativesecure", "host": "abc.example"}
        ]));
        assert_eq!(
            service_dotenv_vars(Some(&https_only), None),
            vec![
                ("CLICKHOUSE_HOST", "abc.example".to_string()),
                ("CLICKHOUSE_HTTP_PORT", "8443".to_string()),
                ("CLICKHOUSE_USER", "default".to_string()),
                ("CLICKHOUSE_SECURE", "true".to_string()),
            ]
        );

        let expected_bare = vec![
            ("CLICKHOUSE_USER", "default".to_string()),
            ("CLICKHOUSE_SECURE", "true".to_string()),
        ];
        assert_eq!(service_dotenv_vars(None, None), expected_bare);
        let no_endpoints: Service = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(
            service_dotenv_vars(Some(&no_endpoints), None),
            expected_bare
        );
    }

    #[test]
    fn check_dotenv_target_accepts_writable_files_and_rejects_bad_paths() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join(".env");
        std::fs::write(&existing, "KEEP=1\n").unwrap();

        check_dotenv_target(&existing).expect("existing file");
        check_dotenv_target(&dir.path().join("new.env")).expect("new file in existing directory");
        check_dotenv_target(Path::new(".env")).expect("bare file name in the current directory");
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "KEEP=1\n");
        assert!(!dir.path().join("new.env").exists());

        for path in [
            dir.path().to_path_buf(),
            dir.path().join("missing").join(".env"),
        ] {
            let error = check_dotenv_target(&path).expect_err("must be rejected");
            assert!(
                matches!(error.kind, crate::cloud::client::CloudErrorKind::Usage),
                "{path:?}"
            );
        }
    }
}
