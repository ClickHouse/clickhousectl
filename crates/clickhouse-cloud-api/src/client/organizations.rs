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

    /// Get list of available organizations
    pub async fn organization_get_list(&self) -> Result<ApiResponse<Vec<Organization>>, Error> {
        let path = "/v1/organizations".to_string();
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
