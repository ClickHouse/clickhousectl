# clickhousectl

`clickhousectl` (`chctl`) is the official CLI for ClickHouse and Postgres, locally and in ClickHouse Cloud.

With `clickhousectl` you can:
- Install, run, and query ClickHouse locally
- Run Docker-backed Postgres instances for local development
- Create a ClickHouse Cloud account and authenticate from the terminal
- Create and manage ClickHouse and Postgres services in ClickHouse Cloud
- Run SQL against local and cloud ClickHouse services
- Create and manage ClickPipes for data ingestion (object storage incl. S3, Kafka, Kinesis, Pub/Sub, Postgres, MySQL, MongoDB, BigQuery)
- Install the official ClickHouse agent skills into supported coding agents
- Move local ClickHouse development to ClickHouse Cloud

`clickhousectl` helps humans and coding agents develop with ClickHouse and Postgres.

The workspace also publishes the [typed Rust Cloud API client](crates/clickhouse-cloud-api/README.md). PgBouncer configuration uses an open string map, preserving arbitrary parameters when reading and writing configurations. API key PATCH requests distinguish an omitted expiry (preserve), a timestamp (set), and explicit null (clear). Its latest OpenAPI snapshot adds credit balances, service profile discovery, ClickPipes workload identity context, and expanded ClickPipes and ClickStack models; [CLI exposure is tracked separately](https://github.com/ClickHouse/clickhousectl/issues/699).

## Installation

### Quick install

```bash
curl -fsSL https://clickhouse.com/cli | sh
```

The install script will download the correct version for your OS and install to `~/.local/bin/clickhousectl`. A `chctl` alias is also created automatically for convenience.

### `cargo binstall`

If you already have [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall), this pulls the prebuilt binary from `builds.clickhouse.com`:

```bash
cargo binstall clickhousectl
```

### npm

```bash
npm install -g clickhousectl
```

This installs an npm wrapper package that downloads the matching prebuilt binary from `builds.clickhouse.com` at install time. Both `clickhousectl` and `chctl` are exposed as commands. If you use `npm install --ignore-scripts`, the download is skipped — fall back to one of the other install paths.

### pip

```bash
pip install clickhousectl
# or
pipx install clickhousectl
# or
uv tool install clickhousectl
```

This installs a prebuilt wheel containing the matching `clickhousectl` binary. Linux (glibc and musl, x86_64 and aarch64) and macOS (Intel and Apple Silicon) wheels are published to PyPI.

### From crates.io

Builds from source:

```bash
cargo install clickhousectl
```

### From this repo

```bash
cargo install --path crates/clickhousectl
```

### Direct download

Prebuilt archives for each release are hosted at `https://builds.clickhouse.com/clickhousectl/`. Archives are named `clickhousectl-{target}-v{version}.tar.gz` and contain a single directory of the same name with the `clickhousectl` binary inside. Supported targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.

## Common workflows

This README focuses on common tasks and representative examples. The CLI help is the complete, version-matched command reference: start with `clickhousectl --help`, then use help at any level, such as `clickhousectl cloud postgres --help` or `clickhousectl local server start --help`.

### Local ClickHouse

A bare start bootstraps the local environment, including installing ClickHouse if needed:

```bash
clickhousectl local server start
clickhousectl local client --query "SELECT version()"
```

### Local Postgres

Start Docker-backed Postgres and query it with `psql` through the CLI. A random password is generated unless one is provided:

```bash
clickhousectl local postgres start
clickhousectl local postgres client --query "SELECT version()"
```

### ClickHouse Cloud account and services

Create an account, then authenticate with browser-based OAuth for read-only access or an API key for read/write access:

```bash
# Create a ClickHouse Cloud account
clickhousectl cloud auth signup

# Opens an OAuth login in the browser (read-only)
clickhousectl cloud auth login
# Or use API keys non-interactively (read/write)
clickhousectl cloud auth login --api-key X --api-secret Y
clickhousectl cloud org list
```

Creating or changing Cloud resources requires an API key with the appropriate role. [Create an API key](https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl). You can also export `CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` or add them to your `.env` file.

Create and query a ClickHouse Cloud service:

```bash
clickhousectl cloud service create \
  --name my-clickhouse \
  --provider aws \
  --region us-east-1

# Repeat until the service state is `running`
clickhousectl cloud service get <service-id>

clickhousectl cloud service query \
  --name my-clickhouse \
  --query "SELECT version()"
```

`service create` returns an initial password, which the CLI prints once; store it securely. SQL through `service query` does not require that password.

Create a managed Postgres service:

```bash
clickhousectl cloud postgres create \
  --name my-postgres \
  --provider aws \
  --region us-east-1 \
  --size c6gd.xlarge \
  --pg-version 18

# Repeat until the Postgres service state is `running`
clickhousectl cloud postgres get <postgres-id>

# If create returned a connection string, assign it securely and query with psql
psql "$POSTGRES_CONNECTION_STRING" --command "SELECT version()"
```

`postgres create` returns an initial password, which the CLI prints once; store it securely.

Manage ClickStack data sources, roles, dashboards, alerts, and webhooks for an existing service with JSON configuration files:

```bash
clickhousectl cloud clickstack source list <service-id> --org-id <org-id>
clickhousectl cloud clickstack source create <service-id> \
  --config-file source.json --org-id <org-id>
clickhousectl cloud clickstack role create <service-id> \
  --config-file role.json --org-id <org-id>
clickhousectl cloud clickstack saved-search create <service-id> \
  --config-file saved-search.json --org-id <org-id>
clickhousectl cloud clickstack saved-search get <service-id> <saved-search-id> \
  --org-id <org-id>
clickhousectl cloud clickstack saved-search update <service-id> <saved-search-id> \
  --config-file saved-search.json --org-id <org-id>
clickhousectl cloud clickstack dashboard validate <service-id> \
  --config-file dashboard.json --org-id <org-id>
clickhousectl cloud clickstack dashboard create <service-id> \
  --config-file dashboard.json --org-id <org-id>
```

Pass `--config-file -` to read the JSON body from stdin. Resource IDs come from the respective
`list` command. A saved search configuration contains `name` and `sourceId`, plus optional `select`,
`where`, `whereLanguage`, `orderBy`, `tags`, and structured `filters`; obtain `sourceId` with
`cloud clickstack source list`. All ClickStack `update` commands use PUT replacement semantics, so
the configuration must contain the complete desired resource rather than only changed fields.

A dashboard configuration contains the complete tile layout and typed chart configuration. Filters may
broadcast selections, expose variables to tile queries, or do both:

```json
{
  "name": "Service health",
  "tiles": [
    {
      "name": "Request rate",
      "x": 0,
      "y": 0,
      "w": 6,
      "h": 3,
      "config": {
        "displayType": "line",
        "sourceId": "<source-id>",
        "select": [{ "aggFn": "count" }],
        "formulas": [{ "expression": "A * 60", "alias": "Requests/min" }],
        "showOperandSeries": false
      }
    }
  ],
  "filters": [
    {
      "name": "Service",
      "expression": "ServiceName",
      "sourceId": "<source-id>",
      "type": "QUERY_EXPRESSION",
      "isBroadcastEnabled": true,
      "isVariableEnabled": true,
      "variableName": "service"
    }
  ],
  "savedFilterValues": [
    { "type": "variable", "name": "service", "values": ["api"] }
  ]
}
```

Run `dashboard validate` before create or update to check the same dashboard body without saving it.
`dashboard update <service-id> <dashboard-id> --config-file dashboard.json` is a full PUT replacement:
include every tile, filter, container, tag, and saved query value that should remain.

Create a notification destination first, then reference its ID from an alert. For example,
`webhook.json` can contain a complete generic webhook body:

```json
{
  "name": "Production incidents",
  "service": "generic",
  "url": "https://alerts.example.com/clickstack",
  "description": "Production alert receiver",
  "body": "{\"title\":\"{{title}}\",\"level\":\"{{level}}\"}",
  "headers": { "Authorization": "Bearer <token>" },
  "queryParams": { "team": "platform" }
}
```

```bash
clickhousectl cloud clickstack webhook create <service-id> \
  --config-file webhook.json --org-id <org-id>
clickhousectl cloud clickstack webhook list <service-id> --org-id <org-id>
```

The current Cloud API request contract requires both the legacy `channel` field and the `channels`
array. An alert sourced from a dashboard tile can use this complete `alert.json` body:

```json
{
  "source": "tile",
  "dashboardId": "<dashboard-id>",
  "tileId": "<tile-id>",
  "threshold": 100,
  "thresholdMax": 500,
  "thresholdType": "between",
  "interval": "5m",
  "scheduleOffsetMinutes": 2,
  "scheduleStartAt": "2026-09-05T10:00:00Z",
  "channel": {
    "type": "webhook",
    "webhookId": "<webhook-id>",
    "webhookService": "generic"
  },
  "channels": [
    {
      "type": "webhook",
      "webhookId": "<webhook-id>",
      "webhookService": "generic",
      "severity": "warning"
    },
    { "type": "email", "emailRecipients": ["on-call@example.com"] }
  ],
  "name": "Sustained request failures",
  "message": "Failure count stayed within the configured range",
  "note": "See the production runbook",
  "numConsecutiveWindows": 3
}
```

```bash
clickhousectl cloud clickstack alert create <service-id> \
  --config-file alert.json --org-id <org-id>
clickhousectl cloud clickstack alert get <service-id> <alert-id> --org-id <org-id>
# The detail output includes state, executionErrors, and all notification channels.
clickhousectl cloud clickstack alert update <service-id> <alert-id> \
  --config-file alert.json --org-id <org-id>
```

Alert and webhook updates are full PUT replacements. A `saved_search` alert uses `savedSearchId`
instead of `dashboardId` and `tileId`. The `30s` alert interval is accepted when the 30-second alert
interval feature is enabled for the ClickStack team.

## Local

### Installing and managing ClickHouse versions

`clickhousectl` downloads ClickHouse binaries from `builds.clickhouse.com`, falling back to `packages.clickhouse.com` (Linux) or [GitHub releases](https://github.com/ClickHouse/ClickHouse/releases) (macOS) when a build isn't available there.

```bash
# Manage default version
clickhousectl local use latest              # Latest master build; installs if needed and creates ~/.local/bin/clickhouse
clickhousectl local use stable              # Latest stable release channel
clickhousectl local use lts                 # Latest LTS release channel
clickhousectl local use 26.8                # Latest 26.8.x.x (installs if needed)
clickhousectl local use 26.8.1.1760         # Exact version
clickhousectl local use latest --no-global  # Set default but don't touch ~/.local/bin/clickhouse
clickhousectl local which                   # Show current default

# Install a version
clickhousectl local install latest          # Latest master build
clickhousectl local install 26              # Latest 26.x.x.x
clickhousectl local install 26.8            # Latest 26.8.x.x
clickhousectl local install 26.8.1.1760      # Exact version
clickhousectl local install 26.8.1.1760 --force  # Re-install even if already present

# List versions
clickhousectl local list                    # Installed versions
clickhousectl local list --remote           # Available for download

# Remove a version
clickhousectl local remove 26.8.1.1760
clickhousectl local remove 26.8.1.1760 --force   # Stop running servers on this version (in any project), and remove it even if it is the default
```

`local use` also creates a symlink at `~/.local/bin/clickhouse` pointing to the selected version's binary, so the plain `clickhouse` command (e.g. `clickhouse local`, `clickhouse client`) is on PATH. Pass `--no-global` to skip. If a regular file already exists at that path it is left alone with a warning.

`local remove` refuses to delete a version while a local server is running on it (it would leave the server pointing at a deleted binary), failing with exit `1` and JSON error code `server_running`. Because versions are shared between projects, the check spans **every** project, not just the current directory: the error names each blocking server with the project root it was started from and its PID, so a server found by `clickhousectl local server list --global` is identifiable. Stop those servers first (`clickhousectl local server stop --global <name>`), or pass `--force` to stop them — in whichever project they run — and then remove the version.

`local remove` also refuses the **current default version** (exit `1`, JSON error code `version_is_default`); `--force` removes it anyway and reports `was_default: true`. See `local remove --help`.

#### ClickHouse binary storage

ClickHouse binaries are stored in a global repository, so they can be used by multiple projects without duplicating storage. Binaries are stored in `~/.clickhouse/`:

```
~/.clickhouse/
├── versions/
│   └── 26.8.1.1760/
│       └── clickhouse
└── default              # tracks the active version
```

### Initializing a project

```bash
clickhousectl local init
```

`init` bootstraps your current working directory with a standard folder structure for your ClickHouse project files. It is optional; you are welcome to use your own folder structure if preferred. 

It creates the following structure:

```
clickhouse/
├── tables/                 # Table definitions (CREATE TABLE ...)
├── materialized_views/     # Materialized view definitions
├── queries/                # Saved queries
└── seed/                   # Seed data / INSERT statements
postgres/
├── tables/                 # Table definitions (CREATE TABLE ...)
├── views/                  # View definitions (CREATE VIEW ...)
├── functions/              # Function / procedure definitions (CREATE FUNCTION ...)
├── queries/                # Saved queries
└── seed/                   # Seed data / INSERT statements
```

### Running queries

```bash
# Connect to a running server with clickhouse-client
clickhousectl local client                           # Connects to "default" server
clickhousectl local client --name dev                # Connects to "dev" server
clickhousectl local client --query "SHOW DATABASES"  # Run a query
clickhousectl local client --query "SELECT 1" --query "SELECT 2" # Run queries in order
clickhousectl local client --queries-file schema.sql # Run queries from a file
clickhousectl local client --queries-file schema.sql seed.sql # Run files in order
clickhousectl local client --host remote-host --port 9000  # Connect to a specific host/port
clickhousectl local client --host remote-host         # Direct mode; port defaults to 9000
clickhousectl local client --port 19000               # Direct mode; host defaults to localhost
clickhousectl local client --host remote-host --version 26.8.1.1760  # Use an installed client binary
clickhousectl local client -- --format Pretty        # Extra clickhouse-client args after --
```

`--name` selects the connection and local client binary from managed server metadata, so named mode does not need a global default. It cannot be combined with direct `--host` or `--port` selectors, and named mode does not accept `--version`.

Without `--host` or `--port`, managed client lookup uses `.clickhouse/servers` from the canonical current directory only. It does not search parent directories. If lookup fails, return to the project root that owns the server, inspect that project's servers with `local server list`, or use direct mode.

In direct mode, `--host` and `--port` select the server connection while `--version` independently selects an already installed local client binary. Numeric selectors such as `26`, `26.8`, and `26.8.1.1760` select the newest installed match. This does not install a binary or change `~/.clickhouse/default`.

Without `--version`, direct mode uses the valid default. If no default exists, zero installed versions is an error, one installed version is used without creating a default, and multiple installed versions require either `--version` or `local use`. A default that names a missing binary is an error; repair it with `local use`, or bypass it for one direct connection with `--version`.

`--query` can be repeated, while each `--queries-file` accepts one or more paths and the flag itself can also be repeated. Values, including empty strings, are passed to the native client unchanged and in order. The two options cannot be combined because the native ClickHouse client rejects that combination, so clickhousectl reports a usage error before resolving a binary. Arguments after `--` are appended after all wrapper-generated arguments. Repeatable `--query` requires ClickHouse 23.9.1.1854 or newer, where [ClickHouse added the native behavior](https://github.com/ClickHouse/ClickHouse/blob/8f9a227de1f530cdbda52c145d41a6b0f1d29961/docs/changelogs/archive/v23.9.1.1854-stable.md); clickhousectl checks the selected client version before execution.

### Creating and managing ClickHouse servers

Start and manage ClickHouse server instances. Each server gets its own isolated data directory at `.clickhouse/servers/<name>/data/`.

A bare `clickhousectl local server start` bootstraps from zero: if no version is installed and no default is set, it installs `latest` and starts with it (it does not set a default, so you keep tracking `latest` on subsequent starts). Pin a version with `--version`, or set a default with `local use`, to opt out. Because `latest` tracks the rolling master build, repeat `latest` installs/starts do a cheap `HEAD` against `builds.clickhouse.com` and skip the ~150 MB re-download when master hasn't changed (the build's `etag` is cached in `~/.clickhouse/versions/.master-builds.json`).

```bash
# Canonical named lifecycle
clickhousectl local server start dev
clickhousectl local server list
clickhousectl local server stop dev
clickhousectl local server remove dev

# Other start options (servers run in background by default)
clickhousectl local server start                          # Named "default" (installs latest if nothing is set up yet)
clickhousectl local server start --version latest         # Use a specific version (installs if needed, doesn't change default)
clickhousectl local server start --foreground             # Run in foreground (-F / --fg)
clickhousectl local server start --no-wait                # Return after spawning without waiting for readiness
clickhousectl local server start --http-port 8124 --tcp-port 9001  # Explicit ports
clickhousectl local server start --config analytics       # Apply a custom config (see "Custom config files" below)
clickhousectl local server start -- --logger.level=debug  # Extra clickhouse-server args after --

# List custom config files available to --config
clickhousectl local server configs

# List all servers in this project (ClickHouse and Postgres, running and stopped)
clickhousectl local server list
clickhousectl local server list --global                  # List running ClickHouse servers across all projects

# Stop servers
clickhousectl local server stop                           # Stop "default", or the sole ClickHouse server
clickhousectl local server stop default --global          # Stop from any project
clickhousectl local server stop default --global --project /path/to/project  # Disambiguate
clickhousectl local server stop-all                       # Stop all ClickHouse and Postgres servers in this project
clickhousectl local server stop-all --global              # Stop all ClickHouse servers system-wide

# Remove a stopped server and its data
clickhousectl local server remove                         # Remove "default" only; never guesses a custom name

# Write connection env vars to .env file
clickhousectl local server dotenv                        # From "default" server → .env
clickhousectl local server dotenv --name dev             # From "dev" server → .env
clickhousectl local server dotenv --local                # Write to .env.local instead
clickhousectl local server dotenv --local --user default --database mydb  # Include user and database
clickhousectl local server dotenv --local --user default --password secret  # Include CLICKHOUSE_PASSWORD
```

Stopping a server preserves its data and identity metadata, so it remains visible in `server list` with a `stopped` status. Version and ports are shown only while running because they are resolved again on each start. Starting the same name resumes the existing data directory.

Project-local server commands select `.clickhouse` under the exact current working directory. They do not search parent directories, so running `list`, `stop`, or `remove` from a child directory selects a different project scope. Change to the local project root where the server was started first; this is where `.clickhouse` typically lives. There is intentionally no project-path override for project-local commands; `server stop --global --project <project-root>` is only for an explicitly confirmed server found with `server list --global`.

Without a name, `server stop` selects an existing `default`, then a sole known ClickHouse server. It succeeds without changing anything when none exist, and requires a name or `server stop-all` when multiple non-default servers exist. Bare `server remove` is deliberately stricter: it removes an existing `default` only and otherwise requires an explicit name, even when there is just one custom server.

Version removal and server-data removal are separate operations:

| Command | Removes |
| --- | --- |
| `clickhousectl local remove <exact-version>` | An installed ClickHouse binary from the global version store. |
| `clickhousectl local server remove <server-name>` | A stopped named server and its data from the exact current project. |

**Server naming:** Without a name, the first server is called "default". If "default" is already running, a random name is generated (e.g. "bold-crane"). Pass a name positionally for stable identities you can start/stop repeatedly.

**Ports:** Defaults are HTTP 8123 and TCP 9000. If these are already in use, free ports are automatically assigned and shown in the output. Use `--http-port` and `--tcp-port` to set explicit ports.

**Readiness:** Background starts wait up to 30 seconds for the HTTP health check and TCP port before reporting success, so a following `local client` command can connect immediately. Startup failures point to `.clickhouse/servers/<name>/server.log`. Use `--no-wait` for fire-and-forget startup.

**Orphaned server recovery:** If server metadata files are lost while the ClickHouse process is still running, the CLI automatically recovers them via process discovery. Running `server list`, `server start`, or any server command will detect orphaned processes belonging to the current project and bring them back under management.

**Global server management:** Use `--global` with `list`, `stop`, and `stop-all` to work across all projects; `server list --global` adds a Project column. The reported PID is the watchdog when there is one, and signalling it stops the supervised server too; `stop`/`stop-all` report success only once both processes are gone.

#### Custom config files

Drop ClickHouse config files into `~/.clickhouse/configs/` and apply one by name when starting a server:

```bash
mkdir -p ~/.clickhouse/configs
cat > ~/.clickhouse/configs/analytics.xml <<'EOF'
<clickhouse>
    <query_log>
        <database>system</database>
        <table>query_log</table>
    </query_log>
</clickhouse>
EOF
clickhousectl local server configs                          # List available config files
clickhousectl local server start --config analytics         # Start a server with it
```

The named file is **overlaid on top of ClickHouse's built-in defaults** (it is staged into the server's `config.d/` directory), so it only needs to contain the settings you want to change — you don't have to reproduce a full config. Files may be `.xml`, `.yaml`, or `.yml`; reference them by name with or without the extension (e.g. `--config analytics` or `--config analytics.xml`). `--config` takes a name within `~/.clickhouse/configs/` **not a path**. (`--config-file` remains supported as a legacy alias.)

The managed data directory (`.clickhouse/servers/<name>/data/`) and the HTTP/TCP ports are always forced as command-line overrides, which take precedence over the config file. This means a custom config can never break the managed server lifecycle (`list`, `stop`, `remove`, `dotenv`) regardless of its contents. Starting a server again without `--config` reverts it to plain defaults.

#### Local Postgres (Docker-backed)

When you also need a local Postgres alongside ClickHouse — e.g. for testing CDC pipelines or ingesting from Postgres — use `local postgres`. Each instance is keyed on `(name, major version)` so the same name can host multiple Postgres majors with isolated data: data lives at `.clickhouse/servers/<name>-pg<major>/data/`, metadata at `.clickhouse/servers/<name>-pg<major>.json`, and the container is `clickhousectl-pg-<name>-<major>`. ClickHouse paths (`<name>/data/`, `<name>.json`) stay separate, so a name can be used by both engines. Requires Docker to be installed and running.

```bash
# Pre-pull a Postgres image (optional; start will pull on demand). Supported: 17, 18 (and any sub-tag like 17-alpine, 17.0, 18-bookworm).
clickhousectl local install postgres@17

# Start a Postgres instance (defaults: postgres:18, port 5432 or a free port, user "postgres", db "postgres")
clickhousectl local postgres start
clickhousectl local postgres start --name dev --version 17 --port 5433
clickhousectl local postgres start --user app --database myapp  # Generates a random password
clickhousectl local postgres start -e POSTGRES_INITDB_ARGS=--data-checksums
clickhousectl local postgres start --wait-timeout 120            # Default: 60s; maximum: 600s

# List everything (ClickHouse + Postgres are merged in `server list`)
clickhousectl local server list

# Connect with psql
clickhousectl local postgres client --name dev
clickhousectl local postgres client --name dev --query "SELECT 1"
clickhousectl local postgres client --name dev --queries-file schema.sql  # Run a SQL file (psql -f)
clickhousectl local postgres client --name dev --version 17               # Disambiguate two majors
clickhousectl local postgres client --host remote-host       # Direct mode; port defaults to 5432
clickhousectl local postgres client --port 55432             # Direct mode; connects locally

# Write POSTGRES_HOST/PORT/USER/PASSWORD/DATABASE into .env.local
clickhousectl local postgres dotenv --name dev --local

# Stop / remove. Pass --version when more than one major shares a name.
clickhousectl local postgres stop                         # Stop "default"
clickhousectl local postgres stop dev
clickhousectl local postgres stop dev --version 17        # disambiguate
clickhousectl local postgres stop-all                     # Stop all Postgres instances in this project
clickhousectl local postgres remove                       # Remove "default"
clickhousectl local postgres remove dev
```

Postgres `--name` and `--version` select a managed instance and cannot be combined with direct `--host` or `--port` selectors.

The Postgres `dotenv` command includes the generated password. Do not commit its output; prefer `--local` when your application reads `.env.local`.

`--env` accepts each valid `KEY=VALUE` key once. `POSTGRES_USER`, `POSTGRES_DB`, and `PGDATA` are generated by clickhousectl and cannot be supplied through `--env`; use `--user` or `--database` for the first two. For compatibility, `-e POSTGRES_PASSWORD=...` remains an alternative to `--password`, but combining the two or repeating `POSTGRES_PASSWORD` is an error. This guarantees that every generated variable appears exactly once in the container environment.

`local postgres start --name dev` (no `--version`) resumes the existing instance when there's exactly one for that name; if multiple majors share the name, the command exits and asks you to pass `--version`. A resume reuses the stored settings, so `--port`, `--user`, `--password`, `--database` and `-e` have no effect on a resumed instance and `start` prints a note to stderr when you pass them. They are still validated first, so an explicitly requested port that is already in use aborts the resume with exit `1` (`port_in_use`), and a malformed `-e`/`--password` exits `2`. Run `local postgres remove <name>` then `start` to change them. Stop preserves the container and metadata so the next start resumes it; only `remove` tears down the container and deletes the data directory. The unified `local server stop-all` stops both ClickHouse and Postgres instances in the current project; the dedicated `local postgres stop-all` remains available when only Postgres should be stopped.

Fresh and resumed starts wait until `pg_isready` reports that PostgreSQL is accepting connections inside the container. The readiness timeout defaults to 60 seconds and can be set from 1 to 600 seconds with `--wait-timeout`. A timeout or early container exit fails the command and prints a bounded tail of the container logs instead of connection credentials. A failed fresh startup removes the newly created container, metadata, and PGDATA created by that attempt only when rollback completes. Pre-existing PGDATA is preserved, and recovery metadata is retained whenever cleanup is incomplete. A failed resume stops the existing container but preserves its metadata and data.

Containers are tagged with `clickhousectl.engine=postgres`, `clickhousectl.name=<name>`, `clickhousectl.major=<major>`, `clickhousectl.project=<cwd>`, and `created_by=clickhousectl_<version>` labels. `server list` recovers orphaned containers belonging to the current project via these labels, so deleting `.clickhouse/servers/<name>-pg<major>.json` is non-destructive — the next list/start rediscovers it.

#### Project-local data directory

All project-local server data lives inside `.clickhouse/` in your project directory. The example below shows ClickHouse entries; Postgres uses the versioned paths described above.

```
.clickhouse/
├── .gitignore              # auto-created, ignores everything
├── credentials.json        # cloud API credentials (if configured)
└── servers/
    ├── default.json         # ClickHouse identity and runtime state
    ├── default/
    │   └── data/           # ClickHouse data files for "default" server
    ├── dev.json             # ClickHouse identity and runtime state
    └── dev/
        └── data/           # ClickHouse data files for "dev" server
```

Each named server has its own data directory, so servers are fully isolated from each other. Data persists between restarts — stop and start a server by name to pick up where you left off. Use `clickhousectl local server remove <name>` to permanently delete a server's data.

## Cloud authentication and account creation

Authenticate to ClickHouse Cloud using OAuth (browser-based) or API keys. OAuth provides **read-only** access. Write operations require an API key; its effective permissions depend on its assigned roles.

If you don't have a ClickHouse Cloud account yet, `clickhousectl cloud auth signup` opens the sign-up page in your browser.

### OAuth login (read-only)

```bash
clickhousectl cloud auth login
```

This opens your browser for authentication via the OAuth device flow. Tokens are saved to `~/.clickhouse/tokens.json` (global, shared across all directories).

> **Note:** OAuth tokens provide **read-only** access. You can list and inspect resources (organizations, services, backups, etc.) but cannot create, modify, or delete them. For write operations, use API key authentication. `cloud service query` works under OAuth too, running SQL as your own identity with **read-only** access — see [Query API auth modes](#query-api-auth-modes).

### API key/secret (required for write operations)

```bash
# Save credentials locally without putting the secret in shell history
clickhousectl cloud auth login --interactive
```

`auth login --interactive` saves credentials to `.clickhouse/credentials.json` (project-local). API keys are org-scoped, so they stay per-project; OAuth tokens represent your user identity and are stored globally in `~/.clickhouse/tokens.json`.

For CI and other automation, inject credentials through your secret manager:

```bash
export CLICKHOUSE_CLOUD_API_KEY=your-key
export CLICKHOUSE_CLOUD_API_SECRET=your-secret
```

Environment credentials remain in the environment and are not saved by `clickhousectl`.

For local development, you can instead place them in a `.env` file, which is read only from the current working directory:

```env
CLICKHOUSE_CLOUD_API_KEY=your-key
CLICKHOUSE_CLOUD_API_SECRET=your-secret
```

Do not commit `.env`; add it to `.gitignore` and restrict its file permissions. Credential flags are also available for one-off use, but secrets passed in command arguments may be exposed through shell history or process listings.

Learn how to [create API keys](https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl).

### Auth status and logout

```bash
clickhousectl cloud auth status    # Show current auth state (including read-only/read-write labels)
clickhousectl cloud auth logout    # Clear all saved credentials (credentials.json & tokens.json)
clickhousectl cloud auth logout --oauth      # Clear only OAuth tokens, keep API keys
clickhousectl cloud auth logout --api-keys   # Clear only API keys, keep OAuth tokens
```

Both forms that clear API keys delete `.clickhouse/credentials.json` entirely, including the per-service Query API key records under `service_query_keys`. Delete the cloud-side keys first (`cloud service delete`, or `cloud key delete <key-id>`) or their IDs are lost.

Credential resolution order:
1. CLI flags
2. `.clickhouse/credentials.json`
3. Environment variables exported in your session
4. Environment variables from `.env`
5. OAuth tokens.

When environment credentials are configured but a credentials file or explicit
CLI flags take precedence, clickhousectl prints a one-line note to stderr.
`cloud auth status` also marks the environment credentials as configured but
inactive and identifies the source that outranked them.

### Debugging which credential source was used

Pass `--debug` to a Cloud resource command to print the resolved credential source (and the API URL) to stderr before the command runs. This works with and without `--json`.

```bash
clickhousectl cloud --debug service list
# [debug] auth source: credentials file (.clickhouse/credentials.json)
# [debug] api url: https://api.clickhouse.cloud/v1
# ... normal output ...

# Target a non-production control plane (env: CLICKHOUSE_CLOUD_QUERY_HOST for the Query API host)
clickhousectl cloud --url https://api.control-plane.example.com service list
```

## Cloud

Manage ClickHouse, Postgres, and other ClickHouse Cloud resources via the API.

Reading a service, Postgres service or organization — or deleting a service — by an identifier that resolves to nothing reports `No such <resource>: <id> (organization <org-id>). The API rejected the identifier: <server text>`, and the stable code `resource_not_found` under `--json`. `org get` omits the `(organization ...)` clause. Every other resource relays the API's own error, so do not branch on `resource_not_found` for a ClickPipe, key, member, backup or endpoint. A malformed (non-UUID) identifier keeps the API's own `invalid` message.

### Organizations

```bash
clickhousectl cloud org list              # List organizations
clickhousectl cloud org get <org-id>      # Get organization details
clickhousectl cloud org quota list --org-id <org-id>
clickhousectl cloud org quota get services-per-organization --org-id <org-id>
clickhousectl cloud org balance --org-id <org-id>  # Active trial and prepaid credits
clickhousectl cloud org update <org-id> --name "Renamed Org"
clickhousectl cloud org update <org-id> \
  --remove-private-endpoint pe-1,cloud-provider=aws,region=us-east-1 \
  --enable-core-dumps false
clickhousectl cloud org prometheus --filtered-metrics true
clickhousectl cloud org prometheus discovery --filtered-metrics false
clickhousectl cloud org usage \
  --from-date 2024-01-01 \
  --to-date 2024-01-31 \
  --filter tag:Environment=Production   # max 31-day window (to-date inclusive), costs in CHC
# Org quota, balance, prometheus, and usage commands auto-detect the org when --org-id is omitted.
# Org list takes no ID; org get/update take a positional <org-id>.
# It is auto-detected only when your credentials reach exactly one organization.
# Organization quota and balance commands are beta and read-only, so they support OAuth.
```

`cloud org prometheus discovery` returns the beta HTTP service-discovery target groups used by Prometheus `http_sd_configs`; `--json` preserves the complete target and label array. The command defaults discovered scrape targets to filtered metrics. The command without `discovery` still calls the deprecated organization metrics endpoint and emits raw Prometheus exposition text for compatibility.

### Services

```bash
# List services
clickhousectl cloud service list

# Get service details
clickhousectl cloud service get <service-id>

# Create a service with explicit placement and network access
# Omitting --ip-allow creates the service with an "Allow all" 0.0.0.0/0 access list
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-public-ip>/32

# Create with scaling options
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-public-ip>/32 \
  --min-replica-memory-gb 8 \
  --max-replica-memory-gb 32 \
  --num-replicas 2 \
  --idle-scaling true --idle-timeout-minutes 10

# Create with specific IP allowlist
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow '<trusted-egress-cidr>=office' \
  --ip-allow '<another-trusted-egress-cidr>=CI runners'

# Create from backup
clickhousectl cloud service create --name restored-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-public-ip>/32 \
  --backup-id <backup-uuid>

# Create with release channel
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-public-ip>/32 \
  --release-channel fast

# Create with GA request-only extras
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-public-ip>/32 \
  --tag env=prod \
  --enable-endpoint mysql \
  --private-preview-terms-checked \
  --enable-core-dumps true

# Create a read-only replica in an existing data warehouse
clickhousectl cloud service create --name my-replica \
  --data-warehouse-id <warehouse-id> --readonly

# Enterprise: instance profile, compliance, customer-managed disk encryption, TDE
clickhousectl cloud service create --name my-service \
  --profile v1-highmem-xs --compliance-type hipaa \
  --encryption-key <kms-key-arn> --encryption-role <kms-role-arn> --enable-tde

# Start, wake, or stop a service
clickhousectl cloud service start <service-id>
clickhousectl cloud service wake <service-id>   # Explicitly wake an idled service
clickhousectl cloud service stop <service-id>

# Run SQL over HTTP via the Query API (no local clickhouse binary needed)
clickhousectl cloud service query --name my-service --query "SELECT 1"
clickhousectl cloud service query --id <service-id> --query "SELECT count() FROM system.tables" --format JSONEachRow
clickhousectl cloud service query --name my-service --queries-file query.sql   # single statement only; "-" reads from stdin
clickhousectl cloud service query --name my-service --database mydb --query "SHOW TABLES"
echo "SELECT 1+1" | clickhousectl cloud service query --name my-service
clickhousectl cloud service query --name my-service --query "SELECT 1" --no-auto-enable
# Loading a CSV: see the stdin INSERT example below

# Deliberately replace clickhousectl's stored Query API key for exactly one
# service (the way forward after a disabled, expired, unbound or IP-restricted key)
clickhousectl cloud service repair-query-key <service-id> --org-id <org-id>

# Update service metadata and patches
clickhousectl cloud service update <service-id> \
  --name my-renamed-service \
  --add-ip-allow '<trusted-egress-cidr>=office' \
  --remove-ip-allow 0.0.0.0/0 \
  --add-private-endpoint-id pe-1 \
  --release-channel fast \
  --enable-endpoint mysql \
  --add-tag env=staging \
  --transparent-data-encryption-key-id tde-key-1 \
  --enable-core-dumps false
clickhousectl cloud service update <service-id> \
  --disable-endpoint mysql \
  --remove-private-endpoint-id pe-1 \
  --remove-tag legacy
# --remove-* flags are idempotent: an entry that matched nothing still exits 0,
# with one stderr warning per miss (tags match by key).

# Update replica scaling (vertical autoscaling — fixed replica count, variable memory)
clickhousectl cloud service scale <service-id> \
  --min-replica-memory-gb 24 \
  --max-replica-memory-gb 48 \
  --num-replicas 3 \
  --idle-scaling true \
  --idle-timeout-minutes 10

# Horizontal autoscaling — fixed memory per replica, variable replica count
# (requires the horizontal autoscaling org feature)
clickhousectl cloud service create --name my-service \
  --provider aws --region us-east-1 --ip-allow <trusted-public-ip>/32 \
  --min-replica-memory-gb 24 --max-replica-memory-gb 24 \
  --min-replicas 2 --max-replicas 8 --autoscaling-mode horizontal
clickhousectl cloud service scale <service-id> \
  --min-replica-memory-gb 24 --max-replica-memory-gb 24 \
  --min-replicas 2 --max-replicas 8 --autoscaling-mode horizontal

# Reset password with generated credentials
clickhousectl cloud service reset-password <service-id>

# Reset password with precomputed hashes
clickhousectl cloud service reset-password <service-id> \
  --new-password-hash <base64-sha256-hash> \
  --new-double-sha1-hash <mysql-double-sha1-hash>

# Query endpoint management (manual, for sharing keys with other tools)
clickhousectl cloud service query-endpoint get <service-id>
clickhousectl cloud service query-endpoint create <service-id> \
  --role sql_console_read_only \
  --open-api-key <api-key-id> \
  --allowed-origins https://app.example.com
clickhousectl cloud service query-endpoint delete <service-id>

# Private endpoint management
clickhousectl cloud service private-endpoint create <service-id> \
  --endpoint-id vpce-0123456789abcdef0 --description 'app vpc'
clickhousectl cloud service private-endpoint get-config <service-id>

# Backup configuration
clickhousectl cloud service backup-config get <service-id>
clickhousectl cloud service backup-config update <service-id> \
  --backup-period-hours 24 \
  --backup-retention-period-hours 720 \
  --backup-start-time 02:00

# Remove the start time again, optionally changing the period in the same call
clickhousectl cloud service backup-config update <service-id> \
  --clear-backup-start-time \
  --backup-period-hours 12

# Service Prometheus configuration (always raw Prometheus exposition text; --json is ignored)
clickhousectl cloud service prometheus <service-id> --filtered-metrics true

# Delete a service (must be stopped first)
clickhousectl cloud service delete <service-id>

# Force delete: stops a running service then deletes
clickhousectl cloud service delete <service-id> --force
```

IP allowlist flags accept `IP_OR_CIDR` or `IP_OR_CIDR=DESCRIPTION`. The `=`
delimiter is safe with IPv6; quote entries whose descriptions contain spaces.

`query-endpoint create` adds and deduplicates API keys while preserving existing browser origins. `--role` is required and replaces the endpoint-wide roles for **all** authorized keys; roles are not assigned per key.
Pass `--allowed-origins` on first creation or to change browser access (`'*'` explicitly allows every origin). Use `--replace-open-api-keys` with `--open-api-key` to deliberately replace the entire authorized-key list.
The command reads the existing configuration before updating; a failed or incomplete read prevents changes to unknown fields. Avoid concurrent changes to the same query endpoint.


`--backup-start-time` requires the backup period to be 24 or 48 hours. Nothing is defaulted when the period is omitted: the API validates the new start time against the period already stored on the service, so either pass `--backup-period-hours 24` or `--backup-period-hours 48` in the same call, or leave the stored period at one of those. When a start time is given without a period, the CLI reads the current configuration first and fails before sending the update if the stored period is something else.

`--clear-backup-start-time` removes a stored start time and lifts that restriction, so a service that has one can go back to any backup period. It sends an explicit `"backupStartTime": null`, which the API accepts even though the OpenAPI spec does not mark the field nullable, and it can be combined with `--backup-period-hours` to clear the start time and set an otherwise incompatible period in a single call. The two start-time flags conflict: pass one or the other.

`--force` stops the service and then polls it until the stop completes, which takes minutes on a real service, printing each state change to stderr (stdout stays reserved for the result). Progress output is best-effort: if whatever was reading it goes away — a pager you quit, a supervising process that stopped reading — the lines are dropped and the deletion still runs to completion and exits 0.

Use `clickhousectl cloud service create --help` for the complete option list. If omitted, `--provider` defaults to `aws`, `--region` defaults to `us-east-1`, and the IP allowlist defaults to `0.0.0.0/0`; production workflows should normally set all three explicitly. When the create response includes an initial password, it is shown only once.

`--query` and `--queries-file` are mutually exclusive. If neither is supplied, `cloud service query` reads SQL from stdin; `--queries-file -` also reads stdin explicitly.

An explicit `--format` wins over agent auto-JSON.

`--query` never reads stdin, so `--query "INSERT INTO trips FORMAT CSV" < data.csv` is refused (exit code `1`) before any request is sent rather than silently inserting nothing. The Query API takes a single request body, so a statement and a separate data stream cannot both be sent; pipe them together instead:

```bash
printf 'INSERT INTO trips FORMAT CSV\n' | cat - data.csv | \
  clickhousectl cloud service query --id <service-id>
```

Only real input counts as a conflict: an empty non-terminal stdin (a CI runner, a coding agent) leaves `--query` working, and a silent pipe is given 250 ms to produce a byte before stdin is treated as empty.

The Query API gateway stops waiting after about 30 seconds. When it does, the request fails (exit code `1`) but **the statement keeps running on the service** — only the HTTP response is lost. The error says so, points at `SELECT query_id, elapsed FROM system.processes` so a still-running statement is not started a second time, and prints the `clickhouse client` command for the service's own native endpoint. Nothing is retried automatically: re-sending a large `INSERT` would load the data twice.

For anything that may run longer than that — large `INSERT`s, backfills, `url()` loads — use the native protocol from the start:

```bash
clickhousectl local use latest   # puts the standard `clickhouse` binary on PATH
clickhouse client --host <nativesecure-host> --secure --port 9440 \
  --user default --password '<password>' --query '<your SQL>'
```

`clickhousectl cloud service get <service-id>` lists the service endpoints, including the `nativesecure` host and port. The password is the one shown when the service was created; `clickhousectl cloud service reset-password <service-id>` issues a new one.

Under `--json` (or when a coding agent is detected) that failure is emitted as one JSON object on stderr, in the same envelope local errors use:

```json
{
  "error": {
    "code": "query_timeout",
    "message": "the query timed out at the Query API gateway, ...",
    "host": "my-service.us-east-1.aws.clickhouse.cloud",
    "port": 9440,
    "command": "clickhouse client --host my-service.us-east-1.aws.clickhouse.cloud --secure --port 9440 --user default --password '<password>' --query '<your SQL>'"
  }
}
```

`code` is a stable machine-readable identifier. `host` and `port` are omitted when the API response carried no native endpoint, in which case `command` names the `<host>` placeholder instead. The suggested command never contains your SQL or your password: both are placeholders. Cloud failures that have no structured remedy stay prose on stderr (`Error: <message>`).

Whatever the source, the SQL must be a single statement. The Query API runs exactly one statement per request, so a multi-statement `.sql` script is rejected by ClickHouse (error 62, `Multi-statements are not allowed`). Run statements one invocation at a time, or put a real client on PATH with `clickhousectl local use latest` and run the script through `clickhouse client` connected to the service.

Private endpoint IDs supplied to `private-endpoint create --endpoint-id` and `service update --add-private-endpoint-id` are format-checked before the request is sent, because adding one registers it for the whole organization and a typo has to be unpicked from both the service and the organization. Each provider uses its own format — AWS a `vpce-` VPC endpoint ID, GCP the numeric Private Service Connect connection ID, Azure the private endpoint Resource ID or `resourceGuid` — and the provider is not known when the flag is parsed, so only provider-independent mistakes are rejected (exit code `2`): empty values, values containing whitespace, and any value carrying `vpce-` that is not exactly a well-formed AWS VPC endpoint ID (`vpce-` plus 8 or 17 lowercase hex characters) — which also catches a pasted VPC endpoint ARN or endpoint service name. Azure Resource IDs (values starting with `/`) are exempt from that check, since an Azure resource may itself be named `vpce-...`. Whether the endpoint actually exists and belongs to you is not validated by the CLI or, currently, by the Cloud API. Removal flags are never format-checked, so an already-registered bogus ID stays removable.

#### Query API auth modes

`cloud service query` is the canonical way to run SQL against a cloud service — over HTTP, with no `clickhouse` binary and no service password required. It works with both credential modes:

- **API key auth** (read + write SQL): when no per-service key is stored, `cloud service query` first uses the authenticated API key directly. This supports services whose Query API endpoint already authorizes that key without requiring permission to create another key. If the key or endpoint is not authorized, the CLI provisions a dedicated API key and binds it to the service with role `sql_console_admin`. Those generated query credentials, the endpoint ID, exact management API key ID, and provisioning organization ID are stored in `.clickhouse/credentials.json` under `service_query_keys.<service-id>`, alongside any user-level API key. Subsequent queries use that key. The generated key is scoped to a single service, so it can read and write (SELECT, INSERT, DDL) against that service but cannot reach any other service in the org. Pass `--no-auto-enable` to fail instead of provisioning.
- **OAuth** (`cloud auth login`): the query runs as your own identity — the CLI sends your bearer token straight to the Query API, which grants **read-only** SQL access (SELECT and other read statements only; no INSERT, DDL, or other writes). No Query API key is provisioned or stored, and no query endpoint needs to be configured on the service. Use API key auth if you need to write. `--no-auto-enable` has no effect in this mode.

Provisioning happens lazily (rather than at `service create` time) because the endpoint can only be bound once the service has finished provisioning, which can take several minutes — `service create` returns immediately instead of blocking on it.

Provisioning is single-flight per project directory; provisioning the same service concurrently from two different project directories can still lose a binding.

The API key itself has no org-level roles, so the binding is the only thing that grants it any access. After deleting a service, `cloud service delete` deletes the auto-provisioned key by its stored management and organization IDs, along with any retired key still listed under `pending_cleanup_api_key_ids`, then removes the local record. Every key is attempted even if one fails; on failure the command exits non-zero naming the keys that remain and keeps the local record so their IDs are not lost. Legacy records without that metadata remain readable, but service deletion will not guess at a cloud key by name; a partial record with a management ID is retained for manual recovery.

If a query with a stored per-service key receives HTTP 401/403, the CLI does not read the rejection as proof that the local secret is stale: an administrator may equally have disabled the key, let it expire, unbound it from the endpoint, or narrowed its IP access list, and replacing the key would undo that decision. Before anything is touched, the CLI reads the key's management record (by the stored organization and management key ID) and, when the key is still enabled, the service's Query API endpoint binding, then classifies the rejection. No verdict changes anything, locally or in the organization; each one names the key ID, the reason, and the deliberate way forward:

- **Key deleted** (the management API returns 404): the stored secret can never work again, but the record is kept and nothing is changed. Deleting the key does not remove its UUID from the endpoint's `openApiKeys`, so the binding is stale too; `repair-query-key` replaces the key and drops that binding in the same upsert, and treats the already-deleted key as retired. Rerunning the query fails the same way until then.
- **Key disabled**, **expired**, or **not bound** to the endpoint (or the service has no endpoint at all): the record is kept, nothing is created or rebound, and the error names the key ID, the reason, and the way forward. A disabled key can be re-enabled with `cloud key update <key-id> --state enabled`; any of these keys can be replaced deliberately with `repair-query-key`.
- **Key enabled, unexpired and bound, yet still rejected**: either the key's IP access list does not cover this machine or the stored secret no longer matches the key. The error prints the access list (CIDRs only) and points at `cloud key update --ip-allow` for the former and `repair-query-key` for the latter. Nothing is changed.
- **Lookup failed** (network error, 5xx, or management credentials that cannot read keys), or a legacy record without a management key ID: the rejection cannot be classified, so nothing is changed and the error says how to retry.

In `--json` mode the failure is one object on stderr with a stable `code` (`query_key_deleted`, `query_key_disabled`, `query_key_expired`, `query_key_unbound`, `query_key_rejected`, `query_key_unverified`), the `api_key_id`, a recovery `command` where one is safe to suggest, and, for `query_key_rejected`, the `ip_access_list`. No path prints the stored secret.

Repair is an explicit API-key-authenticated write operation, and the only way a disabled, expired, unbound or IP-restricted key is ever replaced. It verifies the stored organization, management key ID, and endpoint ID, replaces only that key ID in the endpoint binding, and preserves every other binding and project credential. Concurrent repairs in the same project reuse the first process's replacement instead of rotating it again. Legacy or incomplete records without exact ownership metadata are refused.

A newly created key can take a few seconds to become visible to the endpoint; clickhousectl retries the binding for up to 30 seconds, then fails with the API's own error and rolls the binding back.

The new binding also takes a moment to reach the Query API host, and a query issued in between is rejected. Both first-use provisioning and `repair-query-key` wait for the endpoint to accept the new key, and the repair result reports the outcome under `verification`: `verified`, `skipped` (the key was not probed) or `failed` (the probe failed for a reason unrelated to readiness). Skipped and failed exit 0, print one `Note:` or `Warning:` line on stderr, and leave the next `cloud service query` to verify the key. Only a key the Query API keeps rejecting for the whole readiness window (about two minutes) exits 1, and even then the repair stands: the result is printed with `verification: failed`, followed by an error with code `query_key_repair_unverified`. Do not rerun `repair-query-key` in that case: it would rotate a key that may only be slow to propagate.

A repair also retires the key it replaced and deletes it best-effort; a failed deletion is reported (as `pendingCleanupApiKeyIds` under `--json`) and retried by the next query. Only keys the CLI itself created are ever deleted, identified by the exact management key IDs in the stored record. Delete one by hand with `cloud key delete <key-id>`.

Do not modify the same query endpoint concurrently with a repair, and let a first-use query finish provisioning before running one.

Querying an **idled** service wakes it automatically in both auth modes — under OAuth the Query API first asks for a wake confirmation, which the CLI sends after printing a notice to stderr (the first query may take a minute while the service wakes). A **stopped** service is never woken: the query fails with a hint to run `cloud service start`.

The Query API host is derived from the API base URL per environment (`api.[control-plane.]<domain>` → `queries.<domain>`, e.g. `https://queries.clickhouse.cloud` for production). Set `CLICKHOUSE_CLOUD_QUERY_HOST` to override it.

### Postgres (beta)

Manage ClickHouse Cloud managed Postgres services. All write commands require API key auth.

```bash
# List / get
clickhousectl cloud postgres list
clickhousectl cloud postgres list --filter state=running
clickhousectl cloud postgres list --filter region=us-east-1 --filter isPrimary=true
clickhousectl cloud postgres get <pg-id>

# Create
clickhousectl cloud postgres create \
  --name my-pg \
  --provider aws \
  --region us-east-1 \
  --size c6gd.xlarge \
  --pg-version 18

# Create with HA + tags + advanced config
clickhousectl cloud postgres create \
  --name my-pg \
  --provider aws \
  --region us-east-1 \
  --size c6gd.xlarge \
  --pg-version 18 \
  --ha-type sync \
  --tag env=prod \
  --pg-config-file ./pg.json \
  --pg-bouncer-config-file ./pgbouncer.json

# Update name, size, HA, or tags (all flags optional)
clickhousectl cloud postgres update <pg-id> \
  --name renamed-pg \
  --size c6gd.4xlarge \
  --ha-type sync \
  --add-tag env=prod --remove-tag legacy
clickhousectl cloud postgres update <pg-id> --clear-tags

# --clear-tags replaces the tag list with an empty list and conflicts with
# --add-tag and --remove-tag; omitting all three leaves tags unchanged.

# Delete (works from any state, including running; no stop needed first)
clickhousectl cloud postgres delete <pg-id>

# CA certificates
clickhousectl cloud postgres certs get <pg-id>                   # raw PEM to stdout
clickhousectl cloud postgres certs get <pg-id> --output ca.pem   # file (mode 0600 on unix)

# Runtime configuration (`config get` always prints JSON; --json changes nothing)
clickhousectl cloud postgres config get <pg-id>
clickhousectl cloud postgres config patch <pg-id> --set max_connections=500 --set random_page_cost=1.1
clickhousectl cloud postgres config patch <pg-id> --file patch.json

# Replace the entire configuration only with a complete object obtained from `config get`
clickhousectl cloud postgres config replace <pg-id> --file complete-config.json

# Password
clickhousectl cloud postgres reset-password <pg-id> --generate
clickhousectl cloud postgres reset-password <pg-id> --password '<min-12-upper-lower-digit>'

# Read replica and PITR restore
clickhousectl cloud postgres read-replica create <pg-id> --name replica-1
clickhousectl cloud postgres read-replica create <pg-id> --name replica-2 \
  --tag env=prod --pg-config-file ./pg.json
clickhousectl cloud postgres restore <pg-id> \
  --name restored \
  --restore-target <recent-RFC3339-time-within-retention> \
  --tag env=prod --pg-bouncer-config-file ./pgbouncer.json

# Lifecycle
clickhousectl cloud postgres restart <pg-id>
clickhousectl cloud postgres promote <replica-id>
clickhousectl cloud postgres promote <replica-id> --wait                    # poll until isPrimary=true
clickhousectl cloud postgres switchover <primary-id>
clickhousectl cloud postgres switchover <primary-id> --wait --wait-timeout 600
```

PgBouncer files passed to `--pg-bouncer-config-file` on create, read-replica create, and restore are JSON objects with string values, for example:

```json
{"default_pool_size":"16","pool_mode":"transaction"}
```

For `postgres config patch --file`, put that map under `pgBouncerConfig` alongside `pgConfig`:

```json
{"pgConfig":{},"pgBouncerConfig":{"default_pool_size":"16"}}
```

PgBouncer parameter names are open-ended; values must be quoted strings, including numbers. Invalid value types fail locally before any API request. `config replace` replaces the complete Postgres and PgBouncer configuration: obtain the current document with `config get`, edit it, and retain both sections and every setting you want to keep.

`pgConfig` uses the closed set of GUC names supported by the Cloud API. Unknown names and `null` values are rejected locally on `--set` and every PgConfig file path, and the enum-valued settings accept only `default_transaction_isolation` (`read committed`, `repeatable read`, `serializable`), `ssl_min_protocol_version` (`TLSv1` through `TLSv1.3`), and `wal_compression` (`off`, `on`, `lz4`, `zstd`). Files for `config patch` and `config replace` must contain both `pgConfig` and `pgBouncerConfig`; use an explicit `{}` when a section is intentionally empty rather than omitting it.

Use `clickhousectl cloud postgres create --help` for the complete option list. Save any initial password and connection string in the create response because later `postgres get` responses do not return credentials. If both are omitted, run `clickhousectl cloud postgres reset-password <postgres-id> --generate`.

`postgres list --filter KEY=VALUE` is applied client-side to the listing and is repeatable; every filter must match. Supported keys are `state`, `region`, `name`, `provider` and `isPrimary` (the `Primary` column; `true`/`false`, or the `yes`/`no` the column shows). Keys are case-insensitive, `state` and `provider` match the wire value case-insensitively, and `region`/`name` match exactly. An unknown key, a missing `=` or an empty value is a usage error (exit 2) listing the valid keys — it never returns an unfiltered list. A field the API omitted matches no filter value, so filtering on it excludes that service. This is unrelated to `cloud service list --filter`, which sends server-side resource-tag filters (`tag:env=production`) to the API.

`postgres promote` and `postgres switchover` change which service is primary. Both are issued as-is, and the API acknowledges them before (or without) applying them, so exit 0 on its own means accepted, not applied:

- `--wait` (optionally `--wait-timeout SECONDS`, default 300) is how you confirm the roles actually changed. It polls the target every 5s until it reports the expected `isPrimary` — `true` for `promote`, the opposite of the value read just before the command for `switchover` — and exits 1 with the last observed role if it never does. stdout then carries the polled state rather than the state-change response, which for `promote` omits `isPrimary` entirely. Without `--wait` neither command reads the service. A `switchover --wait` whose pre-command read omits `isPrimary` is refused before the command is issued, because there is no prior role to compare a swap against.
- The previous primary is demoted asynchronously and can keep reporting `isPrimary=true` for minutes afterwards. No client can see that pair from one service, so `promote` always reports the dual-primary window on stderr; verify with `clickhousectl cloud postgres list --filter isPrimary=true` that exactly one service is primary.

### Backups

```bash
clickhousectl cloud backup list <service-id>
clickhousectl cloud backup get <service-id> <backup-id>
```

### ClickPipes

Manage ClickPipes for ingesting data into ClickHouse Cloud from external sources. See the [ClickPipes documentation](https://clickhouse.com/docs/integrations/clickpipes).

```bash
# List ClickPipes for a service
clickhousectl cloud clickpipe list <service-id>

# Get ClickPipe details
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>

# Start/stop/resync a ClickPipe
clickhousectl cloud clickpipe start <service-id> <clickpipe-id>
clickhousectl cloud clickpipe stop <service-id> <clickpipe-id>
clickhousectl cloud clickpipe resync <service-id> <clickpipe-id>   # Postgres and MySQL pipes only

# Delete a ClickPipe
clickhousectl cloud clickpipe delete <service-id> <clickpipe-id>

# Update scaling (at least one of --replicas/--cpu-millicores/--memory-gb is required)
clickhousectl cloud clickpipe scale <service-id> <clickpipe-id> \
  --replicas 2 --cpu-millicores 250 --memory-gb 1

# Get/update settings
clickhousectl cloud clickpipe settings get <service-id> <clickpipe-id>
clickhousectl cloud clickpipe settings update <service-id> <clickpipe-id> \
  --streaming-max-insert-wait-ms 10000

# Object-storage and ClickHouse-side ingestion settings
clickhousectl cloud clickpipe settings update <service-id> <clickpipe-id> \
  --object-storage-concurrency 8 \
  --object-storage-polling-interval-ms 30000 \
  --object-storage-max-file-count 500 \
  --object-storage-max-insert-bytes 268435456 \
  --object-storage-use-cluster-function true \
  --clickhouse-max-threads 16 --clickhouse-max-insert-threads 4 \
  --clickhouse-max-download-threads 8 \
  --clickhouse-min-insert-block-size-bytes 20971520 \
  --clickhouse-parallel-distributed-insert-select 1 \
  --clickhouse-parallel-view-processing false

# Change Kafka consumer isolation explicitly
clickhousectl cloud clickpipe settings update <service-id> <clickpipe-id> \
  --kafka-read-committed true

# Manage reverse private endpoints for private source connectivity
clickhousectl cloud clickpipe reverse-private-endpoint list <service-id>
```

`settings get` and `settings update` apply to streaming (Kafka, Kinesis) and
object-storage pipes only; Pub/Sub counts as streaming and is accepted too,
though the refusal message does not name it.
Database pipes (Postgres, MySQL, MongoDB, BigQuery) are refused with
an explanation instead of the API's `NOT_FOUND`: their settings, such as the
sync interval and pull batch size, live on the pipe itself, so read them with
`clickhousectl cloud clickpipe get <service-id> <clickpipe-id>`.

Pass at least one setting. Omitted object-storage settings retain their current
values; an update does not replace the whole configuration. Explicit `0` and
`false` remain explicit values.
The three additional ClickHouse controls accept download threads 0–32, minimum
insert block size 0–10737418240 bytes, and distributed INSERT SELECT mode 0–2.

For Kafka, `--kafka-read-committed true|false` changes consumer isolation. When
omitted, the CLI preserves the current value; if the API omits that value, pass
the flag explicitly. The flag is refused unless the pipe is confirmed as Kafka.
Other settings are validated by the API for source compatibility, so passing
`--object-storage-max-file-count` to a Kafka pipe is rejected server-side.

#### Creating ClickPipes

Each source type has its own subcommand under `clickpipe create`:

The current source commands accept credentials as command-line options. Load values from your secret manager into environment variables, run them only in a trusted environment, and do not commit source credentials to scripts; expanded values may still be visible in process listings while a command runs.

```bash
# From S3 / object storage (one-shot snapshot)
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name my-s3-pipe \
  --source-url 'https://bucket.s3.us-east-1.amazonaws.com/data/**' \
  --format JSONEachRow \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# From S3 with continuous ingestion (SQS queue) and ingestion control
# --skip-initial-load: skip the initial snapshot load, only ingest new objects
# --start-after: resume ingestion after a specific object key (conflicts with --skip-initial-load)
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name my-s3-continuous-pipe \
  --source-url 'https://bucket.s3.us-east-1.amazonaws.com/data/**' \
  --format JSONEachRow \
  --continuous \
  --queue-url 'https://sqs.us-east-1.amazonaws.com/123/my-queue' \
  --start-after obj-key-001 \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# From Google Cloud Storage (object storage)
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name my-gcs-pipe \
  --storage-type gcs \
  --source-url 'https://storage.googleapis.com/bucket/data/**' \
  --format JSONEachRow \
  --service-account-file ./sa-key.json \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# From Azure Blob Storage
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name my-azure-pipe \
  --storage-type azureblobstorage \
  --source-url 'https://<account>.blob.core.windows.net/events/data/**' \
  --connection-string "$AZURE_CONNECTION_STRING" \
  --azure-container-name events --path 'data/**' \
  --format JSONEachRow \
  --database default --table events \
  --column "event_id:Int64"

# Gzipped CSV in a private bucket, read with an IAM role (or --access-key-id/--secret-key)
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name my-csv-pipe \
  --source-url 'https://bucket.s3.us-east-1.amazonaws.com/data/**' \
  --format CSVWithNames --compression gzip --delimiter ',' \
  --iam-role "$S3_IAM_ROLE_ARN" \
  --database default --table events \
  --column "event_id:Int64"

# From Kafka / Redpanda / Confluent / MSK
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-kafka-pipe \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
  --kafka-type redpanda \
  --auth SCRAM-SHA-256 \
  --username "$KAFKA_USERNAME" --password "$KAFKA_PASSWORD" \
  --ca-certificate ./ca.crt \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# No-auth broker: omit every auth flag. --auth is optional and is inferred from
# the credentials given (--username/--password → PLAIN, --access-key-id/--secret-key
# → IAM_USER, --iam-role → IAM_ROLE, --client-certificate/--client-key → MUTUAL_TLS);
# half a pair is a usage error, not an unauthenticated request.
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-kafka-pipe \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# Avro via a schema registry, mutual TLS, starting from a timestamp
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-avro-pipe \
  --brokers 'broker:9092' --topics events --consumer-group clickpipes-events \
  --format AvroConfluent \
  --schema-registry-url https://registry.example.com \
  --schema-registry-username "$SR_USERNAME" --schema-registry-password "$SR_PASSWORD" \
  --schema-registry-ca-certificate ./sr-ca.crt \
  --auth MUTUAL_TLS --client-certificate ./client.crt --client-key ./client.key \
  --offset from_timestamp --offset-timestamp 2026-01-01T00:00 \
  --database default --table events \
  --column "event_id:Int64"

# From Amazon Kinesis
clickhousectl cloud clickpipe create kinesis <service-id> \
  --name my-kinesis-pipe \
  --stream-name events --region us-east-1 \
  --format JSONEachRow \
  --auth IAM_ROLE --iam-role "$KINESIS_IAM_ROLE_ARN" \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# Kinesis with access keys, enhanced fan-out, starting at a timestamp
clickhousectl cloud clickpipe create kinesis <service-id> \
  --name my-kinesis-replay --stream-name events --region us-east-1 \
  --format JSONEachRow \
  --auth IAM_USER --access-key-id "$AWS_ACCESS_KEY_ID" --secret-key "$AWS_SECRET_ACCESS_KEY" \
  --iterator-type AT_TIMESTAMP --iterator-timestamp 1767225600 --enhanced-fan-out \
  --database default --table events \
  --column "event_id:Int64"

# From PostgreSQL with a publicly trusted certificate (CDC)
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-pg-pipe \
  --host db.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --destination-database analytics \
  --table-mapping "public.users:public_users" \
  --table-mapping "public.orders:public_orders"

# From an RDS or Aurora PostgreSQL with IAM role authentication (CDC)
# IAM_ROLE auth takes no --username/--password: the role ARN is the credential
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-rds-pg-pipe \
  --host db.abcdefg.us-east-1.rds.amazonaws.com --pg-database mydb \
  --postgres-type rdspostgres \
  --auth IAM_ROLE --iam-role "$POSTGRES_IAM_ROLE_ARN" \
  --publication-name clickpipes \
  --table-mapping "public.users:public_users"

# From PostgreSQL with a private or self-signed CA (CDC)
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-private-pg-pipe \
  --host 10.0.0.15 --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --ca-certificate ./postgres-ca.pem \
  --tls-host postgres.internal.example.com \
  --publication-name clickpipes \
  --table-mapping "public.users:public_users"

# Shaping the destination tables (excluded columns, sorting keys, engine): use
# --table-mapping-json, documented under "PostgreSQL table mappings" below

# Reuse an existing replication slot (cdc_only), non-default port
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-slot-pg-pipe \
  --host db.example.com --port 6432 --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --replication-mode cdc_only --replication-slot-name my_existing_slot \
  --table-mapping "public.users:public_users"

# From PostgreSQL, tuning the CDC and initial-load settings
# --enable-failover-slots (PG17+) applies only when ClickPipes creates the slot,
# so it belongs here and not with the --replication-slot-name example above
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-tuned-pg-pipe \
  --host db.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --table-mapping "public.users:public_users" \
  --sync-interval-seconds 30 --pull-batch-size 50000 \
  --initial-load-parallelism 4 \
  --snapshot-rows-per-partition 1000000 --snapshot-parallel-tables 3 \
  --allow-nullable-columns true --delete-on-merge false \
  --enable-failover-slots true

# From MySQL (CDC)
# --server-id sets the replication server ID (useful when multiple pipes read
# from the same MySQL instance, or to avoid colliding with existing replicas)
clickhousectl cloud clickpipe create mysql <service-id> \
  --name my-mysql-pipe \
  --host mysql.example.com \
  --username "$MYSQL_USERNAME" --password "$MYSQL_PASSWORD" \
  --table-mapping "mydb.users:mydb_users" \
  --server-id 4242

# From an RDS or Aurora MySQL with IAM role authentication (CDC)
# IAM_ROLE auth takes no --username/--password: the role ARN is the credential
clickhousectl cloud clickpipe create mysql <service-id> \
  --name my-rds-mysql-pipe \
  --host mysql.abcdefg.us-east-1.rds.amazonaws.com \
  --mysql-type rdsmysql \
  --auth IAM_ROLE --iam-role "$MYSQL_IAM_ROLE_ARN" \
  --table-mapping "mydb.users:mydb_users"

# MySQL over a non-default port, binlog file/position replication, private CA
clickhousectl cloud clickpipe create mysql <service-id> \
  --name my-mariadb-pipe \
  --host mysql.example.com --port 3307 --mysql-type mariadb \
  --username "$MYSQL_USERNAME" --password "$MYSQL_PASSWORD" \
  --replication-mode cdc --replication-mechanism FILE_POS \
  --ca-certificate ./mysql-ca.pem --tls-host mysql.internal.example.com \
  --table-mapping "mydb.users:mydb_users"

# From MongoDB (CDC)
clickhousectl cloud clickpipe create mongodb <service-id> \
  --name my-mongo-pipe \
  --uri 'mongodb+srv://cluster.example.net/mydb' \
  --username "$MONGODB_USERNAME" --password "$MONGODB_PASSWORD" \
  --table-mapping "mydb.users:mydb_users"

# One-shot MongoDB snapshot, read from the primary, private CA
clickhousectl cloud clickpipe create mongodb <service-id> \
  --name my-mongo-snapshot \
  --uri 'mongodb+srv://cluster.example.net/mydb' \
  --username "$MONGODB_USERNAME" --password "$MONGODB_PASSWORD" \
  --replication-mode snapshot --read-preference primary \
  --ca-certificate ./mongo-ca.pem \
  --table-mapping "mydb.users:mydb_users"

# From BigQuery (snapshot)
clickhousectl cloud clickpipe create bigquery <service-id> \
  --name my-bq-pipe \
  --service-account-file ./sa-key.json \
  --staging-path gs://bucket/staging \
  --replication-mode snapshot \
  --allow-nullable-columns true \
  --initial-load-parallelism 4 \
  --snapshot-rows-per-partition 1000000 \
  --snapshot-parallel-tables 3 \
  --table-mapping "dataset.table:target_table"

# From Google Cloud Pub/Sub (limited preview: contact support to enable it)
clickhousectl cloud clickpipe create pubsub <service-id> \
  --name my-pubsub-pipe \
  --topic events --project-id my-gcp-project \
  --format JSONEachRow \
  --seek-type earliest \
  --service-account-file ./sa-key.json \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# Pub/Sub with the optional subscription tuning, reading the key from stdin
# so it never has to be written to disk
gcloud secrets versions access latest --secret=clickpipes-sa-key |
  clickhousectl cloud clickpipe create pubsub <service-id> \
    --name my-tuned-pubsub-pipe \
    --topic events --project-id my-gcp-project \
    --format Avro \
    --seek-type timestamp --seek-timestamp 2026-04-10T12:00:00Z \
    --service-account-file - \
    --filter 'attributes.region = "eu"' \
    --enable-ordering --ack-deadline 120 \
    --database default --table events \
    --column "event_id:Int64" --column "name:String"

# Grant extra ClickHouse roles to the destination user (any create subcommand)
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-kafka-pipe \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
  --database default --table events \
  --column "event_id:Int64" \
  --role analytics_reader --role analytics_writer
```

PostgreSQL, MySQL, MongoDB, and BigQuery creates accept
`--destination-database <DATABASE>` for the ClickHouse database that receives
their mapped tables. It defaults to `default`; source database, schema, and
dataset names remain part of the source flags and table mappings shown above.

BigQuery supports snapshot replication. Its nullability and snapshot tuning
flags are optional; when omitted, the request leaves those settings to the
ClickPipes service defaults.

`--role` is available on every `clickpipe create` subcommand and is repeatable.
ClickPipes creates a ClickHouse user for the pipe; when `--role` is omitted that
user is granted the default role only, and the request omits the field
altogether. Each `--role` grants one more existing role on top of the default,
so roles are added and never taken away. Repeated values are sent once in the
order given. The names `clickpipes` and `clickpipes_system` are reserved by
ClickPipes: the CLI rejects them as usage errors (exit code 2) before making
any request.

#### PostgreSQL ClickPipe prerequisites

TLS and certificate verification are enabled by default. The private-CA example
above is the preferred secure setup: pass the CA certificate or bundle as PEM
with `--ca-certificate <PATH>`, and `--tls-host <HOSTNAME>` only when the
certificate names a different hostname than `--host` (for example when `--host`
is an IP address). The CLI reads the file and sends its contents, not the path.

For controlled diagnosis, add `--skip-cert-verification` to keep the connection
encrypted while accepting an untrusted or mismatched source certificate. Use
`--disable-tls` only for a source intentionally configured for plaintext; source
traffic is then unencrypted. `--disable-tls` cannot be combined with
`--ca-certificate`, `--tls-host`, or `--skip-cert-verification`.

Before creating a PostgreSQL CDC ClickPipe:

- Make the PostgreSQL host and port reachable from ClickHouse Cloud. Allow the
  [ClickPipes static egress IPs](https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips)
  in the source firewall, security group, and `pg_hba.conf`, or configure
  supported private connectivity.
- Enable logical replication (`wal_level=logical`) and provision sufficient WAL
  senders and replication slots.
- Create a publication. The publication must contain every source table named
  by `--table-mapping`; each table must have a primary key or an appropriate
  replica identity.
- Give the source user permission to connect, `USAGE` on each mapped schema,
  `SELECT` on each mapped table, and the PostgreSQL `REPLICATION` privilege.

See the [PostgreSQL ClickPipes setup guide](https://clickhouse.com/docs/integrations/clickpipes/postgres),
the [generic PostgreSQL source setup guide](https://clickhouse.com/docs/integrations/clickpipes/postgres/source/generic),
and the [ClickPipes networking and static IP documentation](https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips).

PostgreSQL ClickPipes require at least one table mapping, from either
`--table-mapping schema.table:target_table` or `--table-mapping-json <JSON>`
(see [PostgreSQL table mappings](#postgresql-table-mappings)). Ports must be in
`1..=65535`. `--auth IAM_ROLE` requires `--iam-role`; the CLI rejects
`--iam-role` with basic auth rather than silently ignoring it.
`--username` and `--password` are basic-auth only and must be given together:
they are required for the default `--auth basic`, and rejected with
`--auth IAM_ROLE`, where the role ARN is the whole credential and
no `credentials` object is sent.
`--replication-slot-name` is valid only with `--replication-mode cdc_only`.

#### PostgreSQL table mappings

`clickpipe create postgres` takes table mappings in two forms, and both flags
are repeatable and may be given together. At least one mapping from either
flag is required.

`--table-mapping schema.table:target_table` is unchanged: it maps one source
table to one destination table and leaves every other per-table option at the
ClickPipes default.

`--table-mapping-json <JSON>` takes the API's table mapping object verbatim,
for the options that shape the destination table ClickPipes creates and
therefore cannot be changed once the pipe exists:

```bash
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-pg-pipe \
  --host db.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --table-mapping-json '{
    "sourceSchemaName": "public",
    "sourceTable": "users",
    "targetTable": "public_users",
    "excludedColumns": ["ssn"],
    "sortingKeys": ["created_at", "id"],
    "partitionByExpr": "toYYYYMM(created_at)",
    "tableEngine": "ReplacingMergeTree"
  }'
```

| Field | Meaning |
| --- | --- |
| `sourceSchemaName` | Source schema. Required, must not be empty. |
| `sourceTable` | Source table. Required, must not be empty. |
| `targetTable` | Destination table. Required, must not be empty. |
| `excludedColumns` | Source columns to leave out of the destination table. |
| `sortingKeys` | Destination `ORDER BY` columns. |
| `useCustomSortingKey` | Whether the API applies `sortingKeys`. Set to `true` automatically when `sortingKeys` is given. |
| `partitionByExpr` | `PARTITION BY` expression for the destination table, for example `toYYYYMM(created_at)`. |
| `partitionKey` | Column used to partition the initial snapshot for parallelism. Unrelated to the destination table's `PARTITION BY`. |
| `tableEngine` | One of `MergeTree`, `ReplacingMergeTree` or `Null`. Defaults to `MergeTree`, which is what the simple form sends. |

Every value is validated before any request is made, and a failure is a usage
error (exit code 2):

- An unknown field is rejected, so a typo such as `excludeColumns` fails loudly
  instead of being dropped on the way to the API.
- `useCustomSortingKey: false` alongside a non-empty `sortingKeys` is rejected,
  because the API would ignore the keys. `useCustomSortingKey: true` with no
  keys is rejected for the same reason.
- An unknown `tableEngine` is rejected locally, since the destination table's
  engine cannot be changed after the pipe is created.
- Each error names the offending occurrence, for example
  `--table-mapping-json #2: targetTable is required and must not be empty`.

The mappings are sent in flag order: the `--table-mapping` values first, then
the `--table-mapping-json` ones. The JSON form is not yet available on
`clickpipe create mysql`, `mongodb` or `bigquery`, whose table mappings have
analogous fields.

#### PostgreSQL CDC pipe settings

`clickpipe create postgres` accepts the full set of CDC and initial-load
settings. Each flag maps to exactly one request field, and an omitted flag is
left out of the request:

| Flag | Meaning |
| --- | --- |
| `--sync-interval-seconds <SECONDS>` | Interval in seconds to sync data from Postgres during CDC replication. |
| `--pull-batch-size <ROWS>` | Number of rows to pull in each batch during CDC replication. |
| `--initial-load-parallelism <WORKERS>` | Number of parallel workers to use per table in the initial snapshot phase. |
| `--snapshot-rows-per-partition <ROWS>` | Number of rows per partition during the snapshot phase. |
| `--snapshot-parallel-tables <TABLES>` | Number of tables to snapshot in parallel during the initial load phase. |
| `--allow-nullable-columns <true\|false>` | Preserve Postgres nullability in the destination table, creating columns without `NOT NULL` as `Nullable(...)`. Nullable types carry a performance cost in ClickHouse. |
| `--enable-failover-slots <true\|false>` | Enable failover support for the replication slot on PG17 and newer. Applies only when ClickPipes creates the slot, so not with `--replication-slot-name`. |
| `--delete-on-merge <true\|false>` | Enable hard delete behaviour in `ReplacingMergeTree` for PostgreSQL `DELETE` operations. |

The three boolean flags take an explicit `true` or `false` value; when omitted
they send `false`, which is the API default.

These are create-time decisions. The Cloud API can patch only
`syncIntervalSeconds` and `pullBatchSize` after the pipe exists, so the
snapshot and initial-load settings cannot be changed later on a pipe that was
created without them. `clickpipe settings update` is a different endpoint for
streaming and object-storage pipes and does not cover these settings.

The same settings are not yet exposed on `clickpipe create mysql`.

`--service-account-file` (on `create pubsub`, `create object-storage` and
`create bigquery`) takes a path to the GCP service account JSON key file, or `-`
to read the key from stdin, the same spelling `service query --queries-file -`
uses. The contents are base64-encoded and sent as the service account key; the
path itself is never sent, the key is never accepted as an inline flag value, so
it stays out of process listings and shell history, and it is never echoed back
in output or errors. An empty key file (or empty stdin) is refused before any
request is made.

`--seek-type` has no default: `earliest` reads the backlog, `latest` only new
messages, and `timestamp` starts from `--seek-timestamp` (required for that seek
type, rejected for the others). Other Pub/Sub limits come from the API:
`--ack-deadline` is in seconds and must be between 10 and 600, and `--filter`
takes a Pub/Sub CEL subscription filter of at most 256 characters. Both are
checked before the request is sent.

Use `clickhousectl cloud clickpipe create <source> --help` for the full list of options per source type.

#### MySQL ClickPipe authentication

`clickpipe create mysql` authenticates with either a username and password
(the default `--auth basic`) or an AWS IAM role on an RDS or Aurora MySQL
source (`--auth IAM_ROLE`).

The same auth rules as the [Postgres source](#postgresql-clickpipe-prerequisites)
apply, with `--mysql-type rdsmysql`/`auroramysql` selecting the source type; the
table-mapping and replication-slot rules stated there are Postgres-only. Each
rule is a usage error (exit code 2) before any request is made.

#### Reverse private endpoints (PrivateLink, Private Service Connect)

A reverse private endpoint gives ClickPipes a private route to a source that is
not reachable over the public internet. Create the endpoint on the service,
wait for it to become `Ready`, then reference it from a pipe.

```bash
# List and inspect the endpoints of a service
clickhousectl cloud clickpipe reverse-private-endpoint list <service-id>
clickhousectl cloud clickpipe reverse-private-endpoint get <service-id> <endpoint-id>

# AWS PrivateLink, VPC endpoint service
clickhousectl cloud clickpipe reverse-private-endpoint create <service-id> \
  --type VPC_ENDPOINT_SERVICE \
  --description 'analytics source' \
  --vpc-endpoint-service-name com.amazonaws.vpce.us-east-1.vpce-svc-12345678901234567

# AWS PrivateLink, VPC resource
clickhousectl cloud clickpipe reverse-private-endpoint create <service-id> \
  --type VPC_RESOURCE \
  --description 'analytics source' \
  --vpc-resource-configuration-id rcfg-12345678901234567 \
  --vpc-resource-share-arn arn:aws:ram:us-east-1:123456789012:resource-share/share-1

# Amazon MSK multi-VPC connectivity
clickhousectl cloud clickpipe reverse-private-endpoint create <service-id> \
  --type MSK_MULTI_VPC \
  --description 'msk source' \
  --msk-cluster-arn arn:aws:kafka:us-east-1:123456789012:cluster/my-cluster \
  --msk-authentication SASL_IAM

# Google Private Service Connect (private preview), with custom private DNS names
clickhousectl cloud clickpipe reverse-private-endpoint create <service-id> \
  --type GCP_PSC_SERVICE_ATTACHMENT \
  --description 'gcp source' \
  --gcp-service-attachment projects/my-project/regions/us-central1/serviceAttachments/my-service \
  --custom-private-dns-mapping db.example.com \
  --custom-private-dns-mapping '*.example.com'

# Replace or clear the custom private DNS mappings, or delete the endpoint
clickhousectl cloud clickpipe reverse-private-endpoint update <service-id> <endpoint-id> \
  --custom-private-dns-mapping db.example.com
clickhousectl cloud clickpipe reverse-private-endpoint update <service-id> <endpoint-id> \
  --clear-custom-private-dns-mappings
clickhousectl cloud clickpipe reverse-private-endpoint delete <service-id> <endpoint-id>

# Reference a Ready endpoint from a Kafka pipe
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-private-kafka-pipe \
  --brokers 'b-1.msk.internal:9098' --topics events --format JSONEachRow \
  --reverse-private-endpoint-id <endpoint-id> \
  --database default --table events \
  --column "event_id:Int64"
```

Using an endpoint from a pipe:

- Kafka: pass the endpoint's `id` with `clickpipe create kafka
  --reverse-private-endpoint-id <endpoint-id>`, which is repeatable.
- Postgres and MySQL CDC: pass one of the endpoint's DNS names as `--host` on
  `clickpipe create postgres` or `clickpipe create mysql`. `reverse-private-endpoint
  get` prints `dnsNames`, along with any custom private DNS names.

A pipe can only use an endpoint that has reached the `Ready` status. An AWS
PrivateLink endpoint stays in `PendingAcceptance` until the connection request
is accepted in the account that owns the source, so check
`reverse-private-endpoint get` before creating the pipe.

Each type has its own required flags, and a flag belonging to another type is a
usage error (exit code 2) raised before any request is made:

| `--type` | Required flags |
| --- | --- |
| `VPC_ENDPOINT_SERVICE` | `--vpc-endpoint-service-name` |
| `VPC_RESOURCE` | `--vpc-resource-configuration-id`, `--vpc-resource-share-arn` |
| `MSK_MULTI_VPC` | `--msk-cluster-arn`, `--msk-authentication` |
| `GCP_PSC_SERVICE_ATTACHMENT` (private preview) | `--gcp-service-attachment` |

`--custom-private-dns-mapping` is repeatable and takes an exact or
leading-wildcard name (`*.example.com`). The API does not support it for
`MSK_MULTI_VPC`, which the CLI also rejects up front, and for the AWS
PrivateLink types it has to be enabled for the service by ClickHouse support.
The custom private DNS mappings are the only field the API's PATCH accepts, so
`update` sends the complete list given on the command line: repeat every mapping
the endpoint should keep, or pass `--clear-custom-private-dns-mappings` to remove
all mappings. The replace and clear flags conflict.

#### Discovering a source schema (beta)

`clickpipe schema-discover` probes a Kafka, Kinesis, object-storage or Google
Cloud Pub/Sub source and returns the inferred fields/types without creating a
pipe. It takes the same source
connection flags as the corresponding `create` subcommand (minus the
destination `--name`/`--database`/`--table`/`--column` options). Object-storage
discovery runs on the destination service, so that service must be running.
Schema discovery requires API-key authentication:

```bash
# Discover schema from Kafka
clickhousectl cloud clickpipe schema-discover <service-id> kafka \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
  --auth SCRAM-SHA-256 \
  --username "$KAFKA_USERNAME" --password "$KAFKA_PASSWORD"

# Discover schema from a Kafka broker that requires no authentication
clickhousectl cloud clickpipe schema-discover <service-id> kafka \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow

# Discover schema from Kinesis
clickhousectl cloud clickpipe schema-discover <service-id> kinesis \
  --stream-name events --region us-east-1 \
  --format JSONEachRow \
  --auth IAM_ROLE --iam-role "$KINESIS_IAM_ROLE_ARN"

# Discover schema from object storage (S3, GCS, Azure Blob Storage)
clickhousectl cloud clickpipe schema-discover <service-id> object-storage \
  --source-url 'https://bucket.s3.us-east-1.amazonaws.com/data/*.json' \
  --format JSONEachRow \
  --iam-role "$S3_IAM_ROLE_ARN"

# Discover schema from Google Cloud Pub/Sub (limited preview)
clickhousectl cloud clickpipe schema-discover <service-id> pubsub \
  --topic events --project-id my-gcp-project \
  --format JSONEachRow \
  --seek-type earliest \
  --service-account-file ./sa-key.json
```

Add `--json` (or run as a coding agent) for machine-readable output.

### Members

Role IDs used by member, invitation, and API-key commands currently come from the ClickHouse Cloud Console or API.

```bash
clickhousectl cloud member list
clickhousectl cloud member get <user-id>
clickhousectl cloud member update <user-id> --role-id <role-id>
clickhousectl cloud member update <user-id> --clear-roles
clickhousectl cloud member remove <user-id>
```

Omitting both member role flags leaves assigned roles unchanged.
`--clear-roles` removes them all and conflicts with `--role-id`.

### Invitations

```bash
clickhousectl cloud invitation list
clickhousectl cloud invitation create --email dev@example.com --role-id <role-id>
clickhousectl cloud invitation get <invitation-id>
clickhousectl cloud invitation delete <invitation-id>
```

### Keys

```bash
clickhousectl cloud key list
clickhousectl cloud key get <key-id>
# the key secret is printed once, at create time only
clickhousectl cloud key create --name ci-key \
  --role-id <role-id> \
  --expires-at <future-RFC3339-time> \
  --ip-allow '<trusted-egress-ip>/32=CI runners' \
  --state disabled   # create the key already disabled
# --hash-key-id/--hash-key-id-suffix/--hash-key-secret submit a pre-hashed key; no secret is returned
clickhousectl cloud key update <key-id> \
  --name renamed-key \
  --state disabled
clickhousectl cloud key update <key-id> --expires-at 2030-12-31T23:59:59Z
clickhousectl cloud key update <key-id> --clear-expiry
clickhousectl cloud key update <key-id> --clear-roles --clear-ip-allow
clickhousectl cloud key delete <key-id>
```

On update, omitting expiry, role, or IP allowlist flags keeps that setting.
`--clear-expiry`, `--clear-roles`, and `--clear-ip-allow` remove the respective
setting and conflict with the corresponding set flag.

### Activity

```bash
clickhousectl cloud activity list --from-date 2024-01-01 --to-date 2024-12-31
clickhousectl cloud activity get <activity-id>
```

### JSON output

Use the `--json` flag for machine-readable output on commands that return structured data.

```bash
clickhousectl cloud --json service list
clickhousectl cloud --json service get <service-id>
```

`clickhousectl` auto-detects coding-agent contexts (Claude Code, Cursor, Codex, Gemini CLI, Goose, Devin, others, and any tool that sets the standard `AGENT` / `AI_AGENT` env vars) and emits JSON to stdout automatically without setting `--json`. Protocol-oriented commands retain their natural output: the legacy `cloud org prometheus` command and `cloud service prometheus` always emit raw Prometheus exposition text and silently ignore `--json`, `cloud service query` uses a ClickHouse format such as `JSONEachRow`, and Postgres runtime configuration is JSON already.

Human-readable detail views (`cloud clickpipe get` and every other `get`-style command) never print PEM-framed material. Each well-formed PEM block in a value is replaced, where it stands, by a one-line summary of that block: `<PEM CERTIFICATE, SHA-256 fingerprint AB:CD:...>` for a certificate, certificate request or CRL, using the fingerprint `openssl x509 -fingerprint -sha256` prints for that block, and `<PEM EC PRIVATE KEY, 121 bytes>` for any other label, because a private key is reported by size and never fingerprinted. Text around the blocks, such as a bundle's header comments, is kept as it was. This affects human output only: `--json` still returns the value verbatim, and `cloud postgres certs get` deliberately still prints the raw PEM, since emitting the certificate is that command's purpose.

Local runtime failures also use structured output when `local --json` is set or a coding agent is detected. The CLI writes exactly one error object to stderr and preserves the documented exit code:

```json
{
  "error": {
    "code": "server_not_found",
    "message": "Server 'default' was not found in the current project",
    "project_scope": {
      "kind": "exact_current_project",
      "path": "/path/to/project",
      "parent_projects_searched": false
    },
    "server": {
      "name": "default"
    },
    "guidance": [
      {
        "action": "return_to_project_root",
        "message": "Change to the local project root where the server was started",
        "command": "cd <project-root>"
      },
      {
        "action": "list_project_servers",
        "message": "List servers after returning to that exact project",
        "command": "clickhousectl local server list"
      },
      {
        "action": "list_global_servers",
        "message": "Locate running ClickHouse servers across projects",
        "command": "clickhousectl local server list --global"
      },
      {
        "action": "stop_global_project_server",
        "message": "After confirming the project, stop the server with explicit global project selection",
        "command": "clickhousectl local server stop <name> --global --project <project-root>"
      }
    ]
  }
}
```

`error.code` and `error.message` are always present. General errors can include an optional top-level `error.command` safe recovery command, which names the step that actually recovers the failure (for example `clickhousectl local server stop dev` when `server remove dev` is refused because that server is running). Project-local `server stop` and `server remove` not-found errors instead include `project_scope`, `server`, and ordered `guidance`; their top-level `command` field is absent.

`error.message` carries the same detail as the human `Error: ...` line whenever clickhousectl composes that text itself — a missing `--config` name lists the configs directory and the available files in both modes. The exception is text that interpolates output clickhousectl does not control: subprocess stderr and log tails (`startup_exit`, `startup_timeout`), Docker daemon strings (`docker_error`), download bodies (`download_failed`), and OS or serialization sources (`io_error`, `local_error`) are summarized instead, so JSON never carries raw I/O errors, credentials, SQL, or container logs. Human local errors retain the concise `Error: ...` format. Clap usage errors, Cloud errors, and child-process output are not wrapped in this local schema.

When `.clickhouse` is absent from the current directory, bare `server stop` includes the same `project_scope` and `guidance` in its successful no-op output, while bare `server remove` includes them in its `server_selection_required` error. This distinguishes a missing project root from an initialized project that has no matching ClickHouse server.

Managed `local client` failures deliberately use dedicated `managed_client_*` codes rather than the general server codes. Their error object includes `project_scope.path` (the canonical directory inspected), `server.selection` and `server.name`, an optional `server.binary_version`, and ordered `guidance` entries with allowlisted messages and optional commands. No raw lock, metadata, or I/O error is included in JSON. The nested shape distinguishes this exact-project lookup contract from failures in other local commands without changing those commands' stable envelopes.

The schema and meanings of existing codes are stable. New optional fields or codes may be added compatibly; failures whose text cannot be rendered safely use the bounded `local_error` fallback.

| Code | Meaning |
| ---- | ------- |
| `server_not_found` | The selected local server does not exist |
| `managed_client_server_not_found` | Managed client lookup did not find the selected server in the current project |
| `managed_client_server_not_running` | The managed client server exists in the current project but is stopped |
| `managed_client_binary_not_found` | The client binary selected by managed server metadata is not installed |
| `managed_client_project_state_unavailable` | Managed client lookup could not read or lock current-project server state |
| `server_selection_required` | A server name (or, for `server stop --global`, a `--project`) is required because the selection is ambiguous or unsafe |
| `server_not_running` | The selected local server exists but is stopped |
| `server_running` | The operation requires a stopped server, or a running server is using the version |
| `invalid_server_name` | The server name contains path separators or `..` |
| `unsupported_argument` | An argument was rejected because it would break the managed server lifecycle (a pass-through `--config`, or `--http-port`/`--tcp-port` `0`) |
| `config_not_found` | The named `server start --config` file does not exist, or the name is ambiguous |
| `invalid_config_name` | The config name is a path rather than a file in the configs dir |
| `invalid_version` | The version selector is invalid |
| `version_not_installed` | The requested or configured version is not installed locally |
| `binary_not_launchable` | The version is installed but its binary cannot be launched (missing, not a regular file, or not executable) |
| `version_selection_required` | A version must be chosen because no default is set or the choice is ambiguous |
| `version_already_installed` | The requested version is already installed |
| `version_unavailable` | The requested version could not be resolved or downloaded |
| `version_is_default` | The version is the current default and `--force` was not passed |
| `unsupported_client_version` | The installed client does not support the requested operation |
| `unsupported_platform` | No ClickHouse build exists for this OS and architecture |
| `port_in_use` | A requested port is occupied or no managed port is available |
| `startup_exit` | A managed server exited before it became ready |
| `startup_timeout` | A managed server did not become ready before its deadline |
| `download_failed` | An artifact or image download or extraction failed |
| `network_error` | An HTTP request failed |
| `docker_unavailable` | Docker could not be reached (the message names the cause and the platform fix) |
| `docker_error` | A Docker operation failed |
| `container_name_conflict` | The container name is held by a container clickhousectl does not manage |
| `postgres_error` | A Postgres validation or state error; the message carries its recovery guidance |
| `io_error` | A local filesystem, metadata, or serialization operation failed |
| `local_error` | A redacted fallback for failures whose text cannot be rendered safely |

### Exit codes

Usage errors and cancelled actions use distinct exit codes.

| Code | Meaning                                                  |
| ---- | -------------------------------------------------------- |
| `0`  | Success                                                  |
| `1`  | Error (anything not classified below)                    |
| `2`  | Usage error (invalid command line)                       |
| `3`  | Cancelled (user aborted)                                 |
| `4`  | Auth required (no credentials, 401/403, OAuth-only writes) |

## Skills

Install the official ClickHouse Agent Skills from [ClickHouse/agent-skills](https://github.com/ClickHouse/agent-skills).

```bash
# Default: interactive mode for humans, choose scope, then choose agents
clickhousectl skills

# Non-interactive: install into every supported project-local agent folder
clickhousectl skills --all

# Non-interactive: install only into detected agents
clickhousectl skills --detected-only

# Non-interactive: install into every supported global agent folder
clickhousectl skills --global --all

# Non-interactive: install only into detected global agents
clickhousectl skills --global --detected-only

# Non-interactive: install into specific project-local agents
clickhousectl skills --agent claude --agent codex

# Comma-separated is equivalent to repeating the flag
clickhousectl skills --agent claude,codex

# Non-interactive: install into specific global agents
clickhousectl skills --global --agent claude --agent codex
```

### Supported Skills paths

The common path `.agents/skills/` is always included regardless of agent selection.

The following agents can be selected, and Skills are installed in the corresponding paths:
- `claude` -> `.claude/skills/`
- `codex` -> `.codex/skills/`
- `cursor` -> `.cursor/skills/`
- `opencode` -> `.opencode/skills/`
- `agent` -> `.agent/skills/`
- `roo` -> `.roo/skills/`
- `trae` -> `.trae/skills/`
- `windsurf` -> `.windsurf/skills/`
- `zencoder` -> `.zencoder/skills/`
- `neovate` -> `.neovate/skills/`
- `pochi` -> `.pochi/skills/`
- `adal` -> `.adal/skills/`
- `openclaw` -> `.openclaw/skills/`
- `cline` -> `.cline/skills/`
- `command-code` -> `.command-code/skills/`
- `kiro-cli` -> `.kiro/skills/`
- `agents` -> `.agents/skills/` (always installed; selectable explicitly too)

Supports global or project scope installation. Project scope installs Skills into the current working directory. Global scope installs Skills into the current user's home directory.

### Non-interactive flags:

- `--agent` name a specific agent to install Skills for, can be repeated
- `--global` use global scope; if omitted, project scope is used
- `--all` install Skills for all supported agents
- `--detected-only` install Skills for supported agents that were detected on the system

## Self-update

`clickhousectl` can update itself to the latest release:

```bash
# Update to the latest version
clickhousectl update

# Check for updates without installing
clickhousectl update --check
```

The CLI checks for updates in the background (at most once per 24 hours) and caches the result. When a newer version is available, a one-line notice is printed to stderr at the end of every command that produces human-readable output. JSON output (`--json` or a detected coding agent) is never affected, so machine consumers stay clean. Running `clickhousectl update` clears the cached notice.

## Telemetry

`clickhousectl` collects anonymous usage data to help us understand which commands matter and improve the CLI. Full details: <https://clickhouse.com/docs/concepts/features/interfaces/cli#telemetry>.

Each event contains exactly:

- the command path (e.g. `local server start`)
- the **names** of the flags passed (e.g. `json`, `org-id`) — never flag values
- the **names** of the positional arguments passed (e.g. `name` for `local server stop dev`) — presence only, never the value
- how the invocation ended and its exit code
- the CLI version, OS, and architecture
- whether it ran in CI (`CI` env var)
- whether it ran under a detected coding agent, and if so which one (e.g. `claude-code`)
- when a command fails at runtime, a bounded description of *how* it failed (see below)

There is no install ID, no device ID, and no fingerprinting of any kind. The payload is built from the clap command definitions rather than the raw command line, so leaking an argument value is structurally impossible — the code that builds the event has no access to values at all.

`clickhousectl local server stop analytics-prod` records `positionals: ["name"]` — that a server was named, not which one. Three exclusions keep that honest:

- only arguments you actually passed count, so a value clap filled in from a default (or from the environment), and a name the CLI generated for you, are absent — which is what makes "you named it" and "we picked one" distinguishable
- arguments forwarded to another program are never recorded: everything after `--` for `local server start`, and the trailing arguments of `local client` and `local postgres client`, belong to `clickhouse-server`, `clickhouse-client`, and `psql`
- when a command fails to parse, the unmatched token is still never recorded — only the slot it would have filled

A failed *runtime* invocation may also carry up to six failure-classification fields, so that "exit code 1" stops being the only thing we know about a broken command. Each one is a closed vocabulary defined in the source, and nothing else can ever appear in it:

- `failure_stage` — which stage failed: `sql_input`, `org_resolution`, `service_resolution`, `query_request`, `key_create`, `key_get`, `key_delete`, `endpoint_get`, `endpoint_upsert`, `response_stream`
- `failure_kind` — what kind of failure it was: `io`, `transport`, `http_4xx`, `http_5xx`, `sql_error`, `service_stopped`, `timeout`, `rate_limited`, `other`
- `http_status` — the exact HTTP status, and only if it is one of a fixed list of common statuses; anything else is dropped (its class is already in `failure_kind`)
- `retry_bucket` — how many retries the run made, as a bucket (`0`, `1`, `2`, `3_5`, `6_10`, `gt_10`), never an exact count
- `provisioning_state` — how far Query API credential provisioning had got: `bearer`, `stored_key`, `management_key`, `provisioning`, `provisioned`, `refused`
- `duration_bucket` — how long the operation ran before failing, as a bucket (`lt_250ms`, `lt_1s`, `lt_5s`, `lt_30s`, `lt_2m`, `ge_2m`)

These are fixed strings compiled into the binary (plus one allowlisted status), set only where a failure is owned — never derived from an error text. No classification is attached to a successful run.

Exactly one event is recorded per invocation. `local client` and `local postgres client` `exec()` into the native client, so clickhousectl's event is recorded just before the handover with the censored outcome `exec_attempt` and a fixed exit code `0` — it means "the handoff was reached", not "the native client succeeded". Failures clickhousectl can see itself (missing/non-executable binary, `psql` not on `PATH`) are refused first and report their real exit code.

Nothing is sent before you have seen the notice unless you explicitly enable telemetry with `clickhousectl telemetry enable`. The first run normally prints a one-time notice to stderr, records that it was shown in `~/.clickhouse/telemetry.json`, and sends nothing. Sending starts from the following run. Explicitly enabling telemetry starts it immediately and skips the notice. The send happens in a short-lived detached process, so command latency is unaffected even when the endpoint is unreachable.

Opt out any of these ways:

```bash
# Persistently, per machine
clickhousectl telemetry disable

# Per environment/shell (https://consoledonottrack.com)
export DO_NOT_TRACK=1

# Show whether telemetry is enabled and why
clickhousectl telemetry status
```

On a machine that has never seen the notice, `telemetry status` reports "not yet configured" and then completes the first run itself: it writes `~/.clickhouse/telemetry.json` and prints the notice, so sending starts from the next run.

To see exactly what would be sent without sending it, set `CHCTL_TELEMETRY_DEBUG=1` — the payload is printed to stderr and nothing leaves the machine.

Distribution packagers can compile telemetry out entirely (including the `telemetry` subcommand) with `cargo build --no-default-features`.

## User-defined functions (Beta)

`cloud udf` manages organization-scoped executable UDFs, versions, and service attachments. All UDF operations are beta. Reads support OAuth; writes require API key authentication.

Create a JSON definition and a [source ZIP archive](https://clickhouse.com/docs/products/cloud/features/sql-console-features/user-defined-functions#manage-udfs-with-the-cloud-api). `--config-file` accepts a file or `-` for stdin. The definition uses the API's field names and excludes `uploadId`, which the CLI obtains from a fresh upload session:

```json
{
  "functionName": "my_udf",
  "type": "executable",
  "runtime": "native",
  "arguments": [{"name": "x", "type": "UInt64"}],
  "returnType": "UInt64",
  "memoryLimitMib": 128,
  "deterministic": false
}
```

```bash
clickhousectl cloud udf create --config-file udf.json --artifact source.zip
clickhousectl cloud udf get my_udf
# Wait for status ready, then attach the latest ready version (or --version 2)
clickhousectl cloud udf attach my_udf <service-id>
clickhousectl cloud udf attachment list my_udf
clickhousectl cloud udf attachment get my_udf <service-id>
clickhousectl cloud udf list --limit 20
# Continue with the returned pagination.nextCursor
clickhousectl cloud udf list --limit 20 --cursor '<nextCursor>'
clickhousectl cloud udf version list my_udf

# version.json contains the complete definition without functionName or uploadId
clickhousectl cloud udf version create my_udf --config-file version.json --artifact source-v2.zip
clickhousectl cloud udf attach my_udf <service-id> --version 2
clickhousectl cloud udf detach my_udf <service-id>
# Detach from every service before deleting an individual version
clickhousectl cloud udf version delete my_udf 1
clickhousectl cloud udf delete my_udf
```

Required definition fields are `type`, `runtime`, `arguments`, and `returnType`; initial creation also requires `functionName`. Supported types are `executable` and `executable_pool`, with runtimes `native` and `python3.11`. Optional fields are `returnName`, `format`, `commandReadTimeout`, `commandWriteTimeout`, `maxCommandExecutionTime`, `memoryLimitMib`, `sendChunkHeader`, `deterministic`, `sandboxType`, `sandboxVersion`, and `poolSize`. Read/write timeouts are milliseconds; maximum execution time is seconds. Memory is 1–1,048,576 MiB or null. `poolSize` accepts a positive integer for `executable_pool` and only null for `executable`. Sandbox type is `basic` or `netenable`; sandbox version is `v1`, `v2`, or `v3`. Set `deterministic` to true only when identical arguments always produce identical results.

Version creation uses defaults for omitted options, without inheriting the previous version's configuration. Supply a complete request definition; GET output includes response-only fields and cannot be used directly as a request. Unknown fields, unsupported enum values, missing required fields and invalid limits fail before upload. Nullable options may be omitted or set to null; both use the API's default behavior.

Creation and version creation each request a new upload URL, stream the ZIP archive, and submit its upload ID once. Failed uploads never submit a create request. Uploads time out after five minutes; rerun the command to obtain a fresh session after any failure. The target service must be running; wake an idle service before attaching. Attachment replaces the service's existing version; omitted `--version` selects the latest ready version. A dependency failure (HTTP 424) exits with an error; inspect the UDF and service before retrying. The latest version and versions still building cannot be deleted individually. Deleting a UDF deletes all its versions and detaches it from every service; service removal finishes asynchronously.

All three list commands expose `--cursor` and `--limit` (1–100) and retain pagination in JSON output. Detail and list output tolerate missing fields and new response status values.

The UDF API request models preserve `deterministic` and nullable `memoryLimitMib` in both executable variants, including version creation. The OpenAPI analyzer checks inline union payload fields and request requiredness; its report format is version 5.

## Cloud integration testing

Maintainer operation, exact-SHA overrides, stacked-PR policy, and the required
check rollout procedure are documented in
[`.github/CLOUD_INTEGRATION.md`](.github/CLOUD_INTEGRATION.md).

Cloud API integration is tested against a real ClickHouse Cloud workspace via the library crate. Affected suites are selected automatically and run once the `run-cloud-integration` label is applied. Tests live in four suites:

- [`tests/integration_test.rs`](crates/clickhouse-cloud-api/tests/integration_test.rs) — ClickHouse service CRUD + service-scoped endpoints
- [`tests/integration_postgres_test.rs`](crates/clickhouse-cloud-api/tests/integration_postgres_test.rs) — Postgres service CRUD
- [`tests/integration_org_test.rs`](crates/clickhouse-cloud-api/tests/integration_org_test.rs) — org-scoped endpoints (members, invitations, roles, activity, prometheus, private endpoint config)
- [`tests/clickpipes/`](crates/clickhouse-cloud-api/tests/clickpipes/) — ClickPipes E2E; only Postgres CDC runs in CI

Required environment variables:

```bash
export CLICKHOUSE_CLOUD_API_KEY=...
export CLICKHOUSE_CLOUD_API_SECRET=...
export CLICKHOUSE_CLOUD_TEST_ORG_ID=...
export CLICKHOUSE_CLOUD_TEST_PROVIDER=aws
export CLICKHOUSE_CLOUD_TEST_REGION=eu-west-1
# Required for the org integration suite (members + invitations need a
# second user in the test org); optional otherwise.
export CLICKHOUSE_CLOUD_TEST_SECONDARY_USER_ID=...
```

Run a suite:

```bash
cargo test -p clickhouse-cloud-api --test integration_test          -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_postgres_test -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_org_test      -- --ignored --nocapture
```

By default, any failed check fails the run. To keep going after `non-blocking` capability failures and collect them in a summary at the end, set:

```bash
export CONTINUE_ON_NON_BLOCKING_FAILURES=1
```

## Requirements

- macOS (aarch64, x86_64) or Linux (aarch64, x86_64)
- Cloud read operations support OAuth; writes and some operations such as ClickPipe schema discovery require a [ClickHouse Cloud API key](https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl)
