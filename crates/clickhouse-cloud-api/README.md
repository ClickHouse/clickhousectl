# clickhouse-cloud-api

Typed Rust client for the [ClickHouse Cloud API](https://clickhouse.com/docs/en/cloud/manage/openapi).
It covers the operations in the published OpenAPI spec with typed request and
response models, plus offline operation metadata and the Query API that runs SQL
against a service. [`clickhousectl`](https://github.com/ClickHouse/clickhousectl) is built on it.

Requires Rust 1.94.1 or later. Changes and migrations for each release are in the
[changelog](https://github.com/ClickHouse/clickhousectl/blob/main/crates/clickhouse-cloud-api/CHANGELOG.md).

## Install

```bash
cargo add clickhouse-cloud-api
cargo add tokio --features macros,rt-multi-thread
```

## Usage

Every operation is an async method on `Client`, named after its OpenAPI operation ID
(`organizationGetList` is `organization_get_list`). Methods return `ApiResponse<T>`,
the API's envelope, with the payload in `result`:

```rust,no_run
use clickhouse_cloud_api::Client;

#[tokio::main]
async fn main() -> Result<(), clickhouse_cloud_api::Error> {
    let client = Client::new("your-key-id", "your-key-secret");
    let orgs = client.organization_get_list(None, None).await?;
    for org in orgs.result.unwrap_or_default() {
        println!("{:?} {:?}", org.id, org.name);
    }
    Ok(())
}
```

Request models are strict: a required field is `T`, an optional one `Option<T>`.
Every response field is `Option<T>`, so a field the API drops or sends as `null`
reads as `None` instead of failing the response. A schema used in both directions is
two types, `{Name}` for the request and `{Name}Response`, and the `convert` module
turns a fetched resource back into a request body. The crate documentation covers the
model rules and the write-back pattern.

## Authentication

- **API key** (HTTP Basic): `Client::new(key_id, key_secret)` for the production API,
  `Client::with_base_url(base_url, key_id, key_secret)` for another one.
- **OAuth Bearer token**: `Client::with_bearer_token(base_url, token)`.
  `set_bearer_token` replaces an expired token without rebuilding the client.

`Client::with_http_client` and `Client::with_http_client_bearer` take a prebuilt
`reqwest::Client`, for example to set a timeout or user agent. The crate re-exports
`reqwest`, so `clickhouse_cloud_api::reqwest::Client::builder()` always matches the
version the client uses.

Some methods need one mode: `run_query_bearer` and `run_postgres_query_bearer` use the
client's OAuth token, and a Basic-auth client gets `Error::AuthMismatch` before any
request. `run_query` takes a per-service Query API key and secret as arguments.

## Errors

`Error` is `#[non_exhaustive]`; keep a `_` arm for variants added later.

| Variant | Meaning |
| --- | --- |
| `Http` | Transport failure from `reqwest` |
| `Json` | A request or response body could not be serialized or parsed |
| `Api { status, message }` | The API answered with an error status |
| `UdfAttachmentUnavailable` | Attaching a UDF failed because the service is unavailable (HTTP 424) |
| `Sql { status, code, details }` | The Query API rejected the SQL statement |
| `InvalidPathParameter` | A path parameter is empty, `.` or `..`; no request was sent |
| `AuthMismatch` | The method needs a different auth mode than the client has |
| `ServiceIdle` | The service is idle; retry the query with `wake_service` |
| `ServiceStopped` | The service is stopped and must be started before it can be queried |
| `QueryTimeout` | The Query API gateway stopped waiting; the statement may still be running |

Rust callers receive `Error::UdfAttachmentUnavailable` for a structured attachment failure (HTTP 424); its `UdfAttachResponse424` payload preserves the error code, service state, wake eligibility, and request ID. Fields tolerate absence and null, and enums retain unknown values. Malformed responses remain `Error::Api` with the original error message.

## Feature flags

The OpenAPI spec marks some response fields as deprecated (e.g. `Service.tier`, `ApiKey.roles`, `Member.role`). In almost all cases, these are not needed. The Cloud API library disables them by default, gated by a Cargo feature flag `deprecated-fields`. Enable this feature if you need to consume deprecated fields.

Cargo unifies features across the build, so another crate can turn on `deprecated-fields` for you. Code that has to compile either way should not name every field of a request struct that has a gated field. Use `new` with the required fields on structs without `Default` (`ServicePostRequest::new(name, provider, region, ip_access_list)`, `ApiKeyPostRequest::new(name)` and so on), or `..Default::default()` on all-`Option` structs. Then set optional fields with struct update syntax.

## Postgres Query API

`Client::run_postgres_query_bearer(org_id, service_id, &RunPostgresQueryRequest)`
runs read-only SQL over HTTPS with the client's OAuth Bearer token. The request
requires `sql` and accepts an optional `database`; omission uses `postgres`.
Basic/API-key clients receive `Error::AuthMismatch` before any network request. The method neither provisions query credentials nor wakes the
service or retries queries.

This console Query API route is outside the published OpenAPI. It accepts
`clickhousectl` audience OAuth tokens with read-only database access; Cloud API
keys are unsupported.

The returned `reqwest::Response` supports streaming and automatically decompresses
gzip using the default HTTP client. Its fixed format is
`JSONCompactEachRowWithNamesAndTypes`: a JSON array of column names, then an array
of type names, followed by one array per row, all newline-delimited. An empty
result can have an empty body. SQL errors retain their status, server `code`
(currently `POSTGRES_ERROR`), and `details` in `Error::Sql`; other HTTP errors retain
their status and body.
Query host selection follows `with_query_host`, `CLICKHOUSE_CLOUD_QUERY_HOST`,
and the management API environment, in that order.

## Operation permissions and metadata

`meta::operations` exposes offline endpoint descriptors alongside the existing typed `Client` methods:

```rust
use clickhouse_cloud_api::meta::operations;

let endpoint = &operations::INSTANCE_GET;
assert_eq!(endpoint.operation_id, "instanceGet");
assert_eq!(endpoint.rust_method, "instance_get");
assert_eq!(endpoint.required_permissions, &["control-plane:service:view"]);
assert_eq!(operations::by_operation_id("instanceGet"), Some(endpoint));
```

Each descriptor includes the exact operation ID, Rust method name, HTTP method, path template, and required Cloud API-key permission IDs. **All listed permissions are required.** An empty slice means the spec requires a valid key without additional named permissions. Unknown operation IDs return `None`; they never appear as an empty permission list. `operations::ALL` iterates every descriptor. These requirements describe API-key authorization; they do not replace OAuth restrictions or SQL privileges. A caller making several API requests can union the permission slices of those operations.

## Development

Contributor documentation, scripts and tests for this crate are in the repository's
[development guide](https://github.com/ClickHouse/clickhousectl/blob/main/docs/development.md#api-library-development).
