<div align="center">
<p>
<a href="https://clickhouse.com">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://clickhouse.design/images/brand/logos/full-logo-white.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://clickhouse.design/images/brand/logos/full-logo-black.svg">
    <img alt="ClickHouse" src="https://clickhouse.design/images/brand/logos/full-logo-black.svg" width="300">
  </picture>
</a>
</p>
<h1>clickhousectl</h1>
</div>

[![Slack community](https://img.shields.io/badge/Slack-Join_the_community-4A154B)](https://clickhouse.com/slack)
[![Follow on X](https://img.shields.io/badge/X-Follow_ClickHouseDB-000000)](https://x.com/ClickHouseDB)
[![YouTube videos](https://img.shields.io/badge/YouTube-Watch_ClickHouseDB-FF0000)](https://www.youtube.com/@ClickHouseDB)

`clickhousectl` (`chctl`) is the official CLI for ClickHouse, from local development to ClickHouse Cloud.


## What can the CLI do?

- **[ClickHouse](https://clickhouse.com/clickhouse)** is the open-source, column-oriented SQL database for analytics. Install and switch local versions, run isolated servers, and execute SQL.
- **[ClickHouse Cloud](https://clickhouse.com/cloud)** runs ClickHouse as a managed service on AWS, GCP, and Azure. Create, configure, scale, back up, and query services from the terminal.
- **[ClickHouse Managed Postgres](https://clickhouse.com/cloud/postgres)** runs PostgreSQL for transactional applications, with ClickHouse integration for analytics. Use Docker-backed Postgres locally, then create, monitor, configure, and restore managed services in Cloud.
- **[ClickPipes](https://clickhouse.com/cloud/clickpipes)** ingests data into ClickHouse Cloud from object storage, streams, and databases through change data capture. Create, inspect, update, scale, and troubleshoot pipelines.
- **[ClickStack](https://clickhouse.com/clickstack)** combines ClickHouse, OpenTelemetry, and the HyperDX UI for observability. Manage sources, roles, saved searches, dashboards, alerts, and webhooks for an existing [Managed ClickStack](https://clickhouse.com/cloud/clickstack) service.

The CLI also manages organization access and installs official ClickHouse skills for coding agents.

## Install

Supports macOS and Linux on Apple Silicon/ARM64 and x86_64.

```bash
curl -fsSL https://clickhouse.com/cli | sh
```

The script installs to `~/.local/bin/clickhousectl` and creates the `chctl` alias. Ensure `~/.local/bin` is on your `PATH`.

Other installation options:

| Tool | Command |
| --- | --- |
| npm | `npm install -g clickhousectl` |
| uv | `uv tool install clickhousectl` |
| pipx | `pipx install clickhousectl` |
| pip | `pip install clickhousectl` |
| cargo-binstall | `cargo binstall clickhousectl` |
| Cargo, from source (Rust 1.94+) | `cargo install clickhousectl` |

Check for or install a CLI update:

```bash
clickhousectl update --check
clickhousectl update
```

## Start locally

Use the `latest` channel for the rolling master build of ClickHouse. A bare server start installs `latest` if no version or default is available.

```bash
clickhousectl local server start dev --version latest
clickhousectl local client dev --query "SELECT version()"
clickhousectl local server stop dev
```

Run these from the same project directory. Each named server has isolated data under `.clickhouse/`; stopping preserves it for the next start.

For local Postgres, start Docker first:

```bash
clickhousectl local postgres start dev
clickhousectl local postgres client dev --query "SELECT version()"
clickhousectl local postgres stop dev
```

Continue with [local development](docs/guides/local-development.md) for SQL files, version selection, configuration, and cleanup.

## Connect to ClickHouse Cloud

Create a [ClickHouse Cloud](https://clickhouse.com/cloud) account if needed to get $300 in free trial credits, then sign in:

```bash
clickhousectl cloud auth signup
clickhousectl cloud auth login
```

Browser OAuth is **read-only**. Creating or changing resources requires an [API key](https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl) with the appropriate roles. Use either an interactive prompt or a `.env` file to supply your API key.

To save the key interactively without putting its secret in shell history:

```bash
clickhousectl cloud auth login --interactive
```

Alternatively, create a `.env` file in your project with your keys:

```dotenv
CLICKHOUSE_CLOUD_API_KEY=your-api-key
CLICKHOUSE_CLOUD_API_SECRET=your-api-secret
```

Keep `.env` out of version control by adding it to `.gitignore`.

OAuth tokens are global (`~/.clickhouse/tokens.json`); saved API keys are project-local (`.clickhouse/credentials.json`). For CI, inject `CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` through your secret manager. See [authentication and permissions](docs/reference/authentication.md) for precedence, organization selection, and logout.

### Create and query ClickHouse

Create a ClickHouse Cloud service. Save the initial password shown at creation; it is only returned once.

```bash
# Output will contain the service ID
clickhousectl cloud service create --name analytics \
  --provider aws --region us-east-1
```


Creation returns before provisioning finishes. Queries use OAuth for read-only SQL or an API key for permitted reads and writes. On first use, API-key queries can bind the caller's key to the service's Query API endpoint; use `--no-auto-enable` to require an existing binding.

The Query API may time out while SQL continues running. Check the outcome before retrying a write. Use a native client for long queries and bulk loads. See [create and operate a Cloud service](docs/guides/cloud-service.md) and [SQL and Query API behavior](docs/reference/query-api.md).

### Create ClickHouse Managed Postgres

```bash
clickhousectl cloud postgres create --name app-db \
  --provider aws --region us-east-1 --size c6gd.xlarge --pg-version 18
clickhousectl cloud postgres get <postgres-id>
clickhousectl cloud postgres certs get <postgres-id> --output ca.pem
```

Save any initial password and connection string, wait for `state=running`, then connect with `psql` using verified TLS. The [ClickHouse Managed Postgres guide](docs/guides/managed-postgres.md) covers that flow, configuration changes, read-only OAuth queries, and recovery. ClickHouse Managed Postgres commands are beta.

### Ingest with ClickPipes

Use ClickPipes to replicate database changes or load files into ClickHouse Cloud. Both examples require API-key authentication and a running ClickHouse destination.

#### Replicate ClickHouse Managed Postgres into ClickHouse

Start with the `analytics` ClickHouse service and `app-db` ClickHouse Managed Postgres service created above. Use their returned IDs below, and wait until both report `state=running`:

```bash
clickhousectl cloud service get <service-id>
clickhousectl cloud postgres get <postgres-id>
clickhousectl cloud postgres certs get <postgres-id> --output ca.pem
```

Set `POSTGRES_HOST`, `POSTGRES_USERNAME`, and `POSTGRES_PASSWORD` from the ClickHouse Managed Postgres connection details saved at creation. Use the direct Postgres endpoint, not PgBouncer. For this quickstart, use the initial database user; [ClickHouse Managed Postgres already has logical replication enabled](https://clickhouse.com/docs/integrations/clickpipes/postgres/source/managed-postgres).

Create a sample table and a publication using `psql`. The `cloud postgres query` command only supports read-only SQL through OAuth, so it cannot run these setup statements:

```bash
PGPASSWORD="$POSTGRES_PASSWORD" psql \
  "host=$POSTGRES_HOST port=5432 dbname=postgres user=$POSTGRES_USERNAME sslmode=verify-full sslrootcert=ca.pem" \
  -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE public.orders (id bigint PRIMARY KEY, status text NOT NULL);
INSERT INTO public.orders VALUES (1, 'new');
CREATE PUBLICATION clickpipes_demo FOR TABLE public.orders;
SQL
```

Create a ClickPipe to copy the existing rows and continuously replicate inserts, updates, and deletes. The CA file keeps certificate verification enabled:

```bash
clickhousectl cloud clickpipe create postgres <service-id> \
  --name orders-cdc --host "$POSTGRES_HOST" --pg-database postgres \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --ca-certificate ca.pem --publication-name clickpipes_demo \
  --replication-mode cdc --destination-database default \
  --table-mapping public.orders:orders
```

Use the returned ClickPipe ID to check its state, then query the current rows in ClickHouse after the initial load:

```bash
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
clickhousectl cloud service query <service-id> \
  --query "SELECT id, status FROM default.orders FINAL WHERE _peerdb_is_deleted = 0"
```

To check ongoing CDC, run `UPDATE public.orders SET status = 'paid' WHERE id = 1;` in Postgres and repeat the ClickHouse query. Replication is asynchronous, so allow time for the change to arrive.

#### Load files from S3

For a private bucket, set `S3_IAM_ROLE_ARN` to a role configured to let ClickPipes read your objects. Replace the example URL, discover the schema, then choose the columns to ingest. This example expects `event_id` and `event_type` fields in JSONEachRow files:

```bash
S3_URL='https://example-bucket.s3.amazonaws.com/events/**'
clickhousectl cloud clickpipe schema-discover object-storage <service-id> \
  --storage-type s3 --source-url "$S3_URL" --format JSONEachRow \
  --iam-role "$S3_IAM_ROLE_ARN"

clickhousectl cloud clickpipe create object-storage <service-id> \
  --name s3-events --storage-type s3 --source-url "$S3_URL" \
  --format JSONEachRow --iam-role "$S3_IAM_ROLE_ARN" \
  --database default --table s3_events \
  --column 'event_id:Int64' --column 'event_type:String'
```

This starts a one-time load of the matching objects. Check the returned ClickPipe ID and verify the loaded data:

```bash
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
clickhousectl cloud service query <service-id> --query "SELECT count() FROM default.s3_events"
```

See the [ClickPipes guide](docs/guides/clickpipes.md) for source permissions and networking, other sources, continuous S3 ingestion, monitoring, and safe updates.

### Configure ClickStack

Use an existing Managed ClickStack service and a complete dashboard definition:

```bash
clickhousectl cloud clickstack source list <service-id>
clickhousectl cloud clickstack dashboard validate <service-id> --file dashboard.json
clickhousectl cloud clickstack dashboard create <service-id> --file dashboard.json
```

All ClickStack updates replace the complete resource. The [ClickStack guide](docs/guides/clickstack.md) includes a dashboard definition and a review-before-replace workflow.

## Use with coding agents and scripts

Install [official ClickHouse skills](https://github.com/ClickHouse/agent-skills) into the current project, or add `--global` for your home directory:

```bash
clickhousectl skills --agent claude --agent codex
```

The common `.agents/skills/` directory is always included. Run `skills` without flags for interactive selection; unattended installs require `--agent`, `--all`, or `--detected-only`.

Structured commands support `--json` and select it automatically for detected coding agents. SQL and native clients retain command-specific formats. Always check the exit status and distinguish an accepted request from a completed operation. See [automation and output contracts](docs/reference/automation.md).

## Documentation

| Documentation | Covers |
| --- | --- |
| [Local development](docs/guides/local-development.md) | Local ClickHouse and Postgres, versions, configuration, and cleanup |
| [ClickHouse Cloud services](docs/guides/cloud-service.md) | Create, query, scale, back up, and remove services |
| [ClickHouse Managed Postgres](docs/guides/managed-postgres.md) | Connect, query, configure, and recover services |
| [ClickPipes](docs/guides/clickpipes.md) | Load files, replicate databases, verify ingestion, and update pipelines |
| [ClickStack](docs/guides/clickstack.md) | Sources, dashboards, alerts, and safe resource updates |
| [User-defined functions](docs/guides/user-defined-functions.md) | Upload, attach, and update executable UDFs |
| [Authentication and access](docs/reference/authentication.md) | OAuth, API keys, credential precedence, roles, and logout |
| [SQL and Query API](docs/reference/query-api.md) | Key binding, SQL input, output, timeouts, and named endpoints |
| [ClickPipe configuration](docs/reference/clickpipe-configuration.md) | Partial updates, table mappings, source credentials, and private connectivity |
| [Automation and output contracts](docs/reference/automation.md) | JSON output, exit codes, asynchronous operations, and retries |
| [Telemetry](docs/reference/telemetry.md) | Recorded fields, opt-out, and local inspection |
| [Contributing](CONTRIBUTING.md) | Build, checks, and documentation placement |
| [CLI development](docs/development.md) | Code layout, tests, live integration suites, and debugging |
| [Rust API library](crates/clickhouse-cloud-api/README.md) | Typed Cloud API client, usage examples, and API coverage |

## Telemetry

The CLI collects anonymous command usage and bounded failure categories, never argument values, SQL, credentials, or an installation/device ID. The first-run notice sends nothing; collection starts on a following run unless disabled.

```bash
clickhousectl telemetry disable
clickhousectl telemetry status
```

`DO_NOT_TRACK=1` also disables collection. See [telemetry details and controls](docs/reference/telemetry.md).

## Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, checks, and documentation placement, and [CLI development](docs/development.md) for the code layout and test workflows.
