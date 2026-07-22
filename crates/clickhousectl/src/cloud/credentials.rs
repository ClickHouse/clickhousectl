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
    pub key_id: String,
    pub key_secret: String,
    pub endpoint_id: String,
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
                key_id: "kid".into(),
                key_secret: "sec".into(),
                endpoint_id: "ep".into(),
                service_name: "demo".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-05-11T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            },
        );

        let s = serde_json::to_string(&creds).unwrap();
        let back: Credentials = serde_json::from_str(&s).unwrap();
        let key = back.service_query_keys.get("svc-1").unwrap();
        assert_eq!(key.key_id, "kid");
        assert_eq!(key.key_secret, "sec");
        assert_eq!(key.endpoint_id, "ep");
        assert_eq!(key.service_name, "demo");
    }
}
