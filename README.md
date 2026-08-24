# clickhousectl

`clickhousectl` (`chctl`) is the official CLI for ClickHouse and Postgres, locally and in ClickHouse Cloud.

With `clickhousectl` you can:
- Install, run, and query ClickHouse locally
- Run Docker-backed Postgres instances for local development
- Create a ClickHouse Cloud account and authenticate from the terminal
- Create and manage ClickHouse and Postgres services in ClickHouse Cloud
- Run SQL against local and cloud ClickHouse services
- Create and manage ClickPipes for data ingestion (S3, Kafka, Kinesis, Postgres, MySQL, MongoDB, BigQuery)
- Install the official ClickHouse agent skills into supported coding agents
- Move local ClickHouse development to ClickHouse Cloud

`clickhousectl` helps humans and coding agents develop with ClickHouse and Postgres.

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

## Local

### Installing and managing ClickHouse versions

`clickhousectl` downloads ClickHouse binaries from `builds.clickhouse.com`, falling back to `packages.clickhouse.com` (Linux) or [GitHub releases](https://github.com/ClickHouse/ClickHouse/releases) (macOS) when a build isn't available there.

```bash
# Manage default version
clickhousectl local use latest              # Latest master build; installs if needed and creates ~/.local/bin/clickhouse
clickhousectl local use 26.8                # Latest 26.8.x.x (installs if needed)
clickhousectl local use 26.8.1.1760         # Exact version
clickhousectl local use latest --no-global  # Set default but don't touch ~/.local/bin/clickhouse
clickhousectl local which                   # Show current default

# Install a version
clickhousectl local install latest          # Latest master build
clickhousectl local install 26              # Latest 26.x.x.x
clickhousectl local install 26.8            # Latest 26.8.x.x
clickhousectl local install 26.8.1.1760      # Exact version

# List versions
clickhousectl local list                    # Installed versions
clickhousectl local list --remote           # Available for download

# Remove a version
clickhousectl local remove 26.8.1.1760
clickhousectl local remove 26.8.1.1760 --force   # Stop running servers on this version first
```

`local use` also creates a symlink at `~/.local/bin/clickhouse` pointing to the selected version's binary, so the plain `clickhouse` command (e.g. `clickhouse local`, `clickhouse client`) is on PATH. Pass `--no-global` to skip. If a regular file already exists at that path it is left alone with a warning. `local remove` of the active default version also clears the symlink.

`local remove` refuses to delete a version while a local server is running on it (it would leave the server pointing at a deleted binary), failing with the running server names. Stop the server first, or pass `--force` to stop the running server(s) and then remove the version.

The two local removal commands delete different state:

| Command | Removes | Keeps |
| --- | --- | --- |
| `clickhousectl local remove <exact-version>` | The globally installed ClickHouse binary in `~/.clickhouse/versions/` | Project-local server data |
| `clickhousectl local server remove <name>` | A stopped server's data and metadata in the current project's `.clickhouse/servers/` | Globally installed ClickHouse versions |

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
clickhousectl local client --query "SELECT 1" --query "SELECT 2"  # Run queries in order
clickhousectl local client --queries-file schema.sql seed.sql      # Run query files in order
clickhousectl local client --host remote-host --port 9000  # Connect to a specific host/port
clickhousectl local client --host remote-host --version 26.8.1.1760  # Use an exact installed client binary
```

`--query` can be repeated. `--queries-file` accepts multiple paths after one flag and can also be repeated. Their values are forwarded in order. The native client does not allow inline queries and query files in the same invocation, so `local client` rejects that combination as a usage error instead of reordering it. Arguments after `--` are forwarded after these generated query arguments.

Named connections and direct connections select the client binary differently. A named connection uses the version recorded in the managed server's metadata, regardless of the global default. A direct connection (`--host`, `--port`, or both) uses the exact installed version passed with `--version`; if that flag is omitted, it uses the version selected by `local use`.

Direct connections never infer a binary from the contents of `~/.clickhouse/versions`. With no valid default, omitting `--version` fails whether zero, one, or multiple versions are installed. Use `local list` to find an exact installed version, then pass it with `--version` or select it globally with `local use`. A missing explicit version and a default that points to a removed version both fail with repair instructions. Direct `--version` selection neither installs a binary nor changes the default.

### Creating and managing ClickHouse servers

Start and manage ClickHouse server instances. Each server gets its own isolated data directory at `.clickhouse/servers/<name>/data/`.

A bare `clickhousectl local server start` bootstraps from zero: if no version is installed and no default is set, it installs `latest` and starts with it (it does not set a default, so you keep tracking `latest` on subsequent starts). Pin a version with `--version`, or set a default with `local use`, to opt out. Because `latest` tracks the rolling master build, repeat `latest` installs/starts do a cheap `HEAD` against `builds.clickhouse.com` and skip the ~150 MB re-download when master hasn't changed (the build's `etag` is cached in `~/.clickhouse/versions/.master-builds.json`).

```bash
# Start a server (runs in background by default)
clickhousectl local server start                          # Named "default" (installs latest if nothing is set up yet)
clickhousectl local server start dev                      # Named "dev"
clickhousectl local server start --version latest         # Use a specific version (installs if needed, doesn't change default)
clickhousectl local server start --foreground             # Run in foreground (-F / --fg)
clickhousectl local server start --no-wait                # Return after spawning without waiting for readiness
clickhousectl local server start --http-port 8124 --tcp-port 9001  # Explicit ports
clickhousectl local server start --config analytics       # Apply a custom config (see "Custom config files" below)

# List custom config files available to --config
clickhousectl local server configs

# List all servers in this project (ClickHouse and Postgres, running and stopped)
clickhousectl local server list
clickhousectl local server list --global                  # List running ClickHouse servers across all projects

# Stop servers
clickhousectl local server stop                           # Stop "default", or the sole known ClickHouse server
clickhousectl local server stop dev                       # Stop by name
clickhousectl local server stop default --global          # Stop from any project
clickhousectl local server stop default --global --project /path/to/project  # Disambiguate
clickhousectl local server stop-all                       # Stop all ClickHouse and Postgres servers in this project
clickhousectl local server stop-all --global              # Stop all ClickHouse servers system-wide

# Remove a stopped server and its data
clickhousectl local server remove                         # Remove "default" only when it exists
clickhousectl local server remove dev                     # Remove by name

# Write connection env vars to .env file
clickhousectl local server dotenv                        # From "default" server → .env
clickhousectl local server dotenv --name dev             # From "dev" server → .env
clickhousectl local server dotenv --local                # Write to .env.local instead
clickhousectl local server dotenv --local --user default --database mydb  # Include user and database
```

Stopping a server preserves its data and identity metadata, so it remains visible in `server list` with a `stopped` status. Version and ports are shown only while running because they are resolved again on each start. Starting the same name resumes the existing data directory.

When a project-scoped `server stop` omits the name, it selects an existing `default` server first. If there is no `default`, it selects the sole known ClickHouse server, whether running or stopped. With no ClickHouse servers it succeeds without doing anything. With multiple non-default ClickHouse servers it exits with guidance to pass a name or use `server stop-all`. An explicit unknown name remains an error so typos are not hidden.

An omitted `server remove` is deliberately more conservative: it removes only an existing `default` server. It never infers a custom name, even when there is only one. If `default` does not exist, the command reports whether custom ClickHouse servers are available and directs you to `server list` before you pass a name explicitly.

Project-scoped `server list`, `stop`, and `remove` use `.clickhouse` under the canonical current directory only. They do not search parent directories, including when the current directory is reached through a symlink or has its own nested `.clickhouse`. Lookup and state errors print that canonical project directory, direct you to change to the intended project directory for stopped servers, and suggest `clickhousectl local server list --global` for finding running servers across projects.

**Server naming:** Without a name, the first server is called "default". If "default" is already running, a random name is generated (e.g. "bold-crane"). Pass a name positionally for stable identities you can start/stop repeatedly. If a name is generated, retain the returned name for later `stop` and `remove` commands.

**Ports:** Defaults are HTTP 8123 and TCP 9000. If these are already in use, free ports are automatically assigned and shown in the output. Use `--http-port` and `--tcp-port` to set explicit ports.

**Readiness:** Background starts wait up to 30 seconds for the HTTP health check and TCP port before reporting success, so a following `local client` command can connect immediately. Startup failures point to `.clickhouse/servers/<name>/server.log`. Use `--no-wait` for fire-and-forget startup.

**Orphaned server recovery:** If server metadata files are lost while the ClickHouse process is still running, the CLI automatically recovers them via process discovery. Running `server list`, `server start`, or any server command will detect orphaned processes belonging to the current project and bring them back under management.

**Global server management:** Use `--global` with `list`, `stop`, and `stop-all` to operate across all projects system-wide. `server list --global` shows all running ClickHouse servers with a Project column indicating which directory each belongs to.

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

# Start a Postgres instance (defaults: postgres:18, port 5432, user "postgres", db "postgres")
clickhousectl local postgres start
clickhousectl local postgres start --name dev --version 17 --port 5433
clickhousectl local postgres start --user app --database myapp  # Generates a random password
clickhousectl local postgres start -e POSTGRES_INITDB_ARGS=--data-checksums
clickhousectl local postgres start --password flag-value -e POSTGRES_PASSWORD=env-value

# List everything (ClickHouse + Postgres are merged in `server list`)
clickhousectl local server list

# Connect with psql (uses host psql if installed; otherwise falls back to docker exec)
clickhousectl local postgres client --name dev
clickhousectl local postgres client --name dev --query "SELECT 1"

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

The Postgres `dotenv` command includes the generated password. Do not commit its output; prefer `--local` when your application reads `.env.local`.

When `--port` is omitted, `start` uses port 5432 or selects the next available port. An explicit `--port` must be available and is never changed. Container variables use `KEY=VALUE` syntax. The first `-e POSTGRES_PASSWORD=...` takes precedence over `--password`, preserving the existing environment override. The dedicated `--user` and `--database` options (or their defaults) take precedence over `-e POSTGRES_USER=...` and `-e POSTGRES_DB=...`; clickhousectl also owns `PGDATA` so the managed data directory cannot be redirected. Fresh and resumed starts wait up to 30 seconds for PostgreSQL to accept connections before reporting success. Fresh startup is transactional after container creation: failures while starting the container, saving metadata, or waiting for readiness remove the container and any metadata and data owned by that attempt. Directories that contained data before the attempt are retained. Data is also retained when the container cannot be removed safely, and incomplete rollback preserves recovery metadata and reports cleanup diagnostics.

`local postgres start --name dev` (no `--version`) resumes the existing instance when there's exactly one for that name; if multiple majors share the name, the command exits and asks you to pass `--version`. Stop preserves the container and metadata so the next start resumes it; only `remove` tears down the container and deletes the data directory. The unified `local server stop-all` stops both ClickHouse and Postgres instances in the current project; the dedicated `local postgres stop-all` remains available when only Postgres should be stopped.

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
```

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
```

## Cloud

Manage ClickHouse, Postgres, and other ClickHouse Cloud resources via the API.

### Organizations

```bash
clickhousectl cloud org list              # List organizations
clickhousectl cloud org get <org-id>      # Get organization details
clickhousectl cloud org update <org-id> --name "Renamed Org"
clickhousectl cloud org update <org-id> \
  --remove-private-endpoint pe-1,cloud-provider=aws,region=us-east-1 \
  --enable-core-dumps false
clickhousectl cloud org prometheus --filtered-metrics true
clickhousectl cloud org usage \
  --from-date 2024-01-01 \
  --to-date 2024-01-31
# Add --org-id <org-id> to either command when your credentials access multiple organizations.
```

### Services

```bash
# List services
clickhousectl cloud service list

# Get service details
clickhousectl cloud service get <service-id>

# Create a service with explicit placement and network access
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
  --num-replicas 2

# Create with specific IP allowlist
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --ip-allow <trusted-egress-cidr> \
  --ip-allow <another-trusted-egress-cidr>

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

# Start/stop a service
clickhousectl cloud service start <service-id>
clickhousectl cloud service stop <service-id>

# Run SQL over HTTP via the Query API (no local clickhouse binary needed)
clickhousectl cloud service query --name my-service --query "SELECT 1"
clickhousectl cloud service query --id <service-id> --query "SELECT count() FROM system.tables" --format JSONEachRow
clickhousectl cloud service query --name my-service --queries-file schema.sql   # "-" reads from stdin
clickhousectl cloud service query --name my-service --database mydb --query "SHOW TABLES"
echo "SELECT 1+1" | clickhousectl cloud service query --name my-service

# Update service metadata and patches
clickhousectl cloud service update <service-id> \
  --name my-renamed-service \
  --add-ip-allow <trusted-egress-cidr> \
  --remove-ip-allow 0.0.0.0/0 \
  --add-private-endpoint-id pe-1 \
  --release-channel fast \
  --enable-endpoint mysql \
  --add-tag env=staging \
  --transparent-data-encryption-key-id tde-key-1 \
  --enable-core-dumps false

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
clickhousectl cloud service private-endpoint create <service-id> --endpoint-id vpce-123
clickhousectl cloud service private-endpoint get-config <service-id>

# Backup configuration
clickhousectl cloud service backup-config get <service-id>
clickhousectl cloud service backup-config update <service-id> \
  --backup-period-hours 24 \
  --backup-retention-period-hours 720 \
  --backup-start-time 02:00

# Service Prometheus configuration
clickhousectl cloud service prometheus <service-id> --filtered-metrics true

# Delete a service (must be stopped first)
clickhousectl cloud service delete <service-id>

# Force delete: stops a running service then deletes
clickhousectl cloud service delete <service-id> --force
```

Use `clickhousectl cloud service create --help` for the complete option list. If omitted, `--provider` defaults to `aws`, `--region` defaults to `us-east-1`, and the IP allowlist defaults to `0.0.0.0/0`; production workflows should normally set all three explicitly. When the create response includes an initial password, it is shown only once.

#### Query API auth modes

`cloud service query` is the canonical way to run SQL against a cloud service — over HTTP, with no `clickhouse` binary and no service password required. It works with both credential modes:

- **API key auth** (read + write SQL): when no per-service key is stored, `cloud service query` first uses the authenticated API key directly. This supports services whose Query API endpoint already authorizes that key without requiring permission to create another key. If the key or endpoint is not authorized, the CLI provisions a dedicated API key and binds it to the service. Those generated query credentials, the endpoint ID, exact management API key ID, and provisioning organization ID are stored in `.clickhouse/credentials.json` under `service_query_keys.<service-id>`, alongside any user-level API key. Subsequent queries use that key. The generated key is scoped to a single service, so it can read and write (SELECT, INSERT, DDL) against that service but cannot reach any other service in the org. Pass `--no-auto-enable` to fail instead of provisioning. If a stored key is later rejected with a non-SQL 401/403, the CLI atomically removes only that matching local credential and asks you to rerun the query; normal first-use provisioning then stores a replacement if the active API key is not already authorized. Ambiguous failures and concurrently replaced credentials are left untouched.
- **OAuth** (`cloud auth login`): the query runs as your own identity — the CLI sends your bearer token straight to the Query API, which grants **read-only** SQL access (SELECT and other read statements only; no INSERT, DDL, or other writes). No Query API key is provisioned or stored, and no query endpoint needs to be configured on the service. Use API key auth if you need to write. `--no-auto-enable` has no effect in this mode.

`--query` and `--queries-file` are mutually exclusive. Omit both to read SQL from stdin; `--queries-file -` also reads stdin explicitly.

Provisioning happens lazily (rather than at `service create` time) because the endpoint can only be bound once the service has finished provisioning, which can take several minutes — `service create` returns immediately instead of blocking on it. Concurrent queries from the same project directory share one provisioning operation and reuse its atomically stored credential.

The control-plane endpoint upsert replaces the complete `openApiKeys` list and does not support conditional updates. Provisioning the same service concurrently from different project directories can therefore still lose another project's endpoint binding; coordinate first use across projects when they share a service.

Per-service scoping is enforced at the query endpoint binding, which is created with role `sql_console_admin` (read + write inside the bound service only). The API key itself has no org-level roles, so the binding is the only thing that grants it any access. After deleting a service, `cloud service delete` deletes an auto-provisioned key by its stored management and organization IDs, then removes the local record. Legacy records without that metadata remain readable, but service deletion will not guess at a cloud key by name; a partial record with a management ID is retained for manual recovery.

Querying an **idled** service wakes it automatically in both auth modes — under OAuth the Query API first asks for a wake confirmation, which the CLI sends after printing a notice to stderr (the first query may take a minute while the service wakes). A **stopped** service is never woken: the query fails with a hint to run `cloud service start`.

The Query API host is derived from the API base URL per environment (`api.[control-plane.]<domain>` → `queries.<domain>`, e.g. `https://queries.clickhouse.cloud` for production). Set `CLICKHOUSE_CLOUD_QUERY_HOST` to override it.

### Postgres (beta)

Manage ClickHouse Cloud managed Postgres services. All write commands require API key auth.

```bash
# List / get
clickhousectl cloud postgres list
clickhousectl cloud postgres list --filter state=running
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
  --pg-config-file ./pg.json

# Update size, HA, or tags (all flags optional)
clickhousectl cloud postgres update <pg-id> \
  --size m7i.4xlarge \
  --add-tag env=prod --remove-tag legacy

# Delete
clickhousectl cloud postgres delete <pg-id>

# CA certificates
clickhousectl cloud postgres certs get <pg-id>                   # raw PEM to stdout
clickhousectl cloud postgres certs get <pg-id> --output ca.pem   # file (mode 0600 on unix)

# Runtime configuration
clickhousectl cloud postgres config get <pg-id>
clickhousectl cloud postgres config patch <pg-id> --set max_connections=500 --set random_page_cost=1.1
clickhousectl cloud postgres config patch <pg-id> --file patch.json

# Replace the entire configuration only with a complete object obtained from `config get`
clickhousectl cloud postgres config replace <pg-id> --file complete-config.json

# Password
clickhousectl cloud postgres reset-password <pg-id> --generate

# Read replica and PITR restore
clickhousectl cloud postgres read-replica create <pg-id> --name replica-1
clickhousectl cloud postgres restore <pg-id> \
  --name restored \
  --restore-target <recent-RFC3339-time-within-retention>

# Lifecycle
clickhousectl cloud postgres restart <pg-id>
clickhousectl cloud postgres promote <pg-id>
clickhousectl cloud postgres switchover <pg-id>
```

Use `clickhousectl cloud postgres create --help` for the complete option list. Save any initial password and connection string in the create response because later `postgres get` responses do not return credentials. If both are omitted, run `clickhousectl cloud postgres reset-password <postgres-id> --generate`.

### Backups

```bash
clickhousectl cloud backup list <service-id>
clickhousectl cloud backup get <service-id> <backup-id>
```

### ClickPipes

Manage ClickPipes for ingesting data into ClickHouse Cloud from external sources.

```bash
# List ClickPipes for a service
clickhousectl cloud clickpipe list <service-id>

# Get ClickPipe details
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>

# Start/stop/resync a ClickPipe
clickhousectl cloud clickpipe start <service-id> <clickpipe-id>
clickhousectl cloud clickpipe stop <service-id> <clickpipe-id>
clickhousectl cloud clickpipe resync <service-id> <clickpipe-id>   # CDC pipes only

# Delete a ClickPipe
clickhousectl cloud clickpipe delete <service-id> <clickpipe-id>

# Update scaling
clickhousectl cloud clickpipe scale <service-id> <clickpipe-id> \
  --replicas 2 --cpu-millicores 250 --memory-gb 1

# Get/update settings
clickhousectl cloud clickpipe settings get <service-id> <clickpipe-id>
clickhousectl cloud clickpipe settings update <service-id> <clickpipe-id> \
  --streaming-max-insert-wait-ms 10000
```

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

# From Amazon Kinesis
clickhousectl cloud clickpipe create kinesis <service-id> \
  --name my-kinesis-pipe \
  --stream-name events --region us-east-1 \
  --format JSONEachRow \
  --auth IAM_ROLE --iam-role "$KINESIS_IAM_ROLE_ARN" \
  --database default --table events \
  --column "event_id:Int64" --column "name:String"

# From PostgreSQL (CDC) with a publicly trusted TLS certificate
# TLS and certificate verification are enabled by default. The certificate
# hostname defaults to --host.
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-pg-pipe \
  --host db.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --table-mapping "public.users:public_users" \
  --table-mapping "public.orders:public_orders"

# From PostgreSQL with a private or self-signed certificate
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-private-pg-pipe \
  --host db.private.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --ca-certificate ./postgres-ca.pem \
  --table-mapping "public.users:public_users"

# From MySQL (CDC)
# --server-id sets the replication server ID (useful when multiple pipes read
# from the same MySQL instance, or to avoid colliding with existing replicas)
clickhousectl cloud clickpipe create mysql <service-id> \
  --name my-mysql-pipe \
  --host mysql.example.com \
  --username "$MYSQL_USERNAME" --password "$MYSQL_PASSWORD" \
  --table-mapping "mydb.users:mydb_users" \
  --server-id 4242

# From MongoDB (CDC)
clickhousectl cloud clickpipe create mongodb <service-id> \
  --name my-mongo-pipe \
  --uri 'mongodb+srv://cluster.example.net/mydb' \
  --username "$MONGODB_USERNAME" --password "$MONGODB_PASSWORD" \
  --table-mapping "mydb.users:mydb_users"

# From BigQuery (snapshot)
clickhousectl cloud clickpipe create bigquery <service-id> \
  --name my-bq-pipe \
  --service-account-file ./sa-key.json \
  --staging-path gs://bucket/staging \
  --table-mapping "dataset.table:target_table"
```

Before creating a PostgreSQL CDC ClickPipe:

- Make the source reachable from ClickPipes and allow the [ClickPipes static IPs](https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips) for your service region.
- Enable logical replication on PostgreSQL.
- Create a publication named `clickpipes` that includes every source table passed with `--table-mapping`.
- Grant the ClickPipes user schema `USAGE`, table `SELECT`, and replication privileges.

See the [PostgreSQL ClickPipes setup guide](https://clickhouse.com/docs/integrations/clickpipes/postgres) for provider-specific prerequisites or the [generic PostgreSQL source setup](https://clickhouse.com/docs/integrations/clickpipes/postgres/source/generic) for self-hosted and other providers. `--ca-certificate` is needed only when the source certificate is signed by a private CA or is self-signed. Use `--tls-host` when the hostname in that certificate differs from `--host`.

Use `clickhousectl cloud clickpipe create <source> --help` for the full list of options per source type.

#### Discovering a source schema (beta)

`clickpipe schema-discover` probes a Kafka or Kinesis source and returns the
inferred fields/types without creating a pipe. It takes the same source
connection flags as the corresponding `create` subcommand (minus the
destination `--name`/`--database`/`--table`/`--column` options). Schema discovery requires API-key authentication:

```bash
# Discover schema from Kafka
clickhousectl cloud clickpipe schema-discover <service-id> kafka \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
  --auth SCRAM-SHA-256 \
  --username "$KAFKA_USERNAME" --password "$KAFKA_PASSWORD"

# Discover schema from Kinesis
clickhousectl cloud clickpipe schema-discover <service-id> kinesis \
  --stream-name events --region us-east-1 \
  --format JSONEachRow \
  --auth IAM_ROLE --iam-role "$KINESIS_IAM_ROLE_ARN"
```

Add `--json` (or run as a coding agent) for machine-readable output.

### Members

Role IDs used by member, invitation, and API-key commands currently come from the ClickHouse Cloud Console or API.

```bash
clickhousectl cloud member list
clickhousectl cloud member get <user-id>
clickhousectl cloud member update <user-id> --role-id <role-id>
clickhousectl cloud member remove <user-id>
```

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
clickhousectl cloud key create --name ci-key \
  --role-id <role-id> \
  --expires-at <future-RFC3339-time> \
  --ip-allow <trusted-egress-ip>/32
clickhousectl cloud key update <key-id> \
  --name renamed-key \
  --state disabled
clickhousectl cloud key delete <key-id>
```

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

`clickhousectl` auto-detects coding-agent contexts (Claude Code, Cursor, Codex, Gemini CLI, Goose, Devin, and any tool that sets the standard `AGENT` env var) and emits JSON to stdout automatically without setting `--json`. Protocol-oriented commands retain their natural output: Prometheus commands emit text, `cloud service query` uses a ClickHouse format such as `JSONEachRow`, and Postgres runtime configuration is JSON already.

When a dispatched `local` command fails in explicit `--json` or detected-agent mode, stdout is empty and stderr contains exactly one JSON object:

```json
{
  "error": {
    "code": "server_not_found",
    "message": "Server 'missing' not found\nProject directory used for lookup: \"/work/app\"\nOnly this exact directory's `.clickhouse` is searched; parent `.clickhouse` directories are not searched.\nRun `clickhousectl local server list --global` to find running servers. For stopped servers, change to the intended project directory and run `clickhousectl local server list`.",
    "command": "clickhousectl local server list --global",
    "project": "/work/app"
  }
}
```

`code` is a stable, bounded value. `command` is fixed CLI guidance and never contains user input. Project-scoped server lookup and state errors also include the canonical `project` directory; that path is emitted only in command output and is never added to telemetry. Opaque download, filesystem, startup, and fallback diagnostics are not copied into the JSON message.

| Local runtime code     | Meaning                                      |
| ---------------------- | -------------------------------------------- |
| `server_not_found`     | A server is absent or name selection is needed |
| `server_not_running`   | The selected local server is stopped         |
| `server_running`       | A running server blocks the requested action |
| `invalid_version`      | The supplied version syntax is invalid       |
| `version_unavailable`  | The required version is not available        |
| `port_in_use`          | A selected local port is unavailable         |
| `startup_exit`         | The server exited during startup             |
| `startup_timeout`      | The server did not become ready in time      |
| `download_failed`      | A local binary or image download failed      |
| `io_error`             | A local filesystem operation failed          |
| `local_error`          | Bounded fallback for other local failures    |

Human-mode runtime errors retain concise `Error: ...` stderr output. Clap usage errors retain clap's own output and exit code. Native child handoffs, including `local client` and foreground servers, retain the child's stdout, stderr, and exit status without adding an envelope. The local envelope does not add raw errors or any new fields to telemetry.

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
- the **names** of the flags passed (e.g. `json`, `org-id`) — never flag values, never positional arguments
- how the invocation ended and its exit code
- the CLI version, OS, and architecture
- whether it ran in CI (`CI` env var)
- whether it ran under a detected coding agent, and if so which one (e.g. `claude-code`)

There is no install ID, no device ID, and no fingerprinting of any kind. The payload is built from the clap command definitions rather than the raw command line, so leaking an argument value is structurally impossible — the code that builds the event has no access to values at all.

Nothing is sent before you have seen the notice unless you explicitly enable telemetry with `clickhousectl telemetry enable`. The first run normally prints a one-time notice to stderr, records that it was shown in `~/.clickhouse/telemetry.json`, and sends nothing. Sending starts from the following run. Explicitly enabling telemetry starts it immediately and skips the notice. The send happens in a short-lived detached process, so command latency is unaffected even when the endpoint is unreachable.

Opt out any of these ways:

```bash
# Persistently, per machine
clickhousectl telemetry disable

# Per environment/shell (https://consoledonottrack.com)
export DO_NOT_TRACK=1
```

To see exactly what would be sent without sending it, set `CHCTL_TELEMETRY_DEBUG=1` — the payload is printed to stderr and nothing leaves the machine.

Distribution packagers can compile telemetry out entirely (including the `telemetry` subcommand) with `cargo build --no-default-features`.

## Cloud integration testing

Cloud API integration is tested against a real ClickHouse Cloud workspace via the library crate. All changes to cloud commands must pass CI testing before merge. Tests live in three binaries, each a single `#[tokio::test]` lifecycle:

- [`tests/integration_test.rs`](crates/clickhouse-cloud-api/tests/integration_test.rs) — ClickHouse service CRUD + service-scoped endpoints
- [`tests/integration_postgres_test.rs`](crates/clickhouse-cloud-api/tests/integration_postgres_test.rs) — Postgres service CRUD
- [`tests/integration_org_test.rs`](crates/clickhouse-cloud-api/tests/integration_org_test.rs) — org-scoped endpoints (members, invitations, roles, activity, prometheus, private endpoint config)

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
- Cloud read operations support OAuth; writes and some operations such as ClickPipe schema discovery require a [ClickHouse Cloud API key](https://clickhouse.com/docs/en/cloud/manage/api)
