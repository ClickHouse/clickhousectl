use super::Client;
use crate::error::Error;
use crate::meta::operations;
use crate::models::*;

impl Client {
    /// List all available roles for an organization
    pub async fn organization_roles_get_list(
        &self,
        organization_id: &str,
    ) -> Result<ApiResponse<Vec<RBACRole>>, Error> {
        let req = self.request(&operations::ORGANIZATION_ROLES_GET_LIST, &[organization_id])?;
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
        let mut req = self.request(&operations::ORGANIZATION_ROLE_POST, &[organization_id])?;
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
        let req = self.request(
            &operations::ORGANIZATION_ROLE_GET,
            &[organization_id, role_id],
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

    /// Update a role
    pub async fn organization_role_patch(
        &self,
        organization_id: &str,
        role_id: &str,
        body: &RoleUpdateRequest,
    ) -> Result<ApiResponse<RBACRole>, Error> {
        let mut req = self.request(
            &operations::ORGANIZATION_ROLE_PATCH,
            &[organization_id, role_id],
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

    /// Delete a role
    pub async fn organization_role_delete(
        &self,
        organization_id: &str,
        role_id: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let req = self.request(
            &operations::ORGANIZATION_ROLE_DELETE,
            &[organization_id, role_id],
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
