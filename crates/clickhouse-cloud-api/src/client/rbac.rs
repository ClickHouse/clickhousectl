use super::Client;
use crate::error::Error;
use crate::models::*;

impl Client {
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
}
