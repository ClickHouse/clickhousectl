use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// List organization members
    pub async fn member_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<Member>>, Error> {
        let req = self.request(&operations::MEMBER_GET_LIST, &[organization_id])?;
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
        let req = self.request(&operations::MEMBER_GET, &[organization_id, user_id])?;
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
        let mut req = self.request(&operations::MEMBER_UPDATE, &[organization_id, user_id])?;
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
        let req = self.request(&operations::MEMBER_DELETE, &[organization_id, user_id])?;
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

    /// Get one page of a member's notification settings (Beta).
    ///
    /// Lists only the member's own overrides of the role defaults. Pass
    /// `next_cursor` unchanged as `cursor` until it is absent. The server
    /// defaults `limit` to 100 and accepts 1 to 250.
    #[allow(clippy::too_many_arguments)]
    pub async fn member_notification_settings_get_list(
        &self,
        organization_id: &str,
        user_id: &str,
        limit: Option<i64>,
        cursor: Option<&str>,
        r#type: Option<&str>,
        resource_id: Option<&str>,
        resource_type: Option<&str>,
    ) -> Result<ApiResponse<Vec<NotificationSetting>>, Error> {
        let mut req = self.request(
            &operations::MEMBER_NOTIFICATION_SETTINGS_GET_LIST,
            &[organization_id, user_id],
        )?;
        if let Some(value) = limit {
            req = req.query(&[("limit", value)]);
        }
        if let Some(value) = cursor {
            req = req.query(&[("cursor", value)]);
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

    /// Update a member's notification settings (Beta).
    ///
    /// Applies every rule in order. A rule with effect `inherit` removes the
    /// setting, so the role default applies again.
    pub async fn member_notification_settings_update(
        &self,
        organization_id: &str,
        user_id: &str,
        body: &MemberNotificationSettingsPatchRequest,
    ) -> Result<ApiResponse<MemberNotificationSettingsPatchResponse>, Error> {
        let mut req = self.request(
            &operations::MEMBER_NOTIFICATION_SETTINGS_UPDATE,
            &[organization_id, user_id],
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
