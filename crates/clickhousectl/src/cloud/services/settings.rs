use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::ServiceClickhouseSettingsPatchRequest;
use std::collections::BTreeMap;
use tabled::{Table, Tabled, settings::Style};

// serde_json preserves i64/u64 integers, but falls back to f64 for overflow
// and exponent/decimal literals. Reject that representation before it can be
// rounded and sent, including numbers nested in future setting value shapes.
fn validate_setting_numbers(value: &serde_json::Value, source: &str) -> CloudResult<()> {
    match value {
        serde_json::Value::Number(number) if number.is_f64() => Err(CloudError::new(format!(
            "{source}: numeric settings must be integers from {} to {}; decimal and exponent values must be JSON strings",
            i64::MIN,
            u64::MAX
        ))),
        serde_json::Value::Array(values) => {
            for value in values {
                validate_setting_numbers(value, source)?;
            }
            Ok(())
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_setting_numbers(value, source)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_settings_map_document(
    raw: &str,
    source: &str,
) -> CloudResult<BTreeMap<String, serde_json::Value>> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| CloudError::new(format!("invalid JSON in {source}: {error}")))?;
    validate_setting_numbers(&value, source)?;
    let object = value.as_object().ok_or_else(|| {
        CloudError::new(format!(
            "{source} must contain a JSON object mapping setting names to values"
        ))
    })?;
    if object.is_empty() {
        return Err(CloudError::new(format!(
            "{source} must contain at least one setting"
        )));
    }
    if object.len() == 1
        && object
            .get("settings")
            .is_some_and(|value| value.is_string() || value.is_object())
    {
        return Err(CloudError::new(format!(
            "{source} looks like the API request wrapper; provide the decoded settings map, for example {{\"compatibility\":\"24.8\"}}"
        )));
    }
    Ok(object
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn parse_setting_assignments(
    assignments: &[String],
) -> CloudResult<BTreeMap<String, serde_json::Value>> {
    let mut settings = BTreeMap::new();
    for (index, assignment) in assignments.iter().enumerate() {
        let position = index + 1;
        let (name, raw_value) = assignment.split_once('=').ok_or_else(|| {
            CloudError::new(format!("--setting #{position} must use NAME=JSON_VALUE"))
        })?;
        if name.trim().is_empty() {
            return Err(CloudError::new(format!(
                "--setting #{position} has an empty setting name"
            )));
        }
        let value = serde_json::from_str(raw_value).map_err(|error| {
            CloudError::new(format!(
                "--setting #{position} value is not valid JSON: {error}; quote string values, for example compatibility=\"24.8\""
            ))
        })?;
        validate_setting_numbers(&value, &format!("--setting #{position} ('{name}')"))?;
        if settings.insert(name.to_string(), value).is_some() {
            return Err(CloudError::new(format!(
                "setting '{name}' was provided more than once"
            )));
        }
    }
    Ok(settings)
}

fn read_service_settings(
    assignments: &[String],
    settings_file: Option<&str>,
) -> CloudResult<BTreeMap<String, serde_json::Value>> {
    if let Some(path) = settings_file {
        use std::io::Read as _;

        let (raw, source) = if path == "-" {
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw).map_err(|error| {
                CloudError::new(format!("failed to read settings from stdin: {error}"))
            })?;
            (raw, "stdin".to_string())
        } else {
            (
                std::fs::read_to_string(path).map_err(|error| {
                    CloudError::new(format!("failed to read settings file '{path}': {error}"))
                })?,
                format!("settings file '{path}'"),
            )
        };
        return parse_settings_map_document(&raw, &source);
    }

    parse_setting_assignments(assignments)
}

fn build_service_settings_patch_request(
    settings: &BTreeMap<String, serde_json::Value>,
) -> CloudResult<ServiceClickhouseSettingsPatchRequest> {
    if settings.is_empty() {
        return Err(CloudError::new("provide at least one ClickHouse setting"));
    }
    Ok(ServiceClickhouseSettingsPatchRequest {
        settings: Some(settings.clone()),
    })
}

pub(super) async fn service_settings_list(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.list_clickhouse_settings(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let Some(settings) = response.settings else {
            println!("No ClickHouse settings returned");
            return Ok(());
        };
        if settings.is_empty() {
            println!("No ClickHouse settings configured");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Value")]
            value: String,
        }
        let rows = settings
            .into_iter()
            .map(|setting| Row {
                name: or_absent(setting.name),
                value: or_absent(setting.value.map(|value| match value {
                    serde_json::Value::String(value) => value,
                    value => value.to_string(),
                })),
            })
            .collect::<Vec<_>>();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub(super) async fn service_setting_get(
    client: &CloudClient,
    service_id: &str,
    setting_name: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let setting = client
        .get_clickhouse_setting(&org_id, service_id, setting_name)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&setting)?);
    } else {
        print_human(&setting)?;
    }
    Ok(())
}

pub(super) async fn service_settings_schema(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let schema = client
        .get_clickhouse_settings_schema(&org_id, service_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    } else {
        print_human(&schema)?;
    }
    Ok(())
}

pub(super) async fn service_settings_set(
    client: &CloudClient,
    service_id: &str,
    assignments: &[String],
    settings_file: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    // Parse before organization discovery so malformed local input never
    // causes an HTTP request, even when --org-id was omitted.
    let settings = read_service_settings(assignments, settings_file)?;
    let request = build_service_settings_patch_request(&settings)?;
    let org_id = resolve_org_id(client).await?;
    let response = client
        .update_clickhouse_settings(&org_id, service_id, &request)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human(&response)?;
    }
    Ok(())
}

pub(super) async fn service_setting_unset(
    client: &CloudClient,
    service_id: &str,
    setting_name: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client
        .reset_clickhouse_setting(&org_id, service_id, setting_name)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Reset ClickHouse setting '{setting_name}' to its platform default");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_settings_patch_with_object_and_preserved_types() {
        let minimal = BTreeMap::from([("compatibility".to_string(), serde_json::json!("24.8"))]);
        let request = build_service_settings_patch_request(&minimal).unwrap();
        assert_eq!(request.settings, Some(minimal));

        let maximal = BTreeMap::from([
            ("bool_setting".to_string(), serde_json::json!(false)),
            ("null_setting".to_string(), serde_json::Value::Null),
            ("number_setting".to_string(), serde_json::json!(42)),
            ("string_setting".to_string(), serde_json::json!("literal")),
            (
                "unknown_future_setting".to_string(),
                serde_json::json!([1, 2]),
            ),
        ]);
        let request = build_service_settings_patch_request(&maximal).unwrap();
        assert_eq!(request.settings, Some(maximal.clone()));
        assert_eq!(
            serde_json::to_value(&request).unwrap()["settings"],
            serde_json::json!({
                "bool_setting": false,
                "null_setting": null,
                "number_setting": 42,
                "string_setting": "literal",
                "unknown_future_setting": [1, 2]
            })
        );
    }

    #[test]
    fn parses_dynamic_setting_assignments_without_a_name_allowlist() {
        let settings = parse_setting_assignments(&[
            "future_setting={\"nested\":true}".to_string(),
            "compatibility=\"24.8\"".to_string(),
        ])
        .unwrap();
        assert_eq!(
            settings["future_setting"],
            serde_json::json!({"nested": true})
        );
        assert_eq!(settings["compatibility"], serde_json::json!("24.8"));
    }

    #[test]
    fn settings_file_rejects_api_wrapper_shape() {
        let error = parse_settings_map_document(
            r#"{"settings":"{\"compatibility\":\"24.8\"}"}"#,
            "settings file",
        )
        .unwrap_err();
        assert!(error.to_string().contains("API request wrapper"));
    }

    #[test]
    fn settings_file_reads_a_plain_dynamic_map() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(&path, r#"{"compatibility":"24.8","future_setting":false}"#).unwrap();
        let settings = read_service_settings(&[], Some(path.to_str().unwrap())).unwrap();
        assert_eq!(settings["compatibility"], serde_json::json!("24.8"));
        assert_eq!(settings["future_setting"], serde_json::json!(false));
    }
}
