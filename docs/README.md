# Documentation

Start with the [README](../README.md) for installation and product examples. These guides follow the CLI in this checkout. Run `<command> --help` for all flags and the authorization requirements of your installed version.

## Task guides

| Task | Guide |
| --- | --- |
| Run ClickHouse and Postgres locally | [Local development](guides/local-development.md) |
| Create, query, scale, back up, and remove ClickHouse in Cloud | [Cloud services](guides/cloud-service.md) |
| Connect to and operate ClickHouse Managed Postgres | [ClickHouse Managed Postgres](guides/managed-postgres.md) |
| Load object storage or replicate Postgres, then verify ingestion | [ClickPipes](guides/clickpipes.md) |
| Create and safely replace a ClickStack dashboard | [ClickStack](guides/clickstack.md) |
| Test an executable UDF locally, then deploy and update it in Cloud | [User-defined functions](guides/user-defined-functions.md) |

## Reference

- [Cloud authentication and access](reference/authentication.md): scopes, credential precedence, selectors, keys, roles, logout.
- [SQL and Query API behavior](reference/query-api.md): key binding, input formats, timeouts, repair, named endpoints.
- [ClickPipe configuration](reference/clickpipe-configuration.md): partial updates, mappings, source authentication, private connectivity.
- [Automation and output contracts](reference/automation.md): JSON shapes, stderr, exit codes, asynchronous operations.
- [Telemetry](reference/telemetry.md): recorded fields, notice, opt-out, local inspection.

## Development

- [Contributing](../CONTRIBUTING.md): build, checks, and where documentation belongs.
- [CLI development](development.md): code layout, test selection, live suites, debugging.
- [API library](../crates/clickhouse-cloud-api/README.md): Rust client and API-specific documentation.
