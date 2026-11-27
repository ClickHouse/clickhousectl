//! Handler for `cloud init` — creates an unclaimed ClickHouse Cloud organisation.
//!
//! The command is unauthenticated: it calls `POST /v1/claimable` without any
//! credentials and saves the returned API key pair to the project's
//! `.clickhouse/credentials.json`. The key secret never appears on stdout or
//! stderr.

use super::permissions::Declaration as Permission;
use crate::cloud::cli::NoCloudConnectionArgs;
use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::cloud::credentials::{self, Credentials};
use crate::cloud::output::{CloudErrorCode, CloudErrorDetail};
use crate::failure::classify_api_error;
use clap::Args;
use clickhouse_cloud_api::meta::operations as op;

pub(super) const PERMISSIONS: &[Permission] =
    &[
        Permission::api("init", &[&op::CLAIMABLE_ORGANIZATION_CREATE])
            .unauthenticated("None; creates a new organization without credentials."),
    ];

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Cloud region identifier (e.g. us-east-1)
    #[arg(long)]
    pub region: Option<String>,

    // Hidden twins of the global --org-id/--org-name/--api-key/--api-secret: they keep
    // those flags out of `init --help`; `no_connection_flag_conflict` rejects them.
    #[command(flatten)]
    pub no_connection: NoCloudConnectionArgs,
}

/// Run `cloud init`. `--debug` prints only the API URL: no credential source
/// is ever resolved.
pub async fn run(args: InitArgs, url: Option<&str>, debug: bool, json: bool) -> CloudResult<()> {
    if debug {
        eprintln!("[debug] api url: {}", resolve_base_url(url));
    }
    run_unauthenticated(url, args.region, json).await
}

fn resolve_base_url(url: Option<&str>) -> String {
    url.map(crate::cloud::auth::normalize_api_url)
        .unwrap_or_else(|| crate::cloud::client::DEFAULT_BASE_URL.to_string())
}

/// Run `cloud init` without any existing credentials.
///
/// 1. Checks that credentials are not already set (preflight outside the lock).
/// 2. Acquires the credential mutation lock.
/// 3. Re-checks inside the lock (TOCTOU guard).
/// 4. Calls `POST /v1/claimable` on the library client (no auth header sent).
/// 5. Extracts required fields — fails fast if any are absent.
/// 6. Persists via `save_claimable_credentials` (key secret never on stdout/stderr).
/// 7. Prints the result.
async fn run_unauthenticated(
    base_url: Option<&str>,
    region: Option<String>,
    json: bool,
) -> CloudResult<()> {
    // Preflight: reject immediately when credentials already exist.
    let existing = credentials::try_load_credentials()?;
    if let Some(err) = check_already_initialized(&existing) {
        return Err(err);
    }

    // Acquire the credential lock before the API call so no other process can
    // write credentials between the check and the save.
    let _lock = credentials::lock_credentials_mutation()?;

    // Re-check inside the lock (TOCTOU guard).
    let existing = credentials::try_load_credentials()?;
    if let Some(err) = check_already_initialized(&existing) {
        return Err(err);
    }

    let resolved_base_url = resolve_base_url(base_url);

    // Build a raw library client with no credentials. The
    // `claimable_organization_create` method skips the per-request auth
    // helper, so the empty strings are never sent as an Authorization header.
    // Strip /v1 because the library client appends the full path template
    // (including /v1) to the base URL.
    let lib_url = resolved_base_url
        .strip_suffix("/v1")
        .unwrap_or(&resolved_base_url)
        .to_string();
    let client = clickhouse_cloud_api::Client::with_base_url(&lib_url, "", "");

    let response = client
        .claimable_organization_create(region.as_deref())
        .await
        .map_err(convert_claimable_error)?;

    let result = response
        .result
        .ok_or_else(|| CloudError::new("claimable organisation response was empty"))?;

    let org_id = result
        .organization_id
        .ok_or_else(|| CloudError::new("claimable organisation response missing organizationId"))?;

    let api_key = result
        .api_key
        .ok_or_else(|| CloudError::new("claimable organisation response missing apiKey"))?;

    let key_id = api_key
        .key_id
        .ok_or_else(|| CloudError::new("claimable organisation response missing apiKey.keyId"))?;

    let key_secret = api_key.key_secret.ok_or_else(|| {
        CloudError::new("claimable organisation response missing apiKey.keySecret")
    })?;

    let expires_at = result
        .expires_at
        .ok_or_else(|| CloudError::new("claimable organisation response missing expiresAt"))?;

    let limits = result.limits;

    // Persist. The key secret lives only in this call; `save_claimable_credentials`
    // moves it into the credentials file. It is never stored in a variable
    // that outlives this point.
    credentials::save_claimable_credentials(
        org_id.clone(),
        key_id,
        key_secret,
        expires_at,
        &resolved_base_url,
    )?;

    // Pin the new org so that subsequent commands resolve the right
    // organization without requiring a `cloud use` call. This overwrites any
    // stale pin that may have been left by a previous `cloud use` for the
    // same API URL.
    let pin_dir = crate::init::local_dir()?;
    crate::cloud::context::save_org_in(
        &pin_dir,
        &resolved_base_url,
        crate::cloud::context::PinnedOrg {
            id: org_id.clone(),
            name: None,
        },
    )?;

    print_init_result(&org_id, expires_at, &limits, json)?;

    Ok(())
}

/// Returns an error when `creds` already has both `api_key` and `api_secret`
/// set. Returns `None` when no credentials file exists or when the file is
/// incomplete (only one of the pair present).
pub(crate) fn check_already_initialized(creds: &Option<Credentials>) -> Option<CloudError> {
    let creds = creds.as_ref()?;
    creds.api_credentials()?;
    Some(already_initialized_error())
}

fn already_initialized_error() -> CloudError {
    let message = "credentials are already saved — delete .clickhouse/credentials.json \
                   to reinitialise, or use a different project directory"
        .to_string();
    CloudError::new(message.clone()).with_details(CloudErrorDetail::new(
        CloudErrorCode::AlreadyInitialized,
        message,
    ))
}

fn convert_claimable_error(err: clickhouse_cloud_api::Error) -> CloudError {
    let failure = classify_api_error(&err);
    match &err {
        clickhouse_cloud_api::Error::Api { status: 503, .. } => {
            let message = format!("claimable organisations are not available: {err}");
            CloudError::new(message.clone())
                .with_failure(failure)
                .with_details(CloudErrorDetail::new(
                    CloudErrorCode::ClaimableUnavailable,
                    message,
                ))
        }
        _ => CloudError::new(err.to_string()).with_failure(failure),
    }
}

fn print_init_result(
    org_id: &str,
    expires_at: jiff::Timestamp,
    limits: &Option<serde_json::Value>,
    json: bool,
) -> CloudResult<()> {
    if json {
        let mut obj = serde_json::json!({
            "organization_id": org_id,
            "expires_at": expires_at.to_string(),
        });
        if let Some(limits) = limits {
            obj["limits"] = limits.clone();
        }
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        use crate::cloud::output::print_human;
        let mut fields = serde_json::json!({
            "organization_id": org_id,
            "expires_at": expires_at.to_string(),
        });
        if let Some(limits) = limits {
            fields["limits"] = limits.clone();
        }
        print_human(&fields)?;
        eprintln!(
            "\nWARNING: This organisation will be permanently deleted unless claimed before {expires_at}.\n\
             To claim it, visit the ClickHouse Cloud console and import organization ID: {org_id}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds_with_key_and_secret() -> Option<Credentials> {
        Some(Credentials {
            api_key: Some("key-id".to_string()),
            api_secret: Some("key-secret".to_string()),
            ..Default::default()
        })
    }

    fn creds_with_key_only() -> Option<Credentials> {
        Some(Credentials {
            api_key: Some("key-id".to_string()),
            api_secret: None,
            ..Default::default()
        })
    }

    #[test]
    fn guard_fires_with_existing_api_credentials() {
        let result = check_already_initialized(&creds_with_key_and_secret());
        assert!(result.is_some(), "expected error when creds are present");
        let err = result.unwrap();
        let details = err.details.unwrap();
        assert_eq!(details.code, CloudErrorCode::AlreadyInitialized);
    }

    #[test]
    fn guard_does_not_fire_with_no_credentials_file() {
        assert!(check_already_initialized(&None).is_none());
    }

    #[test]
    fn guard_does_not_fire_with_incomplete_credentials() {
        // Only api_key set, no api_secret — not a usable credential pair.
        assert!(check_already_initialized(&creds_with_key_only()).is_none());
    }
}
