use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// Get active organization credit balances.
    pub async fn credit_balances_get(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<CreditBalances>, Error> {
        let path = format!("/v1/organizations/{organization_id}/creditBalances");
        let req = self.request(reqwest::Method::GET, &path);

        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization active prepaid balances
    pub async fn active_balances_get(
        &self,
        organization_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<ActiveBalances>, Error> {
        let path = format!("/v1/organizations/{organization_id}/activeBalances");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = limit {
            req = req.query(&[("limit", v)]);
        }
        if let Some(v) = offset {
            req = req.query(&[("offset", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get one page of the organizations available to the caller, oldest first.
    ///
    /// `limit` must be at least 1; omission uses the server default of 250. Pass
    /// the response's `next_cursor` to fetch another page, until it is `None`.
    pub async fn organization_get_list(
        &self,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ApiResponse<Vec<Organization>>, Error> {
        let path = "/v1/organizations".to_string();
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(cursor) = cursor {
            req = req.query(&[("cursor", cursor)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization details
    pub async fn organization_get(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Organization>, Error> {
        let path = format!("/v1/organizations/{organization_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization quotas
    pub async fn organization_quotas_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<OrganizationQuota>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/quotas");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization quota details
    pub async fn organization_quota_get(
        &self,
        organization_id: &str,
        quota_code: &str,
    ) -> Result<ApiResponse<OrganizationQuota>, Error> {
        let path = format!("/v1/organizations/{organization_id}/quotas/{quota_code}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update organization details
    pub async fn organization_update(
        &self,
        organization_id: &str,
        body: &OrganizationPatchRequest,
    ) -> Result<ApiResponse<Organization>, Error> {
        let path = format!("/v1/organizations/{organization_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_create(
        &self,
        organization_id: &str,
        body: &ByocInfrastructurePostRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Validate BYOC infrastructure prerequisites
    ///
    /// Preflight-checks the cloud account's permissions without creating infrastructure.
    pub async fn organization_byoc_infrastructure_validate(
        &self,
        organization_id: &str,
        body: &ByocInfrastructureValidatePostRequest,
    ) -> Result<ApiResponse<ByocInfrastructureValidation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/byocInfrastructure/validate");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get BYOC infrastructure details
    pub async fn organization_byoc_infrastructure_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureDetails>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get BYOC infrastructure provisioning progress
    pub async fn organization_byoc_infrastructure_progress_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureProgress>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/progress"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get one page of organization notifications, most recent first (Beta).
    ///
    /// Pass `next_cursor` unchanged as `cursor` until it is absent. The server
    /// defaults `limit` to 100 and clamps values above 250. Date filters are
    /// RFC 3339 timestamps. Results are restricted to resources the caller can access.
    #[allow(clippy::too_many_arguments)]
    pub async fn notification_get_list(
        &self,
        organization_id: &str,
        limit: Option<i64>,
        cursor: Option<&str>,
        severity: Option<&str>,
        r#type: Option<&str>,
        resource_id: Option<&str>,
        resource_type: Option<&str>,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<ApiResponse<Vec<Notification>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/notifications");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(value) = limit {
            req = req.query(&[("limit", value)]);
        }
        if let Some(value) = cursor {
            req = req.query(&[("cursor", value)]);
        }
        if let Some(value) = severity {
            req = req.query(&[("severity", value)]);
        }
        if let Some(value) = r#type {
            req = req.query(&[("type", value)]);
        }
        if let Some(value) = resource_id {
            req = req.query(&[("resource_id", value)]);
        }
        if let Some(value) = resource_type {
            req = req.query(&[("resource_type", value)]);
        }
        if let Some(value) = from_date {
            req = req.query(&[("from_date", value)]);
        }
        if let Some(value) = to_date {
            req = req.query(&[("to_date", value)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List notification types
    ///
    /// Returns every notification type the organization is entitled to; types
    /// the caller lacks permission to use carry `is_available: Some(false)`.
    pub async fn notification_types_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<NotificationType>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/notificationTypes");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get BYOC infrastructure private endpoint configuration
    ///
    /// For BYOC organizations this replaces the deprecated organization-level
    /// `organization_private_endpoint_config_get_list`.
    pub async fn organization_byoc_infrastructure_private_endpoint_config_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructurePrivateEndpointConfig>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/privateEndpointConfig"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get BYOC infrastructure tags
    ///
    /// Tags are replaced as a whole through `organization_byoc_infrastructure_update`.
    pub async fn organization_byoc_infrastructure_tags_get(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<ByocInfrastructureTagsResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}/tags"
        );
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Remove a BYOC infrastructure
    pub async fn organization_byoc_infrastructure_delete(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
        );
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update BYOC Infrastructure
    pub async fn organization_byoc_infrastructure_update(
        &self,
        organization_id: &str,
        byoc_infrastructure_id: &str,
        body: &ByocInfrastructurePatchRequest,
    ) -> Result<ApiResponse<ByocConfig>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/byocInfrastructure/{byoc_infrastructure_id}"
        );
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List all invitations
    pub async fn invitation_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<Invitation>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create an invitation
    pub async fn invitation_create(
        &self,
        organization_id: &str,
        body: &InvitationPostRequest,
    ) -> Result<ApiResponse<Invitation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get invitation details
    pub async fn invitation_get(
        &self,
        organization_id: &str,
        invitation_id: &str,
    ) -> Result<ApiResponse<Invitation>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations/{invitation_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete organization invitation
    pub async fn invitation_delete(
        &self,
        organization_id: &str,
        invitation_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/invitations/{invitation_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List organization members
    pub async fn member_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<Member>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get member details
    pub async fn member_get(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<ApiResponse<Member>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update organization member
    pub async fn member_update(
        &self,
        organization_id: &str,
        user_id: &str,
        body: &MemberPatchRequest,
    ) -> Result<ApiResponse<Member>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Remove an organization member
    pub async fn member_delete(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/members/{user_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Discover the complete, unpaginated caller-assignable role array.
    ///
    /// Requires Bearer authentication; the server verifies the token's CLI user
    /// identity and current organization authority. This is a read-time snapshot,
    /// not an approval guarantee. Never falls back to the administrative catalog.
    ///
    /// Transport fields tolerate missing/null values. Callers must validate the
    /// required discovery fields before human or JSON success output: absent
    /// result/policies/permissions/resources are not empty permission guarantees.
    pub async fn organization_assignable_roles_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<AssignableRole>>, Error> {
        if !matches!(&self.auth, super::Auth::Bearer { .. }) {
            return Err(Error::AuthMismatch(
                "organization_assignable_roles_get_list requires a CLI user Bearer token".into(),
            ));
        }
        let path = format!("/v1/organizations/{organization_id}/roles");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// List all available roles for an organization
    pub async fn organization_roles_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<RBACRole>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/roles");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Create a new role
    pub async fn organization_role_post(
        &self,
        organization_id: &str,
        body: &RoleCreateRequest,
    ) -> Result<ApiResponse<RBACRole>, Error> {
        let path = format!("/v1/organizations/{organization_id}/roles");
        let mut req = self.request(reqwest::Method::POST, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get role details
    pub async fn organization_role_get(
        &self,
        organization_id: &str,
        role_id: &str,
    ) -> Result<ApiResponse<RBACRole>, Error> {
        let path = format!("/v1/organizations/{organization_id}/roles/{role_id}");
        let req = self.request(reqwest::Method::GET, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Update a role
    pub async fn organization_role_patch(
        &self,
        organization_id: &str,
        role_id: &str,
        body: &RoleUpdateRequest,
    ) -> Result<ApiResponse<RBACRole>, Error> {
        let path = format!("/v1/organizations/{organization_id}/roles/{role_id}");
        let mut req = self.request(reqwest::Method::PATCH, &path);
        req = req.json(body);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Delete a role
    pub async fn organization_role_delete(
        &self,
        organization_id: &str,
        role_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/roles/{role_id}");
        let req = self.request(reqwest::Method::DELETE, &path);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get private endpoint configuration for region within cloud provider for an organization
    #[deprecated]
    #[allow(deprecated)]
    pub async fn organization_private_endpoint_config_get_list(
        &self,
        organization_id: &str,
        cloud_provider: &str,
        region_id: &str,
    ) -> Result<ApiResponse<OrganizationCloudRegionPrivateEndpointConfig>, Error> {
        let path = format!("/v1/organizations/{organization_id}/privateEndpointConfig");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("cloud_provider", cloud_provider)]);
        req = req.query(&[("region_id", region_id)]);
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization metrics
    pub async fn organization_prometheus_get(
        &self,
        organization_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/prometheus");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = filtered_metrics {
            req = req.query(&[("filtered_metrics", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text),
            });
        }
        Ok(resp.text().await?)
    }

    /// Discover Prometheus scrape targets for an organization
    pub async fn organization_prometheus_discovery_get(
        &self,
        organization_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<Vec<PrometheusDiscoveryTargetGroup>, Error> {
        let path = format!("/v1/organizations/{organization_id}/prometheus/discovery");
        let mut req = self.request(reqwest::Method::GET, &path);
        if let Some(v) = filtered_metrics {
            req = req.query(&[("filtered_metrics", v)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }

    /// Get organization usage costs
    pub async fn usage_cost_get(
        &self,
        organization_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[&str],
    ) -> Result<ApiResponse<UsageCost>, Error> {
        let path = format!("/v1/organizations/{organization_id}/usageCost");
        let mut req = self.request(reqwest::Method::GET, &path);
        req = req.query(&[("from_date", from_date)]);
        req = req.query(&[("to_date", to_date)]);
        for f in filters {
            req = req.query(&[("filter", f)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: serde_json::from_str::<ApiResponse<serde_json::Value>>(&body_text)
                    .ok()
                    .and_then(|r| r.error)
                    .unwrap_or(body_text.clone()),
            });
        }
        Ok(serde_json::from_str(&body_text)?)
    }
}
