use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::services::query_key::existing_open_api_keys;
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use crate::failure::FailureStage;
use clickhouse_cloud_api::models::{
    InstanceServiceQueryApiEndpointsPostRequest, QueryEndpointRole,
    ServicPrivateEndpointePostRequest, ServiceQueryAPIEndpoint,
};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct QueryEndpointCreateOptions {
    pub(super) roles: Vec<String>,
    pub(super) open_api_keys: Vec<String>,
    pub(super) replace_open_api_keys: bool,
    pub(super) allowed_origins: Option<String>,
}

fn build_query_endpoint_create_request(
    options: &QueryEndpointCreateOptions,
    existing: Option<&ServiceQueryAPIEndpoint>,
) -> CloudResult<InstanceServiceQueryApiEndpointsPostRequest> {
    if options.roles.is_empty() {
        return Err(CloudError::new("at least one --role is required"));
    }
    let mut open_api_keys = match existing {
        Some(endpoint) if !options.replace_open_api_keys => {
            existing_open_api_keys(endpoint.clone())
                .map_err(|error| error.at_stage(FailureStage::EndpointGet))?
        }
        _ => Vec::new(),
    };
    open_api_keys.extend(options.open_api_keys.iter().cloned());
    let mut seen = HashSet::new();
    open_api_keys.retain(|key| seen.insert(key.clone()));

    // `allowedOrigins` is optional on both sides of the round trip, so an
    // existing endpoint that reports none is written back without one rather
    // than refused or given a default.
    let allowed_origins = match (&options.allowed_origins, existing) {
        (Some(origins), _) => Some(origins.clone()),
        (None, Some(endpoint)) => endpoint.allowed_origins.clone(),
        (None, None) => {
            return Err(CloudError::new(
                "--allowed-origins is required when creating a new query endpoint; \
                 pass browser origins or '*' explicitly to allow all origins",
            ));
        }
    };
    Ok(InstanceServiceQueryApiEndpointsPostRequest {
        roles: options
            .roles
            .iter()
            .map(|role| parse_serde_enum(role, "role", QueryEndpointRole::VALUES))
            .collect::<Result<_, _>>()?,
        open_api_keys,
        allowed_origins,
    })
}

fn build_private_endpoint_create_request(
    endpoint_id: &str,
    description: Option<&str>,
) -> ServicPrivateEndpointePostRequest {
    ServicPrivateEndpointePostRequest {
        id: endpoint_id.to_string(),
        // The spec makes `description` optional; keep sending `""` when the
        // flag is absent so the create request is unchanged.
        description: Some(description.map(String::from).unwrap_or_default()),
    }
}

pub(super) async fn query_endpoint_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let endpoint = client.get_query_endpoint(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        print_human(&endpoint)?;
    }
    Ok(())
}

pub(super) async fn query_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    options: QueryEndpointCreateOptions,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let existing = client
        .get_query_endpoint_for_binding(&org_id, service_id)
        .await
        .map_err(|error| error.at_stage(FailureStage::EndpointGet))?;
    let request = build_query_endpoint_create_request(&options, existing.as_ref())?;
    let endpoint = client
        .create_query_endpoint(&org_id, service_id, &request)
        .await
        .map_err(|error| error.at_stage(FailureStage::EndpointUpsert))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        print_human(&endpoint)?;
    }
    Ok(())
}

pub(super) async fn query_endpoint_delete(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_query_endpoint(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Query endpoint deleted for service {}", service_id);
    }
    Ok(())
}

pub(super) async fn private_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    description: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let request = build_private_endpoint_create_request(endpoint_id, description);
    let endpoint = client
        .create_private_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        println!("Private endpoint created for service {}", service_id);
        println!("  Endpoint ID: {}", or_absent(endpoint.id.as_deref()));
        println!(
            "  Description: {}",
            or_absent(endpoint.description.as_deref())
        );
    }
    Ok(())
}

pub(super) async fn private_endpoint_get_config(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_endpoint_create_request_supports_minimal_fields() {
        let request = build_query_endpoint_create_request(
            &QueryEndpointCreateOptions {
                roles: vec!["sql_console_read_only".into()],
                allowed_origins: Some("https://example.com".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();

        assert_eq!(request.roles, [QueryEndpointRole::SqlConsoleReadOnly]);
        assert!(request.open_api_keys.is_empty());
        assert_eq!(
            request.allowed_origins.as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn build_query_endpoint_create_request_supports_maximal_fields() {
        let request = build_query_endpoint_create_request(
            &QueryEndpointCreateOptions {
                roles: vec!["sql_console_read_only".into(), "sql_console_admin".into()],
                open_api_keys: vec!["key-1".into(), "key-2".into(), "key-1".into()],
                replace_open_api_keys: true,
                allowed_origins: Some("*".into()),
            },
            Some(&ServiceQueryAPIEndpoint {
                open_api_keys: Some(vec!["old-key".into()]),
                allowed_origins: Some("https://before.example.com".into()),
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(
            request.roles,
            [
                QueryEndpointRole::SqlConsoleReadOnly,
                QueryEndpointRole::SqlConsoleAdmin
            ]
        );
        assert_eq!(request.open_api_keys, ["key-1", "key-2"]);
        assert_eq!(request.allowed_origins.as_deref(), Some("*"));
    }

    #[test]
    fn build_query_endpoint_create_request_merges_keys_and_preserves_origins() {
        let request = build_query_endpoint_create_request(
            &QueryEndpointCreateOptions {
                roles: vec!["sql_console_read_only".into()],
                open_api_keys: vec!["key-2".into(), "key-3".into(), "key-3".into()],
                ..Default::default()
            },
            Some(&ServiceQueryAPIEndpoint {
                roles: Some(vec![QueryEndpointRole::SqlConsoleAdmin]),
                open_api_keys: Some(vec!["key-1".into(), "key-2".into(), "key-1".into()]),
                allowed_origins: Some("https://before.example.com".into()),
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(request.roles, [QueryEndpointRole::SqlConsoleReadOnly]);
        assert_eq!(request.open_api_keys, ["key-1", "key-2", "key-3"]);
        assert_eq!(
            request.allowed_origins.as_deref(),
            Some("https://before.example.com")
        );
    }

    #[test]
    fn build_query_endpoint_create_request_requires_roles_and_first_origins() {
        assert!(
            build_query_endpoint_create_request(&QueryEndpointCreateOptions::default(), None)
                .is_err()
        );
        assert!(
            build_query_endpoint_create_request(
                &QueryEndpointCreateOptions {
                    roles: vec!["sql_console_read_only".into()],
                    ..Default::default()
                },
                None
            )
            .is_err()
        );
    }

    #[test]
    fn build_query_endpoint_create_request_preserves_explicitly_empty_state() {
        let request = build_query_endpoint_create_request(
            &QueryEndpointCreateOptions {
                roles: vec!["sql_console_read_only".into()],
                ..Default::default()
            },
            Some(&ServiceQueryAPIEndpoint {
                open_api_keys: Some(vec![]),
                allowed_origins: Some(String::new()),
                ..Default::default()
            }),
        )
        .unwrap();
        assert!(request.open_api_keys.is_empty());
        assert_eq!(request.allowed_origins.as_deref(), Some(""));
    }

    #[test]
    fn build_query_endpoint_create_request_rejects_unknown_keys_and_passes_absent_origins() {
        let existing = ServiceQueryAPIEndpoint::default();
        let mut options = QueryEndpointCreateOptions {
            roles: vec!["sql_console_read_only".into()],
            open_api_keys: vec!["key-1".into()],
            ..Default::default()
        };
        assert!(build_query_endpoint_create_request(&options, Some(&existing)).is_err());
        options.replace_open_api_keys = true;
        // An existing endpoint that reports no `allowedOrigins` is written back
        // without the field.
        let request = build_query_endpoint_create_request(&options, Some(&existing)).unwrap();
        assert_eq!(request.allowed_origins, None);
        assert!(
            serde_json::to_value(&request)
                .unwrap()
                .get("allowedOrigins")
                .is_none()
        );
        options.allowed_origins = Some("https://example.com".into());
        let request = build_query_endpoint_create_request(&options, Some(&existing)).unwrap();
        assert_eq!(request.open_api_keys, ["key-1"]);
        assert_eq!(
            request.allowed_origins.as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn build_private_endpoint_create_request_supports_minimal_fields() {
        let request = build_private_endpoint_create_request("vpce-0123456789abcdef0", None);

        assert_eq!(request.id, "vpce-0123456789abcdef0");
        assert_eq!(request.description.as_deref(), Some(""));
        // The spec makes `description` optional; the wire request still sends `""`.
        assert_eq!(serde_json::to_value(&request).unwrap()["description"], "");
    }

    #[test]
    fn build_private_endpoint_create_request_supports_maximal_fields() {
        let request =
            build_private_endpoint_create_request("vpce-0123456789abcdef0", Some("production"));

        assert_eq!(request.id, "vpce-0123456789abcdef0");
        assert_eq!(request.description.as_deref(), Some("production"));
    }
}
