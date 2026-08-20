use crate::cloud::client::CloudClient;
use chrono::{DateTime, FixedOffset, NaiveDate};
use clickhouse_cloud_api::models::ResourceTagsV1;

/// Resolve an organization ID from an explicit argument or auto-detection.
pub(super) async fn resolve_org_id(
    client: &CloudClient,
    org_id: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
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
) -> Result<T, Box<dyn std::error::Error>> {
    if !known_values.contains(&value) {
        return Err(format!(
            "invalid {}: unknown value '{}', expected one of: {}",
            field,
            value,
            known_values.join(", ")
        )
        .into());
    }
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| format!("invalid {}: {}", field, e).into())
}

pub(super) fn parse_tag(value: &str) -> Result<ResourceTagsV1, Box<dyn std::error::Error>> {
    match value.split_once('=') {
        Some((key, tag_value)) => {
            let key = key.trim();
            if key.is_empty() {
                Err(format!("invalid tag '{}': tag key cannot be empty", value).into())
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
                Err(format!("invalid tag '{}': tag key cannot be empty", value).into())
            } else {
                Ok(ResourceTagsV1 {
                    key: key.to_string(),
                    value: None,
                })
            }
        }
    }
}

pub(super) fn parse_tags(
    values: &[String],
) -> Result<Option<Vec<ResourceTagsV1>>, Box<dyn std::error::Error>> {
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
}
