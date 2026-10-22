# Configure a ClickStack dashboard

[All documentation](../README.md)

Use an existing Managed ClickStack service with a configured source. Reads support OAuth; creation and updates require an [API key with appropriate permissions](../reference/authentication.md). The CLI also manages sources, roles, saved searches, alerts, and webhooks through JSON files.

## Select a source

```bash
clickhousectl cloud clickstack source list <service-id>
clickhousectl cloud clickstack source get <service-id> <source-id>
```

The service ID comes from `cloud service list`. Child IDs come from each ClickStack resource's list command; a child's `--name` never selects the parent service. Use a source appropriate for the data you want to chart.

## Validate and create a dashboard

Save this as `dashboard.json`, replacing `<source-id>`:

```json
{
  "name": "Event volume",
  "tiles": [
    {
      "name": "Event count",
      "x": 0,
      "y": 0,
      "w": 6,
      "h": 3,
      "config": {
        "displayType": "line",
        "sourceId": "<source-id>",
        "select": [{"aggFn": "count"}]
      }
    }
  ]
}
```

```bash
clickhousectl cloud clickstack dashboard validate <service-id> --file dashboard.json
clickhousectl cloud clickstack dashboard create <service-id> --file dashboard.json
clickhousectl cloud clickstack dashboard get <service-id> <dashboard-id>
```

Use the returned ID to inspect the saved resource, then open it in ClickStack to verify the chart against your source data. `--file -` accepts stdin. Validation checks a **create body** without saving it; it does not validate the separate update body.

## Safely update an existing resource

**Every ClickStack update replaces the complete resource.** Before editing, save the current state:

```bash
clickhousectl cloud clickstack dashboard get <service-id> <dashboard-id> --json > dashboard-before.json
```

Build `dashboard-update.json` from the complete writable definition. Preserve the `id` of every existing filter and include every tile, filter, container, tag, and saved query value that should remain. Do not send only the changed fields or assume an unedited GET response is a valid request.

```bash
clickhousectl cloud clickstack dashboard update <service-id> <dashboard-id> \
  --file dashboard-update.json
clickhousectl cloud clickstack dashboard get <service-id> <dashboard-id>
```

Review the resulting resource and chart. Apply the same complete-definition rule to sources, roles, saved searches, alerts, and webhooks. A saved search needs `name` and `sourceId`; include any selection, filters, ordering, tags, and query language you want retained.

## Add notifications

Create a webhook first, then reference its ID in an alert. A generic `webhook.json` can contain:

```json
{
  "name": "Incident receiver",
  "service": "generic",
  "url": "https://alerts.example.com/clickstack"
}
```

```bash
clickhousectl cloud clickstack webhook validate <service-id> --file webhook.json
clickhousectl cloud clickstack webhook create <service-id> --file webhook.json
```

Validation checks the body, URL and body template without saving the webhook; it does not check that the name is unique. Protect JSON files containing authorization headers or other secrets.

An alert needs `threshold`, `thresholdType` and `interval`. `channel` and `channels` are optional; `channels` takes 1 to 10 distinct entries. A tile alert definition is:

```json
{
  "source": "tile",
  "dashboardId": "<dashboard-id>",
  "tileId": "<tile-id>",
  "threshold": 100,
  "thresholdType": "above",
  "interval": "5m",
  "channel": {"type":"webhook","webhookId":"<webhook-id>","webhookService":"generic"},
  "channels": [{"type":"webhook","webhookId":"<webhook-id>","webhookService":"generic"}]
}
```

Save as `alert.json` with IDs from the saved dashboard and webhook, then create and inspect it:

```bash
clickhousectl cloud clickstack alert create <service-id> --file alert.json
clickhousectl cloud clickstack alert get <service-id> <alert-id>
```

Inspect `state` and `executionErrors`, as well as notification destinations. A `saved_search` alert uses `savedSearchId` instead of dashboard/tile IDs. The `30s` interval requires that feature enabled for the team. Updates replace the alert, so retain the channel fields and every other desired setting.

An alert's `displayName` is the name shown in the UI and in `alert list`; its optional `name` is a Handlebars template for the notification title. For `alert get`, `update` and `delete`, `--name` selects the alert whose `displayName` or `name` equals the value, and fails if more than one alert matches.

`displayName` defaults to the name of the saved search, tile or chart the alert queries, and `tags` default to the saved search's or dashboard's tags. An `inline` alert carries its own chart in `chartConfig` (a `line`, `stacked_bar`, `stacked_line` or `number` chart) instead of referencing a dashboard tile, and has no tags unless you set them:

```json
{
  "source": "inline",
  "displayName": "API errors",
  "tags": ["api"],
  "chartConfig": {
    "displayType": "line",
    "sourceId": "<source-id>",
    "select": [{"aggFn": "count"}],
    "where": "ServiceName:api",
    "whereLanguage": "lucene"
  },
  "threshold": 100,
  "thresholdType": "above",
  "interval": "5m",
  "channels": [{"type":"webhook","webhookId":"<webhook-id>","webhookService":"generic"}]
}
```
