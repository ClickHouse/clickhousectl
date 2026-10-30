use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human, print_line};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clickhouse_cloud_api::models::{
    PostgresBackup, PostgresLogEntry, PostgresLogsGetListSortorder, PostgresMetrics,
    SlowQueryPatternsGetListSortby, SlowQueryPatternsGetListSortorder,
};
use tabled::{Table, Tabled, settings::Style};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PostgresLogsQuery {
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) body_contains: Option<String>,
    pub(super) severity: Option<String>,
    pub(super) sort_order: Option<PostgresLogsGetListSortorder>,
    pub(super) limit: Option<i64>,
    pub(super) offset: Option<i64>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_postgres_logs_query(
    from_date: &str,
    to_date: &str,
    body_contains: Option<&str>,
    severity: Option<&str>,
    sort_order: Option<&PostgresLogsGetListSortorder>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> CloudResult<PostgresLogsQuery> {
    let from = chrono::DateTime::parse_from_rfc3339(from_date)
        .map_err(|error| CloudError::new(format!("invalid from-date: {error}")))?;
    let to = chrono::DateTime::parse_from_rfc3339(to_date)
        .map_err(|error| CloudError::new(format!("invalid to-date: {error}")))?;
    if to <= from {
        return Err(CloudError::new("to-date must be after from-date"));
    }
    if to.signed_duration_since(from) > chrono::Duration::days(30) {
        return Err(CloudError::new(
            "the Postgres log time window must not exceed 30 days",
        ));
    }
    if !matches!(limit, None | Some(1..=2000)) {
        return Err(CloudError::new("limit must be between 1 and 2000"));
    }
    if offset.is_some_and(|value| value < 0) {
        return Err(CloudError::new("offset must be at least 0"));
    }

    let sort_order = sort_order
        .cloned()
        .map(validate_postgres_logs_sort_order)
        .transpose()
        .map_err(CloudError::new)?;

    Ok(PostgresLogsQuery {
        from_date: from_date.to_string(),
        to_date: to_date.to_string(),
        body_contains: body_contains.map(str::to_string),
        severity: severity.map(str::to_string),
        sort_order,
        limit,
        offset,
    })
}

fn parse_postgres_logs_sort_order(value: &str) -> Result<PostgresLogsGetListSortorder, String> {
    let parsed = serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|error| format!("invalid sort order '{value}': {error}"))?;
    validate_postgres_logs_sort_order(parsed)
}

pub(super) fn parse_known_postgres_logs_sort_order(
    value: String,
) -> Result<PostgresLogsGetListSortorder, String> {
    parse_postgres_logs_sort_order(&value)
}

fn validate_postgres_logs_sort_order(
    value: PostgresLogsGetListSortorder,
) -> Result<PostgresLogsGetListSortorder, String> {
    match value {
        PostgresLogsGetListSortorder::Unknown(value) => Err(format!(
            "invalid sort order '{value}': expected a value supported by the Cloud API"
        )),
        known => Ok(known),
    }
}

pub(super) async fn postgres_logs(
    client: &CloudClient,
    postgres_id: &str,
    query: &PostgresLogsQuery,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let logs = client
        .list_postgres_logs(&org_id, postgres_id, query)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&logs)?);
    } else if logs.is_empty() {
        println!("No Postgres logs found");
    } else {
        println!("{}", render_postgres_logs_table(&logs));
    }
    Ok(())
}

#[derive(Debug, PartialEq, Tabled)]
struct PostgresLogRow {
    #[tabled(rename = "Timestamp")]
    timestamp: String,
    #[tabled(rename = "Severity")]
    severity: String,
    #[tabled(rename = "Message")]
    message: String,
}

fn render_postgres_logs_table(logs: &[PostgresLogEntry]) -> String {
    let rows = logs.iter().map(postgres_log_row);
    Table::new(rows).with(Style::markdown()).to_string()
}

fn postgres_log_row(log: &PostgresLogEntry) -> PostgresLogRow {
    PostgresLogRow {
        timestamp: or_absent(
            log.timestamp
                .as_ref()
                .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
        ),
        severity: or_absent(log.severity.as_deref()),
        message: postgres_log_message(log.body.as_deref()),
    }
}

/// Extract the useful message from the JSON document carried in `body`.
///
/// The API model deliberately keeps `body` as a string. Returning that string
/// unchanged for every other shape avoids hiding data when the provider sends
/// plain text, malformed JSON, a missing message, or a future message type.
fn postgres_log_message(body: Option<&str>) -> String {
    let Some(body) = body else {
        return ABSENT.to_string();
    };
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.to_string())
}

// These endpoints accept UTC timestamps with exactly three fractional digits.
// Check the original fraction: chrono discards digits beyond nanosecond precision.
pub(super) fn parse_postgres_window_datetime(value: &str) -> Result<String, String> {
    let timestamp = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| format!("invalid datetime '{value}': expected ISO 8601 / RFC 3339"))?;
    if value.split_once('.').is_some_and(|(_, fraction)| {
        fraction
            .bytes()
            .take_while(u8::is_ascii_digit)
            .skip(3)
            .any(|digit| digit != b'0')
    }) {
        return Err(format!(
            "invalid datetime '{value}': Postgres time windows support at most millisecond precision"
        ));
    }
    Ok(timestamp
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn validate_datetime_range(from_date: &str, to_date: &str) -> CloudResult<()> {
    let from = chrono::DateTime::parse_from_rfc3339(from_date)
        .map_err(|_| CloudError::new("invalid --from-date: expected ISO 8601 / RFC 3339"))?;
    let to = chrono::DateTime::parse_from_rfc3339(to_date)
        .map_err(|_| CloudError::new("invalid --to-date: expected ISO 8601 / RFC 3339"))?;
    if from > to {
        return Err(CloudError::new(
            "invalid date range: --from-date must not be after --to-date",
        ));
    }
    Ok(())
}

fn postgres_metric_timestamp(timestamp: Option<i64>) -> String {
    timestamp
        .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
        .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        // The wire type permits epoch values outside chrono's calendar range.
        // Keep such a value visible rather than dropping its data point.
        .unwrap_or_else(|| match timestamp {
            Some(timestamp) => format!("{timestamp} (outside displayable date range)"),
            None => ABSENT.to_string(),
        })
}

/// Compact human view for the nested metrics response.
///
/// The view starts from the response's serde representation, retaining source
/// field order and the same field-hiding behavior as JSON. Only data points are
/// condensed: each becomes a numbered RFC 3339/value line. Numbering preserves
/// duplicate points and API order. Missing collections become `-`, while empty
/// arrays stay `[]`.
fn postgres_metrics_human_view(
    metrics: &PostgresMetrics,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut view = serde_json::to_value(metrics)?;
    let root = view
        .as_object_mut()
        .expect("PostgresMetrics serializes as an object");
    let Some(source_metrics) = metrics.metrics.as_ref() else {
        root.insert(
            "metrics".to_string(),
            serde_json::Value::String(ABSENT.to_string()),
        );
        return Ok(view);
    };
    let view_metrics = root["metrics"]
        .as_array_mut()
        .expect("a present metrics field serializes as an array");

    for (source_metric, view_metric) in source_metrics.iter().zip(view_metrics) {
        let view_metric = view_metric
            .as_object_mut()
            .expect("PostgresMetric serializes as an object");
        let Some(source_series) = source_metric.series.as_ref() else {
            view_metric.insert(
                "series".to_string(),
                serde_json::Value::String(ABSENT.to_string()),
            );
            continue;
        };
        let view_series = view_metric["series"]
            .as_array_mut()
            .expect("a present series field serializes as an array");

        for (source_series, view_series) in source_series.iter().zip(view_series) {
            let view_series = view_series
                .as_object_mut()
                .expect("PostgresMetricSeries serializes as an object");
            let Some(source_points) = source_series.data_points.as_ref() else {
                view_series.insert(
                    "dataPoints".to_string(),
                    serde_json::Value::String(ABSENT.to_string()),
                );
                continue;
            };
            if source_points.is_empty() {
                continue;
            }

            let mut point_lines = serde_json::Map::new();
            for (index, point) in source_points.iter().enumerate() {
                let timestamp = postgres_metric_timestamp(point.timestamp);
                let value = point
                    .value
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| ABSENT.to_string());
                let value = match (source_metric.unit.as_deref(), point.value) {
                    (Some(unit), Some(_)) => format!("{value} {unit}"),
                    _ => value,
                };
                point_lines.insert(
                    (index + 1).to_string(),
                    serde_json::Value::String(format!("{timestamp}  {value}")),
                );
            }
            view_series.insert(
                "dataPoints".to_string(),
                serde_json::Value::Object(point_lines),
            );
        }
    }
    Ok(view)
}

pub async fn postgres_metrics(
    client: &CloudClient,
    postgres_id: &str,
    from_date: &str,
    to_date: &str,
    bucket_size_seconds: Option<i64>,
    json: bool,
) -> CloudResult<()> {
    validate_datetime_range(from_date, to_date)?;
    let org_id = resolve_org_id(client).await?;
    let metrics = client
        .get_postgres_metrics(
            &org_id,
            postgres_id,
            from_date,
            to_date,
            bucket_size_seconds,
        )
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&metrics)?);
    } else {
        print_human(&postgres_metrics_human_view(&metrics)?)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SlowQueryListInput<'a> {
    pub(super) from_date: &'a str,
    pub(super) to_date: &'a str,
    pub(super) db_name: Option<&'a str>,
    pub(super) db_user: Option<&'a str>,
    pub(super) db_operation: Option<&'a str>,
    pub(super) app: Option<&'a str>,
    pub(super) sort_by: Option<&'a str>,
    pub(super) sort_order: Option<&'a str>,
    pub(super) limit: Option<i64>,
    pub(super) offset: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlowQueryListQuery<'a> {
    pub(super) from_date: &'a str,
    pub(super) to_date: &'a str,
    pub(super) db_name: Option<&'a str>,
    pub(super) db_user: Option<&'a str>,
    pub(super) db_operation: Option<&'a str>,
    pub(super) app: Option<&'a str>,
    pub(super) sort_by: Option<SlowQueryPatternsGetListSortby>,
    pub(super) sort_order: Option<SlowQueryPatternsGetListSortorder>,
    pub(super) limit: Option<i64>,
    pub(super) offset: Option<i64>,
}

fn build_slow_query_list_query(
    input: SlowQueryListInput<'_>,
) -> CloudResult<SlowQueryListQuery<'_>> {
    validate_datetime_range(input.from_date, input.to_date)?;
    if input.limit.is_some_and(|limit| !(1..=500).contains(&limit)) {
        return Err(CloudError::new(
            "invalid --limit: expected a value from 1 to 500",
        ));
    }
    if input.offset.is_some_and(|offset| offset < 0) {
        return Err(CloudError::new(
            "invalid --offset: expected a non-negative value",
        ));
    }

    Ok(SlowQueryListQuery {
        from_date: input.from_date,
        to_date: input.to_date,
        db_name: input.db_name,
        db_user: input.db_user,
        db_operation: input.db_operation,
        app: input.app,
        sort_by: input
            .sort_by
            .map(|value| parse_serde_enum(value, "sort_by", SlowQueryPatternsGetListSortby::VALUES))
            .transpose()?,
        sort_order: input
            .sort_order
            .map(|value| {
                parse_serde_enum(
                    value,
                    "sort_order",
                    SlowQueryPatternsGetListSortorder::VALUES,
                )
            })
            .transpose()?,
        limit: input.limit,
        offset: input.offset,
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SlowQueryDetailInput<'a> {
    pub(super) db_name: &'a str,
    pub(super) db_user: &'a str,
    pub(super) db_operation: &'a str,
    pub(super) app: Option<&'a str>,
    pub(super) timestamp: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlowQueryDetailQuery<'a> {
    pub(super) db_name: &'a str,
    pub(super) db_user: &'a str,
    pub(super) db_operation: &'a str,
    pub(super) app: Option<&'a str>,
    pub(super) timestamp: Option<&'a str>,
}

fn build_slow_query_detail_query(
    input: SlowQueryDetailInput<'_>,
) -> CloudResult<SlowQueryDetailQuery<'_>> {
    if let Some(timestamp) = input.timestamp {
        chrono::DateTime::parse_from_rfc3339(timestamp)
            .map_err(|_| CloudError::new("invalid --timestamp: expected ISO 8601 / RFC 3339"))?;
    }
    Ok(SlowQueryDetailQuery {
        db_name: input.db_name,
        db_user: input.db_user,
        db_operation: input.db_operation,
        app: input.app,
        timestamp: input.timestamp,
    })
}

pub(super) async fn postgres_slow_queries_list(
    client: &CloudClient,
    postgres_id: &str,
    input: SlowQueryListInput<'_>,
    json: bool,
) -> CloudResult<()> {
    let query = build_slow_query_list_query(input)?;
    let org_id = resolve_org_id(client).await?;
    let patterns = client
        .list_postgres_slow_query_patterns(&org_id, postgres_id, &query)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&patterns)?);
        return Ok(());
    }

    if patterns.is_empty() {
        println!("No Postgres slow query patterns found");
        return Ok(());
    }

    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Query ID")]
        query_id: String,
        #[tabled(rename = "Query")]
        query: String,
        #[tabled(rename = "Calls")]
        calls: String,
        #[tabled(rename = "Avg duration (µs)")]
        avg_duration_us: String,
        #[tabled(rename = "Total duration (µs)")]
        total_duration_us: String,
    }

    let rows = patterns.into_iter().map(|pattern| Row {
        query_id: or_absent(pattern.query_id),
        query: or_absent(pattern.query_text),
        calls: or_absent(pattern.call_count),
        avg_duration_us: or_absent(pattern.avg_duration_us),
        total_duration_us: or_absent(pattern.total_duration_us),
    });
    println!("{}", Table::new(rows).with(Style::markdown()));
    Ok(())
}

pub(super) async fn postgres_slow_query_get(
    client: &CloudClient,
    postgres_id: &str,
    query_id: &str,
    input: SlowQueryDetailInput<'_>,
    json: bool,
) -> CloudResult<()> {
    let query = build_slow_query_detail_query(input)?;
    let org_id = resolve_org_id(client).await?;
    let pattern = client
        .get_postgres_slow_query_pattern(&org_id, postgres_id, query_id, &query)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&pattern)?);
    } else {
        print_human(&pattern)?;
    }
    Ok(())
}

fn print_prometheus(metrics: &str, json: bool) -> CloudResult<()> {
    if json {
        print_line(serde_json::to_string_pretty(metrics)?);
    } else {
        use std::io::Write;
        crate::stdout::stdout().write_all(metrics.as_bytes())?;
    }
    Ok(())
}

pub(super) async fn postgres_prometheus_service(
    client: &CloudClient,
    postgres_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let metrics = client.get_postgres_prometheus(&org_id, postgres_id).await?;
    print_prometheus(&metrics, json)
}

pub(super) async fn postgres_prometheus_org(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let metrics = client.get_postgres_org_prometheus(&org_id).await?;
    print_prometheus(&metrics, json)
}

/// One backup list page, keeping the envelope's continuation metadata for `--json`.
#[derive(serde::Serialize)]
pub(super) struct PostgresBackupPage {
    pub(super) result: Vec<PostgresBackup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) limit: Option<i64>,
    #[serde(rename = "totalCount", skip_serializing_if = "Option::is_none")]
    pub(super) total_count: Option<i64>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub(super) next_cursor: Option<String>,
}

pub(super) async fn postgres_backup_list(
    client: &CloudClient,
    postgres_id: &str,
    cursor: Option<&str>,
    limit: Option<i64>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let page = client
        .list_postgres_backups_page(&org_id, postgres_id, cursor, limit)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
    } else {
        println!("{}", render_postgres_backup_page(page));
    }
    Ok(())
}

fn render_postgres_backup_page(page: PostgresBackupPage) -> String {
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Key")]
        key: String,
        #[tabled(rename = "Status")]
        status: String,
        #[tabled(rename = "Last modified")]
        last_modified: String,
    }

    let mut out = if page.result.is_empty() {
        "No Postgres backups found".to_owned()
    } else {
        let rows = page.result.into_iter().map(|backup| Row {
            key: or_absent(backup.key),
            status: or_absent(backup.status),
            last_modified: or_absent(
                backup
                    .last_modified
                    .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
            ),
        });
        Table::new(rows).with(Style::markdown()).to_string()
    };
    if let Some(cursor) = page.next_cursor {
        out.push_str(&format!("\nNext cursor: {cursor}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse_cloud_api::models::{
        PostgresMetric, PostgresMetricDataPoint, PostgresMetricSeries,
    };

    #[test]
    fn backup_page_human_view_renders_absent_fields_and_next_cursor() {
        let page = PostgresBackupPage {
            result: vec![
                PostgresBackup {
                    key: Some("basebackups_005/0001_backup_stop_sentinel.json".into()),
                    last_modified: Some("2026-03-31T18:17:37Z".parse().unwrap()),
                    status: Some("ready".into()),
                },
                PostgresBackup::default(),
            ],
            limit: Some(100),
            total_count: Some(2),
            next_cursor: Some("page-2".into()),
        };
        let rendered = render_postgres_backup_page(page);
        assert!(rendered.contains("| Key"), "{rendered}");
        assert!(rendered.contains("Last modified"), "{rendered}");
        assert!(rendered.contains("| Status"), "{rendered}");
        assert!(rendered.contains("| ready"), "{rendered}");
        assert!(rendered.contains("basebackups_005/0001_backup_stop_sentinel.json"));
        assert!(rendered.contains("2026-03-31T18:17:37Z"), "{rendered}");
        assert!(
            rendered.lines().any(|line| {
                line.split('|').map(str::trim).collect::<Vec<_>>() == ["", "-", "-", "-", ""]
            }),
            "{rendered}"
        );
        assert_eq!(rendered.lines().last(), Some("Next cursor: page-2"));

        let empty = render_postgres_backup_page(PostgresBackupPage {
            result: vec![],
            limit: None,
            total_count: None,
            next_cursor: None,
        });
        assert!(!empty.contains("Next cursor"));
        assert!(!empty.contains('|'));
    }

    #[test]
    fn metrics_human_view_preserves_series_points_labels_and_units() {
        let metrics = PostgresMetrics {
            metrics: Some(vec![
                PostgresMetric {
                    key: Some("cpu".to_string()),
                    name: Some("CPU usage".to_string()),
                    description: Some("Average CPU usage".to_string()),
                    unit: Some("percent".to_string()),
                    series: Some(vec![
                        PostgresMetricSeries {
                            label: Some("{role=\"primary\",zone=\"eu-west-1\"}".to_string()),
                            data_points: Some(vec![
                                PostgresMetricDataPoint {
                                    timestamp: Some(1_776_337_200),
                                    value: Some(12.5),
                                },
                                PostgresMetricDataPoint {
                                    timestamp: Some(1_776_337_200),
                                    value: None,
                                },
                            ]),
                        },
                        PostgresMetricSeries {
                            label: Some("line one\nline two".to_string()),
                            data_points: Some(Vec::new()),
                        },
                    ]),
                },
                PostgresMetric {
                    key: Some("replication-lag".to_string()),
                    series: None,
                    ..Default::default()
                },
            ]),
        };

        assert_eq!(
            postgres_metrics_human_view(&metrics).unwrap(),
            serde_json::json!({
                "metrics": [{
                    "description": "Average CPU usage",
                    "key": "cpu",
                    "name": "CPU usage",
                    "series": [{
                        "dataPoints": {
                            "1": "2026-04-16T11:00:00Z  12.5 percent",
                            "2": "2026-04-16T11:00:00Z  -"
                        },
                        "label": "{role=\"primary\",zone=\"eu-west-1\"}"
                    }, {
                        "dataPoints": [],
                        "label": "line one\nline two"
                    }],
                    "unit": "percent"
                }, {
                    "key": "replication-lag",
                    "series": "-"
                }]
            })
        );
    }

    #[test]
    fn metrics_human_view_distinguishes_absent_empty_and_unrepresentable_timestamps() {
        assert_eq!(
            postgres_metrics_human_view(&PostgresMetrics::default()).unwrap(),
            serde_json::json!({"metrics": "-"})
        );
        assert_eq!(
            postgres_metrics_human_view(&PostgresMetrics {
                metrics: Some(Vec::new())
            })
            .unwrap(),
            serde_json::json!({"metrics": []})
        );

        let metrics = PostgresMetrics {
            metrics: Some(vec![PostgresMetric {
                series: Some(vec![PostgresMetricSeries {
                    data_points: Some(vec![PostgresMetricDataPoint {
                        timestamp: Some(i64::MAX),
                        value: Some(1.0),
                    }]),
                    ..Default::default()
                }]),
                ..Default::default()
            }]),
        };
        let output = postgres_metrics_human_view(&metrics).unwrap();
        assert_eq!(
            output.pointer("/metrics/0/series/0/dataPoints/1"),
            Some(&serde_json::Value::String(
                "9223372036854775807 (outside displayable date range)  1".to_string()
            ))
        );
    }

    #[test]
    fn builds_minimal_postgres_logs_query() {
        let query = build_postgres_logs_query(
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(query.from_date, "2026-08-01T00:00:00Z");
        assert_eq!(query.to_date, "2026-08-02T00:00:00Z");
        assert_eq!(query.body_contains, None);
        assert_eq!(query.severity, None);
        assert_eq!(query.sort_order, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn builds_maximal_postgres_logs_query() {
        let query = build_postgres_logs_query(
            "2026-08-01T00:00:00Z",
            "2026-08-31T00:00:00Z",
            Some("checkpoint"),
            Some("LOG"),
            Some(&PostgresLogsGetListSortorder::Asc),
            Some(2000),
            Some(0),
        )
        .unwrap();
        assert_eq!(query.body_contains.as_deref(), Some("checkpoint"));
        assert_eq!(query.severity.as_deref(), Some("LOG"));
        assert_eq!(query.sort_order, Some(PostgresLogsGetListSortorder::Asc));
        assert_eq!(query.limit, Some(2000));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn postgres_log_row_extracts_a_string_message_and_formats_utc() {
        let log = PostgresLogEntry {
            timestamp: Some("2026-08-01T12:00:00.123Z".parse().unwrap()),
            severity: Some("LOG".to_string()),
            body: Some(
                serde_json::json!({
                    "message": "checkpoint complete",
                    "detail": "all buffers written"
                })
                .to_string(),
            ),
        };

        assert_eq!(
            postgres_log_row(&log),
            PostgresLogRow {
                timestamp: "2026-08-01T12:00:00.123Z".to_string(),
                severity: "LOG".to_string(),
                message: "checkpoint complete".to_string(),
            }
        );
    }

    #[test]
    fn postgres_log_message_preserves_raw_and_unusual_bodies() {
        let long_message = "x".repeat(4096);
        for body in [
            "recovery complete",
            r#"{"message":42,"detail":"kept"}"#,
            r#"{"detail":"message absent"}"#,
            r#"{"message":"unfinished""#,
            "null",
            long_message.as_str(),
        ] {
            assert_eq!(postgres_log_message(Some(body)), body);
        }
        assert_eq!(postgres_log_message(None), ABSENT);
        assert_eq!(postgres_log_message(Some("")), "");
    }

    #[test]
    fn rejects_invalid_postgres_logs_time_windows() {
        for (from, to, message) in [
            (
                "2026-08-02T00:00:00Z",
                "2026-08-01T00:00:00Z",
                "to-date must be after from-date",
            ),
            (
                "2026-08-01T00:00:00Z",
                "2026-08-01T00:00:00Z",
                "to-date must be after from-date",
            ),
            (
                "2026-08-01T00:00:00Z",
                "2026-09-01T00:00:01Z",
                "must not exceed 30 days",
            ),
        ] {
            let error =
                build_postgres_logs_query(from, to, None, None, None, None, None).unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn rejects_unknown_typed_postgres_logs_sort_order() {
        let unknown = PostgresLogsGetListSortorder::Unknown("future".to_string());
        let error = build_postgres_logs_query(
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            None,
            None,
            Some(&unknown),
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid sort order 'future'"));
    }

    #[test]
    fn postgres_metrics_normalizes_without_changing_the_instant() {
        for (input, expected) in [
            ("2026-04-16T12:00:00Z", "2026-04-16T12:00:00.000Z"),
            ("2026-04-16T12:00:00.000Z", "2026-04-16T12:00:00.000Z"),
            ("2026-04-16T12:00:00+00:00", "2026-04-16T12:00:00.000Z"),
            ("2026-04-16T12:00:00.1Z", "2026-04-16T12:00:00.100Z"),
            ("2026-04-16T12:00:00.12Z", "2026-04-16T12:00:00.120Z"),
            ("2026-04-16T00:00:00.123+05:30", "2026-04-15T18:30:00.123Z"),
            ("2026-04-16T23:00:00.999-02:30", "2026-04-17T01:30:00.999Z"),
            (
                "2026-04-16T12:00:00.1230000000Z",
                "2026-04-16T12:00:00.123Z",
            ),
        ] {
            let normalized = parse_postgres_window_datetime(input).unwrap();
            assert_eq!(normalized, expected, "{input}");
            assert_eq!(
                chrono::DateTime::parse_from_rfc3339(input).unwrap(),
                chrono::DateTime::parse_from_rfc3339(&normalized).unwrap(),
                "normalization changed {input}"
            );
        }
    }

    #[test]
    fn postgres_metrics_validates_chronological_range() {
        assert!(
            validate_datetime_range("2026-04-16T12:00:00+01:00", "2026-04-16T11:00:00Z").is_ok()
        );
        let error =
            validate_datetime_range("2026-04-16T12:00:01Z", "2026-04-16T12:00:00Z").unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid date range: --from-date must not be after --to-date"
        );
    }

    #[test]
    fn slow_query_list_builder_uses_every_library_sort_value() {
        for &sort_by in SlowQueryPatternsGetListSortby::VALUES {
            let query = build_slow_query_list_query(SlowQueryListInput {
                from_date: "2026-04-16T12:00:00Z",
                to_date: "2026-04-16T13:00:00Z",
                db_name: None,
                db_user: None,
                db_operation: None,
                app: None,
                sort_by: Some(sort_by),
                sort_order: Some("desc"),
                limit: Some(1),
                offset: Some(0),
            })
            .unwrap();
            assert_eq!(query.sort_by.as_ref().unwrap().to_string(), sort_by);
            assert_eq!(
                query.sort_order,
                Some(SlowQueryPatternsGetListSortorder::Desc)
            );
        }
        for &sort_order in SlowQueryPatternsGetListSortorder::VALUES {
            let query = build_slow_query_list_query(SlowQueryListInput {
                from_date: "2026-04-16T12:00:00Z",
                to_date: "2026-04-16T13:00:00Z",
                db_name: None,
                db_user: None,
                db_operation: None,
                app: None,
                sort_by: None,
                sort_order: Some(sort_order),
                limit: None,
                offset: None,
            })
            .unwrap();
            assert_eq!(query.sort_order.as_ref().unwrap().to_string(), sort_order);
        }
    }

    #[test]
    fn slow_query_list_builder_rejects_invalid_ranges_and_typed_values() {
        let input = SlowQueryListInput {
            from_date: "2026-04-16T13:00:00Z",
            to_date: "2026-04-16T12:00:00Z",
            db_name: None,
            db_user: None,
            db_operation: None,
            app: None,
            sort_by: None,
            sort_order: None,
            limit: None,
            offset: None,
        };
        assert!(
            build_slow_query_list_query(input)
                .unwrap_err()
                .to_string()
                .contains("--from-date must not be after --to-date")
        );
        assert!(
            build_slow_query_list_query(SlowQueryListInput {
                from_date: "2026-04-16T12:00:00Z",
                to_date: "2026-04-16T13:00:00Z",
                sort_by: Some("future_sort"),
                ..input
            })
            .unwrap_err()
            .to_string()
            .contains("invalid sort_by")
        );
        assert!(
            build_slow_query_list_query(SlowQueryListInput {
                from_date: "2026-04-16T12:00:00Z",
                to_date: "2026-04-16T13:00:00Z",
                limit: Some(501),
                ..input
            })
            .is_err()
        );
        assert!(
            build_slow_query_list_query(SlowQueryListInput {
                from_date: "2026-04-16T12:00:00Z",
                to_date: "2026-04-16T13:00:00Z",
                offset: Some(-1),
                ..input
            })
            .is_err()
        );
    }

    #[test]
    fn slow_query_builders_preserve_minimal_and_maximal_queries() {
        let minimal = build_slow_query_list_query(SlowQueryListInput {
            from_date: "2026-04-16T12:00:00Z",
            to_date: "2026-04-16T13:00:00Z",
            db_name: None,
            db_user: None,
            db_operation: None,
            app: None,
            sort_by: None,
            sort_order: None,
            limit: None,
            offset: None,
        })
        .unwrap();
        assert_eq!(minimal.from_date, "2026-04-16T12:00:00Z");
        assert_eq!(minimal.to_date, "2026-04-16T13:00:00Z");
        assert_eq!(minimal.db_name, None);
        assert_eq!(minimal.sort_by, None);
        assert_eq!(minimal.limit, None);
        assert_eq!(minimal.offset, None);

        let detail = build_slow_query_detail_query(SlowQueryDetailInput {
            db_name: "app db",
            db_user: "reader",
            db_operation: "SELECT",
            app: Some("reporter"),
            timestamp: Some("2026-04-16T12:30:00+01:00"),
        })
        .unwrap();
        assert_eq!(detail.db_name, "app db");
        assert_eq!(detail.db_user, "reader");
        assert_eq!(detail.db_operation, "SELECT");
        assert_eq!(detail.app, Some("reporter"));
        assert_eq!(detail.timestamp, Some("2026-04-16T12:30:00+01:00"));

        assert!(
            build_slow_query_detail_query(SlowQueryDetailInput {
                db_name: "app",
                db_user: "reader",
                db_operation: "SELECT",
                app: None,
                timestamp: Some("now"),
            })
            .is_err()
        );
    }
}
