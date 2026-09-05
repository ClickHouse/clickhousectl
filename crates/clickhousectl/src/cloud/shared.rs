use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use chrono::{DateTime, FixedOffset, NaiveDate};
use clickhouse_cloud_api::models::{IpAccessListEntry, ResourceTagsV1};
use std::net::IpAddr;

/// Resolve an organization ID from an explicit argument or auto-detection.
pub(super) async fn resolve_org_id(
    client: &CloudClient,
    org_id: Option<&str>,
) -> CloudResult<String> {
    match org_id {
        Some(id) => Ok(id.to_string()),
        None => Ok(client.get_default_org_id().await?),
    }
}

/// Parse a string into a library enum after validating its known wire values.
pub(super) fn parse_serde_enum<T: serde::de::DeserializeOwned>(
    value: &str,
    field: &str,
    known_values: &[&str],
) -> CloudResult<T> {
    if !known_values.contains(&value) {
        return Err(CloudError::new(format!(
            "invalid {}: unknown value '{}', expected one of: {}",
            field,
            value,
            known_values.join(", ")
        )));
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| CloudError::new(format!("invalid {}: {}", field, e)))
}

pub(super) fn parse_tag(value: &str) -> CloudResult<ResourceTagsV1> {
    match value.split_once('=') {
        Some((key, tag_value)) => {
            let key = key.trim();
            if key.is_empty() {
                Err(CloudError::new(format!(
                    "invalid tag '{}': tag key cannot be empty",
                    value
                )))
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: Some(tag_value.to_string()),
                })
            }
        }
        None => {
            let key = value.trim();
            if key.is_empty() {
                Err(CloudError::new(format!(
                    "invalid tag '{}': tag key cannot be empty",
                    value
                )))
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: None,
                })
            }
        }
    }
}

pub(super) fn parse_tags(values: &[String]) -> CloudResult<Option<Vec<ResourceTagsV1>>> {
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            values
                .iter()
                .map(|value| parse_tag(value))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// Parse an IP allowlist argument in `SOURCE[=DESCRIPTION]` form.
///
/// `=` keeps the description delimiter unambiguous for IPv6 sources. The
/// description is kept byte-for-byte, including an explicitly empty value.
fn parse_ip_access_entry(value: &str) -> CloudResult<IpAccessListEntry> {
    let (source, description) = value
        .split_once('=')
        .map_or((value, None), |(source, description)| {
            (source, Some(description.to_string()))
        });
    let source = source.trim();

    let (address, prefix) = match source.split_once('/') {
        Some((address, prefix)) if !prefix.contains('/') => (address, Some(prefix)),
        Some(_) => return Err(invalid_ip_access_entry(value)),
        None => (source, None),
    };
    let address = address
        .parse::<IpAddr>()
        .map_err(|_| invalid_ip_access_entry(value))?;
    if let Some(prefix) = prefix {
        let prefix = prefix
            .parse::<u8>()
            .map_err(|_| invalid_ip_access_entry(value))?;
        let max_prefix = if address.is_ipv4() { 32 } else { 128 };
        if prefix > max_prefix {
            return Err(invalid_ip_access_entry(value));
        }
    }

    Ok(IpAccessListEntry {
        source: source.to_string(),
        description,
    })
}

fn invalid_ip_access_entry(value: &str) -> CloudError {
    CloudError::new(format!(
        "invalid IP allowlist entry '{}': expected IP_OR_CIDR[=DESCRIPTION]",
        value
    ))
}

pub(super) fn parse_ip_access_entries(
    values: &[String],
) -> CloudResult<Option<Vec<IpAccessListEntry>>> {
    if values.is_empty() {
        Ok(None)
    } else {
        values
            .iter()
            .map(|value| parse_ip_access_entry(value))
            .collect::<CloudResult<Vec<_>>>()
            .map(Some)
    }
}

pub(super) fn parse_date_only(value: &str) -> Result<String, String> {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(format!("invalid date '{}': expected YYYY-MM-DD", value));
    }

    Ok(value.to_string())
}

pub(super) fn parse_datetime(value: &str) -> Result<String, String> {
    if DateTime::<FixedOffset>::parse_from_rfc3339(value).is_err() {
        return Err(format!(
            "invalid datetime '{}': expected ISO 8601 / RFC 3339",
            value
        ));
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_rejects_empty_keys() {
        let err = parse_tag("=value").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag '=value': tag key cannot be empty"
        );

        let err = parse_tag("   ").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid tag '   ': tag key cannot be empty"
        );
    }

    #[test]
    fn parse_ip_access_entries_supports_bare_ipv4_ipv6_and_descriptions() {
        let values = vec![
            "192.0.2.7".to_string(),
            "10.0.0.0/8=office".to_string(),
            "2001:db8::/32=\u{6771}\u{4eac} \u{1f5fc}".to_string(),
            "2001:db8::1=".to_string(),
        ];
        let entries = parse_ip_access_entries(&values).unwrap().unwrap();

        assert_eq!(entries[0].source, "192.0.2.7");
        assert!(entries[0].description.is_none());
        assert_eq!(entries[1].source, "10.0.0.0/8");
        assert_eq!(entries[1].description.as_deref(), Some("office"));
        assert_eq!(entries[2].source, "2001:db8::/32");
        assert_eq!(
            entries[2].description.as_deref(),
            Some("\u{6771}\u{4eac} \u{1f5fc}")
        );
        assert_eq!(entries[3].source, "2001:db8::1");
        assert_eq!(entries[3].description.as_deref(), Some(""));
    }

    #[test]
    fn parse_ip_access_entries_rejects_invalid_sources() {
        for value in [
            "",
            "=office",
            "not-an-ip",
            "10.0.0.0/nope",
            "10.0.0.0/33",
            "2001:db8::/129",
            "10.0.0.0/8/9",
        ] {
            let error = parse_ip_access_entries(&[value.to_string()]).unwrap_err();
            assert!(error.to_string().contains(value), "{error}");
        }
    }
}
