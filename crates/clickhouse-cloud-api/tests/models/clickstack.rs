use clickhouse_cloud_api::models::*;

use crate::assert_unknown_variant_round_trips;

#[test]
fn deserialize_clickstack_dashboard_response() {
    let json = r#"{
        "id": "dash-123",
        "name": "My Dashboard",
        "tiles": [],
        "filters": [],
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-02T00:00:00Z"
    }"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(dash.name.as_deref(), Some("My Dashboard"));
}

#[test]
fn clickstack_dashboard_minimal_response() {
    let d: ClickStackDashboardResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(d.id, None);
    assert_eq!(d.name, None);
}

// ===========================================================================
// ClickStack dashboard containers, on-click (issue #203 drift)
// ===========================================================================

#[test]
fn deserialize_clickstack_dashboard_with_containers() {
    let json = r#"{
        "id": "dash-1",
        "name": "Overview",
        "tiles": [
            {
                "id": "tile-1",
                "name": "T1",
                "x": 0, "y": 0, "w": 4, "h": 4,
                "containerId": "c-1",
                "tabId": "t-1"
            }
        ],
        "filters": [],
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-02T00:00:00Z",
        "containers": [
            {
                "id": "c-1",
                "title": "Container A",
                "collapsed": false,
                "tabs": [{"id": "t-1", "title": "Tab 1"}]
            }
        ]
    }"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    let containers = dash.containers.expect("containers should populate");
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].id.as_deref(), Some("c-1"));
    assert_eq!(containers[0].collapsed, Some(false));
    let tabs = containers[0].tabs.as_ref().expect("tabs populated");
    assert_eq!(tabs[0].title.as_deref(), Some("Tab 1"));
    let tiles = dash.tiles.expect("tiles should populate");
    assert_eq!(tiles[0].container_id.as_deref(), Some("c-1"));
    assert_eq!(tiles[0].tab_id.as_deref(), Some("t-1"));
}

#[test]
fn deserialize_clickstack_on_click_search_variant() {
    // ClickStackOnClick is an untagged enum; the Search variant comes first
    // so a "search"-typed payload deserializes through it cleanly.
    let json = r#"{
        "type": "search",
        "target": {"mode": "id", "id": "search-1"}
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClick::ClickStackOnClickSearch(s) => {
            assert_eq!(s.r#type, ClickStackOnClickSearchType::Search);
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_dashboard_struct() {
    // Regression: Search and Dashboard are structurally identical (both require
    // `target` + `type`), so before manual discriminator dispatch a
    // `type: "dashboard"` payload misdispatched to the Search variant. The union
    // now routes on the `type` discriminator.
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"},
        "whereLanguage": "sql",
        "whereTemplate": "x = {{y}}"
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    let dash = match on_click {
        ClickStackOnClick::ClickStackOnClickDashboard(dash) => dash,
        other => panic!("expected dashboard variant, got {other}"),
    };
    assert_eq!(dash.r#type, ClickStackOnClickDashboardType::Dashboard);
    assert_eq!(dash.where_template.as_deref(), Some("x = {{y}}"));
    match dash.target {
        ClickStackOnClickTarget::ClickStackOnClickTargetTemplateVariant(t) => {
            assert_eq!(t.template, "{{x}}");
        }
        other => panic!("expected template target, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_target_id_variant() {
    let json = r#"{"mode": "id", "id": "abc"}"#;
    let target: ClickStackOnClickTarget = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTarget::ClickStackOnClickTargetIdVariant(v) => {
            assert_eq!(v.id, "abc");
            assert_eq!(v.mode, ClickStackOnClickTargetIdVariantMode::Id);
        }
        other => panic!("expected id variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_target_template_variant() {
    let json = r#"{"mode": "template", "template": "{{q}}"}"#;
    let target: ClickStackOnClickTarget = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTarget::ClickStackOnClickTargetTemplateVariant(v) => {
            assert_eq!(v.template, "{{q}}");
            assert_eq!(v.mode, ClickStackOnClickTargetTemplateVariantMode::Template);
        }
        other => panic!("expected template variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_target_unknown_shape_round_trips() {
    // A payload matching neither the id nor template variant must land in the
    // lossless Unknown catch-all and re-serialize to the same JSON object rather
    // than erroring on deserialize.
    let json = r#"{
        "mode": "future_mode",
        "foo": 1
    }"#;
    assert_unknown_variant_round_trips(json, |t: &ClickStackOnClickTarget| {
        matches!(t, ClickStackOnClickTarget::Unknown(_))
    });
}

#[test]
fn deserialize_clickstack_on_click_external_round_trip() {
    let json = r#"{"type": "external", "urlTemplate": "https://example.com/{{ServiceName}}"}"#;
    let ext: ClickStackOnClickExternal = serde_json::from_str(json).unwrap();
    assert_eq!(ext.r#type, ClickStackOnClickExternalType::External);
    assert_eq!(ext.url_template, "https://example.com/{{ServiceName}}");
    let v = serde_json::to_value(&ext).unwrap();
    assert_eq!(v["type"], "external");
    assert_eq!(v["urlTemplate"], "https://example.com/{{ServiceName}}");
}

#[test]
fn deserialize_clickstack_on_click_dispatches_external() {
    // An "external" on-click payload has no `target`, so it cannot match the
    // Search or Dashboard variants (both require `target`) and dispatches
    // through the untagged union to the External variant.
    let json = r#"{"type": "external", "urlTemplate": "https://example.com/{{value}}"}"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match &on_click {
        ClickStackOnClick::ClickStackOnClickExternal(ext) => {
            assert_eq!(ext.r#type, ClickStackOnClickExternalType::External);
            assert_eq!(ext.url_template, "https://example.com/{{value}}");
        }
        other => panic!("expected external variant, got {other}"),
    }
    // Round-trips back to the same wire shape through the union.
    let v = serde_json::to_value(&on_click).unwrap();
    assert_eq!(v["type"], "external");
    assert_eq!(v["urlTemplate"], "https://example.com/{{value}}");
}

#[test]
fn deserialize_clickstack_on_click_dispatches_search() {
    // Regression: adding the External variant must not steal the Search shape.
    let json = r#"{
        "type": "search",
        "target": {"mode": "id", "id": "search-1"}
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClick::ClickStackOnClickSearch(s) => {
            assert_eq!(s.r#type, ClickStackOnClickSearchType::Search);
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_dashboard_still_parses() {
    // Regression: a `type: "dashboard"` payload dispatches to the Dashboard
    // variant through the union rather than being greedily absorbed by the
    // structurally identical Search variant.
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"}
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClick::ClickStackOnClickDashboard(dash) => {
            assert_eq!(dash.r#type, ClickStackOnClickDashboardType::Dashboard);
        }
        other => panic!("expected dashboard variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_unknown_type_round_trip() {
    // An unrecognized `type` discriminator falls back to the Unknown variant,
    // which stores the raw JSON so it round-trips faithfully.
    let json = r#"{"type":"popover","payload":{"nested":[1,2,3]}}"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match &on_click {
        ClickStackOnClick::Unknown(v) => {
            assert_eq!(v["type"], "popover");
            assert_eq!(v["payload"]["nested"][2], 3);
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&on_click).unwrap(), expected);
}

#[test]
fn clickstack_alert_channel_known_variants_deserialize() {
    let email_json = r#"{"emailRecipients": ["a@b.com"], "type": "email"}"#;
    let email: ClickStackAlertChannel = serde_json::from_str(email_json).unwrap();
    match email {
        ClickStackAlertChannel::ClickStackAlertChannelEmail(v) => {
            assert_eq!(v.email_recipients, vec!["a@b.com".to_string()]);
            assert_eq!(v.r#type, ClickStackAlertChannelEmailType::Email);
        }
        other => panic!("expected email variant, got {other}"),
    }

    let webhook_json = r#"{"webhookId": "wh-1", "type": "webhook"}"#;
    let webhook: ClickStackAlertChannel = serde_json::from_str(webhook_json).unwrap();
    match webhook {
        ClickStackAlertChannel::ClickStackAlertChannelWebhook(v) => {
            assert_eq!(v.webhook_id, "wh-1");
            assert_eq!(v.r#type, ClickStackAlertChannelWebhookType::Webhook);
        }
        other => panic!("expected webhook variant, got {other}"),
    }
}

#[test]
fn clickstack_alert_channel_response_known_type_sparse_payload_is_tolerant() {
    // A recognized `type` dispatches hard to its variant, and the response
    // variant's all-`Option` fields are tolerant: a server that drops
    // `emailRecipients` surfaces `None` rather than failing the response.
    let json = r#"{"type":"email"}"#;
    let channel: ClickStackAlertChannelResponse = serde_json::from_str(json).unwrap();
    match channel {
        ClickStackAlertChannelResponse::ClickStackAlertChannelEmail(v) => {
            assert_eq!(v.r#type, Some(ClickStackAlertChannelEmailType::Email));
            assert_eq!(v.email_recipients, None);
        }
        other => panic!("expected email variant, got {other}"),
    }
    // The request variant stays strict: the same sparse payload is not a valid
    // channel to *send*.
    assert!(serde_json::from_str::<ClickStackAlertChannelEmail>(r#"{"type":"email"}"#).is_err());
}

#[test]
fn clickstack_alert_channel_missing_type_key_is_unknown() {
    // This union has no arm for an absent `type` key, in deliberate contrast to
    // the chart-config unions where absence means the Builder variant, so a
    // payload without the discriminator lands in the lossless Unknown catch-all.
    let json = r#"{"webhookId":"wh-1"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_unknown_shape_round_trips() {
    // An unrecognized `type` value must land in the lossless Unknown catch-all
    // and re-serialize to the same JSON object rather than erroring on
    // deserialize.
    let json = r#"{
        "type": "future_channel",
        "foo": 1
    }"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_changed_field_shape_is_unknown() {
    // A recognized `type` whose payload no longer fits the variant — here
    // `emailRecipients` as a string instead of an array — must not fail the
    // response. `list_alerts` returns a Vec, so one such element would otherwise
    // take down the whole call; instead the element lands in Unknown intact.
    let json = r#"{"type":"email","emailRecipients":"a@b.c"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_default_round_trips_to_the_same_variant() {
    // Both alert-channel variants declare the same `enum: ["webhook", "email"]`
    // for their discriminating `type`, so the email variant's default must name
    // `email`: `channel` is defaulted on the alert response, and a default that
    // named `webhook` would come back from its own union as the webhook variant
    // and could be PUT back as a webhook channel with no `webhookId`.
    let default = ClickStackAlertChannel::default();
    assert!(matches!(
        default,
        ClickStackAlertChannel::ClickStackAlertChannelEmail(_)
    ));
    let json = serde_json::to_string(&default).unwrap();
    assert_eq!(json, r#"{"emailRecipients":[],"type":"email"}"#);
    let round_tripped: ClickStackAlertChannel = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, default);

    // A response that drops `channel` entirely surfaces the absence instead of
    // fabricating a default channel.
    let response: ClickStackAlertResponse = serde_json::from_str(r#"{"name":"my-alert"}"#).unwrap();
    assert_eq!(response.channel, None);
}

#[test]
fn deserialize_clickstack_log_source_with_metadata_materialized_views() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "metadataMaterializedViews": {
            "granularity": "1 hour",
            "keyRollupTable": "logs_keys_1h",
            "kvRollupTable": "logs_kv_1h"
        }
    }"#;
    let src: ClickStackLogSourceResponse = serde_json::from_str(json).unwrap();
    let mv = src
        .metadata_materialized_views
        .expect("metadataMaterializedViews should populate");
    assert_eq!(mv.granularity.as_deref(), Some("1 hour"));
    assert_eq!(mv.key_rollup_table.as_deref(), Some("logs_keys_1h"));
    assert_eq!(mv.kv_rollup_table.as_deref(), Some("logs_kv_1h"));
}

#[test]
fn deserialize_clickstack_log_source_without_id() {
    // `id` is server-generated and omitted from create/update request payloads;
    // it must deserialize to None and be dropped on serialize.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts"
    }"#;
    let src: ClickStackLogSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.id, None);
    let v = serde_json::to_value(&src).unwrap();
    assert!(v.get("id").is_none(), "id must be omitted when None");
}

#[test]
fn deserialize_clickstack_trace_source_default_table_select_expression() {
    let json = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "defaultTableSelectExpression": "Timestamp, SpanName",
        "from": {"databaseName": "default", "tableName": "traces"},
        "timestampValueExpression": "Timestamp",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "traceIdExpression": "TraceId",
        "spanIdExpression": "SpanId",
        "parentSpanIdExpression": "ParentSpanId",
        "spanNameExpression": "SpanName",
        "spanKindExpression": "SpanKind"
    }"#;
    let src: ClickStackTraceSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.default_table_select_expression, "Timestamp, SpanName");
    let v = serde_json::to_value(&src).unwrap();
    assert_eq!(v["defaultTableSelectExpression"], "Timestamp, SpanName");

    // `defaultTableSelectExpression` is in the spec `required[]` for trace
    // sources, so the request variant types it as `String`. Responses stay
    // tolerant of a server-side field drop through the response variant, where
    // it is `Option<String>`: a missing field lands as `None` instead of
    // failing the whole payload (and, via the `kind`-dispatched
    // `ClickStackSourceResponse` union, the whole list response).
    let missing = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "traces"},
        "timestampValueExpression": "Timestamp",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "traceIdExpression": "TraceId",
        "spanIdExpression": "SpanId",
        "parentSpanIdExpression": "ParentSpanId",
        "spanNameExpression": "SpanName",
        "spanKindExpression": "SpanKind"
    }"#;
    let src: ClickStackTraceSourceResponse = serde_json::from_str(missing).unwrap();
    assert_eq!(src.default_table_select_expression, None);
    match serde_json::from_str::<ClickStackSourceResponse>(missing).unwrap() {
        ClickStackSourceResponse::ClickStackTraceSource(src) => {
            assert_eq!(src.default_table_select_expression, None);
        }
        other => panic!("expected trace variant, got {other:?}"),
    }
    // The dropped field is absent from the re-serialized payload rather than
    // sent back as `null`.
    let v = serde_json::to_value(&src).unwrap();
    assert!(v.get("defaultTableSelectExpression").is_none());
}

#[test]
fn round_trip_clickstack_log_source_new_fields() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "section": "Billing",
        "disabled": false,
        "knownColumnsListExpression": "Timestamp, Body, ServiceName",
        "useTextIndexForImplicitColumn": "auto"
    }"#;
    let src: ClickStackLogSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.section.as_deref(), Some("Billing"));
    assert_eq!(src.disabled, Some(false));
    assert_eq!(
        src.known_columns_list_expression.as_deref(),
        Some("Timestamp, Body, ServiceName")
    );
    assert_eq!(
        src.use_text_index_for_implicit_column,
        Some(ClickStackLogSourceUsetextindexforimplicitcolumn::Auto)
    );
    let v = serde_json::to_value(&src).unwrap();
    assert_eq!(v["section"], "Billing");
    assert_eq!(v["disabled"], false);
    assert_eq!(
        v["knownColumnsListExpression"],
        "Timestamp, Body, ServiceName"
    );
    assert_eq!(v["useTextIndexForImplicitColumn"], "auto");
}

#[test]
fn clickstack_source_dispatches_promql_variant() {
    let json = r#"{
        "id": "src-1",
        "kind": "promql",
        "name": "Prometheus Metrics",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "metrics"},
        "timestampValueExpression": "timestamp"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackPromqlSource(src) => {
            assert_eq!(src.kind, ClickStackPromqlSourceKind::Promql);
            assert_eq!(src.name, "Prometheus Metrics");
            assert_eq!(src.connection, "conn-1");
            assert_eq!(src.id.as_deref(), Some("src-1"));
        }
        other => panic!("expected Promql source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_log_variant() {
    // Regression: a log-source payload must not be swallowed by the new Promql
    // variant and must still resolve to the log-source variant.
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackLogSource(src) => {
            assert_eq!(src.kind, ClickStackLogSourceKind::Log);
            assert_eq!(src.name, "logs");
        }
        other => panic!("expected log source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_trace_variant() {
    // Regression (PR #311 review P1-1): a spec-valid trace payload shares every
    // Log-required field (defaultTableSelectExpression et al.), so untagged
    // matching would greedily resolve it to the Log variant and drop trace-only
    // fields. Discriminator dispatch on `kind` must route it to the Trace
    // variant with its trace-only fields intact.
    let json = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "traces"},
        "timestampValueExpression": "Timestamp",
        "defaultTableSelectExpression": "Timestamp, SpanName",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "parentSpanIdExpression": "ParentSpanId",
        "spanIdExpression": "SpanId",
        "spanKindExpression": "SpanKind",
        "spanNameExpression": "SpanName",
        "traceIdExpression": "TraceId"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackTraceSource(src) => {
            assert_eq!(src.kind, ClickStackTraceSourceKind::Trace);
            assert_eq!(src.name, "traces");
            assert_eq!(src.default_table_select_expression, "Timestamp, SpanName");
            assert_eq!(src.duration_expression, "Duration");
            assert_eq!(src.duration_precision, 9);
            assert_eq!(src.parent_span_id_expression, "ParentSpanId");
            assert_eq!(src.span_id_expression, "SpanId");
            assert_eq!(src.span_kind_expression, "SpanKind");
            assert_eq!(src.span_name_expression, "SpanName");
            assert_eq!(src.trace_id_expression, "TraceId");
        }
        other => panic!("expected trace source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_metric_variant() {
    let json = r#"{
        "id": "metric-1",
        "kind": "metric",
        "name": "metrics",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "metrics"},
        "timestampValueExpression": "TimeUnix",
        "resourceAttributesExpression": "ResourceAttributes",
        "metricTables": {
            "gauge": "otel_metrics_gauge",
            "histogram": "otel_metrics_histogram",
            "sum": "otel_metrics_sum",
            "summary": "otel_metrics_summary",
            "exponential histogram": "otel_metrics_exponential_histogram"
        }
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackMetricSource(src) => {
            assert_eq!(src.kind, ClickStackMetricSourceKind::Metric);
            assert_eq!(src.name, "metrics");
            assert_eq!(src.resource_attributes_expression, "ResourceAttributes");
            assert_eq!(src.metric_tables.gauge, "otel_metrics_gauge");
        }
        other => panic!("expected metric source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_session_variant() {
    let json = r#"{
        "id": "session-1",
        "kind": "session",
        "name": "sessions",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "sessions"},
        "traceSourceId": "trace-1"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackSessionSource(src) => {
            assert_eq!(src.kind, ClickStackSessionSourceKind::Session);
            assert_eq!(src.name, "sessions");
            assert_eq!(src.trace_source_id, "trace-1");
        }
        other => panic!("expected session source, got {other}"),
    }
}

#[test]
fn clickstack_source_unknown_kind_round_trips() {
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    assert_unknown_variant_round_trips(json, |s: &ClickStackSource| {
        matches!(s, ClickStackSource::Unknown(_))
    });
}

#[test]
fn clickstack_source_serializes_as_inner_log_struct() {
    let log = ClickStackLogSource {
        kind: ClickStackLogSourceKind::Log,
        name: "logs".to_string(),
        connection: "conn-1".to_string(),
        default_table_select_expression: "*".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "logs".to_string(),
        },
        timestamp_value_expression: "ts".to_string(),
        ..Default::default()
    };
    let source = ClickStackSource::ClickStackLogSource(log.clone());
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::to_value(&log).unwrap()
    );
}

#[test]
fn clickstack_source_response_dispatches_on_kind_alone() {
    // The response variants are all-`Option`, so every one of them matches any
    // JSON object: under `untagged` shape matching the first arm would swallow
    // all five kinds. Dispatch reads `kind` off the raw JSON instead, so a
    // payload carrying nothing but its discriminator still resolves to the right
    // variant.
    for (kind, expected) in [
        ("log", "ClickStackLogSource"),
        ("trace", "ClickStackTraceSource"),
        ("metric", "ClickStackMetricSource"),
        ("session", "ClickStackSessionSource"),
        ("promql", "ClickStackPromqlSource"),
    ] {
        let json = format!(r#"{{"kind":"{kind}"}}"#);
        let source: ClickStackSourceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(source.to_string(), expected, "wrong variant for {kind}");
    }
}

#[test]
fn clickstack_source_response_treats_dropped_and_null_fields_as_absent() {
    // Both halves of the tolerance contract on one payload: `connection` is sent
    // as an explicit `null` (which `serde(default)` would not have absorbed) and
    // every other field of the schema is dropped outright.
    let json = r#"{"kind":"log","connection":null,"name":"logs"}"#;
    let source: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let ClickStackSourceResponse::ClickStackLogSource(log) = source else {
        panic!("expected the log variant");
    };
    assert_eq!(log.connection, None);
    assert_eq!(log.from, None);
    assert_eq!(log.timestamp_value_expression, None);
    assert_eq!(log.name.as_deref(), Some("logs"));

    // Absent fields are omitted on the way out, not re-emitted as `null`.
    let v = serde_json::to_value(&log).unwrap();
    assert_eq!(v, serde_json::json!({"kind": "log", "name": "logs"}));
}

#[test]
fn clickstack_source_response_unknown_kind_round_trips() {
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    assert_unknown_variant_round_trips(json, |s: &ClickStackSourceResponse| {
        matches!(s, ClickStackSourceResponse::Unknown(_))
    });
}

#[test]
fn clickstack_source_response_nested_objects_are_tolerant() {
    // The nested objects of a source are response variants too, so a dropped
    // field inside `from`, `filterSettings` or `materializedViews` does not fail
    // the source.
    let json = r#"{
        "kind": "log",
        "from": {"databaseName": "default"},
        "filterSettings": {"columns": [{"name": "ServiceName"}]},
        "materializedViews": [{"tableName": "logs_1h", "aggregatedColumns": [{"aggFn": "sum"}]}],
        "querySettings": [{"setting": "max_threads"}]
    }"#;
    let source: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let ClickStackSourceResponse::ClickStackLogSource(log) = source else {
        panic!("expected the log variant");
    };
    let from = log.from.expect("from present");
    assert_eq!(from.database_name.as_deref(), Some("default"));
    assert_eq!(from.table_name, None);
    let filter_settings = log.filter_settings.expect("filterSettings present");
    assert_eq!(filter_settings.table_name, None);
    let columns = filter_settings.columns.expect("columns present");
    assert_eq!(columns[0].name.as_deref(), Some("ServiceName"));
    assert_eq!(columns[0].label, None);
    let views = log.materialized_views.expect("materializedViews present");
    assert_eq!(views[0].min_granularity, None);
    let aggregated = views[0]
        .aggregated_columns
        .as_deref()
        .expect("aggregatedColumns present");
    assert_eq!(aggregated[0].agg_fn.as_deref(), Some("sum"));
    assert_eq!(aggregated[0].mv_column, None);
    let query_settings = log.query_settings.expect("querySettings present");
    assert_eq!(query_settings[0].value, None);
}

#[test]
fn clickstack_source_try_from_response_converts_every_kind() {
    // One spec-complete payload per kind, each carrying the nested objects that
    // kind owns, so every conversion in the source tree is exercised: a source
    // fetched and written back unchanged must produce the JSON it came from.
    let payloads = [
        serde_json::json!({
            "kind": "log",
            "name": "logs",
            "connection": "conn-1",
            "defaultTableSelectExpression": "*",
            "from": {"databaseName": "default", "tableName": "logs"},
            "timestampValueExpression": "ts",
            "filterSettings": {
                "columns": [{"label": "Service", "name": "ServiceName"}],
                "databaseName": "default",
                "tableName": "logs_filters",
            },
            "highlightedRowAttributeExpressions": [{"sqlExpression": "ServiceName"}],
            "materializedViews": [{
                "aggregatedColumns": [{"aggFn": "sum", "mvColumn": "count"}],
                "databaseName": "default",
                "dimensionColumns": "ServiceName",
                "minGranularity": "1 hour",
                "tableName": "logs_1h",
                "timestampColumn": "ts",
            }],
            "metadataMaterializedViews": {
                "granularity": "1 hour",
                "keyRollupTable": "logs_keys_1h",
                "kvRollupTable": "logs_kv_1h",
            },
            "orderByExpression": "Timestamp DESC",
            "querySettings": [{"setting": "max_threads", "value": "4"}],
        }),
        serde_json::json!({
            "kind": "trace",
            "name": "traces",
            "connection": "conn-1",
            "defaultTableSelectExpression": "*",
            "durationExpression": "Duration",
            "durationPrecision": 9,
            "from": {"databaseName": "default", "tableName": "traces"},
            "parentSpanIdExpression": "ParentSpanId",
            "spanIdExpression": "SpanId",
            "spanKindExpression": "SpanKind",
            "spanNameExpression": "SpanName",
            "timestampValueExpression": "Timestamp",
            "traceIdExpression": "TraceId",
            "displayedTimestampValueExpression": "Timestamp",
            "orderByExpression": "Timestamp DESC",
            "sampleRateExpression": "SampleRate",
            "spanLinksValueExpression": "Links",
            "metadataMaterializedViews": {
                "granularity": "1 hour",
                "keyRollupTable": "traces_keys_1h",
                "kvRollupTable": "traces_kv_1h",
            },
        }),
        serde_json::json!({
            "kind": "metric",
            "name": "metrics",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "metricTables": {
                "exponential histogram": "otel_metrics_exponential_histogram",
                "gauge": "otel_metrics_gauge",
                "histogram": "otel_metrics_histogram",
                "sum": "otel_metrics_sum",
                "summary": "otel_metrics_summary",
            },
            "resourceAttributesExpression": "ResourceAttributes",
            "timestampValueExpression": "TimeUnix",
        }),
        serde_json::json!({
            "kind": "session",
            "name": "sessions",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "sessions"},
            "resourceAttributesExpression": "ResourceAttributes",
            "traceSourceId": "trace-1",
        }),
        serde_json::json!({
            "kind": "promql",
            "name": "prometheus",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp",
        }),
    ];

    for payload in payloads {
        let response: ClickStackSourceResponse = serde_json::from_value(payload.clone()).unwrap();
        let request = ClickStackSource::try_from(response)
            .unwrap_or_else(|e| panic!("{} should convert: {e}", payload["kind"]));
        assert_eq!(serde_json::to_value(&request).unwrap(), payload);
    }
}

#[test]
fn clickstack_source_try_from_response_converts_a_complete_source() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "querySettings": [{"setting": "max_threads", "value": "4"}]
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackSource::try_from(response).expect("conversion should succeed");
    let ClickStackSource::ClickStackLogSource(log) = &request else {
        panic!("expected the log variant");
    };
    assert_eq!(log.connection, "conn-1");
    assert_eq!(log.from.table_name, "logs");
    assert_eq!(
        log.query_settings.as_deref(),
        Some(
            [ClickStackQuerySetting {
                setting: "max_threads".to_string(),
                value: "4".to_string(),
            }]
            .as_slice()
        )
    );
    // A write-back of an untouched source is byte-identical to what was fetched.
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn clickstack_source_try_from_response_names_every_missing_required_field() {
    let response: ClickStackSourceResponse =
        serde_json::from_str(r#"{"kind":"log","name":"logs"}"#).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(
        error.fields(),
        [
            "connection",
            "defaultTableSelectExpression",
            "from",
            "timestampValueExpression"
        ]
    );
    assert_eq!(
        error.to_string(),
        "the API response is missing required field(s): connection, \
         defaultTableSelectExpression, from, timestampValueExpression"
    );
}

#[test]
fn clickstack_source_try_from_response_reports_a_nested_missing_field() {
    // A nested object reports its own wire name: `from` is present, so the
    // failure is `tableName` inside it.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default"},
        "timestampValueExpression": "ts"
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(error.fields(), ["tableName"]);

    // The same holds for an element of a nested list.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "querySettings": [{"setting": "max_threads"}]
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(error.fields(), ["value"]);
}

#[test]
fn clickstack_source_try_from_response_passes_an_unknown_kind_through() {
    // A source kind this crate does not model must still be writable back: the
    // request union's `Unknown` arm holds the raw payload and serializes it
    // verbatim.
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackSource::try_from(response).expect("Unknown converts losslessly");
    assert!(matches!(request, ClickStackSource::Unknown(_)));
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn shared_clickstack_source_types_stay_strict_on_the_request_side() {
    // Sources are sent as well as returned, so each type in the source tree
    // splits: the request variant keeps the schema's required fields as `T` and
    // rejects a payload that omits one, while the response variant accepts any
    // object.
    macro_rules! assert_split_strictness {
        ($($request:ty => $response:ty),+ $(,)?) => {
            $(
                assert!(
                    serde_json::from_str::<$request>("{}").is_err(),
                    "{} accepted a payload missing its required fields",
                    stringify!($request),
                );
                assert_eq!(
                    serde_json::from_str::<$response>("{}").unwrap(),
                    <$response>::default(),
                    "{} rejected an empty payload",
                    stringify!($response),
                );
            )+
        };
    }

    assert_split_strictness!(
        ClickStackAggregatedColumn => ClickStackAggregatedColumnResponse,
        ClickStackClickHouseCASLPermission => ClickStackClickHouseCASLPermissionResponse,
        ClickStackClickHouseCASLPermissionConditions => ClickStackClickHouseCASLPermissionConditionsResponse,
        ClickStackClickHouseCASLPermissionConditionsFrom => ClickStackClickHouseCASLPermissionConditionsFromResponse,
        ClickStackMongoDBCASLPermission => ClickStackMongoDBCASLPermissionResponse,
        ClickStackQueryExpressionFilter => ClickStackQueryExpressionFilterResponse,
        ClickStackStaticListFilter => ClickStackStaticListFilterResponse,
        ClickStackPrometheusLabelFilter => ClickStackPrometheusLabelFilterResponse,
        ClickStackSourceFilter => ClickStackSourceFilterResponse,
        ClickStackFilterSettingsColumn => ClickStackFilterSettingsColumnResponse,
        ClickStackHighlightedAttributeExpression => ClickStackHighlightedAttributeExpressionResponse,
        ClickStackLogSource => ClickStackLogSourceResponse,
        ClickStackLogSourceMetadataMaterializedViews => ClickStackLogSourceMetadataMaterializedViewsResponse,
        ClickStackMaterializedView => ClickStackMaterializedViewResponse,
        ClickStackMetricSource => ClickStackMetricSourceResponse,
        ClickStackMetricSourceFrom => ClickStackMetricSourceFromResponse,
        ClickStackMetricTables => ClickStackMetricTablesResponse,
        ClickStackPromqlSource => ClickStackPromqlSourceResponse,
        ClickStackQuerySetting => ClickStackQuerySettingResponse,
        ClickStackSqlSavedFilterValue => ClickStackSqlSavedFilterValueResponse,
        ClickStackVariableSavedFilterValue => ClickStackVariableSavedFilterValueResponse,
        ClickStackFormula => ClickStackFormulaResponse,
        ClickStackSavedSearchFilter => ClickStackSavedSearchFilterResponse,
        ClickStackSessionSource => ClickStackSessionSourceResponse,
        ClickStackSourceFilterSettings => ClickStackSourceFilterSettingsResponse,
        ClickStackSourceFrom => ClickStackSourceFromResponse,
        ClickStackTraceSource => ClickStackTraceSourceResponse,
        ClickStackTraceSourceMetadataMaterializedViews => ClickStackTraceSourceMetadataMaterializedViewsResponse,
    );
}

#[test]
fn clickstack_response_only_models_accept_an_empty_payload() {
    // Connections, roles and saved searches are only ever returned — the create
    // and update bodies are separate schemas — so they are all-`Option` in place
    // instead of splitting, and none of their fields can fail a response.
    assert_eq!(
        serde_json::from_str::<ClickStackConnection>("{}").unwrap(),
        ClickStackConnection::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackRole>("{}").unwrap(),
        ClickStackRole::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackSavedSearch>("{}").unwrap(),
        ClickStackSavedSearch::default()
    );
    // Their request counterparts stay strict.
    assert!(serde_json::from_str::<ClickStackCreateConnectionRequest>("{}").is_err());
    assert!(serde_json::from_str::<ClickStackCreateRoleRequest>("{}").is_err());
    assert!(serde_json::from_str::<ClickStackSavedSearchInput>("{}").is_err());
}

#[test]
fn shared_clickstack_dashboard_types_stay_strict_on_the_request_side() {
    // Dashboard containers, tiles' chart configs, select items, number formats,
    // color conditions, on-click targets, alert channels and alert chart configs
    // are sent as well as returned, so each splits: the request variant keeps
    // the schema's required fields as `T` and rejects a payload that omits one,
    // while the response variant accepts any object.
    macro_rules! assert_split_strictness {
        ($($request:ty => $response:ty),+ $(,)?) => {
            $(
                assert!(
                    serde_json::from_str::<$request>("{}").is_err(),
                    "{} accepted a payload missing its required fields",
                    stringify!($request),
                );
                assert_eq!(
                    serde_json::from_str::<$response>("{}").unwrap(),
                    <$response>::default(),
                    "{} rejected an empty payload",
                    stringify!($response),
                );
            )+
        };
    }

    assert_split_strictness!(
        ClickStackAlertBarBuilderChartConfig => ClickStackAlertBarBuilderChartConfigResponse,
        ClickStackAlertBarRawSqlChartConfig => ClickStackAlertBarRawSqlChartConfigResponse,
        ClickStackAlertLineBuilderChartConfig => ClickStackAlertLineBuilderChartConfigResponse,
        ClickStackAlertLineRawSqlChartConfig => ClickStackAlertLineRawSqlChartConfigResponse,
        ClickStackAlertNumberBuilderChartConfig => ClickStackAlertNumberBuilderChartConfigResponse,
        ClickStackAlertNumberRawSqlChartConfig => ClickStackAlertNumberRawSqlChartConfigResponse,
        ClickStackAlertStackedLineBuilderChartConfig => ClickStackAlertStackedLineBuilderChartConfigResponse,
        ClickStackAlertStackedLineRawSqlChartConfig => ClickStackAlertStackedLineRawSqlChartConfigResponse,
        ClickStackAlertChannelEmail => ClickStackAlertChannelEmailResponse,
        ClickStackAlertChannelWebhook => ClickStackAlertChannelWebhookResponse,
        ClickStackBackgroundChart => ClickStackBackgroundChartResponse,
        ClickStackBarBuilderChartConfig => ClickStackBarBuilderChartConfigResponse,
        ClickStackBarRawSqlChartConfig => ClickStackBarRawSqlChartConfigResponse,
        ClickStackBetweenColorCondition => ClickStackBetweenColorConditionResponse,
        ClickStackCategoricalBarBuilderChartConfig => ClickStackCategoricalBarBuilderChartConfigResponse,
        ClickStackCategoricalBarRawSqlChartConfig => ClickStackCategoricalBarRawSqlChartConfigResponse,
        ClickStackDashboardContainer => ClickStackDashboardContainerResponse,
        ClickStackDashboardContainerTab => ClickStackDashboardContainerTabResponse,
        ClickStackEqualityColorCondition => ClickStackEqualityColorConditionResponse,
        ClickStackEventPatternsChartConfig => ClickStackEventPatternsChartConfigResponse,
        ClickStackHeatmapDistributionChartConfig => ClickStackHeatmapDistributionChartConfigResponse,
        ClickStackHeatmapSeriesChartConfig => ClickStackHeatmapSeriesChartConfigResponse,
        ClickStackHeatmapSelectItem => ClickStackHeatmapSelectItemResponse,
        ClickStackLineBuilderChartConfig => ClickStackLineBuilderChartConfigResponse,
        ClickStackLineRawSqlChartConfig => ClickStackLineRawSqlChartConfigResponse,
        ClickStackMarkdownChartConfig => ClickStackMarkdownChartConfigResponse,
        ClickStackNumberBuilderChartConfig => ClickStackNumberBuilderChartConfigResponse,
        ClickStackNumberFormat => ClickStackNumberFormatResponse,
        ClickStackNumberRawSqlChartConfig => ClickStackNumberRawSqlChartConfigResponse,
        ClickStackNumericColorCondition => ClickStackNumericColorConditionResponse,
        ClickStackOnClickDashboard => ClickStackOnClickDashboardResponse,
        ClickStackOnClickExternal => ClickStackOnClickExternalResponse,
        ClickStackOnClickFilterTemplate => ClickStackOnClickFilterTemplateResponse,
        ClickStackOnClickSearch => ClickStackOnClickSearchResponse,
        ClickStackOnClickTargetIdVariant => ClickStackOnClickTargetIdVariantResponse,
        ClickStackOnClickTargetTemplateVariant => ClickStackOnClickTargetTemplateVariantResponse,
        ClickStackPieBuilderChartConfig => ClickStackPieBuilderChartConfigResponse,
        ClickStackPieRawSqlChartConfig => ClickStackPieRawSqlChartConfigResponse,
        ClickStackSearchChartConfig => ClickStackSearchChartConfigResponse,
        ClickStackSelectItem => ClickStackSelectItemResponse,
        ClickStackStackedLineBuilderChartConfig => ClickStackStackedLineBuilderChartConfigResponse,
        ClickStackStackedLineRawSqlChartConfig => ClickStackStackedLineRawSqlChartConfigResponse,
        ClickStackTableBuilderChartConfig => ClickStackTableBuilderChartConfigResponse,
        ClickStackTableRawSqlChartConfig => ClickStackTableRawSqlChartConfigResponse,
    );
}

#[test]
fn clickstack_alert_and_dashboard_response_models_accept_an_empty_payload() {
    // Alerts, dashboards, tiles, validation results and webhooks are only ever
    // returned — their request bodies are separate schemas — so they are
    // all-`Option` in place instead of splitting, and none of their fields can
    // fail a response.
    macro_rules! assert_accepts_empty {
        ($($model:ty),+ $(,)?) => {
            $(
                assert_eq!(
                    serde_json::from_str::<$model>("{}").unwrap(),
                    <$model>::default(),
                    "{} rejected an empty payload",
                    stringify!($model),
                );
            )+
        };
    }

    assert_accepts_empty!(
        ClickStackAlertExecutionError,
        ClickStackAlertResponse,
        ClickStackAlertSilenced,
        ClickStackDashboardResponse,
        ClickStackGenericWebhook,
        ClickStackIncidentIOWebhook,
        ClickStackPagerDutyAPIWebhook,
        ClickStackSlackAPIWebhook,
        ClickStackSlackWebhook,
        ClickStackTileOutput,
        ClickStackValidateDashboardError,
        ClickStackValidateDashboardResponse,
        ClickStackValidateWebhookResponse,
        ClickStackValidateWebhookResponseErrorsItem,
    );
}

#[test]
fn clickstack_dashboard_response_null_fields_deserialize_to_none() {
    // An explicit JSON `null` — not just a missing key — must land as `None`:
    // `Option<T>` absorbs it natively, which `serde(default)` never did.
    let json = r#"{"id":null,"name":null,"tiles":null,"filters":null,"tags":null}"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(dash, ClickStackDashboardResponse::default());
    // Absent fields are omitted on serialize, not emitted as `null`.
    assert_eq!(serde_json::to_value(&dash).unwrap(), serde_json::json!({}));
}

#[test]
fn clickstack_tile_config_response_raw_sql_body_without_config_type_falls_to_sub_union_unknown() {
    // Every field of a response builder variant is `Option`, so the builder arm
    // is total and would absorb a raw-SQL payload whose server dropped
    // `configType`, silently retyping it and losing `connectionId` and
    // `sqlTemplate`. The `none unless` guard routes it to Unknown, which keeps
    // the payload verbatim.
    let json = r#"{"displayType":"line","connectionId":"conn-1","sqlTemplate":"SELECT 1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(
            cfg,
            ClickStackTileConfigResponse::ClickStackLineChartConfig(
                ClickStackLineChartConfigResponse::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_dispatches_raw_sql_variant() {
    // A `configType: "sql"` payload dispatches to the raw-SQL response variant
    // even when the raw-SQL-required fields are missing: response tolerance is
    // per-field, not per-variant.
    let json = r#"{"displayType":"line","configType":"sql"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, None);
            assert_eq!(r.sql_template, None);
        }
        other => panic!("expected line raw-SQL variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_changed_field_shape_falls_to_sub_union_unknown() {
    // A recognized discriminator whose payload no longer fits the variant —
    // here `sqlTemplate` as a number — must not fail the response: the element
    // lands in the lossless Unknown catch-all and round-trips verbatim.
    let json = r#"{"displayType":"line","configType":"sql","sqlTemplate":5}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(
            cfg,
            ClickStackTileConfigResponse::ClickStackLineChartConfig(
                ClickStackLineChartConfigResponse::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_unknown_display_type_round_trips() {
    let json = r#"{"displayType":"hologram","select":[]}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(cfg, ClickStackTileConfigResponse::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_response_dispatches_and_absorbs_changed_shapes() {
    // Known discriminators dispatch to their response variants.
    let email: ClickStackAlertChannelResponse =
        serde_json::from_str(r#"{"type":"email","emailRecipients":["a@b.c"]}"#).unwrap();
    match email {
        ClickStackAlertChannelResponse::ClickStackAlertChannelEmail(v) => {
            assert_eq!(
                v.email_recipients.as_deref(),
                Some(&["a@b.c".to_string()][..])
            );
        }
        other => panic!("expected email variant, got {other}"),
    }
    // A recognized `type` whose payload no longer fits the variant — here
    // `emailRecipients` as a string — lands in Unknown intact.
    let json = r#"{"type":"email","emailRecipients":"a@b.c"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannelResponse| {
        matches!(c, ClickStackAlertChannelResponse::Unknown(_))
    });
    // As does an unrecognized `type`.
    let json = r#"{"type":"future_channel","foo":1}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannelResponse| {
        matches!(c, ClickStackAlertChannelResponse::Unknown(_))
    });
}

#[test]
fn clickstack_on_click_response_dispatches_on_type_and_mode() {
    let json = r#"{"type":"external","urlTemplate":"https://example.com/{{id}}"}"#;
    let on_click: ClickStackOnClickResponse = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClickResponse::ClickStackOnClickExternal(e) => {
            assert_eq!(
                e.url_template.as_deref(),
                Some("https://example.com/{{id}}")
            );
        }
        other => panic!("expected external variant, got {other}"),
    }
    // A search on-click whose server dropped `target` still dispatches; the
    // absence is a `None`, not a failure.
    let json = r#"{"type":"search"}"#;
    let on_click: ClickStackOnClickResponse = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClickResponse::ClickStackOnClickSearch(s) => assert_eq!(s.target, None),
        other => panic!("expected search variant, got {other}"),
    }
    let json = r#"{"mode":"template","template":"{{rowId}}"}"#;
    let target: ClickStackOnClickTargetResponse = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTargetResponse::ClickStackOnClickTargetTemplateVariant(t) => {
            assert_eq!(t.template.as_deref(), Some("{{rowId}}"));
        }
        other => panic!("expected template variant, got {other}"),
    }
    // Missing discriminators land in Unknown: these unions have no absence arm.
    let json = r#"{"urlTemplate":"https://example.com"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackOnClickResponse| {
        matches!(c, ClickStackOnClickResponse::Unknown(_))
    });
    let json = r#"{"template":"{{rowId}}"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackOnClickTargetResponse| {
        matches!(c, ClickStackOnClickTargetResponse::Unknown(_))
    });
}

#[test]
fn clickstack_number_tile_color_condition_response_dispatches_on_operator() {
    let json = r#"{"operator":"between","value":[1.0, 2.0]}"#;
    let cond: ClickStackNumberTileColorConditionResponse = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorConditionResponse::ClickStackBetweenColorCondition(b) => {
            assert_eq!(b.value.as_deref(), Some(&[1.0, 2.0][..]));
            assert_eq!(b.color, None);
        }
        other => panic!("expected between variant, got {other}"),
    }
    let json = r#"{"operator":"someday","value":7}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackNumberTileColorConditionResponse| {
        matches!(c, ClickStackNumberTileColorConditionResponse::Unknown(_))
    });
}

#[test]
fn deserialize_clickstack_alert_with_note() {
    let json = r#"{
        "id": "alert-1",
        "name": "High CPU",
        "note": "investigate runaway queries"
    }"#;
    let alert: ClickStackAlertResponse = serde_json::from_str(json).unwrap();
    assert_eq!(alert.note.as_deref(), Some("investigate runaway queries"));
    // Round-trip the optional `note` to confirm it serializes (no rename, but
    // its skip_serializing_if=None gate must let Some(_) through).
    let v = serde_json::to_value(&alert).unwrap();
    assert_eq!(v["note"], "investigate runaway queries");
}

#[test]
fn clickstack_alert_num_consecutive_windows_present_and_absent() {
    let json = r#"{
        "id": "alert-1",
        "name": "High CPU",
        "numConsecutiveWindows": 3
    }"#;
    let alert: ClickStackAlertResponse = serde_json::from_str(json).unwrap();
    assert_eq!(alert.num_consecutive_windows, Some(3));
    let v = serde_json::to_value(&alert).unwrap();
    assert_eq!(v["numConsecutiveWindows"], 3);

    // Absent -> None -> dropped by skip_serializing_if.
    let alert = ClickStackAlertResponse::default();
    assert_eq!(alert.num_consecutive_windows, None);
    let v = serde_json::to_value(&alert).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_create_alert_num_consecutive_windows_round_trip() {
    let v = serde_json::to_value(ClickStackCreateAlertRequest {
        num_consecutive_windows: Some(5),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(v["numConsecutiveWindows"], 5);
    let v = serde_json::to_value(ClickStackCreateAlertRequest::default()).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_update_alert_num_consecutive_windows_round_trip() {
    let v = serde_json::to_value(ClickStackUpdateAlertRequest {
        num_consecutive_windows: Some(7),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(v["numConsecutiveWindows"], 7);
    let v = serde_json::to_value(ClickStackUpdateAlertRequest::default()).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_filter_applies_to_source_ids_round_trip() {
    let json = r#"{
        "expression": "level = 'error'",
        "id": "f-1",
        "name": "errors",
        "sourceId": "src-1",
        "type": "QUERY_EXPRESSION",
        "appliesToSourceIds": ["src-1", "src-2"]
    }"#;
    let filter: ClickStackFilter = serde_json::from_str(json).unwrap();
    let ClickStackFilter::ClickStackQueryExpressionFilter(filter) = filter else {
        panic!("expected the query-expression variant, got {filter:?}");
    };
    assert_eq!(
        filter.applies_to_source_ids,
        Some(vec!["src-1".to_string(), "src-2".to_string()])
    );
    let v = serde_json::to_value(&filter).unwrap();
    assert_eq!(v["appliesToSourceIds"][0], "src-1");
    assert_eq!(v["appliesToSourceIds"][1], "src-2");

    // Absent -> None -> dropped.
    let filter = ClickStackQueryExpressionFilter::default();
    assert_eq!(filter.applies_to_source_ids, None);
    let v = serde_json::to_value(&filter).unwrap();
    assert!(v.get("appliesToSourceIds").is_none());
}

#[test]
fn clickstack_filter_input_applies_to_source_ids_round_trip() {
    let json = r#"{
        "expression": "level = 'error'",
        "name": "errors",
        "sourceId": "src-1",
        "type": "QUERY_EXPRESSION",
        "appliesToSourceIds": ["src-9"]
    }"#;
    let filter: ClickStackFilterInput = serde_json::from_str(json).unwrap();
    let ClickStackFilterInput::ClickStackQueryExpressionFilterInput(filter) = filter else {
        panic!("expected the query-expression variant, got {filter:?}");
    };
    assert_eq!(
        filter.applies_to_source_ids,
        Some(vec!["src-9".to_string()])
    );
    let v = serde_json::to_value(&filter).unwrap();
    assert_eq!(v["appliesToSourceIds"][0], "src-9");

    let v = serde_json::to_value(ClickStackQueryExpressionFilterInput::default()).unwrap();
    assert!(v.get("appliesToSourceIds").is_none());
}

#[test]
fn clickstack_alert_response_state_pending() {
    let parsed: ClickStackAlertResponseState = serde_json::from_str("\"PENDING\"").unwrap();
    assert_eq!(parsed, ClickStackAlertResponseState::PENDING);
    assert_eq!(parsed.to_string(), "PENDING");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "PENDING");
}

#[test]
fn clickstack_number_tile_color_condition_numeric_variant() {
    // The manual `operator`-keyed dispatch routes `gt`/`gte`/`lt`/`lte` to the
    // numeric variant.
    let json = r#"{"operator": "gt", "value": 100, "color": "chart-red"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c) => {
            assert_eq!(c.operator, ClickStackNumericColorConditionOperator::Gt);
            assert_eq!(c.value, 100.0);
            assert_eq!(c.color, ClickStackChartColor::Chart_red);
        }
        other => panic!("expected numeric variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_between_variant() {
    // The manual dispatch routes the `between` operator, with its inclusive
    // [min, max] array value, to the between variant.
    let json = r#"{"operator": "between", "value": [100, 500], "color": "chart-warning"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackBetweenColorCondition(c) => {
            assert_eq!(c.operator, ClickStackBetweenColorConditionOperator::Between);
            assert_eq!(c.value, vec![100.0, 500.0]);
            assert_eq!(c.color, ClickStackChartColor::Chart_warning);
        }
        other => panic!("expected between variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_equality_string_value() {
    // The manual dispatch routes `eq` to the equality variant, whose value
    // accepts strings as well as numbers.
    let json =
        r#"{"operator": "eq", "value": "healthy", "color": "chart-success", "label": "Healthy"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c) => {
            assert_eq!(c.operator, ClickStackEqualityColorConditionOperator::Eq);
            assert_eq!(c.value, serde_json::json!("healthy"));
            assert_eq!(c.color, ClickStackChartColor::Chart_success);
            assert_eq!(c.label.as_deref(), Some("Healthy"));
        }
        other => panic!("expected equality variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_equality_numeric_value() {
    // A numeric-valued `eq` is structurally identical to a numeric condition;
    // only the `operator`-keyed dispatch routes it to the equality variant
    // rather than the numeric one.
    let json = r#"{"operator": "eq", "value": 42, "color": "chart-error"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c) => {
            assert_eq!(c.operator, ClickStackEqualityColorConditionOperator::Eq);
            assert_eq!(c.value.as_f64(), Some(42.0));
            assert_eq!(c.color, ClickStackChartColor::Chart_error);
        }
        other => panic!("expected equality variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_dispatches_every_operator() {
    // Exhaustive guard over the `operator`-keyed dispatch: every wire value in the
    // discriminated_union! invocation must route to its concrete variant, never the
    // Unknown catch-all. A typo in any operator literal in the macro ("gte"/"lt"/
    // "lte"/"neq"/...) would silently misroute a valid payload to Unknown, and this
    // table would fail on that operator. The seven values are cross-checked against
    // the enum members of the three constituent schemas in the OpenAPI snapshot.
    #[derive(Debug, PartialEq)]
    enum Variant {
        Numeric,
        Between,
        Equality,
    }

    let cases: &[(&str, serde_json::Value, Variant)] = &[
        ("gt", serde_json::json!(100), Variant::Numeric),
        ("gte", serde_json::json!(100), Variant::Numeric),
        ("lt", serde_json::json!(100), Variant::Numeric),
        ("lte", serde_json::json!(100), Variant::Numeric),
        ("between", serde_json::json!([100, 500]), Variant::Between),
        ("eq", serde_json::json!(42), Variant::Equality),
        ("neq", serde_json::json!("healthy"), Variant::Equality),
    ];

    for (operator, value, expected) in cases {
        let json = serde_json::json!({
            "operator": operator,
            "value": value,
            "color": "chart-red",
        });
        let cond: ClickStackNumberTileColorCondition = serde_json::from_value(json)
            .unwrap_or_else(|e| panic!("operator {operator:?} failed to deserialize: {e}"));
        match (&cond, expected) {
            (
                ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c),
                Variant::Numeric,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (
                ClickStackNumberTileColorCondition::ClickStackBetweenColorCondition(c),
                Variant::Between,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (
                ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c),
                Variant::Equality,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (other, _) => {
                panic!("operator {operator:?} expected {expected:?} variant, got {other}")
            }
        }
        assert!(
            !matches!(cond, ClickStackNumberTileColorCondition::Unknown(_)),
            "operator {operator:?} misrouted to the Unknown catch-all"
        );
    }
}

#[test]
fn clickstack_number_tile_color_condition_unknown_operator_round_trip() {
    // An unrecognized `operator` discriminator falls back to the Unknown
    // variant, which now stores the raw JSON object and round-trips faithfully.
    let json = r#"{"operator":"contains","value":"warn","color":"chart-red"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match &cond {
        ClickStackNumberTileColorCondition::Unknown(v) => {
            assert_eq!(v["operator"], "contains");
            assert_eq!(v["value"], "warn");
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&cond).unwrap(), expected);
}

#[test]
fn clickstack_tile_config_categorical_bar_builder_variant() {
    // `displayType: "bar"` must dispatch to the categorical bar variant, not the
    // stacked bar variant (whose discriminator is "stacked_bar").
    let json = r#"{
        "displayType": "bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfig::ClickStackCategoricalBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackCategoricalBarBuilderChartConfigDisplaytype::Bar
            );
            assert_eq!(b.select.len(), 1);
        }
        other => panic!("expected categorical bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_categorical_bar_raw_sql_variant() {
    // The Raw SQL categorical bar (configType "sql") also dispatches to the
    // categorical bar variant rather than the stacked bar Raw SQL variant.
    let json = r#"{
        "displayType": "bar",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfig::ClickStackCategoricalBarRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t GROUP BY service");
            assert_eq!(
                r.display_type,
                ClickStackCategoricalBarRawSqlChartConfigDisplaytype::Bar
            );
        }
        other => panic!("expected categorical bar raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_not_categorical_bar() {
    // The manual `displayType`-keyed dispatch routes "stacked_bar" to the
    // stacked-bar variant, never to the structurally identical categorical
    // bar variant (whose displayType is "bar").
    let json = r#"{
        "displayType": "stacked_bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    assert!(
        !matches!(
            cfg,
            ClickStackTileConfig::ClickStackCategoricalBarChartConfig(_)
        ),
        "stacked_bar must not dispatch to the categorical bar variant, got {cfg}"
    );
}

#[test]
fn clickstack_tile_config_event_patterns_variant() {
    // `displayType: "event_patterns"` dispatches to the event-patterns variant.
    let json = r#"{
        "displayType": "event_patterns",
        "sourceId": "src-9",
        "where": "level:error",
        "whereLanguage": "lucene"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackEventPatternsChartConfig(e) => {
            assert_eq!(e.source_id, "src-9");
            assert_eq!(
                e.display_type,
                ClickStackEventPatternsChartConfigDisplaytype::Event_patterns
            );
            assert_eq!(e.r#where.as_deref(), Some("level:error"));
            assert_eq!(
                e.where_language,
                Some(ClickStackEventPatternsChartConfigWherelanguage::Lucene)
            );
        }
        other => panic!("expected event patterns variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_line_variant() {
    // `displayType: "line"` must dispatch to the line variant with a typed
    // display_type, not be swallowed as an Unknown displayType by another
    // structurally-identical builder variant.
    let json = r#"{
        "displayType": "line",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackLineChartConfig(
            ClickStackLineChartConfig::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackLineBuilderChartConfigDisplaytype::Line
            );
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_builder_variant() {
    // `displayType: "stacked_bar"` must dispatch to the (stacked) bar variant
    // with a typed display_type.
    let json = r#"{
        "displayType": "stacked_bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackBarChartConfig(
            ClickStackBarChartConfig::ClickStackBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackBarBuilderChartConfigDisplaytype::Stacked_bar
            );
        }
        other => panic!("expected stacked bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_variant() {
    // `displayType: "table"` must dispatch to the table variant with a typed
    // display_type (the legacy misdispatch parsed it as the first-listed
    // variant with an Unknown display_type).
    let json = r#"{
        "displayType": "table",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackTableChartConfig(
            ClickStackTableChartConfig::ClickStackTableBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackTableBuilderChartConfigDisplaytype::Table
            );
        }
        other => panic!("expected table builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_variant() {
    // `displayType: "number"` must dispatch to the number variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "number",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackNumberChartConfig(
            ClickStackNumberChartConfig::ClickStackNumberBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackNumberBuilderChartConfigDisplaytype::Number
            );
        }
        other => panic!("expected number builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_variant() {
    // `displayType: "pie"` must dispatch to the pie variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "pie",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackPieChartConfig(
            ClickStackPieChartConfig::ClickStackPieBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackPieBuilderChartConfigDisplaytype::Pie
            );
        }
        other => panic!("expected pie builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_search_variant() {
    // `displayType: "search"` must dispatch to the search variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "search",
        "sourceId": "src-1",
        "select": "*",
        "whereLanguage": "lucene"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackSearchChartConfig(s) => {
            assert_eq!(s.source_id, "src-1");
            assert_eq!(s.select, "*");
            assert_eq!(
                s.display_type,
                ClickStackSearchChartConfigDisplaytype::Search
            );
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_markdown_variant() {
    // `displayType: "markdown"` must dispatch to the markdown variant with a
    // typed display_type rather than being absorbed by an earlier variant.
    let json = r#"{
        "displayType": "markdown",
        "markdown": "hello world"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackMarkdownChartConfig(m) => {
            assert_eq!(m.markdown.as_deref(), Some("hello world"));
            assert_eq!(
                m.display_type,
                ClickStackMarkdownChartConfigDisplaytype::Markdown
            );
        }
        other => panic!("expected markdown variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_heatmap_variant() {
    // `displayType: "heatmap"` is the only discriminator that reaches the
    // ClickStackHeatmapChartConfig arm; the heatmap-specific `select` shape with
    // `valueExpression` is then parsed by that variant, not used to select it.
    let json = r#"{
        "displayType": "heatmap",
        "sourceId": "src-1",
        "select": [{"valueExpression": "latency_ms"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackHeatmapChartConfig(
            ClickStackHeatmapChartConfig::ClickStackHeatmapDistributionChartConfig(h),
        ) => {
            assert_eq!(h.source_id, "src-1");
            assert_eq!(
                h.display_type,
                ClickStackHeatmapDistributionChartConfigDisplaytype::Heatmap
            );
            assert_eq!(h.heatmap_mode, None);
            assert_eq!(h.select.len(), 1);
            assert_eq!(h.select[0].value_expression, "latency_ms");
        }
        other => panic!("expected heatmap variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_unknown_display_type_round_trips() {
    // An unrecognized `displayType` falls to the Unknown catch-all, which now
    // stores the raw object and round-trips it faithfully.
    let json = r#"{"displayType":"sankey","sourceId":"src-1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(cfg, ClickStackTileConfig::Unknown(_))
    });
}

#[test]
fn clickstack_tile_config_response_line_novel_shape_dispatches_to_builder() {
    // A known `displayType` carrying no `configType` dispatches to the builder
    // variant: the novel member is ignored and the builder's all-`Option`
    // response fields surface `None` instead of failing the response.
    let json = r#"{"displayType":"line","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_line_minimal_body_dispatches_to_builder() {
    // The bare discriminator with no `configType` key at all: key absence is the
    // builder discriminator, and every other builder response field is absent.
    let json = r#"{"displayType":"line"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(
                b,
                ClickStackLineBuilderChartConfigResponse {
                    display_type: Some(ClickStackLineBuilderChartConfigDisplaytype::Line),
                    ..ClickStackLineBuilderChartConfigResponse::default()
                }
            );
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_non_string_config_type_dispatches_to_builder() {
    // A non-string `configType` is deliberately conflated with an absent one:
    // both dispatch to the builder variant rather than to Unknown.
    let json = r#"{"displayType":"line","configType":123,"sourceId":"src-1"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id.as_deref(), Some("src-1"));
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_line_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the line sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"line","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_raw_sql_body_without_config_type_falls_to_sub_union_unknown() {
    // `configType` is spec-required on the Raw SQL configs, so a server that
    // stops sending it must not silently retype the tile: the builder variant is
    // total and would otherwise absorb the body and drop `connectionId` and
    // `sqlTemplate`. The `unless` guard routes it to Unknown, which keeps the
    // payload verbatim.
    let json = r#"{"displayType":"line","connectionId":"conn-1","sqlTemplate":"SELECT 1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });

    // Either guard key on its own disqualifies the builder variant, and the
    // guard is wired on every chart-config sub-union, not just the line one.
    let json = r#"{"displayType":"number","connectionId":"conn-1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackNumberChartConfig(
                ClickStackNumberChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_changed_field_shape_falls_to_sub_union_unknown() {
    // Field-level tolerance only covers a field the API stops sending. A field
    // whose *shape* changes cannot deserialize into the dispatched variant, so
    // the union hands the payload to its lossless Unknown catch-all rather than
    // failing the whole dashboard response.
    let builder = r#"{"displayType":"line","sourceId":123}"#;
    assert_unknown_variant_round_trips(builder, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });

    let raw_sql = r#"{"displayType":"line","configType":"sql","sqlTemplate":123}"#;
    assert_unknown_variant_round_trips(raw_sql, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_line_raw_sql_variant() {
    // The Raw SQL line config (configType "sql") dispatches to the line
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "line",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY toStartOfMinute(ts)"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackLineChartConfig(
            ClickStackLineChartConfig::ClickStackLineRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT count() FROM t GROUP BY toStartOfMinute(ts)"
            );
            assert_eq!(
                r.display_type,
                ClickStackLineRawSqlChartConfigDisplaytype::Line
            );
        }
        other => panic!("expected line raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_raw_sql_variant() {
    // The Raw SQL (stacked) bar config (configType "sql") dispatches to the bar
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "stacked_bar",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackBarChartConfig(
            ClickStackBarChartConfig::ClickStackBarRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t GROUP BY service");
            assert_eq!(
                r.display_type,
                ClickStackBarRawSqlChartConfigDisplaytype::Stacked_bar
            );
        }
        other => panic!("expected stacked bar raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_raw_sql_variant() {
    // The Raw SQL table config (configType "sql") dispatches to the table
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "table",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT service, count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackTableChartConfig(
            ClickStackTableChartConfig::ClickStackTableRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT service, count() FROM t GROUP BY service"
            );
            assert_eq!(
                r.display_type,
                ClickStackTableRawSqlChartConfigDisplaytype::Table
            );
        }
        other => panic!("expected table raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_raw_sql_variant() {
    // The Raw SQL number config (configType "sql") dispatches to the number
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "number",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackNumberChartConfig(
            ClickStackNumberChartConfig::ClickStackNumberRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t");
            assert_eq!(
                r.display_type,
                ClickStackNumberRawSqlChartConfigDisplaytype::Number
            );
        }
        other => panic!("expected number raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_raw_sql_variant() {
    // The Raw SQL pie config (configType "sql") dispatches to the pie
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "pie",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT service, count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackPieChartConfig(
            ClickStackPieChartConfig::ClickStackPieRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT service, count() FROM t GROUP BY service"
            );
            assert_eq!(
                r.display_type,
                ClickStackPieRawSqlChartConfigDisplaytype::Pie
            );
        }
        other => panic!("expected pie raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_categorical_bar_novel_shape_dispatches_to_builder() {
    // A "bar" body with no `configType` dispatches to the categorical bar builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"bar","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfigResponse::ClickStackCategoricalBarBuilderChartConfig(
                b,
            ),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected categorical bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_categorical_bar_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the categorical bar
    // sub-union's Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"bar","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
                ClickStackCategoricalBarChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_stacked_bar_novel_shape_dispatches_to_builder() {
    // A "stacked_bar" body with no `configType` dispatches to the bar builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"stacked_bar","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackBarChartConfig(
            ClickStackBarChartConfigResponse::ClickStackBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the bar sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"stacked_bar","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackBarChartConfig(ClickStackBarChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_response_table_novel_shape_dispatches_to_builder() {
    // A "table" body with no `configType` dispatches to the table builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"table","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackTableChartConfig(
            ClickStackTableChartConfigResponse::ClickStackTableBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected table builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the table sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"table","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackTableChartConfig(ClickStackTableChartConfig::Unknown(
                _
            ))
        )
    });
}

#[test]
fn clickstack_tile_config_response_number_novel_shape_dispatches_to_builder() {
    // A "number" body with no `configType` dispatches to the number builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"number","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackNumberChartConfig(
            ClickStackNumberChartConfigResponse::ClickStackNumberBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected number builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the number sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"number","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackNumberChartConfig(
                ClickStackNumberChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_pie_novel_shape_dispatches_to_builder() {
    // A "pie" body with no `configType` dispatches to the pie builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"pie","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackPieChartConfig(
            ClickStackPieChartConfigResponse::ClickStackPieBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected pie builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the pie sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"pie","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackPieChartConfig(ClickStackPieChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_chart_color_known_and_unknown_round_trip() {
    // A known palette token maps to its typed variant and back.
    let known: ClickStackChartColor = serde_json::from_str("\"chart-light-blue\"").unwrap();
    assert_eq!(known, ClickStackChartColor::Chart_light_blue);
    assert_eq!(known.to_string(), "chart-light-blue");
    assert_eq!(serde_json::to_value(&known).unwrap(), "chart-light-blue");

    // An unrecognized token round-trips through the Unknown(String) catch-all.
    let unknown: ClickStackChartColor = serde_json::from_str("\"chart-teal\"").unwrap();
    assert_eq!(
        unknown,
        ClickStackChartColor::Unknown("chart-teal".to_string())
    );
    assert_eq!(unknown.to_string(), "chart-teal");
    assert_eq!(serde_json::to_value(&unknown).unwrap(), "chart-teal");
}

#[test]
fn clickstack_background_chart_round_trip() {
    let json = r#"{"type": "area", "color": "chart-blue"}"#;
    let bg: ClickStackBackgroundChart = serde_json::from_str(json).unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Area);
    assert_eq!(bg.color, Some(ClickStackChartColor::Chart_blue));
    let v = serde_json::to_value(&bg).unwrap();
    assert_eq!(v["type"], "area");
    assert_eq!(v["color"], "chart-blue");

    // color is optional and dropped when absent.
    let json = r#"{"type": "line"}"#;
    let bg: ClickStackBackgroundChart = serde_json::from_str(json).unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Line);
    assert_eq!(bg.color, None);
    let v = serde_json::to_value(&bg).unwrap();
    assert!(v.get("color").is_none());
}

#[test]
fn clickstack_number_builder_chart_config_color_fields_round_trip() {
    let json = r#"{
        "displayType": "number",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}],
        "color": "chart-blue",
        "colorRules": [
            {"operator": "gt", "value": 100, "color": "chart-red"}
        ],
        "backgroundChart": {"type": "line", "color": "chart-green"}
    }"#;
    let cfg: ClickStackNumberBuilderChartConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.color, Some(ClickStackChartColor::Chart_blue));
    let rules = cfg.color_rules.as_ref().unwrap();
    assert_eq!(rules.len(), 1);
    match &rules[0] {
        ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c) => {
            assert_eq!(c.operator, ClickStackNumericColorConditionOperator::Gt);
        }
        other => panic!("expected numeric rule, got {other}"),
    }
    let bg = cfg.background_chart.as_ref().unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Line);

    let v = serde_json::to_value(&cfg).unwrap();
    assert_eq!(v["color"], "chart-blue");
    assert_eq!(v["colorRules"][0]["operator"], "gt");
    assert_eq!(v["backgroundChart"]["type"], "line");

    // The three new fields are optional and dropped when absent.
    let v = serde_json::to_value(ClickStackNumberBuilderChartConfig::default()).unwrap();
    assert!(v.get("color").is_none());
    assert!(v.get("colorRules").is_none());
    assert!(v.get("backgroundChart").is_none());
}

#[test]
fn clickstack_number_raw_sql_chart_config_color_round_trip() {
    let json = r#"{
        "configType": "sql",
        "connectionId": "conn-1",
        "displayType": "number",
        "sqlTemplate": "SELECT count() FROM t",
        "color": "chart-purple"
    }"#;
    let cfg: ClickStackNumberRawSqlChartConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.color, Some(ClickStackChartColor::Chart_purple));
    let v = serde_json::to_value(&cfg).unwrap();
    assert_eq!(v["color"], "chart-purple");

    let v = serde_json::to_value(ClickStackNumberRawSqlChartConfig::default()).unwrap();
    assert!(v.get("color").is_none());
}

#[test]
fn deserialize_clickstack_connection_with_null_prefix() {
    let json = r#"{
        "id": "507f1f77bcf86cd799439012",
        "name": "Production ClickHouse",
        "host": "https://clickhouse.example.com:8443",
        "username": "default",
        "hyperdxSettingPrefix": null,
        "isPrometheusEndpoint": false,
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let conn: ClickStackConnection = serde_json::from_str(json).unwrap();
    assert_eq!(conn.id.as_deref(), Some("507f1f77bcf86cd799439012"));
    assert_eq!(conn.name.as_deref(), Some("Production ClickHouse"));
    assert_eq!(
        conn.host.as_deref(),
        Some("https://clickhouse.example.com:8443")
    );
    assert_eq!(conn.username.as_deref(), Some("default"));
    assert_eq!(conn.hyperdx_setting_prefix, None);
    assert_eq!(conn.is_prometheus_endpoint, Some(false));
    assert!(conn.created_at.is_some());
    assert!(conn.updated_at.is_some());
}

#[test]
fn serialize_clickstack_create_connection_request_omits_none() {
    let req = ClickStackCreateConnectionRequest {
        name: "Production ClickHouse".to_string(),
        host: "https://clickhouse.example.com:8443".to_string(),
        username: "default".to_string(),
        password: Some("my-secret-password".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "Production ClickHouse");
    assert_eq!(v["host"], "https://clickhouse.example.com:8443");
    assert_eq!(v["username"], "default");
    assert_eq!(v["password"], "my-secret-password");
    assert!(
        v.get("hyperdxSettingPrefix").is_none(),
        "hyperdxSettingPrefix must be omitted when None"
    );
    assert!(
        v.get("isPrometheusEndpoint").is_none(),
        "isPrometheusEndpoint must be omitted when None"
    );
}

/// A ClickHouse-integration permission built from every property of
/// `ClickStackClickHouseCASLPermission` and its row-policy conditions.
fn clickhouse_casl_permission_json() -> serde_json::Value {
    serde_json::json!({
        "action": "read",
        "subject": "Source",
        "fields": ["Body", "ServiceName"],
        "inverted": false,
        "integration": "clickhouse",
        "conditions": {
            "connection": "507f1f77bcf86cd799439011",
            "from": {"databaseName": "default", "tableName": "otel_logs"},
            "where": "ServiceName = 'checkout'",
            "whereLanguage": "sql"
        }
    })
}

/// A MongoDB-integration permission built from every property of
/// `ClickStackMongoDBCASLPermission`, with both condition value shapes.
fn mongodb_casl_permission_json() -> serde_json::Value {
    serde_json::json!({
        "action": "manage",
        "subject": "Dashboard",
        "fields": ["name"],
        "inverted": true,
        "integration": "mongodb",
        "conditions": {
            "team": "team-1",
            "name": {"$regex": "^prod-", "$options": "i"}
        }
    })
}

#[test]
fn clickstack_casl_permission_request_routes_each_shape_to_its_variant() {
    let permission: ClickStackCASLPermission =
        serde_json::from_value(clickhouse_casl_permission_json()).unwrap();
    let ClickStackCASLPermission::ClickStackClickHouseCASLPermission(clickhouse) = &permission
    else {
        panic!("expected the ClickHouse variant, got {permission:?}");
    };
    assert_eq!(clickhouse.action, ClickStackCASLPermissionAction::Read);
    assert_eq!(clickhouse.subject, ClickStackCASLPermissionSubject::Source);
    assert_eq!(
        clickhouse.integration,
        ClickStackClickHouseCASLPermissionIntegration::Clickhouse
    );
    assert_eq!(
        clickhouse.fields.as_deref(),
        Some(&["Body".to_string(), "ServiceName".to_string()][..])
    );
    assert_eq!(clickhouse.inverted, Some(false));
    let conditions = clickhouse.conditions.as_ref().unwrap();
    assert_eq!(conditions.connection, "507f1f77bcf86cd799439011");
    assert_eq!(conditions.from.database_name, "default");
    assert_eq!(conditions.from.table_name, "otel_logs");
    assert_eq!(conditions.r#where, "ServiceName = 'checkout'");
    assert_eq!(
        conditions.where_language,
        ClickStackClickHouseCASLPermissionConditionsWherelanguage::Sql
    );

    let permission: ClickStackCASLPermission =
        serde_json::from_value(mongodb_casl_permission_json()).unwrap();
    let ClickStackCASLPermission::ClickStackMongoDBCASLPermission(mongodb) = &permission else {
        panic!("expected the MongoDB variant, got {permission:?}");
    };
    assert_eq!(mongodb.action, ClickStackCASLPermissionAction::Manage);
    assert_eq!(mongodb.subject, ClickStackCASLPermissionSubject::Dashboard);
    assert_eq!(
        mongodb.integration,
        ClickStackMongoDBCASLPermissionIntegration::Mongodb
    );
    assert_eq!(mongodb.inverted, Some(true));
    let conditions = mongodb.conditions.as_ref().unwrap();
    assert_eq!(
        conditions["team"],
        ClickStackMongoDBCASLPermissionConditionValue::String("team-1".to_string())
    );
    assert_eq!(
        conditions["name"],
        ClickStackMongoDBCASLPermissionConditionValue::Regex(
            ClickStackMongoDBCASLPermissionRegexCondition {
                regex: "^prod-".to_string(),
                options: Some("i".to_string()),
            }
        )
    );
}

#[test]
fn clickstack_casl_permission_response_routes_each_shape_to_its_variant() {
    let permission: ClickStackCASLPermissionResponse =
        serde_json::from_value(clickhouse_casl_permission_json()).unwrap();
    let ClickStackCASLPermissionResponse::ClickStackClickHouseCASLPermission(clickhouse) =
        &permission
    else {
        panic!("expected the ClickHouse variant, got {permission:?}");
    };
    assert_eq!(
        clickhouse.action,
        Some(ClickStackCASLPermissionAction::Read)
    );
    let conditions = clickhouse.conditions.as_ref().unwrap();
    assert_eq!(
        conditions.connection.as_deref(),
        Some("507f1f77bcf86cd799439011")
    );
    assert_eq!(
        conditions.from.as_ref().unwrap().table_name.as_deref(),
        Some("otel_logs")
    );
    assert_eq!(
        conditions.where_language,
        Some(ClickStackClickHouseCASLPermissionConditionsWherelanguage::Sql)
    );

    let permission: ClickStackCASLPermissionResponse =
        serde_json::from_value(mongodb_casl_permission_json()).unwrap();
    let ClickStackCASLPermissionResponse::ClickStackMongoDBCASLPermission(mongodb) = &permission
    else {
        panic!("expected the MongoDB variant, got {permission:?}");
    };
    assert_eq!(
        mongodb.integration,
        Some(ClickStackMongoDBCASLPermissionIntegration::Mongodb)
    );
    // Response condition values stay raw JSON, whatever shape they take.
    let conditions = mongodb.conditions.as_ref().unwrap();
    assert_eq!(conditions["team"], "team-1");
    assert_eq!(conditions["name"]["$regex"], "^prod-");
}

#[test]
fn clickstack_casl_permission_dispatch_ignores_the_payload_shape() {
    // Dispatch reads `integration` alone. Every response arm is all-`Option`,
    // so shape-based `untagged` matching would accept any object in the first
    // arm; a ClickHouse row policy must not become a MongoDB permission, and a
    // MongoDB query must not become a ClickHouse one.
    let mut clickhouse_conditions_as_mongodb = clickhouse_casl_permission_json();
    clickhouse_conditions_as_mongodb["integration"] = "mongodb".into();
    let permission: ClickStackCASLPermissionResponse =
        serde_json::from_value(clickhouse_conditions_as_mongodb.clone()).unwrap();
    assert!(
        matches!(
            permission,
            ClickStackCASLPermissionResponse::ClickStackMongoDBCASLPermission(_)
        ),
        "{permission:?}"
    );

    let mut mongodb_conditions_as_clickhouse = mongodb_casl_permission_json();
    mongodb_conditions_as_clickhouse["integration"] = "clickhouse".into();
    // `conditions.name` is an object where the ClickHouse shape has no such
    // key, and the ClickHouse `from` is absent: the request variant cannot be
    // built, so the request union keeps the payload verbatim in `Unknown`.
    let permission: ClickStackCASLPermission =
        serde_json::from_value(mongodb_conditions_as_clickhouse.clone()).unwrap();
    assert_eq!(
        permission,
        ClickStackCASLPermission::Unknown(mongodb_conditions_as_clickhouse)
    );

    // No integration, or one this crate does not know, never guesses a variant.
    for integration in [None, Some("postgres")] {
        let mut payload = mongodb_casl_permission_json();
        match integration {
            Some(integration) => payload["integration"] = integration.into(),
            None => {
                payload.as_object_mut().unwrap().remove("integration");
            }
        }
        let request: ClickStackCASLPermission = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(request, ClickStackCASLPermission::Unknown(payload.clone()));
        let response: ClickStackCASLPermissionResponse =
            serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(
            response,
            ClickStackCASLPermissionResponse::Unknown(payload.clone())
        );
        assert_eq!(serde_json::to_value(&response).unwrap(), payload);
    }
}

#[test]
fn clickstack_casl_permission_round_trips_in_both_positions() {
    for payload in [
        clickhouse_casl_permission_json(),
        mongodb_casl_permission_json(),
    ] {
        let request: ClickStackCASLPermission = serde_json::from_value(payload.clone()).unwrap();
        assert!(!matches!(request, ClickStackCASLPermission::Unknown(_)));
        assert_eq!(serde_json::to_value(&request).unwrap(), payload);
        let back: ClickStackCASLPermission =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(back, request);

        let response: ClickStackCASLPermissionResponse =
            serde_json::from_value(payload.clone()).unwrap();
        assert!(!matches!(
            response,
            ClickStackCASLPermissionResponse::Unknown(_)
        ));
        assert_eq!(serde_json::to_value(&response).unwrap(), payload);
    }
}

#[test]
fn clickstack_casl_permission_variant_defaults_round_trip_to_their_own_variant() {
    // Each variant's `integration` defaults to its own wire value, so a
    // default-built request variant still carries its discriminator.
    let clickhouse = ClickStackCASLPermission::ClickStackClickHouseCASLPermission(
        ClickStackClickHouseCASLPermission::default(),
    );
    let mongodb = ClickStackCASLPermission::ClickStackMongoDBCASLPermission(
        ClickStackMongoDBCASLPermission::default(),
    );
    for permission in [clickhouse, mongodb] {
        let json = serde_json::to_value(&permission).unwrap();
        assert_eq!(
            serde_json::from_value::<ClickStackCASLPermission>(json).unwrap(),
            permission
        );
    }
}

#[test]
fn clickstack_casl_permission_try_from_response_converts_every_integration() {
    for payload in [
        clickhouse_casl_permission_json(),
        mongodb_casl_permission_json(),
    ] {
        let response: ClickStackCASLPermissionResponse =
            serde_json::from_value(payload.clone()).unwrap();
        let request = ClickStackCASLPermission::try_from(response).unwrap();
        assert!(
            !matches!(request, ClickStackCASLPermission::Unknown(_)),
            "{request:?}"
        );
        assert_eq!(serde_json::to_value(&request).unwrap(), payload);
    }

    // The variant conversions round-trip on their own too.
    let response: ClickStackClickHouseCASLPermissionResponse =
        serde_json::from_value(clickhouse_casl_permission_json()).unwrap();
    let request = ClickStackClickHouseCASLPermission::try_from(response).unwrap();
    assert_eq!(
        request,
        serde_json::from_value::<ClickStackClickHouseCASLPermission>(
            clickhouse_casl_permission_json()
        )
        .unwrap()
    );
    let response: ClickStackMongoDBCASLPermissionResponse =
        serde_json::from_value(mongodb_casl_permission_json()).unwrap();
    let request = ClickStackMongoDBCASLPermission::try_from(response).unwrap();
    assert_eq!(
        request,
        serde_json::from_value::<ClickStackMongoDBCASLPermission>(mongodb_casl_permission_json())
            .unwrap()
    );
}

#[test]
fn clickstack_casl_permission_try_from_response_names_every_missing_required_field() {
    for (json, expected) in [
        (
            r#"{"integration":"clickhouse"}"#,
            &["action", "subject"][..],
        ),
        (
            r#"{"integration":"mongodb","action":"read"}"#,
            &["subject"][..],
        ),
        // Nested ClickHouse conditions name their own wire fields, unprefixed.
        (
            r#"{"integration":"clickhouse","action":"read","subject":"Source","conditions":{}}"#,
            &["connection", "from", "where", "whereLanguage"][..],
        ),
        (
            r#"{"integration":"clickhouse","action":"read","subject":"Source","conditions":
                {"connection":"c","where":"1","whereLanguage":"sql","from":{"databaseName":"d"}}}"#,
            &["tableName"][..],
        ),
    ] {
        let response: ClickStackCASLPermissionResponse = serde_json::from_str(json).unwrap();
        assert!(
            !matches!(response, ClickStackCASLPermissionResponse::Unknown(_)),
            "{json}"
        );
        let error = ClickStackCASLPermission::try_from(response).unwrap_err();
        assert_eq!(error.fields(), expected, "{json}");
    }

    // The variant conversions also require the discriminator itself.
    let error = ClickStackClickHouseCASLPermission::try_from(
        ClickStackClickHouseCASLPermissionResponse::default(),
    )
    .unwrap_err();
    assert_eq!(error.fields(), ["action", "integration", "subject"]);
    let error = ClickStackMongoDBCASLPermission::try_from(
        ClickStackMongoDBCASLPermissionResponse::default(),
    )
    .unwrap_err();
    assert_eq!(error.fields(), ["action", "integration", "subject"]);

    // An unmodelled integration passes through losslessly.
    let json = r#"{"integration":"postgres","action":"read"}"#;
    let response: ClickStackCASLPermissionResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackCASLPermission::try_from(response).unwrap();
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn clickstack_mongodb_casl_conditions_without_a_request_shape_are_never_dropped() {
    // A condition value that is neither a string nor a `$regex` object.
    let mut payload = mongodb_casl_permission_json();
    payload["conditions"]["count"] = serde_json::json!({"$gt": 3});

    // The variant conversion names `conditions` instead of dropping the value,
    // which would widen the permission.
    let response: ClickStackMongoDBCASLPermissionResponse =
        serde_json::from_value(payload.clone()).unwrap();
    let error = ClickStackMongoDBCASLPermission::try_from(response).unwrap_err();
    assert_eq!(error.fields(), ["conditions"]);

    // The union conversion writes the permission back verbatim, exactly as the
    // request union deserializes the same body.
    let response: ClickStackCASLPermissionResponse =
        serde_json::from_value(payload.clone()).unwrap();
    let request = ClickStackCASLPermission::try_from(response).unwrap();
    assert_eq!(request, ClickStackCASLPermission::Unknown(payload.clone()));
    assert_eq!(
        serde_json::from_value::<ClickStackCASLPermission>(payload.clone()).unwrap(),
        request
    );

    // A missing required field is still reported, alongside `conditions`.
    payload.as_object_mut().unwrap().remove("action");
    let response: ClickStackCASLPermissionResponse = serde_json::from_value(payload).unwrap();
    let error = ClickStackCASLPermission::try_from(response).unwrap_err();
    assert_eq!(error.fields(), ["action", "conditions"]);
}

#[test]
fn clickstack_casl_permission_variants_stay_strict_on_the_request_side() {
    // Each required field of the request variants is `T`: dropping one fails
    // the variant (and so routes the union payload to `Unknown`).
    for (payload, required) in [
        (clickhouse_casl_permission_json(), "action"),
        (clickhouse_casl_permission_json(), "subject"),
        (clickhouse_casl_permission_json(), "integration"),
        (mongodb_casl_permission_json(), "action"),
        (mongodb_casl_permission_json(), "subject"),
        (mongodb_casl_permission_json(), "integration"),
    ] {
        let mut payload = payload;
        payload.as_object_mut().unwrap().remove(required);
        let integration = payload.get("integration").cloned();
        let accepted = if integration.is_none() {
            serde_json::from_value::<ClickStackClickHouseCASLPermission>(payload.clone()).is_ok()
                || serde_json::from_value::<ClickStackMongoDBCASLPermission>(payload.clone())
                    .is_ok()
        } else if integration == Some("clickhouse".into()) {
            serde_json::from_value::<ClickStackClickHouseCASLPermission>(payload.clone()).is_ok()
        } else {
            serde_json::from_value::<ClickStackMongoDBCASLPermission>(payload.clone()).is_ok()
        };
        assert!(!accepted, "accepted a permission without `{required}`");
        assert!(matches!(
            serde_json::from_value::<ClickStackCASLPermission>(payload).unwrap(),
            ClickStackCASLPermission::Unknown(_)
        ));
    }
    for required in ["connection", "from", "where", "whereLanguage"] {
        let mut conditions = clickhouse_casl_permission_json()["conditions"].clone();
        conditions.as_object_mut().unwrap().remove(required);
        assert!(
            serde_json::from_value::<ClickStackClickHouseCASLPermissionConditions>(conditions)
                .is_err(),
            "accepted ClickHouse conditions without `{required}`"
        );
    }
    for required in ["databaseName", "tableName"] {
        let mut from = clickhouse_casl_permission_json()["conditions"]["from"].clone();
        from.as_object_mut().unwrap().remove(required);
        assert!(
            serde_json::from_value::<ClickStackClickHouseCASLPermissionConditionsFrom>(from)
                .is_err(),
            "accepted a ClickHouse `from` without `{required}`"
        );
    }
    // A MongoDB condition value is a string or a `$regex` object, nothing else.
    for value in [
        serde_json::json!({"$options": "i"}),
        serde_json::json!(3),
        serde_json::json!(["a"]),
    ] {
        assert!(
            serde_json::from_value::<ClickStackMongoDBCASLPermissionConditions>(
                serde_json::json!({ "name": value })
            )
            .is_err(),
            "accepted MongoDB condition value {value}"
        );
    }
}

#[test]
fn clickstack_casl_permission_responses_tolerate_missing_and_null_fields() {
    assert_eq!(
        serde_json::from_str::<ClickStackClickHouseCASLPermissionResponse>("{}").unwrap(),
        ClickStackClickHouseCASLPermissionResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackClickHouseCASLPermissionResponse>(
            r#"{"action":null,"subject":null,"fields":null,"inverted":null,"integration":null,"conditions":null}"#
        )
        .unwrap(),
        ClickStackClickHouseCASLPermissionResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackClickHouseCASLPermissionConditionsResponse>(
            r#"{"connection":null,"from":null,"where":null,"whereLanguage":null}"#
        )
        .unwrap(),
        ClickStackClickHouseCASLPermissionConditionsResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackClickHouseCASLPermissionConditionsFromResponse>(
            r#"{"databaseName":null,"tableName":null}"#
        )
        .unwrap(),
        ClickStackClickHouseCASLPermissionConditionsFromResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackMongoDBCASLPermissionResponse>(
            r#"{"action":null,"subject":null,"fields":null,"inverted":null,"integration":null,"conditions":null}"#
        )
        .unwrap(),
        ClickStackMongoDBCASLPermissionResponse::default()
    );

    // A dispatched permission with only its discriminator keeps that variant,
    // and serializes without inventing the absent fields.
    let permission: ClickStackCASLPermissionResponse =
        serde_json::from_str(r#"{"integration":"clickhouse","action":null}"#).unwrap();
    assert_eq!(
        permission,
        ClickStackCASLPermissionResponse::ClickStackClickHouseCASLPermission(
            ClickStackClickHouseCASLPermissionResponse {
                integration: Some(ClickStackClickHouseCASLPermissionIntegration::Clickhouse),
                ..Default::default()
            }
        )
    );
    assert_eq!(
        serde_json::to_value(&permission).unwrap(),
        serde_json::json!({"integration": "clickhouse"})
    );
}

#[test]
fn deserialize_clickstack_role_with_both_permission_kinds() {
    let json = serde_json::json!({
        "id": "role-1",
        "name": "Deploy Bot",
        "description": "Manages dashboards via Terraform",
        "isPredefined": false,
        "permissions": [mongodb_casl_permission_json(), clickhouse_casl_permission_json()]
    });
    let role: ClickStackRole = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.name.as_deref(), Some("Deploy Bot"));
    assert_eq!(role.is_predefined, Some(false));
    let permissions = role.permissions.as_deref().expect("permissions present");
    assert!(matches!(
        permissions,
        [
            ClickStackCASLPermissionResponse::ClickStackMongoDBCASLPermission(_),
            ClickStackCASLPermissionResponse::ClickStackClickHouseCASLPermission(_),
        ]
    ));

    // Optional role fields are omitted when absent, and the role round-trips.
    let sparse = ClickStackRole {
        id: Some("role-1".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&sparse).unwrap();
    assert!(v.get("description").is_none());
    assert!(v.get("createdAt").is_none());
    assert!(v.get("permissions").is_none());
    assert_eq!(serde_json::to_value(&role).unwrap(), json);
    let back: ClickStackRole = serde_json::from_value(json).unwrap();
    assert_eq!(back, role);
}

#[test]
fn serialize_clickstack_casl_permission_omits_none() {
    let perm = ClickStackCASLPermission::ClickStackMongoDBCASLPermission(
        ClickStackMongoDBCASLPermission {
            action: ClickStackCASLPermissionAction::Read,
            subject: ClickStackCASLPermissionSubject::Dashboard,
            ..Default::default()
        },
    );
    assert_eq!(
        serde_json::to_value(&perm).unwrap(),
        serde_json::json!({"action": "read", "subject": "Dashboard", "integration": "mongodb"})
    );
    let perm = ClickStackCASLPermission::ClickStackClickHouseCASLPermission(
        ClickStackClickHouseCASLPermission {
            action: ClickStackCASLPermissionAction::Read,
            subject: ClickStackCASLPermissionSubject::All,
            ..Default::default()
        },
    );
    assert_eq!(
        serde_json::to_value(&perm).unwrap(),
        serde_json::json!({"action": "read", "subject": "all", "integration": "clickhouse"})
    );
}

#[test]
fn serialize_clickstack_create_role_request_omits_none() {
    let req = ClickStackCreateRoleRequest {
        name: "Deploy Bot".to_string(),
        permissions: vec![serde_json::from_value(mongodb_casl_permission_json()).unwrap()],
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "Deploy Bot");
    assert_eq!(v["permissions"][0], mongodb_casl_permission_json());
    assert!(v.get("description").is_none());
}

#[test]
fn serialize_clickstack_update_role_request_omits_none() {
    let req = ClickStackUpdateRoleRequest {
        permissions: vec![serde_json::from_value(clickhouse_casl_permission_json()).unwrap()],
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["permissions"][0], clickhouse_casl_permission_json());
    assert!(v.get("name").is_none());
    assert!(v.get("description").is_none());
}

#[test]
fn deserialize_clickstack_saved_search_full_round_trip() {
    let json = r#"{
        "id": "507f1f77bcf86cd799439011",
        "name": "Production Errors",
        "sourceId": "507f1f77bcf86cd799439012",
        "select": "Timestamp, ServiceName, Body",
        "where": "SeverityText:ERROR",
        "whereLanguage": "lucene",
        "orderBy": "Timestamp DESC",
        "tags": ["production", "errors"],
        "filters": [
            {"type": "sql", "condition": "ServiceName IN ('checkout', 'payments')"}
        ],
        "teamId": "507f1f77bcf86cd799439013",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let search: ClickStackSavedSearch = serde_json::from_str(json).unwrap();
    assert_eq!(search.id.as_deref(), Some("507f1f77bcf86cd799439011"));
    assert_eq!(search.name.as_deref(), Some("Production Errors"));
    assert_eq!(
        search.source_id.as_deref(),
        Some("507f1f77bcf86cd799439012")
    );
    assert_eq!(
        search.where_language,
        Some(ClickStackSavedSearchWherelanguage::Lucene)
    );
    let filters = search.filters.clone().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(
        filters[0].r#type,
        Some(ClickStackSavedSearchFilterType::Sql)
    );
    assert_eq!(
        filters[0].condition.as_deref(),
        Some("ServiceName IN ('checkout', 'payments')")
    );
    assert_eq!(search.team_id, Some("507f1f77bcf86cd799439013".to_string()));

    let v = serde_json::to_value(&search).unwrap();
    assert_eq!(v["whereLanguage"], "lucene");
    assert_eq!(v["filters"][0]["type"], "sql");
    assert_eq!(v["orderBy"], "Timestamp DESC");
    assert_eq!(v["tags"][0], "production");

    let round: ClickStackSavedSearch = serde_json::from_value(v).unwrap();
    assert_eq!(round, search);
}

#[test]
fn serialize_clickstack_saved_search_input_minimal_omits_optionals() {
    let input = ClickStackSavedSearchInput {
        name: "Production Errors".to_string(),
        source_id: "507f1f77bcf86cd799439012".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["name"], "Production Errors");
    assert_eq!(v["sourceId"], "507f1f77bcf86cd799439012");
    assert!(v.get("select").is_none());
    assert!(v.get("where").is_none());
    assert!(v.get("whereLanguage").is_none());
    assert!(v.get("orderBy").is_none());
    assert!(v.get("tags").is_none());
    assert!(v.get("filters").is_none());
}

#[test]
fn clickstack_webhook_input_headers_and_query_params_round_trip() {
    let json = r#"{
        "name": "Production Alerts",
        "service": "incidentio",
        "url": "https://api.incident.io/v2/alert_events/http/xyz",
        "description": "Sends critical alerts",
        "body": "{\"alert\": \"{{title}}\"}",
        "headers": {"Authorization": "Bearer token", "X-Custom": "value"},
        "queryParams": {"source": "clickstack", "env": "prod"}
    }"#;
    let input: ClickStackWebhookInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.name, "Production Alerts");
    assert_eq!(input.service, ClickStackWebhookInputService::Incidentio);
    assert_eq!(
        input.url,
        "https://api.incident.io/v2/alert_events/http/xyz"
    );

    let headers = input.headers.clone().unwrap();
    assert_eq!(
        headers.get("Authorization"),
        Some(&"Bearer token".to_string())
    );
    assert_eq!(headers.get("X-Custom"), Some(&"value".to_string()));
    let query_params = input.query_params.clone().unwrap();
    assert_eq!(query_params.get("source"), Some(&"clickstack".to_string()));
    assert_eq!(query_params.get("env"), Some(&"prod".to_string()));

    // Maps serialize back as JSON objects, and the enum keeps its wire value.
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["service"], "incidentio");
    assert!(v["headers"].is_object());
    assert_eq!(v["headers"]["Authorization"], "Bearer token");
    assert!(v["queryParams"].is_object());
    assert_eq!(v["queryParams"]["source"], "clickstack");

    let back: ClickStackWebhookInput = serde_json::from_value(v).unwrap();
    assert_eq!(back, input);
}

#[test]
fn serialize_clickstack_webhook_input_minimal_omits_optionals() {
    let input = ClickStackWebhookInput {
        name: "Slack Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["name"], "Slack Alerts");
    assert_eq!(v["service"], "slack");
    assert_eq!(v["url"], "https://hooks.slack.com/services/T/B/X");
    assert!(v.get("description").is_none());
    assert!(v.get("body").is_none());
    assert!(v.get("headers").is_none());
    assert!(v.get("queryParams").is_none());
}

#[test]
fn deserialize_clickstack_webhook_dispatches_slack() {
    let json = r#"{
        "id": "webhook-1",
        "name": "Slack Alerts",
        "service": "slack",
        "url": "https://hooks.slack.com/services/T/B/X",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackSlackWebhook(s) => {
            assert_eq!(s.id.as_deref(), Some("webhook-1"));
            assert_eq!(s.name.as_deref(), Some("Slack Alerts"));
        }
        other => panic!("expected Slack webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_incidentio() {
    let json = r#"{
        "id": "webhook-2",
        "name": "Incident Alerts",
        "service": "incidentio",
        "url": "https://api.incident.io/v2/alert_events/http/abc",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackIncidentIOWebhook(i) => {
            assert_eq!(i.id.as_deref(), Some("webhook-2"));
            assert_eq!(i.name.as_deref(), Some("Incident Alerts"));
        }
        other => panic!("expected IncidentIO webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_generic_preserves_body() {
    let json = r#"{
        "id": "webhook-3",
        "name": "Generic Alerts",
        "service": "generic",
        "url": "https://example.com/hook",
        "body": "{\"text\": \"{{ message }}\"}",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackGenericWebhook(g) => {
            assert_eq!(g.id.as_deref(), Some("webhook-3"));
            assert_eq!(g.body.as_deref(), Some("{\"text\": \"{{ message }}\"}"));
        }
        other => panic!("expected Generic webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_slack_api() {
    let json = r#"{
        "id": "webhook-4",
        "name": "Slack API Alerts",
        "service": "slack_api",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackSlackAPIWebhook(s) => {
            assert_eq!(s.id.as_deref(), Some("webhook-4"));
            assert_eq!(s.name.as_deref(), Some("Slack API Alerts"));
        }
        other => panic!("expected SlackAPI webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_pagerduty_api() {
    let json = r#"{
        "id": "webhook-5",
        "name": "PagerDuty Alerts",
        "service": "pagerduty_api",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackPagerDutyAPIWebhook(p) => {
            assert_eq!(p.id.as_deref(), Some("webhook-5"));
            assert_eq!(p.name.as_deref(), Some("PagerDuty Alerts"));
        }
        other => panic!("expected PagerDutyAPI webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_unknown_service_round_trips() {
    let json = r#"{
        "id": "webhook-6",
        "name": "Future Alerts",
        "service": "future_service",
        "extraField": "kept",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    assert_unknown_variant_round_trips(json, |w: &ClickStackWebhook| {
        matches!(w, ClickStackWebhook::Unknown(_))
    });
}

#[test]
fn clickstack_source_expression_fields_missing_or_null_are_none() {
    // `orderByExpression` (log, trace), the log-only OpenTelemetry
    // `eventNameExpression` and `severityNumberExpression`,
    // `resourceAttributesExpression` (session), and the trace-only display,
    // sampling and span-link expressions are optional in the spec: absent or `null` lands as `None`
    // on the response variants and is omitted from the request variants.
    let log = serde_json::json!({
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
    });
    let trace = serde_json::json!({
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "from": {"databaseName": "default", "tableName": "traces"},
        "parentSpanIdExpression": "ParentSpanId",
        "spanIdExpression": "SpanId",
        "spanKindExpression": "SpanKind",
        "spanNameExpression": "SpanName",
        "timestampValueExpression": "Timestamp",
        "traceIdExpression": "TraceId",
    });
    let session = serde_json::json!({
        "kind": "session",
        "name": "sessions",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "sessions"},
        "traceSourceId": "trace-1",
    });
    let with_nulls = |base: &serde_json::Value, keys: &[&str]| {
        let mut v = base.clone();
        for key in keys {
            v[*key] = serde_json::Value::Null;
        }
        v
    };
    let trace_keys = [
        "displayedTimestampValueExpression",
        "orderByExpression",
        "sampleRateExpression",
        "spanLinksValueExpression",
    ];

    let log_keys = [
        "eventNameExpression",
        "orderByExpression",
        "severityNumberExpression",
    ];

    for payload in [log.clone(), with_nulls(&log, &log_keys)] {
        let response: ClickStackLogSourceResponse = serde_json::from_value(payload).unwrap();
        assert_eq!(response.event_name_expression, None);
        assert_eq!(response.order_by_expression, None);
        assert_eq!(response.severity_number_expression, None);
        let request = ClickStackLogSource::try_from(response).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), log);
    }
    for payload in [trace.clone(), with_nulls(&trace, &trace_keys)] {
        let response: ClickStackTraceSourceResponse = serde_json::from_value(payload).unwrap();
        assert_eq!(response.displayed_timestamp_value_expression, None);
        assert_eq!(response.order_by_expression, None);
        assert_eq!(response.sample_rate_expression, None);
        assert_eq!(response.span_links_value_expression, None);
        let request = ClickStackTraceSource::try_from(response).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), trace);
    }
    for payload in [
        session.clone(),
        with_nulls(&session, &["resourceAttributesExpression"]),
    ] {
        let response: ClickStackSessionSourceResponse = serde_json::from_value(payload).unwrap();
        assert_eq!(response.resource_attributes_expression, None);
        let request = ClickStackSessionSource::try_from(response).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), session);
    }
}

#[test]
fn clickstack_source_expression_fields_serialize_with_their_wire_names() {
    let log = ClickStackLogSource {
        event_name_expression: Some("EventName".to_string()),
        order_by_expression: Some("Timestamp DESC".to_string()),
        severity_number_expression: Some("SeverityNumber".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&log).unwrap();
    assert_eq!(v["eventNameExpression"], "EventName");
    assert_eq!(v["orderByExpression"], "Timestamp DESC");
    assert_eq!(v["severityNumberExpression"], "SeverityNumber");

    let session = ClickStackSessionSource {
        resource_attributes_expression: Some("ResourceAttributes".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&session).unwrap();
    assert_eq!(v["resourceAttributesExpression"], "ResourceAttributes");

    let trace = ClickStackTraceSource {
        displayed_timestamp_value_expression: Some("Timestamp".to_string()),
        order_by_expression: Some("Timestamp DESC".to_string()),
        sample_rate_expression: Some("SampleRate".to_string()),
        span_links_value_expression: Some("Links".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&trace).unwrap();
    assert_eq!(v["displayedTimestampValueExpression"], "Timestamp");
    assert_eq!(v["orderByExpression"], "Timestamp DESC");
    assert_eq!(v["sampleRateExpression"], "SampleRate");
    assert_eq!(v["spanLinksValueExpression"], "Links");
}

#[test]
fn clickstack_log_source_otel_expressions_write_back() {
    let wire = serde_json::json!({
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "Timestamp",
        "eventNameExpression": "EventName",
        "severityNumberExpression": "SeverityNumber",
        "severityTextExpression": "SeverityText",
    });
    let response: ClickStackLogSourceResponse = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(response.event_name_expression.as_deref(), Some("EventName"));
    assert_eq!(
        response.severity_number_expression.as_deref(),
        Some("SeverityNumber")
    );
    let request = ClickStackLogSource::try_from(response).unwrap();
    assert_eq!(request.event_name_expression.as_deref(), Some("EventName"));
    assert_eq!(
        request.severity_number_expression.as_deref(),
        Some("SeverityNumber")
    );
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
}

#[test]
fn clickstack_validate_webhook_response_null_fields_are_none() {
    let null: ClickStackValidateWebhookResponse =
        serde_json::from_str(r#"{"valid": null, "errors": null, "normalized": null}"#).unwrap();
    assert_eq!(null, ClickStackValidateWebhookResponse::default());
    // Absent fields are omitted on serialize, not emitted as `null`.
    assert_eq!(serde_json::to_value(&null).unwrap(), serde_json::json!({}));

    let item: ClickStackValidateWebhookResponseErrorsItem =
        serde_json::from_str(r#"{"path": null, "message": null}"#).unwrap();
    assert_eq!(item, ClickStackValidateWebhookResponseErrorsItem::default());

    let resp: ClickStackValidateWebhookResponse = serde_json::from_str(
        r#"{"valid": false, "errors": [{"path": "", "message": "Top-level error"}],
            "normalized": {"name": "hook"}}"#,
    )
    .unwrap();
    assert_eq!(resp.valid, Some(false));
    let errors = resp.errors.as_ref().expect("errors should populate");
    assert_eq!(errors[0].path.as_deref(), Some(""));
    assert_eq!(errors[0].message.as_deref(), Some("Top-level error"));
    // `normalized` is a free-form Value; its payload is preserved verbatim.
    assert_eq!(resp.normalized.as_ref().unwrap()["name"], "hook");
}

#[test]
fn clickstack_validate_dashboard_response_round_trip() {
    let json = r#"{
        "valid": false,
        "errors": [
            {"path": "tiles.0.config", "message": "Required"},
            {"path": "", "message": "Top-level error"}
        ],
        "normalized": {"name": "My Dashboard", "tiles": [{"id": "t1"}]}
    }"#;
    let resp: ClickStackValidateDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.valid, Some(false));
    let errors = resp.errors.as_ref().expect("errors should populate");
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].path.as_deref(), Some("tiles.0.config"));
    assert_eq!(errors[0].message.as_deref(), Some("Required"));
    assert_eq!(errors[1].path.as_deref(), Some(""));

    // `normalized` is a free-form Value; its arbitrary payload is preserved.
    let normalized = resp.normalized.as_ref().unwrap();
    assert_eq!(normalized["name"], "My Dashboard");
    assert_eq!(normalized["tiles"][0]["id"], "t1");

    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["valid"], false);
    assert_eq!(v["errors"][0]["path"], "tiles.0.config");
    assert_eq!(v["normalized"]["name"], "My Dashboard");

    let back: ClickStackValidateDashboardResponse = serde_json::from_value(v).unwrap();
    assert_eq!(back, resp);
}

#[test]
fn clickstack_validate_dashboard_response_valid_null_normalized() {
    let json = r#"{"valid": true, "errors": [], "normalized": null}"#;
    let resp: ClickStackValidateDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.valid, Some(true));
    assert_eq!(resp.errors.as_deref(), Some(&[][..]));
    assert_eq!(resp.normalized, None);

    // An explicit `null` lands as `None` and, like every absent response
    // field, is omitted on the way out rather than re-emitted as `null`.
    let v = serde_json::to_value(&resp).unwrap();
    assert!(
        v.get("normalized").is_none(),
        "absent response fields must be omitted from --json output"
    );
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_time() {
    // The manual `type`-keyed dispatch routes "time" to the time-series variant.
    let json = r#"{
        "aggFn": "count",
        "groupBy": ["service"],
        "sourceId": "src-1",
        "type": "time",
        "where": "x = 1",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackTimeChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackTimeChartSeriesType::Time);
            assert_eq!(s.agg_fn, ClickStackTimeChartSeriesAggfn::Count);
        }
        other => panic!("expected time variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_table() {
    // Regression: Time and Table share every required field (aggFn, groupBy,
    // sourceId, type, where, whereLanguage), so before manual discriminator
    // dispatch a `type: "table"` payload greedily resolved to the Time variant
    // and silently dropped table-only fields like `sortOrder`. The union now
    // routes on the `type` discriminator and preserves `sortOrder`.
    let json = r#"{
        "aggFn": "count",
        "groupBy": ["service"],
        "sortOrder": "desc",
        "sourceId": "src-2",
        "type": "table",
        "where": "y = 2",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackTableChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackTableChartSeriesType::Table);
            assert_eq!(
                s.sort_order,
                Some(ClickStackTableChartSeriesSortorder::Desc)
            );
        }
        other => panic!("expected table variant, got {other}"),
    }
    // The table-only `sortOrder` survives the round-trip through the union.
    let v = serde_json::to_value(&series).unwrap();
    assert_eq!(v["type"], "table");
    assert_eq!(v["sortOrder"], "desc");
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_number() {
    // The manual `type`-keyed dispatch routes "number" to the number-series variant.
    let json = r#"{
        "aggFn": "count",
        "sourceId": "src-3",
        "type": "number",
        "where": "z = 3",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackNumberChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackNumberChartSeriesType::Number);
        }
        other => panic!("expected number variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_search() {
    // The manual `type`-keyed dispatch routes "search" to the search-series variant.
    let json = r#"{
        "fields": ["message"],
        "sourceId": "src-4",
        "type": "search",
        "where": "w = 4",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackSearchChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackSearchChartSeriesType::Search);
            assert_eq!(s.fields, vec!["message".to_string()]);
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_markdown() {
    // The manual `type`-keyed dispatch routes "markdown" to the markdown-series variant.
    let json = r##"{"content": "# Title", "type": "markdown"}"##;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackMarkdownChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackMarkdownChartSeriesType::Markdown);
            assert_eq!(s.content, "# Title");
        }
        other => panic!("expected markdown variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_unknown_type_round_trip() {
    // An unrecognized `type` discriminator falls back to the Unknown variant,
    // which stores the raw JSON so it round-trips faithfully.
    let json = r#"{"type":"heatmap","payload":{"nested":[1,2,3]}}"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::Unknown(v) => {
            assert_eq!(v["type"], "heatmap");
            assert_eq!(v["payload"]["nested"][2], 3);
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&series).unwrap(), expected);
}

#[test]
fn saved_filter_and_formula_writeback_reports_required_wire_fields() {
    let sql: ClickStackSavedFilterValueResponse =
        serde_json::from_value(serde_json::json!({"condition": "x = 1"})).unwrap();
    let request = ClickStackSavedFilterValue::try_from(sql).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"condition": "x = 1"})
    );
    let variable = serde_json::json!({"type": "variable", "name": "service", "values": ["api"]});
    let response: ClickStackSavedFilterValueResponse =
        serde_json::from_value(variable.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(ClickStackSavedFilterValue::try_from(response).unwrap()).unwrap(),
        variable
    );
    assert_eq!(
        ClickStackSqlSavedFilterValue::try_from(ClickStackSqlSavedFilterValueResponse::default())
            .unwrap_err()
            .fields(),
        &["condition"]
    );
    assert_eq!(
        ClickStackVariableSavedFilterValue::try_from(
            ClickStackVariableSavedFilterValueResponse::default()
        )
        .unwrap_err()
        .fields(),
        &["type", "name", "values"]
    );
    assert_eq!(
        ClickStackFormula::try_from(ClickStackFormulaResponse::default())
            .unwrap_err()
            .fields(),
        &["expression"]
    );
    let formula = ClickStackFormula::try_from(ClickStackFormulaResponse {
        expression: Some("A / B".into()),
        alias: Some("Rate".into()),
        number_format: None,
    })
    .unwrap();
    assert_eq!(formula.expression, "A / B");
    assert_eq!(formula.alias.as_deref(), Some("Rate"));
    let invalid = ClickStackFormulaResponse {
        expression: Some("A".into()),
        number_format: Some(ClickStackNumberFormatResponse::default()),
        ..Default::default()
    };
    assert!(
        ClickStackFormula::try_from(invalid)
            .unwrap_err()
            .fields()
            .contains(&"currencySymbol")
    );
}

#[test]
fn new_clickstack_channels_and_chart_fields_round_trip() {
    let request = ClickStackCreateAlertRequest {
        interval: ClickStackCreateAlertRequestInterval::Value30s,
        channel: Some(ClickStackAlertChannel::default()),
        channels: Some(vec![ClickStackAlertChannel::default()]),
        ..Default::default()
    };
    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(wire["channels"].as_array().unwrap().len(), 1);
    assert_eq!(wire["interval"], "30s");
    assert!(wire.get("channel").is_some());
    assert_eq!(
        serde_json::from_value::<ClickStackCreateAlertRequest>(wire).unwrap(),
        request
    );
    let chart: ClickStackLineBuilderChartConfigResponse =
        serde_json::from_value(serde_json::json!({
            "formulas": [{"expression": "A / B", "alias": "Rate"}], "seriesLimit": 15,
            "showOperandSeries": false
        }))
        .unwrap();
    assert_eq!(chart.series_limit, Some(15));
    assert_eq!(chart.show_operand_series, Some(false));
    assert_eq!(
        chart.formulas.unwrap()[0].expression.as_deref(),
        Some("A / B")
    );
}

#[test]
fn clickstack_nested_writeback_preserves_number_format_and_source_expressions() {
    let formula = serde_json::json!({
        "expression": "A / B", "alias": "Success rate", "numberFormat": {
            "average": false, "currencySymbol": "$", "decimalBytes": true,
            "factor": 100.0, "mantissa": 2, "numericUnit": "bytes_si",
            "output": "percent", "thousandSeparated": true, "unit": "%"
        }
    });
    let response: ClickStackFormulaResponse = serde_json::from_value(formula.clone()).unwrap();
    let request = ClickStackFormula::try_from(response).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), formula);

    let log = serde_json::json!({
        "kind": "log", "connection": "connection-1", "name": "Application logs",
        "defaultTableSelectExpression": "*", "timestampValueExpression": "Timestamp",
        "from": {"databaseName": "default", "tableName": "otel_logs"},
        "serviceVersionExpression": "ResourceAttributes['service.version']",
        "filterSettings": {
            "databaseName": "default", "tableName": "service_names",
            "columns": [{"name": "ServiceName", "label": "Service", "allowAll": true,
                "valueExpression": "lower(service_name)"}]
        }
    });
    let response: ClickStackSourceResponse = serde_json::from_value(log.clone()).unwrap();
    let request = ClickStackSource::try_from(response).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), log);

    let mut trace = log;
    trace["kind"] = "trace".into();
    trace["name"] = "Application traces".into();
    trace["from"]["tableName"] = "otel_traces".into();
    trace["durationExpression"] = "Duration".into();
    trace["durationPrecision"] = 9.into();
    trace["parentSpanIdExpression"] = "ParentSpanId".into();
    trace["spanIdExpression"] = "SpanId".into();
    trace["spanKindExpression"] = "SpanKind".into();
    trace["spanNameExpression"] = "SpanName".into();
    trace["traceIdExpression"] = "TraceId".into();
    let response: ClickStackSourceResponse = serde_json::from_value(trace.clone()).unwrap();
    let request = ClickStackSource::try_from(response).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), trace);
}

/// One spec-shaped payload per `ClickStackFilter` branch, each built from every
/// property of its `*FilterInput` schema plus the `id` the full schema adds.
fn clickstack_filter_kind_payloads() -> [(&'static str, serde_json::Value); 4] {
    [
        (
            "QUERY_EXPRESSION",
            serde_json::json!({
                "type": "QUERY_EXPRESSION", "id": "f-1", "name": "Environment",
                "expression": "environment", "sourceId": "src-1",
                "sourceMetricType": "exponential histogram", "where": "ServiceName:api",
                "whereLanguage": "lucene", "appliesToSourceIds": ["src-1"],
                "isBroadcastEnabled": true, "isVariableEnabled": true,
                "variableName": "environment", "minSelections": 1,
                "isGlobalRequirement": true, "maxSelections": 1
            }),
        ),
        (
            "STATIC_LIST",
            serde_json::json!({
                "type": "STATIC_LIST", "id": "f-2", "name": "Environment",
                "options": ["prod", "staging"], "isBroadcastEnabled": false,
                "isVariableEnabled": true, "variableName": "environment",
                "minSelections": 0, "isGlobalRequirement": false, "maxSelections": 1
            }),
        ),
        (
            "PROMETHEUS_LABEL",
            serde_json::json!({
                "type": "PROMETHEUS_LABEL", "id": "f-3", "name": "Pod",
                "sourceId": "src-2", "label": "pod", "match": "up{job=\"api\"}",
                "isBroadcastEnabled": false, "isVariableEnabled": true,
                "variableName": "pod", "minSelections": 1,
                "isGlobalRequirement": false, "maxSelections": 1
            }),
        ),
        (
            "SOURCE_FILTER",
            serde_json::json!({
                "type": "SOURCE_FILTER", "id": "f-4", "name": "Cluster",
                "sourceId": "src-3", "expression": "cluster",
                "appliesToSourceIds": ["src-3"], "isBroadcastEnabled": true,
                "isVariableEnabled": true, "variableName": "cluster",
                "minSelections": 0, "isGlobalRequirement": true, "maxSelections": 1
            }),
        ),
    ]
}

#[test]
fn clickstack_filter_unions_route_each_kind_to_its_variant() {
    // Dispatch reads `type` alone, so each spec-shaped payload must land in its
    // own arm of the update-request, create-request and response unions — and
    // never in the first arm that happens to deserialize — then serialize back
    // unchanged.
    for (kind, payload) in clickstack_filter_kind_payloads() {
        let filter: ClickStackFilter = serde_json::from_value(payload.clone()).unwrap();
        let matched = match (kind, &filter) {
            ("QUERY_EXPRESSION", ClickStackFilter::ClickStackQueryExpressionFilter(f)) => {
                assert_eq!(f.id, "f-1");
                assert_eq!(f.expression, "environment");
                assert_eq!(
                    f.source_metric_type,
                    Some(
                        ClickStackQueryExpressionFilterInputSourcemetrictype::Exponential_histogram
                    )
                );
                assert_eq!(
                    f.where_language,
                    Some(ClickStackQueryExpressionFilterInputWherelanguage::Lucene)
                );
                assert_eq!(f.min_selections, Some(ClickStackFilterMinSelections::One));
                assert_eq!(f.max_selections, Some(ClickStackFilterMaxSelections::One));
                true
            }
            ("STATIC_LIST", ClickStackFilter::ClickStackStaticListFilter(f)) => {
                assert_eq!(f.id, "f-2");
                assert_eq!(f.options, ["prod", "staging"]);
                assert_eq!(f.min_selections, Some(ClickStackFilterMinSelections::Zero));
                true
            }
            ("PROMETHEUS_LABEL", ClickStackFilter::ClickStackPrometheusLabelFilter(f)) => {
                assert_eq!(f.id, "f-3");
                assert_eq!(f.label, "pod");
                assert_eq!(f.r#match.as_deref(), Some("up{job=\"api\"}"));
                true
            }
            ("SOURCE_FILTER", ClickStackFilter::ClickStackSourceFilter(f)) => {
                assert_eq!(f.id, "f-4");
                assert_eq!(f.variable_name, "cluster");
                assert_eq!(f.is_global_requirement, Some(true));
                true
            }
            _ => false,
        };
        assert!(matched, "{kind} routed to {filter:?}");
        assert_eq!(serde_json::to_value(&filter).unwrap(), payload);

        let mut input_payload = payload.clone();
        input_payload.as_object_mut().unwrap().remove("id");
        let input: ClickStackFilterInput = serde_json::from_value(input_payload.clone()).unwrap();
        let matched = matches!(
            (kind, &input),
            (
                "QUERY_EXPRESSION",
                ClickStackFilterInput::ClickStackQueryExpressionFilterInput(_)
            ) | (
                "STATIC_LIST",
                ClickStackFilterInput::ClickStackStaticListFilterInput(_)
            ) | (
                "PROMETHEUS_LABEL",
                ClickStackFilterInput::ClickStackPrometheusLabelFilterInput(_)
            ) | (
                "SOURCE_FILTER",
                ClickStackFilterInput::ClickStackSourceFilterInput(_)
            )
        );
        assert!(matched, "{kind} input routed to {input:?}");
        assert_eq!(serde_json::to_value(&input).unwrap(), input_payload);

        let response: ClickStackFilterResponse = serde_json::from_value(payload.clone()).unwrap();
        let matched = match (kind, &response) {
            ("QUERY_EXPRESSION", ClickStackFilterResponse::ClickStackQueryExpressionFilter(f)) => {
                f.expression.as_deref() == Some("environment")
            }
            ("STATIC_LIST", ClickStackFilterResponse::ClickStackStaticListFilter(f)) => {
                f.options.as_deref() == Some(&["prod".to_string(), "staging".to_string()][..])
            }
            ("PROMETHEUS_LABEL", ClickStackFilterResponse::ClickStackPrometheusLabelFilter(f)) => {
                f.label.as_deref() == Some("pod")
            }
            ("SOURCE_FILTER", ClickStackFilterResponse::ClickStackSourceFilter(f)) => {
                f.variable_name.as_deref() == Some("cluster")
            }
            _ => false,
        };
        assert!(matched, "{kind} response routed to {response:?}");
        assert_eq!(serde_json::to_value(&response).unwrap(), payload);
    }
}

#[test]
fn clickstack_filter_dispatch_ignores_the_payload_shape() {
    // A static-list body relabelled as a source filter is still a source filter
    // in the tolerant response union: dispatch never falls through to whichever
    // all-`Option` arm accepts the shape.
    let mut payload = clickstack_filter_kind_payloads()[1].1.clone();
    payload["type"] = "SOURCE_FILTER".into();
    let response: ClickStackFilterResponse = serde_json::from_value(payload.clone()).unwrap();
    assert!(
        matches!(
            response,
            ClickStackFilterResponse::ClickStackSourceFilter(_)
        ),
        "{response:?}"
    );
    // The strict request union cannot fit it (a source filter needs
    // `expression` and `sourceId`), so it keeps the raw JSON rather than
    // retrying the static-list arm the shape would fit.
    let filter: ClickStackFilter = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(filter, ClickStackFilter::Unknown(payload));
}

#[test]
fn clickstack_filter_unknown_or_missing_type_round_trips() {
    for json in [
        r#"{"type":"FUTURE_KIND","id":"f-9","name":"x"}"#,
        r#"{"id":"f-9","name":"x","expression":"e","sourceId":"s"}"#,
    ] {
        assert_unknown_variant_round_trips(json, |f: &ClickStackFilter| {
            matches!(f, ClickStackFilter::Unknown(_))
        });
        assert_unknown_variant_round_trips(json, |f: &ClickStackFilterInput| {
            matches!(f, ClickStackFilterInput::Unknown(_))
        });
        assert_unknown_variant_round_trips(json, |f: &ClickStackFilterResponse| {
            matches!(f, ClickStackFilterResponse::Unknown(_))
        });
    }
}

#[test]
fn clickstack_filter_response_treats_dropped_and_null_fields_as_absent() {
    for (kind, _) in clickstack_filter_kind_payloads() {
        for json in [
            serde_json::json!({"type": kind}),
            serde_json::json!({
                "type": kind, "id": null, "name": null, "expression": null,
                "sourceId": null, "options": null, "label": null, "match": null,
                "sourceMetricType": null, "where": null, "whereLanguage": null,
                "appliesToSourceIds": null, "isBroadcastEnabled": null,
                "isVariableEnabled": null, "variableName": null, "minSelections": null,
                "isGlobalRequirement": null, "maxSelections": null
            }),
        ] {
            let response: ClickStackFilterResponse = serde_json::from_value(json).unwrap();
            let sparse = match &response {
                ClickStackFilterResponse::ClickStackQueryExpressionFilter(f) => {
                    f.id.is_none()
                        && f.expression.is_none()
                        && f.source_metric_type.is_none()
                        && f.min_selections.is_none()
                        && f.max_selections.is_none()
                        && f.is_global_requirement.is_none()
                }
                ClickStackFilterResponse::ClickStackStaticListFilter(f) => {
                    f.id.is_none() && f.options.is_none() && f.variable_name.is_none()
                }
                ClickStackFilterResponse::ClickStackPrometheusLabelFilter(f) => {
                    f.label.is_none() && f.r#match.is_none() && f.source_id.is_none()
                }
                ClickStackFilterResponse::ClickStackSourceFilter(f) => {
                    f.expression.is_none() && f.variable_name.is_none()
                }
                ClickStackFilterResponse::Unknown(_) => false,
            };
            assert!(sparse, "{kind}: {response:?}");
            // Absent fields are omitted, never re-emitted as `null`.
            assert_eq!(
                serde_json::to_value(&response).unwrap(),
                serde_json::json!({"type": kind})
            );
        }
    }
}

#[test]
fn clickstack_filter_try_from_response_converts_every_kind() {
    for (kind, payload) in clickstack_filter_kind_payloads() {
        let response: ClickStackFilterResponse = serde_json::from_value(payload.clone()).unwrap();
        let request = ClickStackFilter::try_from(response).unwrap();
        assert!(
            !matches!(request, ClickStackFilter::Unknown(_)),
            "{kind}: {request:?}"
        );
        assert_eq!(serde_json::to_value(&request).unwrap(), payload);
    }
}

#[test]
fn clickstack_filter_try_from_response_names_every_missing_required_field() {
    for (json, expected) in [
        (
            r#"{"type":"QUERY_EXPRESSION"}"#,
            &["expression", "id", "name", "sourceId"][..],
        ),
        (
            r#"{"type":"STATIC_LIST","id":"f"}"#,
            &["name", "options"][..],
        ),
        (
            r#"{"type":"PROMETHEUS_LABEL","name":"n"}"#,
            &["id", "label", "sourceId"][..],
        ),
        (
            r#"{"type":"SOURCE_FILTER","id":"f","name":"n","sourceId":"s"}"#,
            &["expression", "variableName"][..],
        ),
    ] {
        let response: ClickStackFilterResponse = serde_json::from_str(json).unwrap();
        let error = ClickStackFilter::try_from(response).unwrap_err();
        assert_eq!(error.fields(), expected, "{json}");
    }

    // An unmodelled kind passes through losslessly.
    let json = r#"{"type":"FUTURE_KIND","id":"f"}"#;
    let response: ClickStackFilterResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackFilter::try_from(response).unwrap();
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn clickstack_filter_request_variants_stay_strict() {
    // Each create-request filter variant requires its schema's required fields.
    macro_rules! assert_rejects_empty {
        ($($model:ty),+ $(,)?) => {
            $(
                assert!(
                    serde_json::from_str::<$model>("{}").is_err(),
                    "{} accepted a payload missing its required fields",
                    stringify!($model),
                );
            )+
        };
    }
    assert_rejects_empty!(
        ClickStackQueryExpressionFilterInput,
        ClickStackStaticListFilterInput,
        ClickStackPrometheusLabelFilterInput,
        ClickStackSourceFilterInput,
    );
    // The update-request variant adds a required `id` on top of the input.
    let mut payload = clickstack_filter_kind_payloads()[0].1.clone();
    payload.as_object_mut().unwrap().remove("id");
    assert!(serde_json::from_value::<ClickStackQueryExpressionFilter>(payload.clone()).is_err());
    serde_json::from_value::<ClickStackQueryExpressionFilterInput>(payload).unwrap();
}

#[test]
fn clickstack_filter_selection_counts_are_integer_enums() {
    let filter: ClickStackStaticListFilterInput = serde_json::from_value(serde_json::json!({
        "type": "STATIC_LIST", "name": "n", "options": ["a"],
        "minSelections": 1, "maxSelections": 1
    }))
    .unwrap();
    assert_eq!(
        filter.min_selections,
        Some(ClickStackFilterMinSelections::One)
    );
    assert_eq!(
        filter.max_selections,
        Some(ClickStackFilterMaxSelections::One)
    );
    let v = serde_json::to_value(&filter).unwrap();
    assert_eq!(v["minSelections"], 1);
    assert_eq!(v["maxSelections"], 1);

    // A value the spec does not list is kept, not rejected.
    let min: ClickStackFilterMinSelections = serde_json::from_str("2").unwrap();
    assert_eq!(min, ClickStackFilterMinSelections::Unknown(2));
    assert_eq!(serde_json::to_string(&min).unwrap(), "2");
    assert_eq!(min.to_string(), "2");
    let zero: ClickStackFilterMinSelections = serde_json::from_str("0").unwrap();
    assert_eq!(zero, ClickStackFilterMinSelections::Zero);
    let max: ClickStackFilterMaxSelections = serde_json::from_str("3").unwrap();
    assert_eq!(max, ClickStackFilterMaxSelections::Unknown(3));
}

#[test]
fn clickstack_dashboard_response_carries_every_filter_kind() {
    let filters: Vec<serde_json::Value> = clickstack_filter_kind_payloads()
        .into_iter()
        .map(|(_, payload)| payload)
        .collect();
    let dashboard: ClickStackDashboardResponse = serde_json::from_value(serde_json::json!({
        "id": "d-1", "name": "Overview", "tiles": [], "filters": filters
    }))
    .unwrap();
    assert!(matches!(
        dashboard.filters.as_deref(),
        Some([
            ClickStackFilterResponse::ClickStackQueryExpressionFilter(_),
            ClickStackFilterResponse::ClickStackStaticListFilter(_),
            ClickStackFilterResponse::ClickStackPrometheusLabelFilter(_),
            ClickStackFilterResponse::ClickStackSourceFilter(_),
        ])
    ));
}

#[test]
fn clickstack_heatmap_chart_config_routes_each_mode_to_its_variant() {
    let distribution = serde_json::json!({
        "displayType": "heatmap", "heatmapMode": "distribution", "sourceId": "src-1",
        "select": [{"valueExpression": "Duration", "countExpression": "count()", "heatmapScaleType": "log"}],
        "where": "ServiceName = 'api'", "whereLanguage": "sql"
    });
    let series = serde_json::json!({
        "displayType": "heatmap", "heatmapMode": "series", "sourceId": "src-1",
        "select": [{"aggFn": "count"}], "groupBy": "ServiceName"
    });
    let mut implicit_distribution = distribution.clone();
    implicit_distribution
        .as_object_mut()
        .unwrap()
        .remove("heatmapMode");

    for payload in [&distribution, &implicit_distribution] {
        let config: ClickStackHeatmapChartConfig = serde_json::from_value(payload.clone()).unwrap();
        let ClickStackHeatmapChartConfig::ClickStackHeatmapDistributionChartConfig(d) = &config
        else {
            panic!("expected the distribution variant, got {config:?}");
        };
        assert_eq!(d.select[0].value_expression, "Duration");
        assert_eq!(
            d.where_language,
            Some(ClickStackHeatmapDistributionChartConfigWherelanguage::Sql)
        );
        assert_eq!(serde_json::to_value(&config).unwrap(), *payload);

        let response: ClickStackHeatmapChartConfigResponse =
            serde_json::from_value(payload.clone()).unwrap();
        assert!(
            matches!(
                response,
                ClickStackHeatmapChartConfigResponse::ClickStackHeatmapDistributionChartConfig(_)
            ),
            "{response:?}"
        );
    }

    let config: ClickStackHeatmapChartConfig = serde_json::from_value(series.clone()).unwrap();
    let ClickStackHeatmapChartConfig::ClickStackHeatmapSeriesChartConfig(s) = &config else {
        panic!("expected the series variant, got {config:?}");
    };
    assert_eq!(
        s.heatmap_mode,
        ClickStackHeatmapSeriesChartConfigHeatmapmode::Series
    );
    assert_eq!(s.group_by.as_deref(), Some("ServiceName"));
    assert_eq!(s.select[0].agg_fn, ClickStackSelectItemAggfn::Count);
    assert_eq!(serde_json::to_value(&config).unwrap(), series);

    let response: ClickStackHeatmapChartConfigResponse =
        serde_json::from_value(series.clone()).unwrap();
    let ClickStackHeatmapChartConfigResponse::ClickStackHeatmapSeriesChartConfig(s) = &response
    else {
        panic!("expected the series variant, got {response:?}");
    };
    assert_eq!(s.group_by.as_deref(), Some("ServiceName"));

    // Reached through the tile config's `displayType: "heatmap"` arm.
    let tile: ClickStackTileConfigResponse = serde_json::from_value(series.clone()).unwrap();
    assert!(matches!(
        tile,
        ClickStackTileConfigResponse::ClickStackHeatmapChartConfig(
            ClickStackHeatmapChartConfigResponse::ClickStackHeatmapSeriesChartConfig(_)
        )
    ));
}

#[test]
fn clickstack_heatmap_chart_config_guards_series_payloads_without_a_mode() {
    // A series payload whose `heatmapMode` is dropped still carries the
    // series-only `groupBy`, so it must not be read as a distribution heatmap.
    let json = r#"{"displayType":"heatmap","sourceId":"s","select":[{"aggFn":"count"}],"groupBy":"ServiceName"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackHeatmapChartConfig| {
        matches!(c, ClickStackHeatmapChartConfig::Unknown(_))
    });
    assert_unknown_variant_round_trips(json, |c: &ClickStackHeatmapChartConfigResponse| {
        matches!(c, ClickStackHeatmapChartConfigResponse::Unknown(_))
    });
    let json = r#"{"displayType":"heatmap","heatmapMode":"timeline","sourceId":"s"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackHeatmapChartConfigResponse| {
        matches!(c, ClickStackHeatmapChartConfigResponse::Unknown(_))
    });
}

#[test]
fn clickstack_heatmap_chart_config_response_treats_dropped_and_null_fields_as_absent() {
    let response: ClickStackHeatmapChartConfigResponse =
        serde_json::from_str(r#"{"heatmapMode":"series","groupBy":null,"select":null}"#).unwrap();
    assert_eq!(
        response,
        ClickStackHeatmapChartConfigResponse::ClickStackHeatmapSeriesChartConfig(
            ClickStackHeatmapSeriesChartConfigResponse {
                heatmap_mode: Some(ClickStackHeatmapSeriesChartConfigHeatmapmode::Series),
                ..Default::default()
            }
        )
    );
    let response: ClickStackHeatmapChartConfigResponse =
        serde_json::from_str(r#"{"displayType":"heatmap","where":null}"#).unwrap();
    assert_eq!(
        response,
        ClickStackHeatmapChartConfigResponse::ClickStackHeatmapDistributionChartConfig(
            ClickStackHeatmapDistributionChartConfigResponse {
                display_type: Some(ClickStackHeatmapDistributionChartConfigDisplaytype::Heatmap),
                ..Default::default()
            }
        )
    );
}

#[test]
fn clickstack_tile_config_stacked_line_routes_builder_and_raw_sql() {
    let builder = serde_json::json!({
        "displayType": "stacked_line", "sourceId": "src-1", "select": [{"aggFn": "count"}],
        "groupBy": "service", "asRatio": false, "alignDateRangeToGranularity": true,
        "fillNulls": true, "lineInterpolation": "step", "seriesLimit": 5,
        "formulas": [{"expression": "A * 100"}], "showOperandSeries": false
    });
    let raw = serde_json::json!({
        "displayType": "stacked_line", "configType": "sql", "connectionId": "conn-1",
        "sqlTemplate": "SELECT 1", "sourceId": "src-1", "fillNulls": false,
        "alignDateRangeToGranularity": false, "lineInterpolation": "smooth"
    });

    let tile: ClickStackTileConfig = serde_json::from_value(builder.clone()).unwrap();
    let ClickStackTileConfig::ClickStackStackedLineChartConfig(
        ClickStackStackedLineChartConfig::ClickStackStackedLineBuilderChartConfig(b),
    ) = &tile
    else {
        panic!("expected the stacked-line builder variant, got {tile:?}");
    };
    assert_eq!(
        b.display_type,
        ClickStackStackedLineBuilderChartConfigDisplaytype::Stacked_line
    );
    assert_eq!(
        b.line_interpolation,
        Some(ClickStackStackedLineBuilderChartConfigLineinterpolation::Step)
    );
    assert_eq!(b.series_limit, Some(5));
    assert_eq!(serde_json::to_value(&tile).unwrap(), builder);

    let tile: ClickStackTileConfig = serde_json::from_value(raw.clone()).unwrap();
    let ClickStackTileConfig::ClickStackStackedLineChartConfig(
        ClickStackStackedLineChartConfig::ClickStackStackedLineRawSqlChartConfig(r),
    ) = &tile
    else {
        panic!("expected the stacked-line raw SQL variant, got {tile:?}");
    };
    assert_eq!(r.connection_id, "conn-1");
    assert_eq!(
        r.line_interpolation,
        Some(ClickStackStackedLineRawSqlChartConfigLineinterpolation::Smooth)
    );
    assert_eq!(serde_json::to_value(&tile).unwrap(), raw);

    let tile: ClickStackTileConfigResponse = serde_json::from_value(builder).unwrap();
    assert!(matches!(
        tile,
        ClickStackTileConfigResponse::ClickStackStackedLineChartConfig(
            ClickStackStackedLineChartConfigResponse::ClickStackStackedLineBuilderChartConfig(_)
        )
    ));
    let tile: ClickStackTileConfigResponse = serde_json::from_value(raw.clone()).unwrap();
    assert!(matches!(
        tile,
        ClickStackTileConfigResponse::ClickStackStackedLineChartConfig(
            ClickStackStackedLineChartConfigResponse::ClickStackStackedLineRawSqlChartConfig(_)
        )
    ));

    // A raw-SQL body that lost its `configType` is kept out of the builder arm.
    let mut unmarked = raw;
    unmarked.as_object_mut().unwrap().remove("configType");
    assert_unknown_variant_round_trips(
        &unmarked.to_string(),
        |c: &ClickStackStackedLineChartConfigResponse| {
            matches!(c, ClickStackStackedLineChartConfigResponse::Unknown(_))
        },
    );
}

#[test]
fn clickstack_stacked_line_response_treats_dropped_and_null_fields_as_absent() {
    let response: ClickStackStackedLineChartConfigResponse = serde_json::from_str(
        r#"{"displayType":"stacked_line","lineInterpolation":null,"select":null,"seriesLimit":null}"#,
    )
    .unwrap();
    assert_eq!(
        response,
        ClickStackStackedLineChartConfigResponse::ClickStackStackedLineBuilderChartConfig(
            ClickStackStackedLineBuilderChartConfigResponse {
                display_type: Some(
                    ClickStackStackedLineBuilderChartConfigDisplaytype::Stacked_line
                ),
                ..Default::default()
            }
        )
    );
    let response: ClickStackStackedLineChartConfigResponse =
        serde_json::from_str(r#"{"configType":"sql","connectionId":null}"#).unwrap();
    assert_eq!(
        response,
        ClickStackStackedLineChartConfigResponse::ClickStackStackedLineRawSqlChartConfig(
            ClickStackStackedLineRawSqlChartConfigResponse {
                config_type: Some(ClickStackStackedLineRawSqlChartConfigConfigtype::Sql),
                ..Default::default()
            }
        )
    );
}

#[test]
fn clickstack_line_charts_carry_line_interpolation() {
    let builder: ClickStackLineBuilderChartConfig = serde_json::from_value(serde_json::json!({
        "displayType": "line", "sourceId": "s", "select": [], "lineInterpolation": "linear"
    }))
    .unwrap();
    assert_eq!(
        builder.line_interpolation,
        Some(ClickStackLineBuilderChartConfigLineinterpolation::Linear)
    );
    assert_eq!(
        serde_json::to_value(&builder).unwrap()["lineInterpolation"],
        "linear"
    );
    let raw: ClickStackLineRawSqlChartConfig = serde_json::from_value(serde_json::json!({
        "displayType": "line", "configType": "sql", "connectionId": "c",
        "sqlTemplate": "SELECT 1", "lineInterpolation": "step"
    }))
    .unwrap();
    assert_eq!(
        raw.line_interpolation,
        Some(ClickStackLineRawSqlChartConfigLineinterpolation::Step)
    );
    let unknown: ClickStackLineBuilderChartConfigLineinterpolation =
        serde_json::from_str(r#""cubic""#).unwrap();
    assert_eq!(
        unknown,
        ClickStackLineBuilderChartConfigLineinterpolation::Unknown("cubic".to_string())
    );

    // Absent or null in a response is `None`, and absent is omitted on output.
    for json in [r#"{}"#, r#"{"lineInterpolation":null}"#] {
        let response: ClickStackLineBuilderChartConfigResponse =
            serde_json::from_str(json).unwrap();
        assert_eq!(response.line_interpolation, None);
        let response: ClickStackLineRawSqlChartConfigResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.line_interpolation, None);
        assert!(
            serde_json::to_value(&response)
                .unwrap()
                .get("lineInterpolation")
                .is_none()
        );
    }
}

#[test]
fn clickstack_number_raw_sql_chart_carries_color_rules() {
    let config: ClickStackNumberRawSqlChartConfig = serde_json::from_value(serde_json::json!({
        "displayType": "number", "configType": "sql", "connectionId": "c",
        "sqlTemplate": "SELECT 1",
        "colorRules": [{"operator": "gt", "value": 100.0, "color": "chart-error"}]
    }))
    .unwrap();
    let rules = config.color_rules.as_deref().unwrap();
    assert!(matches!(
        rules,
        [ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(_)]
    ));
    assert_eq!(
        serde_json::to_value(&config).unwrap()["colorRules"][0]["operator"],
        "gt"
    );
    for json in [r#"{}"#, r#"{"colorRules":null}"#] {
        let response: ClickStackNumberRawSqlChartConfigResponse =
            serde_json::from_str(json).unwrap();
        assert_eq!(response.color_rules, None);
    }
    let response: ClickStackNumberRawSqlChartConfigResponse = serde_json::from_str(
        r#"{"colorRules":[{"operator":"eq","value":"ok","color":"chart-success"}]}"#,
    )
    .unwrap();
    assert!(matches!(
        response.color_rules.as_deref(),
        Some([ClickStackNumberTileColorConditionResponse::ClickStackEqualityColorCondition(_)])
    ));
}

#[test]
fn clickstack_tiles_carry_a_description() {
    let tile: ClickStackTileInput = serde_json::from_value(serde_json::json!({
        "name": "Error Rate", "x": 0, "y": 0, "w": 6, "h": 3,
        "description": "Share of API requests that returned a 5xx response."
    }))
    .unwrap();
    assert_eq!(
        tile.description.as_deref(),
        Some("Share of API requests that returned a 5xx response.")
    );
    let v = serde_json::to_value(ClickStackTileInput::default()).unwrap();
    assert!(v.get("description").is_none());

    let output: ClickStackTileOutput =
        serde_json::from_str(r#"{"id":"t-1","description":"tooltip"}"#).unwrap();
    assert_eq!(output.description.as_deref(), Some("tooltip"));
    for json in [r#"{}"#, r#"{"description":null}"#] {
        let output: ClickStackTileOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.description, None);
    }
}

#[test]
fn clickstack_variable_saved_filter_value_carries_all() {
    let saved: ClickStackSavedFilterValue = serde_json::from_value(serde_json::json!({
        "type": "variable", "name": "cluster", "values": [], "all": true
    }))
    .unwrap();
    let ClickStackSavedFilterValue::ClickStackVariableSavedFilterValue(value) = &saved else {
        panic!("expected the variable variant, got {saved:?}");
    };
    assert_eq!(value.all, Some(true));

    for json in [
        r#"{"type":"variable","name":"n","values":[]}"#,
        r#"{"type":"variable","name":"n","values":[],"all":null}"#,
    ] {
        let response: ClickStackVariableSavedFilterValueResponse =
            serde_json::from_str(json).unwrap();
        assert_eq!(response.all, None);
    }

    // Write-back keeps `all`.
    let response: ClickStackVariableSavedFilterValueResponse =
        serde_json::from_str(r#"{"type":"variable","name":"n","values":[],"all":true}"#).unwrap();
    let request = ClickStackVariableSavedFilterValue::try_from(response).unwrap();
    assert_eq!(request.all, Some(true));
}

#[test]
fn clickstack_time_chart_series_accepts_stacked_line() {
    let display: ClickStackTimeChartSeriesDisplaytype =
        serde_json::from_str(r#""stacked_line""#).unwrap();
    assert_eq!(display, ClickStackTimeChartSeriesDisplaytype::Stacked_line);
    assert_eq!(display.to_string(), "stacked_line");
    assert_eq!(
        serde_json::to_string(&display).unwrap(),
        r#""stacked_line""#
    );
}

/// Spec-shaped payloads for every arm of `ClickStackAlertChartConfig`:
/// `(kind, builder, raw SQL)`. Each carries fields only its own variant
/// defines, plus the alert overlay's `name` (and, on builders, `where`).
fn clickstack_alert_chart_config_payloads()
-> [(&'static str, serde_json::Value, serde_json::Value); 4] {
    [
        (
            "Line",
            serde_json::json!({
                "displayType": "line", "sourceId": "src-1", "select": [{"aggFn": "count"}],
                "groupBy": "ServiceName", "lineInterpolation": "step", "fitYAxisToData": true,
                "name": "Error Rate Query", "where": "ServiceName:api", "whereLanguage": "lucene"
            }),
            serde_json::json!({
                "displayType": "line", "configType": "sql", "connectionId": "conn-1",
                "sqlTemplate": "SELECT 1", "compareToPreviousPeriod": true,
                "name": "Error Rate Query"
            }),
        ),
        (
            "Bar",
            serde_json::json!({
                "displayType": "stacked_bar", "sourceId": "src-1", "select": [{"aggFn": "count"}],
                "seriesLimit": 5, "name": "Errors by service", "where": "level = 'error'",
                "whereLanguage": "sql"
            }),
            serde_json::json!({
                "displayType": "stacked_bar", "configType": "sql", "connectionId": "conn-1",
                "sqlTemplate": "SELECT 1", "fillNulls": true, "name": "Errors by service"
            }),
        ),
        (
            "StackedLine",
            serde_json::json!({
                "displayType": "stacked_line", "sourceId": "src-1", "select": [{"aggFn": "count"}],
                "lineInterpolation": "linear", "showOperandSeries": false,
                "name": "Latency bands", "where": "ServiceName:api", "whereLanguage": "lucene"
            }),
            serde_json::json!({
                "displayType": "stacked_line", "configType": "sql", "connectionId": "conn-1",
                "sqlTemplate": "SELECT 1", "lineInterpolation": "smooth", "name": "Latency bands"
            }),
        ),
        (
            "Number",
            serde_json::json!({
                "displayType": "number", "sourceId": "src-1", "select": [{"aggFn": "count"}],
                "color": "chart-red", "name": "Error count", "where": "level:error",
                "whereLanguage": "lucene"
            }),
            serde_json::json!({
                "displayType": "number", "configType": "sql", "connectionId": "conn-1",
                "sqlTemplate": "SELECT count() FROM otel_logs", "color": "chart-green",
                "name": "Error count"
            }),
        ),
    ]
}

/// The `{outer}/{inner}` arm names a request alert chart config landed in.
fn clickstack_alert_chart_config_arm(config: &ClickStackAlertChartConfig) -> String {
    let inner = match config {
        ClickStackAlertChartConfig::ClickStackAlertLineChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfig::ClickStackAlertBarChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfig::ClickStackAlertStackedLineChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfig::ClickStackAlertNumberChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfig::Unknown(v) => format!("Unknown({v})"),
    };
    format!("{config}/{inner}")
}

/// The `{outer}/{inner}` arm names a response alert chart config landed in.
fn clickstack_alert_chart_config_response_arm(
    config: &ClickStackAlertChartConfigResponse,
) -> String {
    let inner = match config {
        ClickStackAlertChartConfigResponse::ClickStackAlertLineChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfigResponse::ClickStackAlertBarChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfigResponse::ClickStackAlertStackedLineChartConfig(c) => {
            c.to_string()
        }
        ClickStackAlertChartConfigResponse::ClickStackAlertNumberChartConfig(c) => c.to_string(),
        ClickStackAlertChartConfigResponse::Unknown(v) => format!("Unknown({v})"),
    };
    format!("{config}/{inner}")
}

#[test]
fn clickstack_alert_chart_config_routes_each_display_type_and_config_type_to_its_variant() {
    for (kind, builder, raw) in clickstack_alert_chart_config_payloads() {
        for (payload, variant) in [(builder, "Builder"), (raw, "RawSql")] {
            let expected = format!(
                "ClickStackAlert{kind}ChartConfig/ClickStackAlert{kind}{variant}ChartConfig"
            );

            let config: ClickStackAlertChartConfig =
                serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(clickstack_alert_chart_config_arm(&config), expected);
            assert_eq!(serde_json::to_value(&config).unwrap(), payload);

            let response: ClickStackAlertChartConfigResponse =
                serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(
                clickstack_alert_chart_config_response_arm(&response),
                expected
            );
            assert_eq!(serde_json::to_value(&response).unwrap(), payload);
        }
    }

    // The overlay's fields land on the variant structs themselves.
    let [(_, line_builder, _), _, _, (_, _, number_raw)] = clickstack_alert_chart_config_payloads();
    let config: ClickStackAlertChartConfig = serde_json::from_value(line_builder).unwrap();
    let ClickStackAlertChartConfig::ClickStackAlertLineChartConfig(
        ClickStackAlertLineChartConfig::ClickStackAlertLineBuilderChartConfig(b),
    ) = &config
    else {
        panic!("expected the line builder variant, got {config:?}");
    };
    assert_eq!(b.name.as_deref(), Some("Error Rate Query"));
    assert_eq!(b.r#where.as_deref(), Some("ServiceName:api"));
    assert_eq!(
        b.where_language,
        Some(ClickStackAlertChartConfigOverlayWherelanguage::Lucene)
    );
    assert_eq!(
        b.line_interpolation,
        Some(ClickStackLineBuilderChartConfigLineinterpolation::Step)
    );
    assert_eq!(b.select[0].agg_fn, ClickStackSelectItemAggfn::Count);

    let response: ClickStackAlertChartConfigResponse = serde_json::from_value(number_raw).unwrap();
    let ClickStackAlertChartConfigResponse::ClickStackAlertNumberChartConfig(
        ClickStackAlertNumberChartConfigResponse::ClickStackAlertNumberRawSqlChartConfig(r),
    ) = &response
    else {
        panic!("expected the number raw SQL variant, got {response:?}");
    };
    assert_eq!(r.connection_id.as_deref(), Some("conn-1"));
    assert_eq!(r.name.as_deref(), Some("Error count"));
    assert_eq!(
        r.config_type,
        Some(ClickStackNumberRawSqlChartConfigConfigtype::Sql)
    );
}

#[test]
fn clickstack_alert_chart_config_keeps_payloads_out_of_variants_they_do_not_fit() {
    for (kind, builder, raw) in clickstack_alert_chart_config_payloads() {
        // A raw-SQL body that lost its `configType` stays out of the builder
        // arm, request and response alike, and round-trips verbatim.
        let mut unmarked = raw.clone();
        unmarked.as_object_mut().unwrap().remove("configType");
        // An inner `Unknown` displays as its raw JSON.
        let expected = format!("ClickStackAlert{kind}ChartConfig/{unmarked}");
        let config: ClickStackAlertChartConfig = serde_json::from_value(unmarked.clone()).unwrap();
        assert_eq!(clickstack_alert_chart_config_arm(&config), expected);
        assert_eq!(serde_json::to_value(&config).unwrap(), unmarked);
        let response: ClickStackAlertChartConfigResponse =
            serde_json::from_value(unmarked.clone()).unwrap();
        assert_eq!(
            clickstack_alert_chart_config_response_arm(&response),
            expected
        );
        assert_eq!(serde_json::to_value(&response).unwrap(), unmarked);

        // The request builder keeps the chart config's required fields.
        let mut sourceless = builder.clone();
        sourceless.as_object_mut().unwrap().remove("sourceId");
        let config: ClickStackAlertChartConfig =
            serde_json::from_value(sourceless.clone()).unwrap();
        assert_eq!(
            clickstack_alert_chart_config_arm(&config),
            format!("ClickStackAlert{kind}ChartConfig/{sourceless}")
        );
        // The request raw-SQL variant does too.
        let mut templateless = raw.clone();
        templateless.as_object_mut().unwrap().remove("sqlTemplate");
        let config: ClickStackAlertChartConfig =
            serde_json::from_value(templateless.clone()).unwrap();
        assert_eq!(
            clickstack_alert_chart_config_arm(&config),
            format!("ClickStackAlert{kind}ChartConfig/{templateless}")
        );
        // A response tolerates both, per field.
        let response: ClickStackAlertChartConfigResponse =
            serde_json::from_value(sourceless).unwrap();
        assert_eq!(
            clickstack_alert_chart_config_response_arm(&response),
            format!("ClickStackAlert{kind}ChartConfig/ClickStackAlert{kind}BuilderChartConfig")
        );
        let response: ClickStackAlertChartConfigResponse =
            serde_json::from_value(templateless).unwrap();
        assert_eq!(
            clickstack_alert_chart_config_response_arm(&response),
            format!("ClickStackAlert{kind}ChartConfig/ClickStackAlert{kind}RawSqlChartConfig")
        );

        // Display types an alert cannot use, or none at all, are Unknown.
        for display_type in [Some("pie"), Some("table"), Some("bar"), None] {
            let mut other = builder.clone();
            match display_type {
                Some(value) => other["displayType"] = serde_json::Value::from(value),
                None => {
                    other.as_object_mut().unwrap().remove("displayType");
                }
            }
            assert_unknown_variant_round_trips(
                &other.to_string(),
                |c: &ClickStackAlertChartConfig| {
                    matches!(c, ClickStackAlertChartConfig::Unknown(_))
                },
            );
            assert_unknown_variant_round_trips(
                &other.to_string(),
                |c: &ClickStackAlertChartConfigResponse| {
                    matches!(c, ClickStackAlertChartConfigResponse::Unknown(_))
                },
            );
        }
    }

    // A recognized discriminator whose payload changed shape is kept verbatim.
    let json = r#"{"displayType":"number","configType":"sql","sqlTemplate":5}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChartConfigResponse| {
        matches!(
            c,
            ClickStackAlertChartConfigResponse::ClickStackAlertNumberChartConfig(
                ClickStackAlertNumberChartConfigResponse::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_alert_chart_config_responses_treat_dropped_and_null_fields_as_absent() {
    macro_rules! assert_absent_and_null_are_none {
        ($($response:ty),+ $(,)?) => {
            $(
                for wire in [
                    "{}",
                    r#"{"name":null,"where":null,"whereLanguage":null,"sourceId":null,"numberFormat":null}"#,
                ] {
                    let parsed: $response = serde_json::from_str(wire).unwrap();
                    assert_eq!(parsed, <$response>::default(), "{} from {wire}", stringify!($response));
                    assert_eq!(serde_json::to_value(&parsed).unwrap(), serde_json::json!({}));
                }
            )+
        };
    }
    assert_absent_and_null_are_none!(
        ClickStackAlertBarBuilderChartConfigResponse,
        ClickStackAlertBarRawSqlChartConfigResponse,
        ClickStackAlertLineBuilderChartConfigResponse,
        ClickStackAlertLineRawSqlChartConfigResponse,
        ClickStackAlertNumberBuilderChartConfigResponse,
        ClickStackAlertNumberRawSqlChartConfigResponse,
        ClickStackAlertStackedLineBuilderChartConfigResponse,
        ClickStackAlertStackedLineRawSqlChartConfigResponse,
    );

    let response: ClickStackAlertChartConfigResponse =
        serde_json::from_str(r#"{"displayType":"stacked_line","name":null,"select":null}"#)
            .unwrap();
    assert_eq!(
        response,
        ClickStackAlertChartConfigResponse::ClickStackAlertStackedLineChartConfig(
            ClickStackAlertStackedLineChartConfigResponse::ClickStackAlertStackedLineBuilderChartConfig(
                ClickStackAlertStackedLineBuilderChartConfigResponse {
                    display_type: Some(
                        ClickStackStackedLineBuilderChartConfigDisplaytype::Stacked_line
                    ),
                    ..Default::default()
                }
            )
        )
    );
}

#[test]
fn clickstack_alert_chart_config_overlay_fields_are_all_optional() {
    let overlay: ClickStackAlertChartConfigOverlay = serde_json::from_str("{}").unwrap();
    assert_eq!(overlay, ClickStackAlertChartConfigOverlay::default());
    let wire = serde_json::json!({"name": "Q", "where": "a:b", "whereLanguage": "sql"});
    let overlay: ClickStackAlertChartConfigOverlay = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        overlay.where_language,
        Some(ClickStackAlertChartConfigOverlayWherelanguage::Sql)
    );
    assert_eq!(serde_json::to_value(&overlay).unwrap(), wire);
    let language: ClickStackAlertChartConfigOverlayWherelanguage =
        serde_json::from_str(r#""kql""#).unwrap();
    assert_eq!(
        language,
        ClickStackAlertChartConfigOverlayWherelanguage::Unknown("kql".into())
    );
}

#[test]
fn clickstack_alert_sources_accept_inline() {
    assert_eq!(
        serde_json::from_str::<ClickStackAlertResponseSource>(r#""inline""#).unwrap(),
        ClickStackAlertResponseSource::Inline
    );
    assert_eq!(
        serde_json::from_str::<ClickStackCreateAlertRequestSource>(r#""inline""#).unwrap(),
        ClickStackCreateAlertRequestSource::Inline
    );
    assert_eq!(
        serde_json::from_str::<ClickStackUpdateAlertRequestSource>(r#""inline""#).unwrap(),
        ClickStackUpdateAlertRequestSource::Inline
    );
    assert_eq!(ClickStackAlertResponseSource::Inline.to_string(), "inline");
    assert_eq!(
        ClickStackCreateAlertRequestSource::Inline.to_string(),
        "inline"
    );
    assert_eq!(
        ClickStackUpdateAlertRequestSource::Inline.to_string(),
        "inline"
    );
}

#[test]
fn clickstack_alert_requests_send_inline_chart_configs_display_names_and_tags() {
    let [(_, builder, _), ..] = clickstack_alert_chart_config_payloads();
    let wire = serde_json::json!({
        "source": "inline", "chartConfig": builder, "threshold": 10.0, "interval": "5m",
        "thresholdType": "above", "displayName": "Checkout error spike",
        "tags": ["checkout", "p1"]
    });

    let create: ClickStackCreateAlertRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        create.source,
        Some(ClickStackCreateAlertRequestSource::Inline)
    );
    assert!(matches!(
        create.chart_config,
        Some(ClickStackAlertChartConfig::ClickStackAlertLineChartConfig(
            ClickStackAlertLineChartConfig::ClickStackAlertLineBuilderChartConfig(_)
        ))
    ));
    assert_eq!(create.display_name.as_deref(), Some("Checkout error spike"));
    assert_eq!(
        create.tags.as_deref(),
        Some(&["checkout".into(), "p1".into()][..])
    );
    assert_eq!(
        (create.channel.as_ref(), create.channels.as_ref()),
        (None, None)
    );
    assert_eq!(serde_json::to_value(&create).unwrap(), wire);

    let update: ClickStackUpdateAlertRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        update.source,
        Some(ClickStackUpdateAlertRequestSource::Inline)
    );
    assert_eq!(update.display_name.as_deref(), Some("Checkout error spike"));
    assert_eq!(serde_json::to_value(&update).unwrap(), wire);
}

#[test]
fn clickstack_alert_requests_omit_unset_source_and_channels_but_stay_strict() {
    // `source`, `channel` and `channels` are no longer required, so an unset
    // value is omitted from the wire rather than defaulted.
    let create = serde_json::to_value(ClickStackCreateAlertRequest::default()).unwrap();
    let update = serde_json::to_value(ClickStackUpdateAlertRequest::default()).unwrap();
    for wire in [&create, &update] {
        for key in [
            "source",
            "channel",
            "channels",
            "chartConfig",
            "displayName",
            "tags",
        ] {
            assert!(wire.get(key).is_none(), "{key} was sent unset: {wire}");
        }
    }

    // `threshold`, `interval` and `thresholdType` stay required.
    let minimal = serde_json::json!({"threshold": 1.0, "interval": "1m", "thresholdType": "above"});
    let parsed: ClickStackCreateAlertRequest = serde_json::from_value(minimal.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), minimal);
    let parsed: ClickStackUpdateAlertRequest = serde_json::from_value(minimal.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), minimal);
    for key in ["threshold", "interval", "thresholdType"] {
        let mut missing = minimal.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<ClickStackCreateAlertRequest>(missing.clone()).is_err());
        assert!(serde_json::from_value::<ClickStackUpdateAlertRequest>(missing).is_err());
    }
}

#[test]
fn clickstack_alert_response_carries_inline_chart_config_display_name_and_tags() {
    let [_, (_, _, bar_raw), ..] = clickstack_alert_chart_config_payloads();
    let alert: ClickStackAlertResponse = serde_json::from_value(serde_json::json!({
        "id": "alert-1", "source": "inline", "chartConfig": bar_raw,
        "displayName": "Errors by service", "tags": [], "state": "OK"
    }))
    .unwrap();
    assert_eq!(alert.source, Some(ClickStackAlertResponseSource::Inline));
    assert_eq!(alert.display_name.as_deref(), Some("Errors by service"));
    assert_eq!(alert.tags, Some(vec![]));
    assert!(matches!(
        alert.chart_config,
        Some(
            ClickStackAlertChartConfigResponse::ClickStackAlertBarChartConfig(
                ClickStackAlertBarChartConfigResponse::ClickStackAlertBarRawSqlChartConfig(_)
            )
        )
    ));

    // The spec requires displayName and tags on the response, but a response
    // that drops them or sends `null` still deserializes, to `None`.
    let alert: ClickStackAlertResponse = serde_json::from_str(
        r#"{"id":"alert-1","chartConfig":null,"displayName":null,"tags":null}"#,
    )
    .unwrap();
    assert_eq!(alert.chart_config, None);
    assert_eq!(alert.display_name, None);
    assert_eq!(alert.tags, None);
    assert_eq!(
        serde_json::to_value(&alert).unwrap(),
        serde_json::json!({"id": "alert-1"})
    );
}
