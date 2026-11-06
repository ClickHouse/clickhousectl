use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human, print_line};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{JwtProvider, JwtProviderPostRequest, JwtProviderPutRequest};
use tabled::{Table, Tabled, settings::Style};

/// The provider configuration `create` and `update` both send.
pub(super) struct JwtProviderConfig {
    pub(super) issuer: String,
    pub(super) audience: String,
    pub(super) jwks_url: String,
    pub(super) roles_claim: Option<String>,
}

fn build_jwt_provider_create_request(
    name: &str,
    config: JwtProviderConfig,
) -> JwtProviderPostRequest {
    JwtProviderPostRequest {
        name: name.to_string(),
        issuer: config.issuer,
        audience: config.audience,
        jwks_url: config.jwks_url,
        roles_claim: config.roles_claim,
    }
}

/// The PUT replaces the whole configuration: a `None` roles claim removes
/// the stored one rather than leaving it unchanged.
fn build_jwt_provider_update_request(config: JwtProviderConfig) -> JwtProviderPutRequest {
    JwtProviderPutRequest {
        issuer: config.issuer,
        audience: config.audience,
        jwks_url: config.jwks_url,
        roles_claim: config.roles_claim,
    }
}

fn print_jwt_provider(provider: &JwtProvider, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(provider)?);
    } else {
        print_human(provider)?;
    }
    Ok(())
}

pub(super) async fn jwt_provider_list(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let providers = client.list_jwt_providers(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&providers)?);
        return Ok(());
    }
    if providers.is_empty() {
        println!("No JWT providers configured");
        return Ok(());
    }
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "Issuer")]
        issuer: String,
        #[tabled(rename = "Audience")]
        audience: String,
        #[tabled(rename = "JWKS URL")]
        jwks_url: String,
        #[tabled(rename = "Roles claim")]
        roles_claim: String,
    }
    let rows = providers
        .into_iter()
        .map(|provider| Row {
            name: or_absent(provider.name),
            issuer: or_absent(provider.issuer),
            audience: or_absent(provider.audience),
            jwks_url: or_absent(provider.jwks_url),
            roles_claim: or_absent(provider.roles_claim),
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(rows).with(Style::markdown()));
    Ok(())
}

pub(super) async fn jwt_provider_get(
    client: &CloudClient,
    service_id: &str,
    provider_name: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let provider = client
        .get_jwt_provider(&org_id, service_id, provider_name)
        .await?;
    print_jwt_provider(&provider, json)
}

pub(super) async fn jwt_provider_create(
    client: &CloudClient,
    service_id: &str,
    provider_name: &str,
    config: JwtProviderConfig,
    json: bool,
) -> CloudResult<()> {
    let request = build_jwt_provider_create_request(provider_name, config);
    let org_id = resolve_org_id(client).await?;
    let provider = client
        .create_jwt_provider(&org_id, service_id, &request)
        .await?;
    print_jwt_provider(&provider, json)
}

pub(super) async fn jwt_provider_update(
    client: &CloudClient,
    service_id: &str,
    provider_name: &str,
    config: JwtProviderConfig,
    json: bool,
) -> CloudResult<()> {
    let request = build_jwt_provider_update_request(config);
    let org_id = resolve_org_id(client).await?;
    let provider = client
        .update_jwt_provider(&org_id, service_id, provider_name, &request)
        .await?;
    print_jwt_provider(&provider, json)
}

pub(super) async fn jwt_provider_delete(
    client: &CloudClient,
    service_id: &str,
    provider_name: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client
        .delete_jwt_provider(&org_id, service_id, provider_name)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_line(format!(
            "JWT provider {provider_name} deleted from service {service_id}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(roles_claim: Option<&str>) -> JwtProviderConfig {
        JwtProviderConfig {
            issuer: "https://auth.example.com".to_string(),
            audience: "analytics".to_string(),
            jwks_url: "https://auth.example.com/.well-known/jwks.json".to_string(),
            roles_claim: roles_claim.map(str::to_string),
        }
    }

    #[test]
    fn build_jwt_provider_create_request_with_minimal_and_maximal_input() {
        let minimal = build_jwt_provider_create_request("okta", config(None));
        assert_eq!(minimal.name, "okta");
        assert_eq!(minimal.issuer, "https://auth.example.com");
        assert_eq!(minimal.audience, "analytics");
        assert_eq!(
            minimal.jwks_url,
            "https://auth.example.com/.well-known/jwks.json"
        );
        assert!(minimal.roles_claim.is_none());

        let maximal = build_jwt_provider_create_request("okta", config(Some("clickhouse:roles")));
        assert_eq!(maximal.roles_claim.as_deref(), Some("clickhouse:roles"));
        assert_eq!(maximal.name, "okta");
    }

    #[test]
    fn build_jwt_provider_update_request_with_minimal_and_maximal_input() {
        let minimal = build_jwt_provider_update_request(config(None));
        assert_eq!(minimal.issuer, "https://auth.example.com");
        assert_eq!(minimal.audience, "analytics");
        assert_eq!(
            minimal.jwks_url,
            "https://auth.example.com/.well-known/jwks.json"
        );
        assert!(minimal.roles_claim.is_none());
        // Omitting the claim from the body is what removes a stored one.
        let body = serde_json::to_value(&minimal).unwrap();
        assert!(body.get("rolesClaim").is_none(), "{body}");

        let maximal = build_jwt_provider_update_request(config(Some("clickhouse:roles")));
        assert_eq!(maximal.roles_claim.as_deref(), Some("clickhouse:roles"));
    }
}
