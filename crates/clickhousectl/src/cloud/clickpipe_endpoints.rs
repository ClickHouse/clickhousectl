//! `clickhousectl cloud clickpipe reverse-private-endpoint` — CRUD for the
//! reverse private endpoints ClickPipes uses to reach a source privately.
//!
//! The commands are nested under `clickpipe` because an endpoint is only ever
//! useful to a pipe: a Kafka pipe references it by ID
//! (`clickpipe create kafka --reverse-private-endpoint-id`), and a Postgres or
//! MySQL CDC pipe references it by passing one of its DNS names as `--host`.
//! The surface lives in its own module rather than in `clickpipes.rs`, which is
//! already several thousand lines of pipe-creation surface.

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clap::builder::PossibleValuesParser;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{
    CreateReversePrivateEndpoint, CreateReversePrivateEndpointMskauthentication,
    CreateReversePrivateEndpointType, CustomPrivateDnsMapping, UpdateReversePrivateEndpoint,
};
use tabled::{Table, Tabled, settings::Style};

/// Wire value of each reverse private endpoint type, so the per-type flag rules
/// below read as the spec does rather than as string literals.
const TYPE_VPC_ENDPOINT_SERVICE: &str = "VPC_ENDPOINT_SERVICE";
const TYPE_VPC_RESOURCE: &str = "VPC_RESOURCE";
const TYPE_MSK_MULTI_VPC: &str = "MSK_MULTI_VPC";
const TYPE_GCP_PSC_SERVICE_ATTACHMENT: &str = "GCP_PSC_SERVICE_ATTACHMENT";

#[derive(Subcommand)]
pub enum ReversePrivateEndpointCommands {
    /// List reverse private endpoints for a service
    List {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get reverse private endpoint details
    Get {
        /// Service ID
        service_id: String,

        /// Reverse private endpoint ID
        endpoint_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create a reverse private endpoint
    Create(Box<ReversePrivateEndpointCreateArgs>),

    /// Replace the custom private DNS mappings of a reverse private endpoint
    Update {
        /// Service ID
        service_id: String,

        /// Reverse private endpoint ID
        endpoint_id: String,

        /// Custom private DNS name (repeatable; replaces the whole list)
        #[arg(long = "custom-private-dns-mapping", required = true)]
        custom_private_dns_mappings: Vec<String>,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete a reverse private endpoint
    Delete {
        /// Service ID
        service_id: String,

        /// Reverse private endpoint ID
        endpoint_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct ReversePrivateEndpointCreateArgs {
    /// Service ID
    pub service_id: String,

    /// Endpoint type
    #[arg(long = "type", value_name = "TYPE", value_parser = PossibleValuesParser::new(CreateReversePrivateEndpointType::VALUES))]
    pub endpoint_type: String,

    /// Endpoint description (max 255 characters)
    #[arg(long)]
    pub description: String,

    /// VPC endpoint service name (required for --type VPC_ENDPOINT_SERVICE)
    #[arg(long)]
    pub vpc_endpoint_service_name: Option<String>,

    /// VPC resource configuration ID (required for --type VPC_RESOURCE)
    #[arg(long)]
    pub vpc_resource_configuration_id: Option<String>,

    /// VPC resource share ARN (required for --type VPC_RESOURCE)
    #[arg(long)]
    pub vpc_resource_share_arn: Option<String>,

    /// MSK cluster ARN (required for --type MSK_MULTI_VPC)
    #[arg(long)]
    pub msk_cluster_arn: Option<String>,

    /// MSK cluster authentication (required for --type MSK_MULTI_VPC)
    #[arg(long, value_parser = PossibleValuesParser::new(CreateReversePrivateEndpointMskauthentication::VALUES))]
    pub msk_authentication: Option<String>,

    /// GCP PSC service attachment URI (required for --type GCP_PSC_SERVICE_ATTACHMENT)
    ///
    /// Form: projects/{project}/regions/{region}/serviceAttachments/{name}
    #[arg(long)]
    pub gcp_service_attachment: Option<String>,

    /// Custom private DNS name, exact or leading wildcard (repeatable)
    ///
    /// Rejected with --type MSK_MULTI_VPC. On the AWS PrivateLink types ClickHouse
    /// support must enable it for the service first.
    #[arg(long = "custom-private-dns-mapping")]
    pub custom_private_dns_mappings: Vec<String>,

    /// Organization ID (auto-detected only if you have one org)
    #[arg(long)]
    pub org_id: Option<String>,
}

impl ReversePrivateEndpointCommands {
    pub fn is_write(&self) -> bool {
        match self {
            ReversePrivateEndpointCommands::List { .. } => false,
            ReversePrivateEndpointCommands::Get { .. } => false,
            ReversePrivateEndpointCommands::Create(_) => true,
            ReversePrivateEndpointCommands::Update { .. } => true,
            ReversePrivateEndpointCommands::Delete { .. } => true,
        }
    }

    /// The client-side `create` validation message, for the post-parse usage
    /// error in `main.rs`. `None` for every other subcommand and for a valid
    /// `create`.
    pub(crate) fn create_validation_error(&self) -> Option<String> {
        let ReversePrivateEndpointCommands::Create(args) = self else {
            return None;
        };
        validate_create_args(args).err().map(|error| error.message)
    }
}

pub async fn run(
    client: &CloudClient,
    command: ReversePrivateEndpointCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        ReversePrivateEndpointCommands::List { service_id, org_id } => {
            endpoint_list(client, &service_id, org_id.as_deref(), json).await
        }
        ReversePrivateEndpointCommands::Get {
            service_id,
            endpoint_id,
            org_id,
        } => endpoint_get(client, &service_id, &endpoint_id, org_id.as_deref(), json).await,
        ReversePrivateEndpointCommands::Create(args) => endpoint_create(client, &args, json).await,
        ReversePrivateEndpointCommands::Update {
            service_id,
            endpoint_id,
            custom_private_dns_mappings,
            org_id,
        } => {
            endpoint_update(
                client,
                &service_id,
                &endpoint_id,
                &custom_private_dns_mappings,
                org_id.as_deref(),
                json,
            )
            .await
        }
        ReversePrivateEndpointCommands::Delete {
            service_id,
            endpoint_id,
            org_id,
        } => endpoint_delete(client, &service_id, &endpoint_id, org_id.as_deref(), json).await,
    }
}

/// Render a list-valued response field, treating an empty list the same as an
/// absent one: neither tells the caller a name to connect to.
fn join_names(names: Option<&Vec<String>>) -> String {
    match names {
        Some(names) if !names.is_empty() => names.join(", "),
        _ => ABSENT.to_string(),
    }
}

async fn endpoint_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let endpoints = client
        .list_clickpipe_reverse_private_endpoints(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoints)?);
    } else {
        if endpoints.is_empty() {
            println!("No reverse private endpoints found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Type")]
            endpoint_type: String,
            #[tabled(rename = "Description")]
            description: String,
            #[tabled(rename = "Status")]
            status: String,
            #[tabled(rename = "DNS Names")]
            dns_names: String,
        }
        let rows: Vec<Row> = endpoints
            .iter()
            .map(|endpoint| Row {
                id: or_absent(endpoint.id.as_ref()),
                endpoint_type: or_absent(endpoint.r#type.as_ref()),
                description: or_absent(endpoint.description.as_deref()),
                status: or_absent(endpoint.status.as_ref()),
                dns_names: join_names(endpoint.dns_names.as_ref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn endpoint_get(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let endpoint = client
        .get_clickpipe_reverse_private_endpoint(&org_id, service_id, endpoint_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        print_human(&endpoint)?;
    }
    Ok(())
}

async fn endpoint_create(
    client: &CloudClient,
    args: &ReversePrivateEndpointCreateArgs,
    json: bool,
) -> CloudResult<()> {
    // Built before the org is resolved so an invalid flag combination costs no
    // request at all, even when --org-id was omitted.
    let request = build_create_reverse_private_endpoint_request(args)?;
    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let endpoint = client
        .create_clickpipe_reverse_private_endpoint(&org_id, &args.service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        println!("Reverse private endpoint created");
        println!("  ID: {}", or_absent(endpoint.id.as_ref()));
        println!("  Type: {}", or_absent(endpoint.r#type.as_ref()));
        println!("  Status: {}", or_absent(endpoint.status.as_ref()));
        println!("  DNS names: {}", join_names(endpoint.dns_names.as_ref()));
    }
    Ok(())
}

async fn endpoint_update(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    custom_private_dns_mappings: &[String],
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_update_reverse_private_endpoint_request(custom_private_dns_mappings);
    let endpoint = client
        .update_clickpipe_reverse_private_endpoint(&org_id, service_id, endpoint_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        print_human(&endpoint)?;
    }
    Ok(())
}

async fn endpoint_delete(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    client
        .delete_clickpipe_reverse_private_endpoint(&org_id, service_id, endpoint_id)
        .await?;

    if json {
        println!("{}", serde_json::json!({ "deleted": endpoint_id }));
    } else {
        println!("Reverse private endpoint {} deleted", endpoint_id);
    }
    Ok(())
}

/// The type-specific create flags, each paired with the endpoint type it
/// belongs to and whether the invocation supplied it.
///
/// The pairing is the spec's own: `vpcResourceConfigurationId`,
/// `vpcResourceShareArn`, `mskClusterArn`, `mskAuthentication` and
/// `gcpServiceAttachment` are each documented as "Required for <TYPE> type".
/// `vpcEndpointServiceName` carries no such marker, but it names its type
/// exactly and no other type has any use for a VPC endpoint service name, so it
/// is treated as belonging to `VPC_ENDPOINT_SERVICE` too.
fn type_specific_flags(
    args: &ReversePrivateEndpointCreateArgs,
) -> [(&'static str, &'static str, bool); 6] {
    [
        (
            "vpc-endpoint-service-name",
            TYPE_VPC_ENDPOINT_SERVICE,
            args.vpc_endpoint_service_name.is_some(),
        ),
        (
            "vpc-resource-configuration-id",
            TYPE_VPC_RESOURCE,
            args.vpc_resource_configuration_id.is_some(),
        ),
        (
            "vpc-resource-share-arn",
            TYPE_VPC_RESOURCE,
            args.vpc_resource_share_arn.is_some(),
        ),
        (
            "msk-cluster-arn",
            TYPE_MSK_MULTI_VPC,
            args.msk_cluster_arn.is_some(),
        ),
        (
            "msk-authentication",
            TYPE_MSK_MULTI_VPC,
            args.msk_authentication.is_some(),
        ),
        (
            "gcp-service-attachment",
            TYPE_GCP_PSC_SERVICE_ATTACHMENT,
            args.gcp_service_attachment.is_some(),
        ),
    ]
}

/// Reject a `create` whose flags cannot describe the chosen type.
///
/// Only the rules the spec states are enforced: a missing flag the spec marks
/// "Required for <TYPE> type", a flag that belongs to a different type, and
/// `customPrivateDnsMappings` on `MSK_MULTI_VPC` ("Not supported for MSK
/// multi-VPC"). An unrecognized `--type` is left to the API — clap's possible
/// values already limit it to the four the library models.
fn validate_create_args(args: &ReversePrivateEndpointCreateArgs) -> CloudResult<()> {
    let requested = args.endpoint_type.as_str();
    let flags = type_specific_flags(args);

    for (flag, owner, present) in flags {
        if present && owner != requested {
            return Err(CloudError::new(format!(
                "--{flag} applies to --type {owner}, not --type {requested}"
            )));
        }
    }

    let missing: Vec<String> = flags
        .iter()
        .filter(|(_, owner, present)| *owner == requested && !*present)
        .map(|(flag, _, _)| format!("--{flag}"))
        .collect();
    if !missing.is_empty() {
        return Err(CloudError::new(format!(
            "--type {requested} requires {}",
            missing.join(" and ")
        )));
    }

    if requested == TYPE_MSK_MULTI_VPC && !args.custom_private_dns_mappings.is_empty() {
        return Err(CloudError::new(
            "--custom-private-dns-mapping is not supported for --type MSK_MULTI_VPC",
        ));
    }

    Ok(())
}

/// Map repeated `--custom-private-dns-mapping` values onto the API's array of
/// objects, omitting the field entirely when no mapping was given.
fn build_custom_private_dns_mappings(names: &[String]) -> Option<Vec<CustomPrivateDnsMapping>> {
    if names.is_empty() {
        return None;
    }
    Some(
        names
            .iter()
            .map(|name| CustomPrivateDnsMapping {
                private_dns_name: Some(name.clone()),
            })
            .collect(),
    )
}

fn build_create_reverse_private_endpoint_request(
    args: &ReversePrivateEndpointCreateArgs,
) -> CloudResult<CreateReversePrivateEndpoint> {
    validate_create_args(args)?;

    Ok(CreateReversePrivateEndpoint {
        description: args.description.clone(),
        r#type: parse_serde_enum(
            &args.endpoint_type,
            "type",
            CreateReversePrivateEndpointType::VALUES,
        )?,
        vpc_endpoint_service_name: args.vpc_endpoint_service_name.clone(),
        vpc_resource_configuration_id: args.vpc_resource_configuration_id.clone(),
        vpc_resource_share_arn: args.vpc_resource_share_arn.clone(),
        msk_cluster_arn: args.msk_cluster_arn.clone(),
        msk_authentication: args
            .msk_authentication
            .as_deref()
            .map(|value| {
                parse_serde_enum(
                    value,
                    "msk-authentication",
                    CreateReversePrivateEndpointMskauthentication::VALUES,
                )
            })
            .transpose()?,
        gcp_service_attachment: args.gcp_service_attachment.clone(),
        custom_private_dns_mappings: build_custom_private_dns_mappings(
            &args.custom_private_dns_mappings,
        ),
    })
}

fn build_update_reverse_private_endpoint_request(names: &[String]) -> UpdateReversePrivateEndpoint {
    UpdateReversePrivateEndpoint {
        custom_private_dns_mappings: build_custom_private_dns_mappings(names),
    }
}

impl CloudClient {
    pub async fn list_clickpipe_reverse_private_endpoints(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::ReversePrivateEndpoint>>
    {
        let response = self
            .api()
            .click_pipe_reverse_private_endpoint_get_list(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_clickpipe_reverse_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ReversePrivateEndpoint> {
        let response = self
            .api()
            .click_pipe_reverse_private_endpoint_get(org_id, service_id, endpoint_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_clickpipe_reverse_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &CreateReversePrivateEndpoint,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ReversePrivateEndpoint> {
        let response = self
            .api()
            .click_pipe_reverse_private_endpoint_create(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_clickpipe_reverse_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        endpoint_id: &str,
        request: &UpdateReversePrivateEndpoint,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ReversePrivateEndpoint> {
        let response = self
            .api()
            .click_pipe_reverse_private_endpoint_update(org_id, service_id, endpoint_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_clickpipe_reverse_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        endpoint_id: &str,
    ) -> crate::cloud::client::Result<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_pipe_reverse_private_endpoint_delete(org_id, service_id, endpoint_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_endpoint_command(args: &[&str]) -> ReversePrivateEndpointCommands {
        let cli = Cli::try_parse_from(
            [
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
            ]
            .into_iter()
            .chain(args.iter().copied()),
        )
        .expect("reverse-private-endpoint command should parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = cloud.command else {
            panic!("expected clickpipe command");
        };
        let crate::cloud::clickpipes::ClickPipeCommands::ReversePrivateEndpoint { command } =
            *command
        else {
            panic!("expected reverse-private-endpoint command");
        };
        command
    }

    fn parse_error(args: &[&str]) -> clap::Error {
        Cli::try_parse_from(
            [
                "clickhousectl",
                "cloud",
                "clickpipe",
                "reverse-private-endpoint",
            ]
            .into_iter()
            .chain(args.iter().copied()),
        )
        .err()
        .unwrap_or_else(|| panic!("expected parse failure for: {}", args.join(" ")))
    }

    fn create_args(extra: &[&str]) -> Box<ReversePrivateEndpointCreateArgs> {
        let mut args = vec!["create", "svc-1", "--description", "endpoint"];
        args.extend_from_slice(extra);
        let ReversePrivateEndpointCommands::Create(args) = parse_endpoint_command(&args) else {
            panic!("expected create command");
        };
        args
    }

    /// A `VPC_ENDPOINT_SERVICE` create with only its own required flag.
    fn minimal_vpc_endpoint_service_args() -> Box<ReversePrivateEndpointCreateArgs> {
        create_args(&[
            "--type",
            TYPE_VPC_ENDPOINT_SERVICE,
            "--vpc-endpoint-service-name",
            "com.amazonaws.vpce.us-east-1.vpce-svc-1",
        ])
    }

    #[test]
    fn parses_list_and_get() {
        let ReversePrivateEndpointCommands::List { service_id, org_id } =
            parse_endpoint_command(&["list", "svc-1"])
        else {
            panic!("expected list command");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(org_id, None);

        let ReversePrivateEndpointCommands::Get {
            service_id,
            endpoint_id,
            org_id,
        } = parse_endpoint_command(&["get", "svc-1", "rpe-1", "--org-id", "org-1"])
        else {
            panic!("expected get command");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(endpoint_id, "rpe-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_delete() {
        let ReversePrivateEndpointCommands::Delete {
            service_id,
            endpoint_id,
            org_id,
        } = parse_endpoint_command(&["delete", "svc-1", "rpe-1"])
        else {
            panic!("expected delete command");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(endpoint_id, "rpe-1");
        assert_eq!(org_id, None);
    }

    #[test]
    fn parses_update_with_repeatable_mappings() {
        let ReversePrivateEndpointCommands::Update {
            service_id,
            endpoint_id,
            custom_private_dns_mappings,
            org_id,
        } = parse_endpoint_command(&[
            "update",
            "svc-1",
            "rpe-1",
            "--custom-private-dns-mapping",
            "db.example.com",
            "--custom-private-dns-mapping",
            "*.example.com",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected update command");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(endpoint_id, "rpe-1");
        assert_eq!(
            custom_private_dns_mappings,
            vec!["db.example.com".to_string(), "*.example.com".to_string()]
        );
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    /// The PATCH body only carries the mappings, so an update with none would
    /// send `{}`: clap must reject it instead.
    #[test]
    fn update_requires_at_least_one_mapping() {
        let error = parse_error(&["update", "svc-1", "rpe-1"]);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(
            error.to_string().contains("--custom-private-dns-mapping"),
            "error should name the flag: {error}"
        );
    }

    #[test]
    fn parses_create_flags_for_every_type() {
        let args = minimal_vpc_endpoint_service_args();
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.endpoint_type, TYPE_VPC_ENDPOINT_SERVICE);
        assert_eq!(args.description, "endpoint");
        assert_eq!(
            args.vpc_endpoint_service_name.as_deref(),
            Some("com.amazonaws.vpce.us-east-1.vpce-svc-1")
        );
        assert!(args.custom_private_dns_mappings.is_empty());

        let args = create_args(&[
            "--type",
            TYPE_VPC_RESOURCE,
            "--vpc-resource-configuration-id",
            "rcfg-1",
            "--vpc-resource-share-arn",
            "arn:aws:ram:us-east-1:1:resource-share/share-1",
        ]);
        assert_eq!(
            args.vpc_resource_configuration_id.as_deref(),
            Some("rcfg-1")
        );
        assert_eq!(
            args.vpc_resource_share_arn.as_deref(),
            Some("arn:aws:ram:us-east-1:1:resource-share/share-1")
        );

        let args = create_args(&[
            "--type",
            TYPE_MSK_MULTI_VPC,
            "--msk-cluster-arn",
            "arn:aws:kafka:us-east-1:1:cluster/c",
            "--msk-authentication",
            "SASL_SCRAM",
        ]);
        assert_eq!(
            args.msk_cluster_arn.as_deref(),
            Some("arn:aws:kafka:us-east-1:1:cluster/c")
        );
        assert_eq!(args.msk_authentication.as_deref(), Some("SASL_SCRAM"));

        let args = create_args(&[
            "--type",
            TYPE_GCP_PSC_SERVICE_ATTACHMENT,
            "--gcp-service-attachment",
            "projects/p/regions/us-central1/serviceAttachments/s",
            "--custom-private-dns-mapping",
            "db.example.com",
            "--custom-private-dns-mapping",
            "*.example.com",
            "--org-id",
            "org-1",
        ]);
        assert_eq!(
            args.gcp_service_attachment.as_deref(),
            Some("projects/p/regions/us-central1/serviceAttachments/s")
        );
        assert_eq!(
            args.custom_private_dns_mappings,
            vec!["db.example.com".to_string(), "*.example.com".to_string()]
        );
        assert_eq!(args.org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn create_rejects_unknown_type_and_authentication_values() {
        for args in [
            vec![
                "create",
                "svc-1",
                "--description",
                "d",
                "--type",
                "VPC_ENDPOINT",
            ],
            vec![
                "create",
                "svc-1",
                "--description",
                "d",
                "--type",
                TYPE_MSK_MULTI_VPC,
                "--msk-cluster-arn",
                "arn",
                "--msk-authentication",
                "SASL_PLAIN",
            ],
        ] {
            assert_eq!(
                parse_error(&args).kind(),
                clap::error::ErrorKind::InvalidValue,
                "expected an invalid-value error for: {}",
                args.join(" ")
            );
        }
    }

    #[test]
    fn create_requires_type_and_description() {
        for args in [
            vec!["create", "svc-1", "--description", "d"],
            vec!["create", "svc-1", "--type", TYPE_VPC_RESOURCE],
        ] {
            assert_eq!(
                parse_error(&args).kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "expected a missing-argument error for: {}",
                args.join(" ")
            );
        }
    }

    #[test]
    fn validation_names_the_flags_each_type_requires() {
        for (endpoint_type, expected) in [
            (
                TYPE_VPC_ENDPOINT_SERVICE,
                "--type VPC_ENDPOINT_SERVICE requires --vpc-endpoint-service-name",
            ),
            (
                TYPE_VPC_RESOURCE,
                "--type VPC_RESOURCE requires --vpc-resource-configuration-id and \
                 --vpc-resource-share-arn",
            ),
            (
                TYPE_MSK_MULTI_VPC,
                "--type MSK_MULTI_VPC requires --msk-cluster-arn and --msk-authentication",
            ),
            (
                TYPE_GCP_PSC_SERVICE_ATTACHMENT,
                "--type GCP_PSC_SERVICE_ATTACHMENT requires --gcp-service-attachment",
            ),
        ] {
            let args = create_args(&["--type", endpoint_type]);
            let error = validate_create_args(&args).expect_err("expected a validation error");
            assert_eq!(error.message, expected);
            // The same message reaches main.rs for the usage error.
            assert_eq!(
                ReversePrivateEndpointCommands::Create(args).create_validation_error(),
                Some(expected.to_string())
            );
        }
    }

    #[test]
    fn validation_rejects_a_flag_belonging_to_another_type() {
        let args = create_args(&[
            "--type",
            TYPE_VPC_ENDPOINT_SERVICE,
            "--vpc-endpoint-service-name",
            "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            "--msk-cluster-arn",
            "arn:aws:kafka:us-east-1:1:cluster/c",
        ]);
        assert_eq!(
            validate_create_args(&args).unwrap_err().message,
            "--msk-cluster-arn applies to --type MSK_MULTI_VPC, not --type VPC_ENDPOINT_SERVICE"
        );

        let args = create_args(&[
            "--type",
            TYPE_VPC_RESOURCE,
            "--vpc-resource-configuration-id",
            "rcfg-1",
            "--vpc-resource-share-arn",
            "arn:aws:ram:us-east-1:1:resource-share/share-1",
            "--gcp-service-attachment",
            "projects/p/regions/us-central1/serviceAttachments/s",
        ]);
        assert_eq!(
            validate_create_args(&args).unwrap_err().message,
            "--gcp-service-attachment applies to --type GCP_PSC_SERVICE_ATTACHMENT, not --type \
             VPC_RESOURCE"
        );
    }

    /// "Not supported for MSK multi-VPC", per the spec's field description.
    #[test]
    fn validation_rejects_custom_dns_mappings_for_msk() {
        let args = create_args(&[
            "--type",
            TYPE_MSK_MULTI_VPC,
            "--msk-cluster-arn",
            "arn:aws:kafka:us-east-1:1:cluster/c",
            "--msk-authentication",
            "SASL_IAM",
            "--custom-private-dns-mapping",
            "db.example.com",
        ]);
        assert_eq!(
            validate_create_args(&args).unwrap_err().message,
            "--custom-private-dns-mapping is not supported for --type MSK_MULTI_VPC"
        );
    }

    #[test]
    fn every_valid_type_combination_passes_validation() {
        for args in [
            minimal_vpc_endpoint_service_args(),
            create_args(&[
                "--type",
                TYPE_VPC_RESOURCE,
                "--vpc-resource-configuration-id",
                "rcfg-1",
                "--vpc-resource-share-arn",
                "arn:aws:ram:us-east-1:1:resource-share/share-1",
            ]),
            create_args(&[
                "--type",
                TYPE_MSK_MULTI_VPC,
                "--msk-cluster-arn",
                "arn:aws:kafka:us-east-1:1:cluster/c",
                "--msk-authentication",
                "SASL_IAM",
            ]),
            create_args(&[
                "--type",
                TYPE_GCP_PSC_SERVICE_ATTACHMENT,
                "--gcp-service-attachment",
                "projects/p/regions/us-central1/serviceAttachments/s",
            ]),
        ] {
            validate_create_args(&args).expect("valid combination should pass");
            assert_eq!(
                ReversePrivateEndpointCommands::Create(args).create_validation_error(),
                None
            );
        }
    }

    /// A non-create subcommand has nothing to validate.
    #[test]
    fn create_validation_error_is_none_for_other_subcommands() {
        assert_eq!(
            parse_endpoint_command(&["list", "svc-1"]).create_validation_error(),
            None
        );
    }

    #[test]
    fn build_create_request_minimal_sends_only_the_type_and_description() {
        let request =
            build_create_reverse_private_endpoint_request(&minimal_vpc_endpoint_service_args())
                .expect("minimal create should build");

        assert_eq!(request.description, "endpoint");
        assert_eq!(
            request.r#type,
            CreateReversePrivateEndpointType::VPC_ENDPOINT_SERVICE
        );
        assert_eq!(
            request.vpc_endpoint_service_name.as_deref(),
            Some("com.amazonaws.vpce.us-east-1.vpce-svc-1")
        );
        assert_eq!(request.vpc_resource_configuration_id, None);
        assert_eq!(request.vpc_resource_share_arn, None);
        assert_eq!(request.msk_cluster_arn, None);
        assert_eq!(request.msk_authentication, None);
        assert_eq!(request.gcp_service_attachment, None);
        assert_eq!(request.custom_private_dns_mappings, None);
    }

    #[test]
    fn build_create_request_maximal_maps_every_flag() {
        let request = build_create_reverse_private_endpoint_request(&create_args(&[
            "--type",
            TYPE_GCP_PSC_SERVICE_ATTACHMENT,
            "--gcp-service-attachment",
            "projects/p/regions/us-central1/serviceAttachments/s",
            "--custom-private-dns-mapping",
            "db.example.com",
            "--custom-private-dns-mapping",
            "*.example.com",
        ]))
        .expect("maximal create should build");

        assert_eq!(
            request.r#type,
            CreateReversePrivateEndpointType::GCP_PSC_SERVICE_ATTACHMENT
        );
        assert_eq!(
            request.gcp_service_attachment.as_deref(),
            Some("projects/p/regions/us-central1/serviceAttachments/s")
        );
        assert_eq!(
            request.custom_private_dns_mappings,
            Some(vec![
                CustomPrivateDnsMapping {
                    private_dns_name: Some("db.example.com".into()),
                },
                CustomPrivateDnsMapping {
                    private_dns_name: Some("*.example.com".into()),
                },
            ])
        );
    }

    #[test]
    fn build_create_request_maps_the_msk_authentication_enum() {
        let request = build_create_reverse_private_endpoint_request(&create_args(&[
            "--type",
            TYPE_MSK_MULTI_VPC,
            "--msk-cluster-arn",
            "arn:aws:kafka:us-east-1:1:cluster/c",
            "--msk-authentication",
            "SASL_SCRAM",
        ]))
        .expect("MSK create should build");

        assert_eq!(
            request.msk_authentication,
            Some(CreateReversePrivateEndpointMskauthentication::SASL_SCRAM)
        );
        assert_eq!(
            request.r#type,
            CreateReversePrivateEndpointType::MSK_MULTI_VPC
        );
    }

    /// The builder validates too, so a handler reached by any other route
    /// cannot send an invalid body.
    #[test]
    fn build_create_request_refuses_an_invalid_combination() {
        let error = build_create_reverse_private_endpoint_request(&create_args(&[
            "--type",
            TYPE_VPC_RESOURCE,
            "--vpc-resource-configuration-id",
            "rcfg-1",
        ]))
        .expect_err("expected a validation error");
        assert_eq!(
            error.message,
            "--type VPC_RESOURCE requires --vpc-resource-share-arn"
        );
    }

    #[test]
    fn build_update_request_sends_the_complete_mapping_list() {
        let request = build_update_reverse_private_endpoint_request(&[
            "db.example.com".to_string(),
            "*.example.com".to_string(),
        ]);
        assert_eq!(
            request.custom_private_dns_mappings,
            Some(vec![
                CustomPrivateDnsMapping {
                    private_dns_name: Some("db.example.com".into()),
                },
                CustomPrivateDnsMapping {
                    private_dns_name: Some("*.example.com".into()),
                },
            ])
        );

        // Clap requires a mapping, so this shape is unreachable from the CLI;
        // pinned so the builder never invents an empty array either.
        assert_eq!(
            build_update_reverse_private_endpoint_request(&[]).custom_private_dns_mappings,
            None
        );
    }

    /// Every field of the response model is `Option`, so a list row must render
    /// absence rather than unwrap.
    #[test]
    fn join_names_renders_absence_and_an_empty_list_alike() {
        assert_eq!(join_names(None), ABSENT);
        assert_eq!(join_names(Some(&vec![])), ABSENT);
        assert_eq!(
            join_names(Some(&vec!["a.example.com".to_string(), "b".to_string()])),
            "a.example.com, b"
        );
    }

    #[test]
    fn read_and_write_classification() {
        assert!(!parse_endpoint_command(&["list", "svc-1"]).is_write());
        assert!(!parse_endpoint_command(&["get", "svc-1", "rpe-1"]).is_write());
        assert!(
            ReversePrivateEndpointCommands::Create(minimal_vpc_endpoint_service_args()).is_write()
        );
        assert!(
            parse_endpoint_command(&[
                "update",
                "svc-1",
                "rpe-1",
                "--custom-private-dns-mapping",
                "db.example.com",
            ])
            .is_write()
        );
        assert!(parse_endpoint_command(&["delete", "svc-1", "rpe-1"]).is_write());
    }
}
