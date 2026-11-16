use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// ClickStack: List Alerts
    ///
    /// The API defaults to 1,000 records from offset zero. Request successive
    /// pages with `limit` (1–1,000) and `offset` until a page is shorter than the limit.
    pub async fn click_stack_list_alerts(
        &self,
        organization_id: &str,
        service_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<ClickStackAlertResponse>>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_LIST_ALERTS,
            &[organization_id, service_id],
        )?;
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(offset) = offset {
            req = req.query(&[("offset", offset)]);
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
    /// ClickStack: Create Alert
    pub async fn click_stack_create_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_ALERT,
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

    /// ClickStack: Get Alert
    pub async fn click_stack_get_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_GET_ALERT,
            &[organization_id, service_id, click_stack_alert_id],
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

    /// ClickStack: Update Alert
    pub async fn click_stack_update_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
        body: &ClickStackUpdateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_ALERT,
            &[organization_id, service_id, click_stack_alert_id],
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

    /// ClickStack: Delete Alert
    pub async fn click_stack_delete_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_ALERT,
            &[organization_id, service_id, click_stack_alert_id],
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

    /// ClickStack: List Saved Searches
    ///
    /// The API defaults to 1,000 records from offset zero. Request successive
    /// pages with `limit` (1–1,000) and `offset` until a page is shorter than the limit.
    pub async fn click_stack_list_saved_searches(
        &self,
        organization_id: &str,
        service_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<ClickStackSavedSearch>>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_LIST_SAVED_SEARCHES,
            &[organization_id, service_id],
        )?;
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(offset) = offset {
            req = req.query(&[("offset", offset)]);
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

    /// ClickStack: Create Saved Search
    pub async fn click_stack_create_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackSavedSearchInput,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_SAVED_SEARCH,
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

    /// ClickStack: Get Saved Search
    pub async fn click_stack_get_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_GET_SAVED_SEARCH,
            &[organization_id, service_id, click_stack_saved_search_id],
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

    /// ClickStack: Update Saved Search
    pub async fn click_stack_update_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
        body: &ClickStackSavedSearchInput,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_SAVED_SEARCH,
            &[organization_id, service_id, click_stack_saved_search_id],
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

    /// ClickStack: Delete Saved Search
    pub async fn click_stack_delete_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_SAVED_SEARCH,
            &[organization_id, service_id, click_stack_saved_search_id],
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

    /// ClickStack: List Dashboards
    pub async fn click_stack_list_dashboards(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackDashboardResponse>>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_LIST_DASHBOARDS,
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

    /// ClickStack: Create Dashboard
    pub async fn click_stack_create_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_DASHBOARD,
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

    /// ClickStack: Get Dashboard
    pub async fn click_stack_get_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_GET_DASHBOARD,
            &[organization_id, service_id, click_stack_dashboard_id],
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

    /// ClickStack: Update Dashboard
    pub async fn click_stack_update_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
        body: &ClickStackUpdateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_DASHBOARD,
            &[organization_id, service_id, click_stack_dashboard_id],
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

    /// ClickStack: Delete Dashboard
    pub async fn click_stack_delete_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_DASHBOARD,
            &[organization_id, service_id, click_stack_dashboard_id],
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

    /// ClickStack: List Sources
    pub async fn click_stack_list_sources(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackSourceResponse>>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_LIST_SOURCES,
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

    /// ClickStack: Create Source
    pub async fn click_stack_create_source(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackSource,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_SOURCE,
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

    /// ClickStack: Get Source
    pub async fn click_stack_get_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_GET_SOURCE,
            &[organization_id, service_id, click_stack_source_id],
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

    /// ClickStack: Update Source
    pub async fn click_stack_update_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
        body: &ClickStackSource,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_SOURCE,
            &[organization_id, service_id, click_stack_source_id],
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

    /// ClickStack: Delete Source
    pub async fn click_stack_delete_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_SOURCE,
            &[organization_id, service_id, click_stack_source_id],
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

    /// ClickStack: List Roles
    pub async fn click_stack_list_roles(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackRole>>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_LIST_ROLES,
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

    /// ClickStack: Create Role
    pub async fn click_stack_create_role(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateRoleRequest,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_ROLE,
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

    /// ClickStack: Get Role
    pub async fn click_stack_get_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_GET_ROLE,
            &[organization_id, service_id, click_stack_role_id],
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

    /// ClickStack: Update Role
    pub async fn click_stack_update_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
        body: &ClickStackUpdateRoleRequest,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_ROLE,
            &[organization_id, service_id, click_stack_role_id],
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

    /// ClickStack: Delete Role
    pub async fn click_stack_delete_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_ROLE,
            &[organization_id, service_id, click_stack_role_id],
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

    /// ClickStack: List Webhooks
    ///
    /// The API defaults to 1,000 records from offset zero. Request successive
    /// pages with `limit` (1–1,000) and `offset` until a page is shorter than the limit.
    pub async fn click_stack_list_webhooks(
        &self,
        organization_id: &str,
        service_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<ClickStackWebhook>>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_LIST_WEBHOOKS,
            &[organization_id, service_id],
        )?;
        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(offset) = offset {
            req = req.query(&[("offset", offset)]);
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

    /// ClickStack: Create Webhook
    pub async fn click_stack_create_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackWebhookInput,
    ) -> Result<ApiResponse<ClickStackWebhook>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_CREATE_WEBHOOK,
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

    /// ClickStack: Update Webhook
    pub async fn click_stack_update_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_webhook_id: &str,
        body: &ClickStackWebhookInput,
    ) -> Result<ApiResponse<ClickStackWebhook>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_UPDATE_WEBHOOK,
            &[organization_id, service_id, click_stack_webhook_id],
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

    /// ClickStack: Delete Webhook
    pub async fn click_stack_delete_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_webhook_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::CLICK_STACK_DELETE_WEBHOOK,
            &[organization_id, service_id, click_stack_webhook_id],
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

    /// ClickStack: Validate Webhook
    ///
    /// Checks a webhook body without saving it; the API does not check name
    /// uniqueness.
    pub async fn click_stack_validate_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackWebhookInput,
    ) -> Result<ApiResponse<ClickStackValidateWebhookResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_VALIDATE_WEBHOOK,
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

    /// ClickStack: Validate Dashboard
    pub async fn click_stack_validate_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackValidateDashboardResponse>, Error> {
        let mut req = self.request(
            &operations::CLICK_STACK_VALIDATE_DASHBOARD,
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
}
