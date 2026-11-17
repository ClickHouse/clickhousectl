//! `cloud postgres private-endpoint`: the AWS private endpoint service of a
//! Postgres service.

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::print_human;
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    PostgresPrivateEndpoint, PostgresPrivateEndpointPatchRequest,
    PostgresPrivateEndpointPostRequest, PostgresPrivateEndpointVpcEndpoint,
};

/// Parse `VPCE_ID[=DESCRIPTION]` for `--add-vpc-endpoint`.
pub(super) fn parse_vpc_endpoint(
    value: &str,
) -> Result<PostgresPrivateEndpointVpcEndpoint, String> {
    let (id, description) = value
        .split_once('=')
        .map_or((value, None), |(id, description)| (id, Some(description)));
    let id = parse_vpc_endpoint_id(id)?;
    if description.is_some_and(|description| description.chars().count() > 255) {
        return Err("description must be at most 255 characters".into());
    }
    Ok(PostgresPrivateEndpointVpcEndpoint {
        vpc_endpoint_id: id,
        description: description.map(str::to_string),
    })
}

/// Parse an AWS VPC endpoint ID: `vpce-` and 17 lowercase hex digits.
fn parse_vpc_endpoint_id(value: &str) -> Result<String, String> {
    let valid = value.strip_prefix("vpce-").is_some_and(|hex| {
        hex.len() == 17
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    });
    if valid {
        Ok(value.to_string())
    } else {
        Err("expected a VPC endpoint ID: vpce- and 17 lowercase hex digits".into())
    }
}

pub(super) fn build_private_endpoint_create_request(
    supported_regions: &[String],
) -> PostgresPrivateEndpointPostRequest {
    PostgresPrivateEndpointPostRequest {
        supported_regions: (!supported_regions.is_empty()).then(|| supported_regions.to_vec()),
    }
}

/// Changes requested by `private-endpoint update`, applied to the current lists.
pub(super) struct PrivateEndpointUpdate<'a> {
    pub add_vpc_endpoint: &'a [PostgresPrivateEndpointVpcEndpoint],
    pub remove_vpc_endpoint: &'a [String],
    pub add_region: &'a [String],
    pub remove_region: &'a [String],
}

/// The patch for `private-endpoint update`, plus a warning for each removal
/// that matched nothing.
#[derive(Debug)]
pub(super) struct PrivateEndpointPatch {
    pub request: PostgresPrivateEndpointPatchRequest,
    pub warnings: Vec<String>,
}

/// Merge the requested changes into `current` and return a patch that sends only
/// the lists that change, because each list sent replaces the stored one in full.
///
/// Removals apply before additions. As with `cloud service update`, removing an
/// entry that is not there is idempotent: it adds a warning instead of failing,
/// so a typo stays visible. Adding an approved VPC endpoint again updates its
/// description when one is given.
pub(super) fn build_private_endpoint_update_request(
    current: &PostgresPrivateEndpoint,
    update: &PrivateEndpointUpdate<'_>,
) -> CloudResult<PrivateEndpointPatch> {
    let mut request = PostgresPrivateEndpointPatchRequest::default();
    let mut warnings = Vec::new();

    if !update.add_vpc_endpoint.is_empty() || !update.remove_vpc_endpoint.is_empty() {
        let mut endpoints = current
            .allowed_vpc_endpoints
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(PostgresPrivateEndpointVpcEndpoint::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|missing| {
                CloudError::new(format!(
                    "private endpoint response has an approved VPC endpoint without {}; \
                     the list cannot be written back safely",
                    missing.fields().join(", ")
                ))
            })?;
        for id in update.remove_vpc_endpoint {
            let before = endpoints.len();
            endpoints.retain(|endpoint| &endpoint.vpc_endpoint_id != id);
            if endpoints.len() == before {
                warnings.push(format!(
                    "--remove-vpc-endpoint {id} did not match any approved VPC endpoint; nothing was removed"
                ));
            }
        }
        for added in update.add_vpc_endpoint {
            match endpoints
                .iter_mut()
                .find(|endpoint| endpoint.vpc_endpoint_id == added.vpc_endpoint_id)
            {
                Some(existing) => {
                    if added.description.is_some() {
                        existing.description = added.description.clone();
                    }
                }
                None => endpoints.push(added.clone()),
            }
        }
        request.allowed_vpc_endpoints = Some(endpoints);
    }

    if !update.add_region.is_empty() || !update.remove_region.is_empty() {
        let mut regions = current.supported_regions.clone().unwrap_or_default();
        for region in update.remove_region {
            let before = regions.len();
            regions.retain(|existing| existing != region);
            if regions.len() == before {
                warnings.push(format!(
                    "--remove-region {region} did not match any supported region; nothing was removed"
                ));
            }
        }
        for region in update.add_region {
            if !regions.contains(region) {
                regions.push(region.clone());
            }
        }
        request.supported_regions = Some(regions);
    }

    Ok(PrivateEndpointPatch { request, warnings })
}

fn print_endpoint(endpoint: &PostgresPrivateEndpoint, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(endpoint)?);
    } else {
        print_human(endpoint)?;
    }
    Ok(())
}

pub(super) async fn private_endpoint_get(
    client: &CloudClient,
    postgres_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let endpoint = client
        .get_postgres_private_endpoint(&org_id, postgres_id)
        .await?;
    print_endpoint(&endpoint, json)
}

pub(super) async fn private_endpoint_create(
    client: &CloudClient,
    postgres_id: &str,
    supported_regions: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let request = build_private_endpoint_create_request(supported_regions);
    let endpoint = client
        .create_postgres_private_endpoint(&org_id, postgres_id, &request)
        .await?;
    print_endpoint(&endpoint, json)?;
    if !json {
        println!(
            "Next: clickhousectl cloud postgres private-endpoint get {postgres_id} \
             (wait for state=available, about 5 to 10 minutes)"
        );
    }
    Ok(())
}

pub(super) async fn private_endpoint_update(
    client: &CloudClient,
    postgres_id: &str,
    update: PrivateEndpointUpdate<'_>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let current = client
        .get_postgres_private_endpoint(&org_id, postgres_id)
        .await?;
    let patch = build_private_endpoint_update_request(&current, &update)?;
    for warning in &patch.warnings {
        eprintln!("Warning: {warning}");
    }
    let endpoint = client
        .update_postgres_private_endpoint(&org_id, postgres_id, &patch.request)
        .await?;
    print_endpoint(&endpoint, json)
}

pub(super) async fn private_endpoint_delete(
    client: &CloudClient,
    postgres_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    // The delete response has no `result`; print the endpoint read beforehand,
    // as `cloud postgres delete` does for the service.
    let endpoint = client
        .get_postgres_private_endpoint(&org_id, postgres_id)
        .await?;
    client
        .delete_postgres_private_endpoint(&org_id, postgres_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        println!("Private endpoint of Postgres service {postgres_id} deleted");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse_cloud_api::models::PostgresPrivateEndpointVpcEndpointResponse;

    const VPCE_A: &str = "vpce-0123456789abcdef0";
    const VPCE_B: &str = "vpce-0123456789abcdef1";

    fn vpce(id: &str, description: Option<&str>) -> PostgresPrivateEndpointVpcEndpoint {
        PostgresPrivateEndpointVpcEndpoint {
            vpc_endpoint_id: id.into(),
            description: description.map(str::to_string),
        }
    }

    fn current() -> PostgresPrivateEndpoint {
        PostgresPrivateEndpoint {
            allowed_vpc_endpoints: Some(vec![PostgresPrivateEndpointVpcEndpointResponse {
                vpc_endpoint_id: Some(VPCE_A.into()),
                description: Some("prod".into()),
            }]),
            supported_regions: Some(vec!["us-west-2".into()]),
            ..Default::default()
        }
    }

    fn no_change() -> PrivateEndpointUpdate<'static> {
        PrivateEndpointUpdate {
            add_vpc_endpoint: &[],
            remove_vpc_endpoint: &[],
            add_region: &[],
            remove_region: &[],
        }
    }

    #[test]
    fn vpc_endpoint_parser_accepts_id_and_optional_description() {
        assert_eq!(parse_vpc_endpoint(VPCE_A).unwrap(), vpce(VPCE_A, None));
        assert_eq!(
            parse_vpc_endpoint(&format!("{VPCE_A}=Production VPC")).unwrap(),
            vpce(VPCE_A, Some("Production VPC"))
        );
        assert_eq!(
            parse_vpc_endpoint(&format!("{VPCE_A}=")).unwrap(),
            vpce(VPCE_A, Some(""))
        );
        for invalid in [
            "vpce-0123456789ABCDEF0",
            "vpce-0123456789abcdef",
            "vpce-0123456789abcdef00",
            "0123456789abcdef0",
            "",
        ] {
            assert!(parse_vpc_endpoint(invalid).is_err(), "{invalid}");
        }
        assert!(parse_vpc_endpoint(&format!("{VPCE_A}={}", "x".repeat(256))).is_err());
        assert!(parse_vpc_endpoint(&format!("{VPCE_A}={}", "x".repeat(255))).is_ok());
    }

    #[test]
    fn create_request_omits_regions_unless_given() {
        assert_eq!(
            build_private_endpoint_create_request(&[]).supported_regions,
            None
        );
        assert_eq!(
            build_private_endpoint_create_request(&["us-west-2".into(), "eu-west-1".into()])
                .supported_regions,
            Some(vec!["us-west-2".into(), "eu-west-1".into()])
        );
    }

    #[test]
    fn update_sends_only_the_lists_that_change() {
        let add = [vpce(VPCE_B, None)];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                add_vpc_endpoint: &add,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        // The existing approval and its description are written back.
        assert_eq!(
            request.allowed_vpc_endpoints,
            Some(vec![vpce(VPCE_A, Some("prod")), vpce(VPCE_B, None)])
        );
        assert_eq!(request.supported_regions, None);

        let regions = ["eu-west-1".to_string()];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                add_region: &regions,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        assert_eq!(request.allowed_vpc_endpoints, None);
        assert_eq!(
            request.supported_regions,
            Some(vec!["us-west-2".into(), "eu-west-1".into()])
        );
    }

    #[test]
    fn update_removes_before_adding_and_can_empty_a_list() {
        let remove = [VPCE_A.to_string()];
        let remove_region = ["us-west-2".to_string()];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                remove_vpc_endpoint: &remove,
                remove_region: &remove_region,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        assert_eq!(request.allowed_vpc_endpoints, Some(vec![]));
        assert_eq!(request.supported_regions, Some(vec![]));

        // Removing and re-adding replaces the description with the new one.
        let add = [vpce(VPCE_A, None)];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                add_vpc_endpoint: &add,
                remove_vpc_endpoint: &remove,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        assert_eq!(
            request.allowed_vpc_endpoints,
            Some(vec![vpce(VPCE_A, None)])
        );
    }

    #[test]
    fn update_re_adding_an_approval_keeps_or_replaces_its_description() {
        let keep = [vpce(VPCE_A, None)];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                add_vpc_endpoint: &keep,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        assert_eq!(
            request.allowed_vpc_endpoints,
            Some(vec![vpce(VPCE_A, Some("prod"))])
        );

        let relabel = [vpce(VPCE_A, Some("staging"))];
        let request = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                add_vpc_endpoint: &relabel,
                ..no_change()
            },
        )
        .unwrap()
        .request;
        assert_eq!(
            request.allowed_vpc_endpoints,
            Some(vec![vpce(VPCE_A, Some("staging"))])
        );
    }

    #[test]
    fn update_warns_when_a_removal_matches_nothing() {
        let remove = [VPCE_B.to_string()];
        let remove_region = ["ap-south-1".to_string()];
        let patch = build_private_endpoint_update_request(
            &current(),
            &PrivateEndpointUpdate {
                remove_vpc_endpoint: &remove,
                remove_region: &remove_region,
                ..no_change()
            },
        )
        .unwrap();
        // The unchanged lists are still sent, so the request stays idempotent.
        assert_eq!(
            patch.request.allowed_vpc_endpoints,
            Some(vec![vpce(VPCE_A, Some("prod"))])
        );
        assert_eq!(
            patch.request.supported_regions,
            Some(vec!["us-west-2".into()])
        );
        assert_eq!(patch.warnings.len(), 2);
        assert!(
            patch.warnings[0].contains("--remove-vpc-endpoint")
                && patch.warnings[0].contains(VPCE_B)
        );
        assert!(
            patch.warnings[1].contains("--remove-region")
                && patch.warnings[1].contains("ap-south-1")
        );
    }

    #[test]
    fn update_refuses_to_write_back_an_approval_without_an_id() {
        let mut current = current();
        current.allowed_vpc_endpoints = Some(vec![PostgresPrivateEndpointVpcEndpointResponse {
            vpc_endpoint_id: None,
            description: Some("mystery".into()),
        }]);
        let add = [vpce(VPCE_B, None)];
        let error = build_private_endpoint_update_request(
            &current,
            &PrivateEndpointUpdate {
                add_vpc_endpoint: &add,
                ..no_change()
            },
        )
        .unwrap_err();
        assert!(!matches!(
            error.kind,
            crate::cloud::client::CloudErrorKind::Usage
        ));
        assert!(error.message.contains("vpcEndpointId"));

        // Region-only changes never touch the approval list.
        let regions = ["eu-west-1".to_string()];
        assert!(
            build_private_endpoint_update_request(
                &current,
                &PrivateEndpointUpdate {
                    add_region: &regions,
                    ..no_change()
                },
            )
            .is_ok()
        );
    }
}
