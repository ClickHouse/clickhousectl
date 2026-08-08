use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
    /// ClickStack: List Alerts
    pub async fn click_stack_list_alerts(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackAlertResponse>>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts");
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
    /// ClickStack: Create Alert
    pub async fn click_stack_create_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts");
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

    /// ClickStack: Get Alert
    pub async fn click_stack_get_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}"
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

    /// ClickStack: Update Alert
    pub async fn click_stack_update_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
        body: &ClickStackUpdateAlertRequest,
    ) -> Result<ApiResponse<ClickStackAlertResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}"
        );
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

    /// ClickStack: Delete Alert
    pub async fn click_stack_delete_alert(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_alert_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/alerts/{click_stack_alert_id}"
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

    /// ClickStack: List Saved Searches
    pub async fn click_stack_list_saved_searches(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackSavedSearch>>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/saved-searches"
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

    /// ClickStack: Create Saved Search
    pub async fn click_stack_create_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackSavedSearchInput,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/saved-searches"
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

    /// ClickStack: Get Saved Search
    pub async fn click_stack_get_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/saved-searches/{click_stack_saved_search_id}"
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

    /// ClickStack: Update Saved Search
    pub async fn click_stack_update_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
        body: &ClickStackSavedSearchInput,
    ) -> Result<ApiResponse<ClickStackSavedSearch>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/saved-searches/{click_stack_saved_search_id}"
        );
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

    /// ClickStack: Delete Saved Search
    pub async fn click_stack_delete_saved_search(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_saved_search_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/saved-searches/{click_stack_saved_search_id}"
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

    /// ClickStack: List Dashboards
    pub async fn click_stack_list_dashboards(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackDashboardResponse>>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards"
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

    /// ClickStack: Create Dashboard
    pub async fn click_stack_create_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards"
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

    /// ClickStack: Get Dashboard
    pub async fn click_stack_get_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}"
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

    /// ClickStack: Update Dashboard
    pub async fn click_stack_update_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
        body: &ClickStackUpdateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackDashboardResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}"
        );
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

    /// ClickStack: Delete Dashboard
    pub async fn click_stack_delete_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_dashboard_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/{click_stack_dashboard_id}"
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

    /// ClickStack: List Sources
    pub async fn click_stack_list_sources(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackSourceResponse>>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources");
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

    /// ClickStack: Create Source
    pub async fn click_stack_create_source(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackSource,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources");
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

    /// ClickStack: Get Source
    pub async fn click_stack_get_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources/{click_stack_source_id}"
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

    /// ClickStack: Update Source
    pub async fn click_stack_update_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
        body: &ClickStackSource,
    ) -> Result<ApiResponse<ClickStackSourceResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources/{click_stack_source_id}"
        );
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

    /// ClickStack: Delete Source
    pub async fn click_stack_delete_source(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_source_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/sources/{click_stack_source_id}"
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

    /// ClickStack: List Roles
    pub async fn click_stack_list_roles(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackRole>>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/roles");
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

    /// ClickStack: Create Role
    pub async fn click_stack_create_role(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateRoleRequest,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let path =
            format!("/v1/organizations/{organization_id}/services/{service_id}/clickstack/roles");
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

    /// ClickStack: Get Role
    pub async fn click_stack_get_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/roles/{click_stack_role_id}"
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

    /// ClickStack: Update Role
    pub async fn click_stack_update_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
        body: &ClickStackUpdateRoleRequest,
    ) -> Result<ApiResponse<ClickStackRole>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/roles/{click_stack_role_id}"
        );
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

    /// ClickStack: Delete Role
    pub async fn click_stack_delete_role(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_role_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/roles/{click_stack_role_id}"
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

    /// ClickStack: List Webhooks
    pub async fn click_stack_list_webhooks(
        &self,
        organization_id: &str,
        service_id: &str,
    ) -> Result<ApiResponse<Vec<ClickStackWebhook>>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/webhooks"
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

    /// ClickStack: Create Webhook
    pub async fn click_stack_create_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackWebhookInput,
    ) -> Result<ApiResponse<ClickStackWebhook>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/webhooks"
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

    /// ClickStack: Update Webhook
    pub async fn click_stack_update_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_webhook_id: &str,
        body: &ClickStackWebhookInput,
    ) -> Result<ApiResponse<ClickStackWebhook>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/webhooks/{click_stack_webhook_id}"
        );
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

    /// ClickStack: Delete Webhook
    pub async fn click_stack_delete_webhook(
        &self,
        organization_id: &str,
        service_id: &str,
        click_stack_webhook_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/webhooks/{click_stack_webhook_id}"
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

    /// ClickStack: Validate Dashboard
    pub async fn click_stack_validate_dashboard(
        &self,
        organization_id: &str,
        service_id: &str,
        body: &ClickStackCreateDashboardRequest,
    ) -> Result<ApiResponse<ClickStackValidateDashboardResponse>, Error> {
        let path = format!(
            "/v1/organizations/{organization_id}/services/{service_id}/clickstack/dashboards/validate"
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
}
