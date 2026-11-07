use super::cli::NotificationListArgs;
use super::org::NotificationPage;
use crate::cloud::client::{
    CloudClient, CloudError, ResourceKind, ResourceLookup, Result as CloudResult,
};
use crate::cloud::types::DeleteResponse;
use clickhouse_cloud_api::models::{
    ByocCustomerAuthorizedAccess, ByocCustomerAuthorizedAccessPatchRequest,
    ByocInfrastructureDetails, ByocInfrastructurePatchRequest, ByocInfrastructurePostRequest,
    ByocInfrastructurePrivateEndpointConfig, ByocInfrastructureProgress,
    ByocInfrastructureTagsResponse, ByocInfrastructureValidatePostRequest,
    ByocInfrastructureValidation, InvitationPostRequest, MemberPatchRequest, NotificationType,
    Organization, OrganizationPatchRequest, PublicPreview, PublicPreviewPatchRequest,
    RoleCreateRequest, RoleUpdateRequest,
};

impl CloudClient {
    /// Every organization available to the caller, read across all pages.
    pub async fn list_organizations(
        &self,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Organization>> {
        Self::collect_cursor_pages("organization", |cursor| async move {
            self.api()
                .organization_get_list(None, cursor.as_deref())
                .await
                .map_err(|error| self.convert_error(error))
        })
        .await
    }

    pub async fn get_organization(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Organization> {
        let response = self.api().organization_get(org_id).await.map_err(|error| {
            // A read by identifier: a 400 over a well-formed UUID is a
            // missing organization, not a bad request (#666).
            self.convert_error_for_lookup(error, ResourceLookup::organization(org_id))
        })?;
        Self::unwrap_response(response)
    }

    pub async fn list_organization_quotas(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::OrganizationQuota>> {
        let response = self
            .api()
            .organization_quotas_get_list(org_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub(super) async fn list_notifications(
        &self,
        org_id: &str,
        args: &NotificationListArgs,
    ) -> CloudResult<NotificationPage> {
        let mut response = self
            .api()
            .notification_get_list(
                org_id,
                args.limit,
                args.cursor.as_deref(),
                args.severity.as_deref(),
                args.notification_type.as_deref(),
                args.resource_id.as_deref(),
                args.resource_type.as_deref(),
                args.from_date.as_deref(),
                args.to_date.as_deref(),
            )
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        let limit = response.limit;
        let next_cursor = response.next_cursor.take();
        Ok(NotificationPage {
            result: Self::unwrap_response(response)?,
            limit,
            next_cursor,
        })
    }

    pub async fn list_notification_types(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<NotificationType>> {
        let response = self
            .api()
            .notification_types_get_list(org_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_credit_balances(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::CreditBalances> {
        let response = self
            .api()
            .credit_balances_get(org_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization_quota(
        &self,
        org_id: &str,
        quota_code: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::OrganizationQuota> {
        let response = self
            .api()
            .organization_quota_get(org_id, quota_code)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_organization(
        &self,
        org_id: &str,
        request: &OrganizationPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Organization> {
        let response = self
            .api()
            .organization_update(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructureDetails> {
        let response = self
            .api()
            .organization_byoc_infrastructure_get(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_infrastructure_progress(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructureProgress> {
        let response = self
            .api()
            .organization_byoc_infrastructure_progress_get(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_private_endpoint_config(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructurePrivateEndpointConfig> {
        let response = self
            .api()
            .organization_byoc_infrastructure_private_endpoint_config_get(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_infrastructure_tags(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructureTagsResponse> {
        let response = self
            .api()
            .organization_byoc_infrastructure_tags_get(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_customer_authorized_access(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocCustomerAuthorizedAccess> {
        let response = self
            .api()
            .organization_byoc_infrastructure_customer_authorized_access_get(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn update_byoc_customer_authorized_access(
        &self,
        org_id: &str,
        byoc_id: &str,
        request: &ByocCustomerAuthorizedAccessPatchRequest,
    ) -> crate::cloud::client::Result<ByocCustomerAuthorizedAccess> {
        let response = self
            .api()
            .organization_byoc_infrastructure_customer_authorized_access_update(
                org_id, byoc_id, request,
            )
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn list_preview_features(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<PublicPreview>> {
        let response = self
            .api()
            .organization_public_previews_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_preview_feature(
        &self,
        org_id: &str,
        preview_id: &str,
        request: &PublicPreviewPatchRequest,
    ) -> crate::cloud::client::Result<PublicPreview> {
        let response = self
            .api()
            .organization_public_preview_patch(org_id, preview_id, request)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::PreviewFeature, preview_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn validate_byoc_infrastructure(
        &self,
        org_id: &str,
        request: &ByocInfrastructureValidatePostRequest,
    ) -> crate::cloud::client::Result<ByocInfrastructureValidation> {
        let response = self
            .api()
            .organization_byoc_infrastructure_validate(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_byoc_infrastructure(
        &self,
        org_id: &str,
        request: &ByocInfrastructurePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ByocConfig> {
        let response = self
            .api()
            .organization_byoc_infrastructure_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
        request: &ByocInfrastructurePatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ByocConfig> {
        let response = self
            .api()
            .organization_byoc_infrastructure_update(org_id, byoc_id, request)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn delete_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .organization_byoc_infrastructure_delete(org_id, byoc_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Byoc, byoc_id, org_id),
                )
            })?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn get_org_prometheus(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<String> {
        let filtered_metrics = filtered_metrics.map(|value| if value { "true" } else { "false" });
        self.api()
            .organization_prometheus_get(org_id, filtered_metrics)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })
    }

    pub async fn discover_org_prometheus_targets(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<
        Vec<clickhouse_cloud_api::models::PrometheusDiscoveryTargetGroup>,
    > {
        let filtered_metrics = filtered_metrics.map(|value| if value { "true" } else { "false" });
        self.api()
            .organization_prometheus_discovery_get(org_id, filtered_metrics)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })
    }

    pub async fn get_org_usage(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[String],
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::UsageCost> {
        let filters: Vec<&str> = filters.iter().map(String::as_str).collect();
        let response = self
            .api()
            .usage_cost_get(org_id, from_date, to_date, &filters)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub async fn list_organization_roles(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::RBACRole>> {
        let response = self
            .api()
            .organization_roles_get_list(org_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_get(org_id, role_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::OrganizationRole, role_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn create_organization_role(
        &self,
        org_id: &str,
        request: &RoleCreateRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_post(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
        request: &RoleUpdateRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_patch(org_id, role_id, request)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::OrganizationRole, role_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn delete_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .organization_role_delete(org_id, role_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::OrganizationRole, role_id, org_id),
                )
            })?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_members(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Member>> {
        let response = self.api().member_get_list(org_id).await.map_err(|error| {
            self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
        })?;
        Self::unwrap_response(response)
    }

    pub async fn get_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_get(org_id, user_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Member, user_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn update_member(
        &self,
        org_id: &str,
        user_id: &str,
        request: &MemberPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_update(org_id, user_id, request)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Member, user_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn delete_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .member_delete(org_id, user_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Member, user_id, org_id),
                )
            })?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_invitations(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Invitation>> {
        let response = self
            .api()
            .invitation_get_list(org_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(error, ResourceLookup::organization(org_id))
            })?;
        Self::unwrap_response(response)
    }

    pub async fn create_invitation(
        &self,
        org_id: &str,
        request: &InvitationPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_get(org_id, invitation_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Invitation, invitation_id, org_id),
                )
            })?;
        Self::unwrap_response(response)
    }

    pub async fn delete_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .invitation_delete(org_id, invitation_id)
            .await
            .map_err(|error| {
                self.convert_error_for_resource(
                    error,
                    ResourceLookup::in_org(ResourceKind::Invitation, invitation_id, org_id),
                )
            })?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn get_default_org_id(&self) -> crate::cloud::client::Result<String> {
        let organizations = self.list_organizations().await?;
        match organizations.len() {
            0 => Err(CloudError::new("No organization found for this API key")),
            1 => organizations[0]
                .id
                .map(|id| id.to_string())
                .ok_or_else(|| CloudError::new("Organization response is missing its id")),
            // The caller left out a required selector: a usage error that
            // names every candidate, since they are already fetched.
            _ => Err(CloudError::usage(multiple_organizations_message(
                &organizations,
            ))),
        }
    }
}

fn multiple_organizations_message(organizations: &[Organization]) -> String {
    let mut message = String::from(
        "these credentials can see more than one organization; \
         choose one with --org-id <ID> or --org-name <NAME>:",
    );
    for organization in organizations {
        let id = organization
            .id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "(no id)".to_string());
        let name = organization.name.as_deref().unwrap_or("(no name)");
        message.push_str(&format!("\n  {id}  {name}"));
    }
    message
}
