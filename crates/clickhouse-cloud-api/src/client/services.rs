use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// List available custom service profiles.
    ///
    /// Supply `region_id` unless `byoc_id` selects an infrastructure. When both
    /// are supplied, the region must match the infrastructure's region.
    pub async fn service_profiles_list(
        &self,
        organization_id: &str,
        region_id: Option<&str>,
        byoc_id: Option<&str>,
    ) -> Result<ApiResponse<Vec<ServiceProfile>>, Error> {
        let req = self.request(&operations::SERVICE_PROFILES_LIST, &[organization_id])?;
        let req = if let Some(region_id) = region_id {
            req.query(&[("region_id", region_id)])
        } else {
            req
        };
        let req = if let Some(byoc_id) = byoc_id {
            req.query(&[("byoc_id", byoc_id)])
        } else {
            req
        };
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

    /// List of organization services
    pub async fn instance_get_list(
        &self,
        organization_id: &str,
        filters: &[&str],
    ) -> Result<ApiResponse<Vec<Service>>, Error> {
        let mut req = self.request(&operations::INSTANCE_GET_LIST, &[organization_id])?;
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
        let mut req = self.request(&operations::INSTANCE_CREATE, &[organization_id])?;
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
        let req = self.request(&operations::INSTANCE_GET, &[organization_id, service_id])?;
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
        let mut req = self.request(&operations::INSTANCE_UPDATE, &[organization_id, service_id])?;
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
        let req = self.request(&operations::INSTANCE_DELETE, &[organization_id, service_id])?;
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
        let mut req = self.request(
            &operations::INSTANCE_PASSWORD_UPDATE,
            &[organization_id, service_id],
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

    /// Create a private endpoint
    pub async fn instance_private_endpoint_create(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServicPrivateEndpointePostRequest,
    ) -> Result<ApiResponse<InstancePrivateEndpoint>, Error> {
        let mut req = self.request(
            &operations::INSTANCE_PRIVATE_ENDPOINT_CREATE,
            &[organization_id, service_id],
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

    /// Get private endpoint configuration
    pub async fn instance_private_endpoint_config_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<PrivateEndpointConfig>, Error> {
        let req = self.request(
            &operations::INSTANCE_PRIVATE_ENDPOINT_CONFIG_GET,
            &[organization_id, service_id],
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

    /// Get service metrics
    pub async fn instance_prometheus_get(
        &self,
        organization_id: &str,
        service_id: &str,
        filtered_metrics: Option<&str>,
    ) -> Result<String, Error> {
        let mut req = self.request(
            &operations::INSTANCE_PROMETHEUS_GET,
            &[organization_id, service_id],
        )?;
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

    /// Get replica aware routing
    pub async fn replica_aware_routing_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ReplicaAwareRouting>, Error> {
        let req = self.request(
            &operations::REPLICA_AWARE_ROUTING_GET,
            &[organization_id, service_id],
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

    /// Update replica aware routing
    ///
    /// Provide at least one field; an omitted field is left unchanged.
    pub async fn replica_aware_routing_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ReplicaAwareRoutingPatchRequest,
    ) -> Result<ApiResponse<ReplicaAwareRouting>, Error> {
        let mut req = self.request(
            &operations::REPLICA_AWARE_ROUTING_UPDATE,
            &[organization_id, service_id],
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

    /// Update service auto scaling settings
    pub async fn instance_replica_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceReplicaScalingPatchRequest,
    ) -> Result<ApiResponse<ServiceScalingPatchResponse>, Error> {
        let mut req = self.request(
            &operations::INSTANCE_REPLICA_SCALING_UPDATE,
            &[organization_id, service_id],
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

    /// Update service auto scaling settings
    #[deprecated]
    #[allow(deprecated)]
    pub async fn instance_scaling_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceScalingPatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let mut req = self.request(
            &operations::INSTANCE_SCALING_UPDATE,
            &[organization_id, service_id],
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

    /// Get service autoscaling schedule
    pub async fn scaling_schedule_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ScalingSchedule>, Error> {
        let req = self.request(
            &operations::SCALING_SCHEDULE_GET,
            &[organization_id, service_id],
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

    /// Create or replace service autoscaling schedule
    pub async fn scaling_schedule_upsert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ScalingSchedulePostRequest,
    ) -> Result<ApiResponse<ScalingSchedule>, Error> {
        let mut req = self.request(
            &operations::SCALING_SCHEDULE_UPSERT,
            &[organization_id, service_id],
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

    /// Delete service scheduled scaling
    pub async fn scaling_schedule_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::SCALING_SCHEDULE_DELETE,
            &[organization_id, service_id],
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

    /// Get service upgrade window
    pub async fn upgrade_window_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<UpgradeWindow>, Error> {
        let req = self.request(
            &operations::UPGRADE_WINDOW_GET,
            &[organization_id, service_id],
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

    /// Set service upgrade window
    pub async fn upgrade_window_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &UpgradeWindowPutRequest,
    ) -> Result<ApiResponse<UpgradeWindow>, Error> {
        let mut req = self.request(
            &operations::UPGRADE_WINDOW_UPDATE,
            &[organization_id, service_id],
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

    /// Delete service upgrade window
    pub async fn upgrade_window_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::UPGRADE_WINDOW_DELETE,
            &[organization_id, service_id],
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

    /// Get the service query endpoint for a given instance
    pub async fn instance_query_endpoint_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let req = self.request(
            &operations::INSTANCE_QUERY_ENDPOINT_GET,
            &[organization_id, service_id],
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

    /// Delete the service query endpoint for a given instance
    pub async fn instance_query_endpoint_delete(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::INSTANCE_QUERY_ENDPOINT_DELETE,
            &[organization_id, service_id],
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

    /// Upsert the service query endpoint for a given instance
    pub async fn instance_query_endpoint_upsert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> Result<ApiResponse<ServiceQueryAPIEndpoint>, Error> {
        let mut req = self.request(
            &operations::INSTANCE_QUERY_ENDPOINT_UPSERT,
            &[organization_id, service_id],
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

    /// Update service state
    pub async fn instance_state_update(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceStatePatchRequest,
    ) -> Result<ApiResponse<Service>, Error> {
        let mut req = self.request(
            &operations::INSTANCE_STATE_UPDATE,
            &[organization_id, service_id],
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

    /// List ClickHouse settings
    pub async fn service_clickhouse_settings_list_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsList>, Error> {
        let req = self.request(
            &operations::SERVICE_CLICKHOUSE_SETTINGS_LIST_GET,
            &[organization_id, service_id],
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

    /// Update ClickHouse settings
    pub async fn service_clickhouse_settings_update<T: serde::Serialize>(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ServiceClickhouseSettingsPatchRequest<T>,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsPatchResponse>, Error> {
        let mut req = self.request(
            &operations::SERVICE_CLICKHOUSE_SETTINGS_UPDATE,
            &[organization_id, service_id],
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

    /// Get ClickHouse settings schema
    pub async fn service_clickhouse_settings_schema_get(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSettingsSchema>, Error> {
        let req = self.request(
            &operations::SERVICE_CLICKHOUSE_SETTINGS_SCHEMA_GET,
            &[organization_id, service_id],
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

    /// Get ClickHouse setting
    pub async fn service_clickhouse_setting_get(
        &self,
        organization_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> Result<ApiResponse<ServiceClickhouseSetting>, Error> {
        let req = self.request(
            &operations::SERVICE_CLICKHOUSE_SETTING_GET,
            &[organization_id, service_id, setting_name],
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
    /// Delete ClickHouse setting
    pub async fn service_clickhouse_setting_delete(
        &self,
        organization_id: &str,
        service_id: &str,
        setting_name: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::SERVICE_CLICKHOUSE_SETTING_DELETE,
            &[organization_id, service_id, setting_name],
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
}
