use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AUDIENCE: &str = "clickhousectl";
const SCOPE: &str = "openid profile email offline_access";

const DEFAULT_API_URL: &str = "https://api.clickhouse.cloud/v1";

struct AuthConfig {
    auth_url: &'static str,
    client_id: &'static str,
}

/// Known API host → auth configuration mappings.
const KNOWN_CONFIGS: &[(&str, AuthConfig)] = &[
    (
        "api.clickhouse.cloud",
        AuthConfig {
            auth_url: "https://auth.clickhouse.cloud",
            client_id: "9q6XAueAs47R4X5d1d6FbjbJqjsrA2ZJ",
        },
    ),
    (
        "api.clickhouse-staging.com",
        AuthConfig {
            auth_url: "https://auth.control-plane.clickhouse-staging.com",
            client_id: "ZC8AupPshQt2UNO2hEDutnKitx4PhizY",
        },
    ),
    (
        "api.clickhouse-dev.com",
        AuthConfig {
            auth_url: "https://auth.control-plane.clickhouse-dev.com",
            client_id: "bVVcrqNw1t5dya9WFzfnM7PSsAgmfzwY",
        },
    ),
];

fn auth_config_for_url(api_url: &str) -> Option<&'static AuthConfig> {
    let parsed = url::Url::parse(api_url).ok()?;
    let host = parsed.host_str()?;
    KNOWN_CONFIGS
        .iter()
        .find(|(known_host, _)| host == *known_host)
        .map(|(_, config)| config)
}

/// Normalize a user-provided URL into the API base URL we store and use.
/// Ensures it has a scheme and the /v1 path suffix.
pub fn normalize_api_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.ends_with("/v1") {
        url.to_string()
    } else {
        format!("{url}/v1")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStore {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    /// The API base URL these tokens were issued for (e.g. "https://api.clickhouse.cloud/v1").
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[allow(dead_code)]
    error_description: Option<String>,
}

pub fn tokens_path() -> PathBuf {
    crate::init::local_dir().join("tokens.json")
}

pub fn load_tokens() -> Option<TokenStore> {
    let path = tokens_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_tokens(tokens: &TokenStore) -> Result<(), Box<dyn std::error::Error>> {
    let path = tokens_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(tokens)?;
    std::fs::write(&path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn clear_tokens() {
    let path = tokens_path();
    let _ = std::fs::remove_file(path);
}

pub fn is_token_valid(tokens: &TokenStore) -> bool {
    let now = chrono::Utc::now().timestamp();
    tokens.expires_at > now + 60
}

pub async fn device_auth_login(api_url: &str) -> Result<TokenStore, Box<dyn std::error::Error>> {
    let api_url = normalize_api_url(api_url);
    let config = auth_config_for_url(&api_url).ok_or_else(|| {
        format!(
            "Unknown API host in URL '{}'. Known hosts: {}",
            api_url,
            KNOWN_CONFIGS
                .iter()
                .map(|(h, _)| *h)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let client = reqwest::Client::builder()
        .user_agent(super::client::user_agent())
        .build()?;

    // Step 1: Request device code
    let form_body = format!(
        "client_id={}&scope={}&audience={}",
        urlencoding::encode(config.client_id),
        urlencoding::encode(SCOPE),
        urlencoding::encode(AUDIENCE),
    );
    let resp = client
        .post(format!("{}/oauth/device/code", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;

    let status = resp.status();
    let resp_body = resp.bytes().await?;

    if !status.is_success() {
        return Err(format!(
            "Failed to start device authorization: {}",
            String::from_utf8_lossy(&resp_body)
        )
        .into());
    }

    let device_resp: DeviceCodeResponse = serde_json::from_slice(&resp_body)?;

    // Step 2: Display instructions
    let verification_url = device_resp
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&device_resp.verification_uri);

    println!("Login to ClickHouse Cloud:");
    println!();
    println!("  Code: {}", device_resp.user_code);
    println!("  URL:  {verification_url}");
    println!();

    // Best-effort browser open
    if open::that(verification_url).is_ok() {
        println!("Browser opened. Waiting for authentication...");
    } else {
        println!("Open the URL above in your browser. Waiting for authentication...");
    }

    // Step 3: Poll for token
    let mut interval = device_resp.interval;
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(device_resp.expires_in);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        if std::time::Instant::now() > deadline {
            return Err("Device authorization timed out".into());
        }

        let poll_body = format!(
            "grant_type={}&device_code={}&client_id={}",
            urlencoding::encode("urn:ietf:params:oauth:grant-type:device_code"),
            urlencoding::encode(&device_resp.device_code),
            urlencoding::encode(config.client_id),
        );
        let resp = client
            .post(format!("{}/oauth/token", config.auth_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(poll_body)
            .send()
            .await?;

        let status = resp.status();
        let body_bytes = resp.bytes().await?;
        let body = String::from_utf8_lossy(&body_bytes);

        if status.is_success() {
            let token_resp: TokenResponse = serde_json::from_str(&body)?;
            let now = chrono::Utc::now().timestamp();
            let tokens = TokenStore {
                access_token: token_resp.access_token,
                refresh_token: token_resp.refresh_token.unwrap_or_default(),
                expires_at: now + token_resp.expires_in,
                api_url: api_url.clone(),
            };
            return Ok(tokens);
        }

        let error_resp: TokenErrorResponse = serde_json::from_str(&body)?;
        match error_resp.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += 5;
                continue;
            }
            "expired_token" => return Err("Device code expired. Please try again.".into()),
            "access_denied" => return Err("Authorization denied by user.".into()),
            _ => {
                return Err(format!(
                    "Authorization failed: {} ({})",
                    error_resp.error,
                    error_resp.error_description.unwrap_or_default()
                )
                .into());
            }
        }
    }
}

pub async fn refresh_access_token(
    tokens: &TokenStore,
) -> Result<TokenStore, Box<dyn std::error::Error>> {
    let config = auth_config_for_url(&tokens.api_url)
        .ok_or_else(|| format!("Cannot refresh: unknown API host in '{}'", tokens.api_url))?;

    let client = reqwest::Client::builder()
        .user_agent(super::client::user_agent())
        .build()?;

    let form_body = format!(
        "grant_type={}&client_id={}&refresh_token={}",
        urlencoding::encode("refresh_token"),
        urlencoding::encode(config.client_id),
        urlencoding::encode(&tokens.refresh_token),
    );
    let resp = client
        .post(format!("{}/oauth/token", config.auth_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await?;

    let status = resp.status();
    let resp_body = resp.bytes().await?;

    if !status.is_success() {
        return Err(format!(
            "Token refresh failed: {}",
            String::from_utf8_lossy(&resp_body)
        )
        .into());
    }

    let token_resp: TokenResponse = serde_json::from_slice(&resp_body)?;
    let now = chrono::Utc::now().timestamp();

    Ok(TokenStore {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| tokens.refresh_token.clone()),
        expires_at: now + token_resp.expires_in,
        api_url: tokens.api_url.clone(),
    })
}

/// If tokens exist and are near-expiry, refresh them. Returns Ok(()) even if
/// no tokens are present (the user may be using API keys instead).
pub async fn ensure_fresh_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let Some(tokens) = load_tokens() else {
        return Ok(());
    };

    if is_token_valid(&tokens) {
        return Ok(());
    }

    if tokens.refresh_token.is_empty() {
        clear_tokens();
        return Ok(());
    }

    match refresh_access_token(&tokens).await {
        Ok(new_tokens) => {
            save_tokens(&new_tokens)?;
        }
        Err(_) => {
            // Refresh failed — clear stale tokens so we fall back to API keys
            clear_tokens();
            eprintln!("Warning: OAuth token refresh failed. Tokens cleared.");
            eprintln!("Run `clickhousectl cloud auth login` to re-authenticate, or use API keys.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_serialization() {
        let tokens = TokenStore {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
            expires_at: 1700000000,
            api_url: "https://api.clickhouse.cloud/v1".into(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: TokenStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "access123");
        assert_eq!(parsed.refresh_token, "refresh456");
        assert_eq!(parsed.expires_at, 1700000000);
        assert_eq!(parsed.api_url, "https://api.clickhouse.cloud/v1");
    }

    #[test]
    fn test_token_serialization_with_custom_url() {
        let tokens = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: 1700000000,
            api_url: "https://api.clickhouse-staging.com/v1".into(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("clickhouse-staging.com"));
        let parsed: TokenStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.api_url, "https://api.clickhouse-staging.com/v1");
    }

    #[test]
    fn test_token_deserialization_defaults_to_production() {
        let json = r#"{"access_token":"a","refresh_token":"r","expires_at":1700000000}"#;
        let parsed: TokenStore = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.api_url, "https://api.clickhouse.cloud/v1");
    }

    #[test]
    fn test_token_validity() {
        let now = chrono::Utc::now().timestamp();

        let valid = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 3600,
            api_url: DEFAULT_API_URL.into(),
        };
        assert!(is_token_valid(&valid));

        let expired = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now - 10,
            api_url: DEFAULT_API_URL.into(),
        };
        assert!(!is_token_valid(&expired));

        let near_expiry = TokenStore {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now + 30, // within 60s buffer
            api_url: DEFAULT_API_URL.into(),
        };
        assert!(!is_token_valid(&near_expiry));
    }

    #[test]
    fn test_tokens_path() {
        let path = tokens_path();
        assert!(path.ends_with(".clickhouse/tokens.json"));
    }

    #[test]
    fn test_auth_config_lookup() {
        assert!(auth_config_for_url("https://api.clickhouse.cloud/v1").is_some());
        assert!(auth_config_for_url("https://api.clickhouse-staging.com/v1").is_some());
        assert!(auth_config_for_url("https://api.clickhouse-dev.com/v1").is_some());
        assert!(auth_config_for_url("https://api.unknown.com/v1").is_none());

        // Verify distinct configs
        let prod = auth_config_for_url("https://api.clickhouse.cloud/v1").unwrap();
        let staging = auth_config_for_url("https://api.clickhouse-staging.com/v1").unwrap();
        let dev = auth_config_for_url("https://api.clickhouse-dev.com/v1").unwrap();
        assert_ne!(prod.client_id, staging.client_id);
        assert_ne!(prod.client_id, dev.client_id);
        assert_ne!(staging.client_id, dev.client_id);

        // Must not match on substring — these are hostile URLs that embed a known host
        assert!(auth_config_for_url("https://api.clickhouse.cloud.evil.com/v1").is_none());
        assert!(auth_config_for_url("https://evil-api.clickhouse.cloud.attacker.com/v1").is_none());
        assert!(auth_config_for_url("https://not-api.clickhouse-staging.com.bad.com/v1").is_none());
    }

    #[test]
    fn test_normalize_api_url() {
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud/v1"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.clickhouse.cloud/"),
            "https://api.clickhouse.cloud/v1"
        );
        assert_eq!(
            normalize_api_url("https://api.clickhouse-staging.com/v1/"),
            "https://api.clickhouse-staging.com/v1"
        );
    }
}
