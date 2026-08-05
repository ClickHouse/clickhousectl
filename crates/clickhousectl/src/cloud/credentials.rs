use crate::init;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub service_query_keys: HashMap<String, ServiceQueryKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceQueryKey {
    /// Organization in which the management API key was provisioned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    /// Management API resource ID used to delete this exact key.
    ///
    /// Records written before the cleanup metadata was introduced remain
    /// usable for queries, but cannot be safely cleaned up by service deletion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
    pub key_id: String,
    pub key_secret: String,
    /// The query endpoint the key is bound to, when the upsert echoed it.
    /// Recorded for diagnostics only — authentication uses the key pair — so
    /// an endpoint response that omits `id` still yields a usable record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    pub service_name: String,
    pub created_at: DateTime<Utc>,
}

pub fn credentials_path() -> PathBuf {
    init::local_dir().join("credentials.json")
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn clear_credentials() {
    let path = credentials_path();
    let _ = std::fs::remove_file(path);
}

pub fn save_credentials(creds: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let dir = init::local_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(".gitignore"), "*\n")?;
    }

    let path = credentials_path();
    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn get_service_query_key(service_id: &str) -> Option<ServiceQueryKey> {
    let creds = load_credentials()?;
    creds.service_query_keys.get(service_id).cloned()
}

pub fn try_get_service_query_key(
    service_id: &str,
) -> Result<Option<ServiceQueryKey>, Box<dyn std::error::Error>> {
    let path = credentials_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!("failed to read {}: {error}", path.display()).into());
        }
    };
    let creds: Credentials = serde_json::from_str(&data)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(creds.service_query_keys.get(service_id).cloned())
}

pub fn set_service_query_key(
    service_id: &str,
    key: ServiceQueryKey,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut creds = load_credentials().unwrap_or_default();
    creds.service_query_keys.insert(service_id.to_string(), key);
    save_credentials(&creds)
}

pub fn remove_service_query_key(service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut creds) = load_credentials() else {
        return Ok(());
    };
    if creds.service_query_keys.remove(service_id).is_some() {
        save_credentials(&creds)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_credentials_round_trip() {
        // Legacy creds files have only api_key/api_secret as bare strings.
        let raw = r#"{"api_key":"k","api_secret":"s"}"#;
        let creds: Credentials = serde_json::from_str(raw).unwrap();
        assert_eq!(creds.api_key.as_deref(), Some("k"));
        assert_eq!(creds.api_secret.as_deref(), Some("s"));
        assert!(creds.service_query_keys.is_empty());

        let written = serde_json::to_string(&creds).unwrap();
        assert!(written.contains("\"api_key\":\"k\""));
        assert!(!written.contains("service_query_keys"));
    }

    #[test]
    fn service_query_keys_round_trip() {
        let mut creds = Credentials::default();
        creds.service_query_keys.insert(
            "svc-1".into(),
            ServiceQueryKey {
                organization_id: Some("org-1".into()),
                api_key_id: Some("api-key-uuid".into()),
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: Some("ep".into()),
                service_name: "demo".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            },
        );

        let s = serde_json::to_string(&creds).unwrap();
        assert!(s.contains("\"organization_id\":\"org-1\""));
        assert!(s.contains("\"api_key_id\":\"api-key-uuid\""));
        let back: Credentials = serde_json::from_str(&s).unwrap();
        let key = back.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.organization_id.as_deref(), Some("org-1"));
        assert_eq!(key.api_key_id.as_deref(), Some("api-key-uuid"));
        assert_eq!(key.key_id, "kid");
        assert_eq!(key.key_secret, "sec");
        assert_eq!(key.endpoint_id.as_deref(), Some("ep"));
        assert_eq!(key.service_name, "demo");
    }

    #[test]
    fn stored_key_without_an_endpoint_id_round_trips_with_the_field_omitted() {
        let mut creds = Credentials::default();
        creds.service_query_keys.insert(
            "svc-1".into(),
            ServiceQueryKey {
                organization_id: Some("org-1".into()),
                api_key_id: Some("api-key-uuid".into()),
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: None,
                service_name: "demo".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            },
        );

        let s = serde_json::to_string(&creds).unwrap();
        assert!(!s.contains("endpoint_id"), "absent means omitted: {s}");
        let back: Credentials = serde_json::from_str(&s).unwrap();
        let key = back.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.key_id, "kid");
        assert_eq!(key.key_secret, "sec");
        assert_eq!(key.endpoint_id, None);
    }

    #[test]
    fn existing_query_keys_without_cleanup_metadata_still_deserialize() {
        // Existing files contain query credentials and an endpoint ID, but
        // not the ownership metadata added for exact cloud-side cleanup.
        let raw = r#"{"service_query_keys":{"svc-1":{"key_id":"kid","key_secret":"sec",
            "endpoint_id":"ep","service_name":"demo","created_at":"2026-05-11T12:00:00Z"}}}"#;
        let creds: Credentials = serde_json::from_str(raw).unwrap();
        let key = creds.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.organization_id, None);
        assert_eq!(key.api_key_id, None);
        assert_eq!(key.endpoint_id.as_deref(), Some("ep"));
        assert_eq!(key.key_id, "kid");

        let written = serde_json::to_string(&creds).unwrap();
        assert!(!written.contains("organization_id"));
        assert!(!written.contains("api_key_id"));
    }
}
