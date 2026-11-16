//! `cloud service client`: hand the terminal to the local `clickhouse client`,
//! connected to the service's native TLS endpoint and signed in with the
//! client's own `--login` browser flow (#1154).
//!
//! The CLI only looks the endpoint up. The client owns the sign-in and the
//! token, so no password or token ever passes through clickhousectl or its
//! argv.

use super::cli::ServiceClientArgs;
use super::query::{NativeEndpoint, native_secure_endpoint, resolve_service};
use crate::cloud::client::{CloudClient, CloudError};
use crate::cloud::cloud_error_to_top_level;
use crate::cloud::output::or_absent;
use crate::cloud::shared::resolve_org_id;
use crate::error::{Error, Result};
use crate::failure::FailureStage;
use crate::local::{exec_clickhouse_client, resolve_installed_client};
use crate::version_manager::list::compare_versions;

/// The first release whose client has `--login`: ClickHouse/ClickHouse#89261
/// merged before the 25.12 branch (first stable: v25.12.1.649) and is absent
/// from 25.11.
const LOGIN_MIN_VERSION: &str = "25.12";

pub(crate) async fn service_client(client: &CloudClient, args: ServiceClientArgs) -> Result<()> {
    // Local checks first: a missing or too-old client needs no API call.
    let (version, binary) = resolve_installed_client(args.version)?;
    ensure_login_supported(&version)?;

    let endpoint = resolve_native_endpoint(client, &args.service_id)
        .await
        .map_err(cloud_error_to_top_level)?;
    let client_args = native_client_args(&endpoint, &args.query, &args.args);
    exec_clickhouse_client(&binary, &version, &client_args)
}

fn ensure_login_supported(version: &str) -> Result<()> {
    if compare_versions(version, LOGIN_MIN_VERSION) == std::cmp::Ordering::Less {
        return Err(Error::ClientLoginUnsupported {
            version: version.to_string(),
            minimum: LOGIN_MIN_VERSION,
        });
    }
    Ok(())
}

async fn resolve_native_endpoint(
    client: &CloudClient,
    selector: &crate::cloud::shared::NameSelector,
) -> crate::cloud::client::Result<NativeEndpoint> {
    let org_id = resolve_org_id(client)
        .await
        .map_err(|error| error.at_stage(FailureStage::OrgResolution))?;
    let service = resolve_service(
        client,
        &org_id,
        selector.name.as_deref(),
        selector.id.as_deref(),
    )
    .await
    .map_err(|error| error.at_stage(FailureStage::ServiceResolution))?;
    let service_name = or_absent(service.name.as_deref());
    let endpoint = native_secure_endpoint(service.endpoints.as_deref()).ok_or_else(|| {
        CloudError::new(format!(
            "service '{service_name}' has no native TLS endpoint in the API response, so there is \
             nothing for clickhouse client to connect to"
        ))
    })?;
    validate_endpoint(&endpoint)?;
    Ok(endpoint)
}

/// The host and port go into another program's argv, so accept only a plain
/// DNS name and a real port: a value the API returned that the client could
/// read as an option, or that is not a hostname at all, is refused.
fn validate_endpoint(endpoint: &NativeEndpoint) -> crate::cloud::client::Result<()> {
    let host = endpoint.host.as_str();
    let valid_host = !host.is_empty()
        && host.len() <= 253
        && !host.starts_with(['-', '.'])
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'));
    if !valid_host || !(1..=65535).contains(&endpoint.port) {
        return Err(CloudError::new(
            "the API returned a native endpoint that is not a valid host and port",
        ));
    }
    Ok(())
}

/// The native client's argv after `client`: the CLI's own connection flags
/// first, then the queries, then the user's passthrough arguments last.
fn native_client_args(
    endpoint: &NativeEndpoint,
    queries: &[String],
    passthrough: &[String],
) -> Vec<String> {
    let mut args = vec![
        "--host".to_string(),
        endpoint.host.clone(),
        "--port".to_string(),
        endpoint.port.to_string(),
        "--secure".to_string(),
        "--login".to_string(),
    ];
    for query in queries {
        args.extend(["--query".to_string(), query.clone()]);
    }
    args.extend(passthrough.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(host: &str, port: i64) -> NativeEndpoint {
        NativeEndpoint {
            host: host.into(),
            port,
        }
    }

    #[test]
    fn login_needs_25_12_or_newer() {
        for version in ["25.12", "25.12.1.649", "25.12.9.61", "26.1.1.912", "100.1"] {
            assert!(ensure_login_supported(version).is_ok(), "{version}");
        }
        for version in ["25.11.2.24", "25.1.1.1", "24.12.1.1", "9.99"] {
            let error = ensure_login_supported(version).unwrap_err();
            assert!(
                matches!(
                    &error,
                    Error::ClientLoginUnsupported { version: v, minimum: LOGIN_MIN_VERSION }
                        if v == version
                ),
                "{version}: {error:?}"
            );
            assert_eq!(error.exit_code(), 1);
        }
    }

    #[test]
    fn argv_puts_connection_flags_first_and_passthrough_last() {
        let args = native_client_args(
            &endpoint("abc.us-east-1.aws.clickhouse.cloud", 9440),
            &["SELECT 1".into(), "--not-a-flag".into()],
            &["--format".into(), "CSV".into(), "--host".into()],
        );
        assert_eq!(
            args,
            [
                "--host",
                "abc.us-east-1.aws.clickhouse.cloud",
                "--port",
                "9440",
                "--secure",
                "--login",
                "--query",
                "SELECT 1",
                "--query",
                "--not-a-flag",
                "--format",
                "CSV",
                "--host",
            ]
        );
        // No query and no passthrough: only the connection flags.
        assert_eq!(
            native_client_args(&endpoint("h.example", 9440), &[], &[]),
            [
                "--host",
                "h.example",
                "--port",
                "9440",
                "--secure",
                "--login"
            ]
        );
        // The CLI never puts a credential on the command line.
        for arg in native_client_args(&endpoint("h.example", 9440), &[], &[]) {
            assert!(!arg.starts_with("--password") && !arg.starts_with("--jwt"));
            assert!(!arg.starts_with("--user"));
        }
    }

    #[test]
    fn only_a_dns_name_and_real_port_reach_the_argv() {
        for (host, port) in [
            ("abc.us-east-1.aws.clickhouse.cloud", 9440),
            ("h-1.example", 1),
            ("h.example", 65535),
        ] {
            assert!(validate_endpoint(&endpoint(host, port)).is_ok(), "{host}");
        }
        for (host, port) in [
            ("", 9440),
            ("--config-file=/tmp/x", 9440),
            ("-h", 9440),
            (".example", 9440),
            ("h.example extra", 9440),
            ("h.example\n", 9440),
            ("h.example", 0),
            ("h.example", 65536),
            ("h.example", -1),
        ] {
            assert!(
                validate_endpoint(&endpoint(host, port)).is_err(),
                "{host:?}:{port}"
            );
        }
    }
}
