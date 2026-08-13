use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// List of organization services
    pub async fn instance_get_list(
        &self,
        organization_id: &str,
        filters: &[&str],
    ) -> Result<ApiResponse<Vec<Service>>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services");
        let mut req = self.request(reqwest::Method::GET, &path);
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
    /// Create new service
    pub async fn instance_create(
        &self,
        organization_id: &str,
        body: &ServicePostRequest,
    ) -> Result<ApiResponse<ServicePostResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services");
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

    /// Get service details
    pub async fn instance_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
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

    /// Update service basic details
    pub async fn instance_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicePatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
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

    /// Delete service
    pub async fn instance_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}");
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

    /// Update service password
    pub async fn instance_password_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicePasswordPatchRequest,
    ) -> Result<ApiResponse<ServicePasswordPatchResponse>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/password");
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

    /// Create a private endpoint
    pub async fn instance_private_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicPrivateEndpointePostRequest,
    ) -> Result<ApiResponse<InstancePrivateEndpoint>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/privateEndpoint");
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

    /// Get private endpoint configuration
    pub async fn instance_private_endpoint_config_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<PrivateEndpointConfig>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/privateEndpointConfig"
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

    /// Get service metrics
    pub async fn instance_prometheus_get(
        &self,
        organization_id: &str,
        service_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<String, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/prometheus");
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

    /// Update service auto scaling settings
    pub async fn instance_replica_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceReplicaScalingPatchRequest,
    ) -> Result<ApiResponse<ServiceScalingPatchResponse>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/replicaScaling");
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

    /// Update service auto scaling settings
    #[deprecated]
    #[allow(deprecated)]
    pub async fn instance_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceScalingPatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/scaling");
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

    /// Get service autoscaling schedule
    pub async fn scaling_schedule_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ScalingSchedule>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/scalingSchedule");
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

    /// Create or replace service autoscaling schedule
    pub async fn scaling_schedule_upsert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ScalingSchedulePostRequest,
    ) -> Result<ApiResponse<ScalingSchedule>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/scalingSchedule");
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

    /// Delete service scheduled scaling
    pub async fn scaling_schedule_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/scalingSchedule");
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

    /// Get service upgrade window
    pub async fn upgrade_window_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<UpgradeWindow>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/upgradeWindow");
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

    /// Set service upgrade window
    pub async fn upgrade_window_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &UpgradeWindowPutRequest,
    ) -> Result<ApiResponse<UpgradeWindow>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/upgradeWindow");
        let mut req = self.request(reqwest::Method::PUT, &path);
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

    /// Delete service upgrade window
    pub async fn upgrade_window_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/upgradeWindow");
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

    /// Get the service query endpoint for a given instance
    pub async fn instance_query_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint"
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

    /// Delete the service query endpoint for a given instance
    pub async fn instance_query_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint"
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

    /// Upsert the service query endpoint for a given instance
    pub async fn instance_query_endpoint_upsert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/serviceQueryEndpoint"
        );
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

    /// Update service state
    pub async fn instance_state_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceStatePatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let path = format!("/v1/organizations/{organization_id}/services/{service_id}/state");
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

    /// List ClickHouse settings
    pub async fn service_clickhouse_settings_list_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsList>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickhouseSettings");
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

    /// Update ClickHouse settings
    pub async fn service_clickhouse_settings_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceClickhouseSettingsPatchRequest,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsPatchResponse>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickhouseSettings");
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

    /// Get ClickHouse settings schema
    pub async fn service_clickhouse_settings_schema_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsSchema>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickhouseSettings/schema"
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

    /// Get ClickHouse setting
    pub async fn service_clickhouse_setting_get(
        &self,
        organization_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSetting>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickhouseSettings/{setting_name}"
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
    /// Delete ClickHouse setting
    pub async fn service_clickhouse_setting_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickhouseSettings/{setting_name}"
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
}
