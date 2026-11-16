use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// Get active organization credit balances.
    pub async fn credit_balances_get(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<CreditBalances>, Error> {
        let req = self.request(&operations::CREDIT_BALANCES_GET, &[organization_id])?;

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
        let mut req = self.request(&operations::ACTIVE_BALANCES_GET, &[organization_id])?;
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
        let mut req = self.request(&operations::ORGANIZATION_GET_LIST, &[])?;
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
        let req = self.request(&operations::ORGANIZATION_GET, &[organization_id])?;
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
        let req = self.request(
            &operations::ORGANIZATION_QUOTAS_GET_LIST,
            &[organization_id],
        )?;
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
        let req = self.request(
            &operations::ORGANIZATION_QUOTA_GET,
            &[organization_id, quota_code],
        )?;
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
        let mut req = self.request(&operations::ORGANIZATION_UPDATE, &[organization_id])?;
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
        let mut req = self.request(&operations::NOTIFICATION_GET_LIST, &[organization_id])?;
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
        let req = self.request(&operations::NOTIFICATION_TYPES_GET_LIST, &[organization_id])?;
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
        let mut req = self.request(
            &operations::ORGANIZATION_PRIVATE_ENDPOINT_CONFIG_GET_LIST,
            &[organization_id],
        )?;
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
        let mut req = self.request(&operations::ORGANIZATION_PROMETHEUS_GET, &[organization_id])?;
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
        let mut req = self.request(
            &operations::ORGANIZATION_PROMETHEUS_DISCOVERY_GET,
            &[organization_id],
        )?;
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
        let mut req = self.request(&operations::USAGE_COST_GET, &[organization_id])?;
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

    /// Get list of organization preview features (Beta)
    pub async fn organization_public_previews_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<PublicPreview>>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_PUBLIC_PREVIEWS_GET_LIST,
            &[organization_id],
        )?;
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

    /// Update organization preview feature (Beta)
    ///
    /// `preview_id` is a `PublicPreview.id`. Only previews with `can_toggle`
    /// can be switched.
    pub async fn organization_public_preview_patch(
        &self,
        organization_id: &str,
        preview_id: &str,
        body: &PublicPreviewPatchRequest,
    ) -> Result<ApiResponse<PublicPreview>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_PUBLIC_PREVIEW_PATCH,
            &[organization_id, preview_id],
        )?;
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
}
