use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::{eprint_line, or_absent, print_human};
use crate::cloud::shared::{parse_datetime, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use crate::failure::FailureStage;
use clap::Subcommand;
use clickhouse_cloud_api::models::{
    ApiKeyPatchRequest, ApiKeyPatchRequestState, ApiKeyPostRequest, ApiKeyPostRequestState,
    IpAccessListEntry,
};
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum KeyCommands {
    /// List API keys
    List {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create an API key
    Create {
        /// Key name
        #[arg(long)]
        name: String,

        /// Role UUID to assign (repeatable)
        #[arg(long)]
        role_id: Vec<String>,

        /// Expiry as RFC 3339 (e.g. 2025-12-31T23:59:59Z); omit for never
        #[arg(long, value_parser = parse_datetime)]
        expires_at: Option<String>,

        /// Key state (enabled or disabled)
        #[arg(long)]
        state: Option<String>,

        /// IP or CIDR allowed to use the key (repeatable)
        #[arg(long = "ip-allow")]
        ip_allow: Vec<String>,

        /// Pre-hashed key ID digest; needs --hash-key-id-suffix and --hash-key-secret
        #[arg(long)]
        hash_key_id: Option<String>,

        /// Suffix of the pre-hashed key ID; needs --hash-key-id and --hash-key-secret
        #[arg(long)]
        hash_key_id_suffix: Option<String>,

        /// Pre-hashed key secret digest; needs --hash-key-id and --hash-key-id-suffix
        #[arg(long)]
        hash_key_secret: Option<String>,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get API key details
    Get {
        /// API key ID
        key_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update an API key
    Update {
        /// API key ID
        key_id: String,

        /// New key name
        #[arg(long)]
        name: Option<String>,

        /// Role UUID to assign (repeatable; conflicts with --clear-roles)
        #[arg(long, conflicts_with = "clear_roles")]
        role_id: Vec<String>,

        /// Remove all assigned roles; conflicts with --role-id
        #[arg(long, conflicts_with = "role_id")]
        clear_roles: bool,

        /// New expiry as RFC 3339; conflicts with --clear-expiry
        #[arg(long, value_parser = parse_datetime, conflicts_with = "clear_expiry")]
        expires_at: Option<String>,

        /// Remove the expiry; conflicts with --expires-at
        #[arg(long, conflicts_with = "expires_at")]
        clear_expiry: bool,

        /// Key state (enabled or disabled)
        #[arg(long)]
        state: Option<String>,

        /// IP or CIDR to allow (repeatable; conflicts with --clear-ip-allow)
        #[arg(long = "ip-allow", conflicts_with = "clear_ip_allow")]
        ip_allow: Vec<String>,

        /// Clear the IP allowlist; conflicts with --ip-allow
        #[arg(long, conflicts_with = "ip_allow")]
        clear_ip_allow: bool,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete an API key
    Delete {
        /// API key ID
        key_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl KeyCommands {
    pub fn is_write(&self) -> bool {
        match self {
            KeyCommands::List { .. } => false,
            KeyCommands::Get { .. } => false,
            KeyCommands::Create { .. } => true,
            KeyCommands::Update { .. } => true,
            KeyCommands::Delete { .. } => true,
        }
    }
}

pub async fn run(client: &CloudClient, command: KeyCommands, json: bool) -> CloudResult<()> {
    match command {
        KeyCommands::List { org_id } => key_list(client, org_id.as_deref(), json).await,
        KeyCommands::Create {
            name,
            role_id,
            expires_at,
            state,
            ip_allow,
            hash_key_id,
            hash_key_id_suffix,
            hash_key_secret,
            org_id,
        } => {
            let options = KeyCreateOptions {
                name,
                role_ids: role_id,
                expires_at,
                state,
                ip_allow,
                hash_key_id,
                hash_key_id_suffix,
                hash_key_secret,
                org_id,
            };
            key_create(client, options, json).await
        }
        KeyCommands::Get { key_id, org_id } => {
            key_get(client, &key_id, org_id.as_deref(), json).await
        }
        KeyCommands::Update {
            key_id,
            name,
            role_id,
            clear_roles,
            expires_at,
            clear_expiry,
            state,
            ip_allow,
            clear_ip_allow,
            org_id,
        } => {
            let options = KeyUpdateOptions {
                name,
                role_ids: role_id,
                clear_roles,
                expires_at,
                clear_expiry,
                state,
                ip_allow,
                clear_ip_allow,
                org_id,
            };
            key_update(client, &key_id, options, json).await
        }
        KeyCommands::Delete { key_id, org_id } => {
            key_delete(client, &key_id, org_id.as_deref(), json).await
        }
    }
}

#[derive(Default)]
struct KeyCreateOptions {
    name: String,
    role_ids: Vec<String>,
    expires_at: Option<String>,
    state: Option<String>,
    ip_allow: Vec<String>,
    hash_key_id: Option<String>,
    hash_key_id_suffix: Option<String>,
    hash_key_secret: Option<String>,
    org_id: Option<String>,
}

#[derive(Default)]
struct KeyUpdateOptions {
    name: Option<String>,
    role_ids: Vec<String>,
    clear_roles: bool,
    expires_at: Option<String>,
    clear_expiry: bool,
    state: Option<String>,
    ip_allow: Vec<String>,
    clear_ip_allow: bool,
    org_id: Option<String>,
}

fn parse_api_key_hash_data(
    key_id_hash: Option<&str>,
    key_id_suffix: Option<&str>,
    key_secret_hash: Option<&str>,
) -> CloudResult<Option<clickhouse_cloud_api::models::ApiKeyHashData>> {
    match (key_id_hash, key_id_suffix, key_secret_hash) {
        (None, None, None) => Ok(None),
        (Some(key_id_hash), Some(key_id_suffix), Some(key_secret_hash)) => {
            Ok(Some(clickhouse_cloud_api::models::ApiKeyHashData {
                key_id_hash: key_id_hash.to_string(),
                key_id_suffix: key_id_suffix.to_string(),
                key_secret_hash: key_secret_hash.to_string(),
            }))
        }
        _ => Err(CloudError::new(
            "pre-hashed API key input requires --hash-key-id, --hash-key-id-suffix, and --hash-key-secret together",
        )),
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

fn parse_uuid_list(values: &[String], field: &str) -> CloudResult<Vec<uuid::Uuid>> {
    values
        .iter()
        .map(|value| {
            uuid::Uuid::parse_str(value).map_err(|error| {
                CloudError::new(format!("invalid {} UUID '{}': {}", field, value, error))
            })
        })
        .collect()
}

fn parse_api_key_state_post(value: &str) -> CloudResult<ApiKeyPostRequestState> {
    match value {
        "enabled" => Ok(ApiKeyPostRequestState::Enabled),
        "disabled" => Ok(ApiKeyPostRequestState::Disabled),
        _ => Err(CloudError::new(format!(
            "invalid state: unknown value '{}', expected one of: enabled, disabled",
            value
        ))),
    }
}

fn parse_api_key_state_patch(value: &str) -> CloudResult<ApiKeyPatchRequestState> {
    match value {
        "enabled" => Ok(ApiKeyPatchRequestState::Enabled),
        "disabled" => Ok(ApiKeyPatchRequestState::Disabled),
        _ => Err(CloudError::new(format!(
            "invalid state: unknown value '{}', expected one of: enabled, disabled",
            value
        ))),
    }
}

fn parse_expire_at(value: &str) -> CloudResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&chrono::Utc))
        .map_err(|error| {
            CloudError::new(format!(
                "invalid expire_at '{}': expected ISO 8601 / RFC 3339 format (e.g. 2025-12-31T23:59:59Z): {}",
                value, error
            ))
        })
}

fn build_api_key_create_request(options: &KeyCreateOptions) -> CloudResult<ApiKeyPostRequest> {
    Ok(ApiKeyPostRequest {
        name: options.name.clone(),
        expire_at: options
            .expires_at
            .as_deref()
            .map(parse_expire_at)
            .transpose()?,
        state: match options.state.as_deref() {
            Some(value) => parse_api_key_state_post(value)?,
            None => ApiKeyPostRequestState::default(),
        },
        assigned_role_ids: parse_uuid_list(&options.role_ids, "role_id")?,
        ip_access_list: parse_ip_access_entries(&options.ip_allow).unwrap_or_default(),
        hash_data: parse_api_key_hash_data(
            options.hash_key_id.as_deref(),
            options.hash_key_id_suffix.as_deref(),
            options.hash_key_secret.as_deref(),
        )?,
        #[cfg(feature = "deprecated-fields")]
        roles: None,
    })
}

fn build_api_key_update_request(options: &KeyUpdateOptions) -> CloudResult<ApiKeyPatchRequest> {
    if options.clear_expiry && options.expires_at.is_some() {
        return Err(CloudError::new(
            "--clear-expiry conflicts with --expires-at",
        ));
    }
    Ok(ApiKeyPatchRequest {
        name: options.name.clone(),
        assigned_role_ids: if options.clear_roles {
            Some(Vec::new())
        } else if options.role_ids.is_empty() {
            None
        } else {
            Some(parse_uuid_list(&options.role_ids, "role_id")?)
        },
        expire_at: if options.clear_expiry {
            Some(None)
        } else {
            options
                .expires_at
                .as_deref()
                .map(parse_expire_at)
                .transpose()?
                .map(Some)
        },
        state: options
            .state
            .as_deref()
            .map(parse_api_key_state_patch)
            .transpose()?,
        ip_access_list: if options.clear_ip_allow {
            Some(Vec::new())
        } else {
            parse_ip_access_entries(&options.ip_allow)
        },
        #[cfg(feature = "deprecated-fields")]
        roles: None,
    })
}

async fn key_list(client: &CloudClient, org_id: Option<&str>, json: bool) -> CloudResult<()> {
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
            .map(|key| Row {
                name: or_absent(key.name.as_deref()),
                id: or_absent(key.id),
                state: or_absent(key.state.as_ref()),
                expires: key
                    .expire_at
                    .map(|time| time.to_rfc3339())
                    .unwrap_or_else(|| "never".into()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

#[derive(Debug)]
enum KeyCreateMaterial<'a> {
    Generated {
        key_id: &'a str,
        key_secret: &'a str,
    },
    PreHashed,
}

fn resolve_key_create_material<'a>(
    pre_hashed: bool,
    key_id: Option<&'a str>,
    key_secret: Option<&'a str>,
    key_name: Option<&str>,
) -> CloudResult<KeyCreateMaterial<'a>> {
    if pre_hashed {
        return Ok(KeyCreateMaterial::PreHashed);
    }
    match (key_id, key_secret) {
        (Some(key_id), Some(key_secret)) => Ok(KeyCreateMaterial::Generated { key_id, key_secret }),
        _ => {
            let named = match key_name {
                Some(name) => format!(" '{}'", name),
                None => String::new(),
            };
            Err(CloudError::new(format!(
                "the API response omitted the generated key material, so the one-time key secret \
                 cannot be shown: the key{} may still have been created — list the organization's \
                 keys and delete it if so",
                named
            )))
        }
    }
}

async fn key_create(
    client: &CloudClient,
    options: KeyCreateOptions,
    json: bool,
) -> CloudResult<()> {
    // Validate before organization resolution so malformed inputs make no network call.
    let request = build_api_key_create_request(&options)?;
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let response = client.create_api_key(&org_id, &request).await?;

    let name = response.key.as_ref().and_then(|key| key.name.as_deref());
    // Validate before either output branch: generated material is returned only once.
    let material = resolve_key_create_material(
        request.hash_data.is_some(),
        response.key_id.as_deref(),
        response.key_secret.as_deref(),
        name,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
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

async fn key_get(
    client: &CloudClient,
    key_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let key = client.get_api_key(&org_id, key_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&key)?);
    } else {
        print_human(&key)?;
    }
    Ok(())
}

async fn key_update(
    client: &CloudClient,
    key_id: &str,
    options: KeyUpdateOptions,
    json: bool,
) -> CloudResult<()> {
    // Validate before organization resolution so malformed inputs make no network call.
    let request = build_api_key_update_request(&options)?;
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
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

async fn key_delete(
    client: &CloudClient,
    key_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client.delete_api_key(&org_id, key_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("API key {} deleted", key_id);
    }
    Ok(())
}

/// Return query API keys only when their exact resource and organization IDs
/// were saved during provisioning or repair. The boolean indicates that
/// partial cleanup metadata must remain on disk because discarding it would
/// lose the key ID.
pub(crate) fn service_query_key_cleanup(
    org_id: &str,
    service_id: &str,
) -> CloudResult<(Vec<String>, bool)> {
    let Some(key) = credentials::try_get_service_query_key(service_id)? else {
        return Ok((vec![], false));
    };
    let Some(api_key_id) = key.api_key_id else {
        // `eprint_line`, not `eprintln!`: a warning on the way to deleting a
        // service must not panic on a closed stderr (#598).
        eprint_line(format!(
            "Warning: the stored query key for service {service_id} predates exact management \
             API key IDs; service deletion will continue without unsafe cloud key cleanup."
        ));
        return Ok((vec![], false));
    };
    let Some(key_org_id) = key.organization_id else {
        eprint_line(format!(
            "Warning: the stored query key for service {service_id} has a management API key ID \
             but no provisioning organization; cloud key cleanup was skipped and the local \
             record was retained."
        ));
        return Ok((vec![], true));
    };
    if key_org_id != org_id {
        return Err(CloudError::new(format!(
            "the stored query key for service {service_id} belongs to organization {key_org_id}, \
             not {org_id}; refusing to delete either resource"
        )));
    }
    let mut api_key_ids = key.pending_cleanup_api_key_ids;
    if !api_key_ids.iter().any(|key_id| key_id == &api_key_id) {
        api_key_ids.push(api_key_id);
    }
    Ok((api_key_ids, false))
}

/// Delete every owned query API key of a deleted service: the current key and
/// any retired key whose deletion is still pending (#527). Every key is
/// attempted, so one failure does not leave the others behind; the failures
/// are then reported together. The caller keeps the local record on failure,
/// so the exact IDs remain available for manual deletion.
pub(crate) async fn cleanup_service_query_key(
    client: &CloudClient,
    org_id: &str,
    service_id: &str,
    api_key_ids: &[String],
) -> CloudResult<()> {
    let mut failures: Vec<(String, CloudError)> = vec![];
    for api_key_id in api_key_ids {
        if let Err(error) = client
            .delete_api_key_if_exists(org_id, api_key_id)
            .await
            .map_err(|error| error.at_stage(FailureStage::KeyDelete))
        {
            failures.push((api_key_id.clone(), error));
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    let detail = failures
        .iter()
        .map(|(api_key_id, error)| format!("{api_key_id}: {}", error.message))
        .collect::<Vec<_>>()
        .join("; ");
    let noun = if failures.len() == 1 {
        "query API key"
    } else {
        "query API keys"
    };
    // The first failure's classification stands for the whole cleanup.
    let (_, first) = failures.swap_remove(0);
    Err(CloudError {
        message: format!(
            "failed to delete the auto-provisioned {noun} for service {service_id} ({detail}). The \
             local record was kept so the exact IDs are not lost; delete each key with \
             `clickhousectl cloud key delete <key-id> --org-id {org_id}`"
        ),
        ..first
    })
}

impl CloudClient {
    pub async fn list_api_keys(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::ApiKey>> {
        let response = self
            .api()
            .openapi_key_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_api_key(
        &self,
        org_id: &str,
        request: &ApiKeyPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ApiKeyPostResponse> {
        let response = self
            .api()
            .openapi_key_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_api_key(
        &self,
        org_id: &str,
        key_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ApiKey> {
        let response = self
            .api()
            .openapi_key_get(org_id, key_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_api_key(
        &self,
        org_id: &str,
        key_id: &str,
        request: &ApiKeyPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ApiKey> {
        let response = self
            .api()
            .openapi_key_update(org_id, key_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_api_key(
        &self,
        org_id: &str,
        key_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .openapi_key_delete(org_id, key_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn delete_api_key_if_exists(
        &self,
        org_id: &str,
        key_id: &str,
    ) -> crate::cloud::client::Result<Option<DeleteResponse>> {
        match self.api().openapi_key_delete(org_id, key_id).await {
            Ok(response) => Ok(Some(DeleteResponse {
                status: response.status,
                request_id: response.request_id,
            })),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[derive(Parser)]
    struct KeyCli {
        #[command(subcommand)]
        command: KeyCommands,
    }

    fn parse_key(args: &[&str]) -> KeyCommands {
        assert_eq!(args.get(1), Some(&"cloud"));
        assert_eq!(args.get(2), Some(&"key"));
        KeyCli::try_parse_from(std::iter::once(args[0]).chain(args.iter().skip(3).copied()))
            .expect("parse")
            .command
    }

    fn parse_top_level_key(args: &[&str]) -> KeyCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Key { command } = cloud_args.command else {
            panic!("expected key command");
        };
        command
    }

    #[test]
    fn parses_key_body_command_defaults() {
        let KeyCommands::Create {
            name,
            role_id,
            expires_at,
            state,
            ip_allow,
            hash_key_id,
            hash_key_id_suffix,
            hash_key_secret,
            org_id,
        } = parse_top_level_key(&[
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "ci-key",
        ])
        else {
            panic!("expected key create");
        };
        assert_eq!(name, "ci-key");
        assert!(role_id.is_empty());
        assert!(expires_at.is_none());
        assert!(state.is_none());
        assert!(ip_allow.is_empty());
        assert!(hash_key_id.is_none());
        assert!(hash_key_id_suffix.is_none());
        assert!(hash_key_secret.is_none());
        assert!(org_id.is_none());

        let KeyCommands::Update {
            key_id,
            name,
            role_id,
            clear_roles,
            expires_at,
            clear_expiry,
            state,
            ip_allow,
            clear_ip_allow,
            org_id,
        } = parse_top_level_key(&["clickhousectl", "cloud", "key", "update", "key-1"])
        else {
            panic!("expected key update");
        };
        assert_eq!(key_id, "key-1");
        assert!(name.is_none());
        assert!(role_id.is_empty());
        assert!(!clear_roles);
        assert!(expires_at.is_none());
        assert!(!clear_expiry);
        assert!(state.is_none());
        assert!(ip_allow.is_empty());
        assert!(!clear_ip_allow);
        assert!(org_id.is_none());
    }

    #[test]
    fn parses_key_create_flags() {
        let command = parse_top_level_key(&[
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "ci-key",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--ip-allow",
            "10.0.0.0/8",
            "--ip-allow",
            "192.0.2.0/24",
            "--hash-key-id",
            "id-hash",
            "--hash-key-id-suffix",
            "abcd",
            "--hash-key-secret",
            "secret-hash",
            "--expires-at",
            "2025-12-31T23:59:59Z",
            "--state",
            "disabled",
            "--org-id",
            "org-1",
        ]);

        let KeyCommands::Create {
            name,
            role_id,
            expires_at,
            state,
            ip_allow,
            hash_key_id,
            hash_key_id_suffix,
            hash_key_secret,
            org_id,
        } = command
        else {
            panic!("expected key create");
        };
        assert_eq!(name, "ci-key");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert_eq!(expires_at.as_deref(), Some("2025-12-31T23:59:59Z"));
        assert_eq!(state.as_deref(), Some("disabled"));
        assert_eq!(ip_allow, vec!["10.0.0.0/8", "192.0.2.0/24"]);
        assert_eq!(hash_key_id.as_deref(), Some("id-hash"));
        assert_eq!(hash_key_id_suffix.as_deref(), Some("abcd"));
        assert_eq!(hash_key_secret.as_deref(), Some("secret-hash"));
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_key_update_flags() {
        let command = parse_top_level_key(&[
            "clickhousectl",
            "cloud",
            "key",
            "update",
            "key-1",
            "--name",
            "renamed",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--expires-at",
            "2025-01-01T00:00:00Z",
            "--state",
            "enabled",
            "--ip-allow",
            "10.0.0.0/8",
            "--ip-allow",
            "192.0.2.0/24",
            "--org-id",
            "org-1",
        ]);

        let KeyCommands::Update {
            key_id,
            name,
            role_id,
            clear_roles,
            expires_at,
            clear_expiry,
            state,
            ip_allow,
            clear_ip_allow,
            org_id,
        } = command
        else {
            panic!("expected key update");
        };
        assert_eq!(key_id, "key-1");
        assert_eq!(name.as_deref(), Some("renamed"));
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert!(!clear_roles);
        assert_eq!(expires_at.as_deref(), Some("2025-01-01T00:00:00Z"));
        assert!(!clear_expiry);
        assert_eq!(state.as_deref(), Some("enabled"));
        assert_eq!(ip_allow, vec!["10.0.0.0/8", "192.0.2.0/24"]);
        assert!(!clear_ip_allow);
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_key_update_list_clear_flags() {
        let command = parse_top_level_key(&[
            "clickhousectl",
            "cloud",
            "key",
            "update",
            "key-1",
            "--clear-roles",
            "--clear-ip-allow",
        ]);
        let KeyCommands::Update {
            role_id,
            clear_roles,
            ip_allow,
            clear_ip_allow,
            ..
        } = command
        else {
            panic!("expected key update");
        };
        assert!(role_id.is_empty());
        assert!(clear_roles);
        assert!(ip_allow.is_empty());
        assert!(clear_ip_allow);
    }

    #[test]
    fn rejects_conflicting_key_update_list_flags() {
        for flags in [
            ["--role-id", "role-1", "--clear-roles"],
            ["--clear-roles", "--role-id", "role-1"],
            ["--ip-allow", "10.0.0.0/8", "--clear-ip-allow"],
            ["--clear-ip-allow", "--ip-allow", "10.0.0.0/8"],
        ] {
            let result = Cli::try_parse_from(
                ["clickhousectl", "cloud", "key", "update", "key-1"]
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
    fn parses_key_expires_at_rfc3339_timestamps() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "ci-key",
            "--expires-at",
            "2025-12-31T23:59:59Z",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Key { command } = args.command else {
            panic!("expected key command");
        };
        let crate::cloud::cli::KeyCommands::Create { expires_at, .. } = command else {
            panic!("expected key create");
        };
        assert_eq!(expires_at.as_deref(), Some("2025-12-31T23:59:59Z"));
    }

    #[test]
    fn rejects_invalid_key_expires_at_timestamps() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "update",
            "key-1",
            "--expires-at",
            "2025-12-31",
        ]);

        match result {
            Ok(_) => panic!("expected invalid expires-at input to be rejected"),
            Err(error) => assert!(error.to_string().contains("expected ISO 8601 / RFC 3339")),
        }
    }

    #[test]
    fn parses_key_update_clear_expiry_as_a_write() {
        let command = parse_top_level_key(&[
            "clickhousectl",
            "cloud",
            "key",
            "update",
            "key-1",
            "--clear-expiry",
        ]);
        assert!(command.is_write());
        let KeyCommands::Update {
            clear_expiry,
            expires_at,
            ..
        } = command
        else {
            panic!("expected key update");
        };
        assert!(clear_expiry);
        assert!(expires_at.is_none());
    }

    #[test]
    fn rejects_conflicting_key_update_expiry_flags() {
        for flags in [
            ["--clear-expiry", "--expires-at", "2030-01-01T00:00:00Z"],
            ["--expires-at", "2030-01-01T00:00:00Z", "--clear-expiry"],
        ] {
            let result = Cli::try_parse_from(
                ["clickhousectl", "cloud", "key", "update", "key-1"]
                    .into_iter()
                    .chain(flags),
            );
            let Err(error) = result else {
                panic!("expected conflicting arguments");
            };
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn key_create_does_not_accept_clear_expiry() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "key",
            "--clear-expiry",
        ]);
        let Err(error) = result else {
            panic!("expected unknown argument");
        };
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn build_api_key_update_request_clears_only_expiry() {
        let request = build_api_key_update_request(&KeyUpdateOptions {
            clear_expiry: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            request,
            ApiKeyPatchRequest {
                expire_at: Some(None),
                ..Default::default()
            }
        );
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"expireAt": null})
        );
    }

    #[test]
    fn build_api_key_update_request_clears_lists_explicitly() {
        let request = build_api_key_update_request(&KeyUpdateOptions {
            clear_roles: true,
            clear_ip_allow: true,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.assigned_role_ids, Some(Vec::new()));
        assert_eq!(request.ip_access_list, Some(Vec::new()));
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"assignedRoleIds": [], "ipAccessList": []})
        );
    }

    #[test]
    fn build_api_key_update_request_combines_clear_with_explicit_changes() {
        let role_id = uuid::Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let request = build_api_key_update_request(&KeyUpdateOptions {
            name: Some("renamed".into()),
            role_ids: vec![role_id.to_string()],
            clear_expiry: true,
            state: Some("disabled".into()),
            ip_allow: vec!["10.0.0.0/8".into()],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(request.expire_at, Some(None));
        assert_eq!(request.name.as_deref(), Some("renamed"));
        assert_eq!(request.assigned_role_ids, Some(vec![role_id]));
        assert_eq!(request.state, Some(ApiKeyPatchRequestState::Disabled));
        assert_eq!(
            request.ip_access_list,
            Some(vec![IpAccessListEntry {
                source: "10.0.0.0/8".into(),
                description: None,
            }])
        );
    }

    #[test]
    fn build_api_key_update_request_rejects_conflicting_expiry_changes() {
        assert!(
            build_api_key_update_request(&KeyUpdateOptions {
                expires_at: Some("2030-01-01T00:00:00Z".into()),
                clear_expiry: true,
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn key_write_classification_is_exhaustive() {
        assert!(!parse_key(&["clickhousectl", "cloud", "key", "list"]).is_write());
        assert!(!parse_key(&["clickhousectl", "cloud", "key", "get", "key-1"]).is_write());
        assert!(
            parse_key(&["clickhousectl", "cloud", "key", "create", "--name", "key",]).is_write()
        );
        assert!(parse_key(&["clickhousectl", "cloud", "key", "update", "key-1",]).is_write());
        assert!(parse_key(&["clickhousectl", "cloud", "key", "delete", "key-1"]).is_write());
    }

    #[test]
    fn build_api_key_requests_support_minimal_inputs() {
        let create = build_api_key_create_request(&KeyCreateOptions {
            name: "ci-key".to_string(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(create.name, "ci-key");
        assert!(create.assigned_role_ids.is_empty());
        assert!(create.expire_at.is_none());
        assert!(create.hash_data.is_none());
        assert!(create.ip_access_list.is_empty());
        assert_eq!(create.state, ApiKeyPostRequestState::Enabled);
        #[cfg(feature = "deprecated-fields")]
        assert!(create.roles.is_none());

        let update = build_api_key_update_request(&KeyUpdateOptions::default()).unwrap();
        assert!(update.name.is_none());
        assert!(update.assigned_role_ids.is_none());
        assert!(update.expire_at.is_none());
        assert!(update.state.is_none());
        assert!(update.ip_access_list.is_none());
        #[cfg(feature = "deprecated-fields")]
        assert!(update.roles.is_none());
    }

    #[test]
    fn build_api_key_requests_support_maximal_inputs() {
        let role_id = "a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6";
        let create = build_api_key_create_request(&KeyCreateOptions {
            name: "ci-key".to_string(),
            role_ids: vec![role_id.to_string()],
            expires_at: Some("2025-12-31T23:59:59Z".to_string()),
            state: Some("disabled".to_string()),
            ip_allow: vec!["10.0.0.0/8".to_string()],
            hash_key_id: Some("id-hash".to_string()),
            hash_key_id_suffix: Some("abcd".to_string()),
            hash_key_secret: Some("secret-hash".to_string()),
            org_id: None,
        })
        .unwrap();
        let expected_role_id = uuid::Uuid::parse_str(role_id).unwrap();
        let expected_create_expiration =
            chrono::DateTime::parse_from_rfc3339("2025-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
        assert_eq!(create.name, "ci-key");
        assert_eq!(create.assigned_role_ids, vec![expected_role_id]);
        assert_eq!(create.expire_at, Some(expected_create_expiration));
        assert_eq!(create.state, ApiKeyPostRequestState::Disabled);
        assert_eq!(create.ip_access_list.len(), 1);
        assert_eq!(create.ip_access_list[0].source, "10.0.0.0/8");
        assert!(create.ip_access_list[0].description.is_none());
        let hash_data = create.hash_data.as_ref().expect("maximal hash data");
        assert_eq!(hash_data.key_id_hash, "id-hash");
        assert_eq!(hash_data.key_id_suffix, "abcd");
        assert_eq!(hash_data.key_secret_hash, "secret-hash");
        #[cfg(feature = "deprecated-fields")]
        assert!(create.roles.is_none());

        let update = build_api_key_update_request(&KeyUpdateOptions {
            name: Some("renamed".to_string()),
            role_ids: vec![role_id.to_string()],
            clear_roles: false,
            expires_at: Some("2025-01-01T00:00:00Z".to_string()),
            clear_expiry: false,
            state: Some("disabled".to_string()),
            ip_allow: vec!["0.0.0.0/0".to_string()],
            clear_ip_allow: false,
            org_id: None,
        })
        .unwrap();
        let expected_update_expiration =
            chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
        assert_eq!(update.name.as_deref(), Some("renamed"));
        assert_eq!(update.assigned_role_ids, Some(vec![expected_role_id]));
        assert_eq!(update.expire_at, Some(Some(expected_update_expiration)));
        assert_eq!(update.state, Some(ApiKeyPatchRequestState::Disabled));
        let ip_access_list = update.ip_access_list.as_ref().unwrap();
        assert_eq!(ip_access_list.len(), 1);
        assert_eq!(ip_access_list[0].source, "0.0.0.0/0");
        assert!(ip_access_list[0].description.is_none());
        #[cfg(feature = "deprecated-fields")]
        assert!(update.roles.is_none());
    }

    #[test]
    fn build_api_key_create_request_rejects_invalid_uuid() {
        let options = KeyCreateOptions {
            name: "ci-key".to_string(),
            role_ids: vec!["not-a-uuid".to_string()],
            ..Default::default()
        };
        let error = build_api_key_create_request(&options).unwrap_err();
        assert!(error.to_string().contains("not-a-uuid"));
    }

    #[test]
    fn build_api_key_create_request_rejects_invalid_expire_at() {
        let options = KeyCreateOptions {
            name: "ci-key".to_string(),
            expires_at: Some("next-tuesday".to_string()),
            ..Default::default()
        };
        let error = build_api_key_create_request(&options).unwrap_err();
        assert!(error.to_string().contains("next-tuesday"));
    }

    #[test]
    fn build_api_key_requests_reject_invalid_states() {
        let create_error = build_api_key_create_request(&KeyCreateOptions {
            name: "ci-key".to_string(),
            state: Some("broken".to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(create_error.to_string().contains("broken"));

        let update_error = build_api_key_update_request(&KeyUpdateOptions {
            state: Some("broken".to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(update_error.to_string().contains("broken"));
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
        assert!(matches!(
            resolve_key_create_material(true, Some("key-id"), None, None).unwrap(),
            KeyCreateMaterial::PreHashed
        ));
    }

    #[test]
    fn resolve_key_create_material_fails_when_generated_material_is_absent() {
        let error = resolve_key_create_material(false, None, Some("key-secret"), Some("ci"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("omitted the generated key material"));
        assert!(error.contains("the key 'ci' may still have been created"));

        let material = resolve_key_create_material(false, Some(""), Some(""), Some("ci")).unwrap();
        assert!(matches!(material, KeyCreateMaterial::Generated { .. }));

        let error = resolve_key_create_material(false, Some("key-id"), None, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("the key may still have been created"));
    }
}
