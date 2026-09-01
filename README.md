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
clickhousectl local remove 26.8.1.1760 --force   # Stop running servers on this version (in any project), and remove it even if it is the default
```

`local use` also creates a symlink at `~/.local/bin/clickhouse` pointing to the selected version's binary, so the plain `clickhouse` command (e.g. `clickhouse local`, `clickhouse client`) is on PATH. Pass `--no-global` to skip. If a regular file already exists at that path it is left alone with a warning.

`local remove` refuses to delete a version while a local server is running on it (it would leave the server pointing at a deleted binary), failing with exit `1` and JSON error code `server_running`. Because versions are shared between projects, the check spans **every** project, not just the current directory: the error names each blocking server with the project root it was started from and its PID, so a server found by `clickhousectl local server list --global` is identifiable. Stop those servers first (`clickhousectl local server stop --global <name>`), or pass `--force` to stop them — in whichever project they run — and then remove the version.

`local remove` also refuses to delete the **current default version** (exit `1`, JSON error code `version_is_default`): removing it clears the `~/.clickhouse/default` marker and the global `~/.local/bin/clickhouse` symlink, and the exact build is not always re-downloadable — `builds.clickhouse.com` does not serve every exact build, which can leave a master-channel build unrecoverable. Switch the default first with `clickhousectl local use <other-version>`, or pass `--force` to remove it anyway. A forced removal stops any running servers on the version, warns on stderr before the version itself is deleted, then reports `was_default: true` in its output and clears both the default marker and the global symlink; set a new default with `clickhousectl local use latest`.

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
clickhousectl local postgres start --wait-timeout 120            # Default: 60s; maximum: 600s

# List everything (ClickHouse + Postgres are merged in `server list`)
clickhousectl local server list

# Connect with psql (uses host psql if installed; otherwise falls back to docker exec)
clickhousectl local postgres client --name dev
clickhousectl local postgres client --name dev --query "SELECT 1"
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

`local postgres start --name dev` (no `--version`) resumes the existing instance when there's exactly one for that name; if multiple majors share the name, the command exits and asks you to pass `--version`. Stop preserves the container and metadata so the next start resumes it; only `remove` tears down the container and deletes the data directory. The unified `local server stop-all` stops both ClickHouse and Postgres instances in the current project; the dedicated `local postgres stop-all` remains available when only Postgres should be stopped.

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
clickhousectl cloud service query --name my-service --queries-file query.sql   # single statement only; "-" reads from stdin
clickhousectl cloud service query --name my-service --database mydb --query "SHOW TABLES"
echo "SELECT 1+1" | clickhousectl cloud service query --name my-service

# Load a CSV: the statement and its data travel together on stdin
printf 'INSERT INTO trips FORMAT CSV\n' | cat - data.csv | \
  clickhousectl cloud service query --id <service-id>

# Replace a stale clickhousectl-owned Query API key for exactly one service
clickhousectl cloud service repair-query-key <service-id> --org-id <org-id>

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
# `--remove-ip-allow`/`--remove-private-endpoint-id`/`--remove-tag` are
# idempotent: removing something already absent still exits 0, but
# clickhousectl warns on stderr for each entry that matched nothing (tags are
# matched by key), so a
# typo doesn't silently no-op.

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
  --endpoint-id vpce-0123456789abcdef0
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

# Service Prometheus configuration
clickhousectl cloud service prometheus <service-id> --filtered-metrics true

# Delete a service (must be stopped first)
clickhousectl cloud service delete <service-id>

# Force delete: stops a running service then deletes
clickhousectl cloud service delete <service-id> --force
```

`--backup-start-time` requires the backup period to be 24 or 48 hours. Nothing is defaulted when the period is omitted: the API validates the new start time against the period already stored on the service, so either pass `--backup-period-hours 24` or `--backup-period-hours 48` in the same call, or leave the stored period at one of those. When a start time is given without a period, the CLI reads the current configuration first and fails before sending the update if the stored period is something else.

`--clear-backup-start-time` removes a stored start time and lifts that restriction, so a service that has one can go back to any backup period. It sends an explicit `"backupStartTime": null`, which the API accepts even though the OpenAPI spec does not mark the field nullable, and it can be combined with `--backup-period-hours` to clear the start time and set an otherwise incompatible period in a single call. The two start-time flags conflict: pass one or the other.

`--force` stops the service and then polls it until the stop completes, which takes minutes on a real service, printing each state change to stderr (stdout stays reserved for the result). Progress output is best-effort: if whatever was reading it goes away — a pager you quit, a supervising process that stopped reading — the lines are dropped and the deletion still runs to completion and exits 0.

Use `clickhousectl cloud service create --help` for the complete option list. If omitted, `--provider` defaults to `aws`, `--region` defaults to `us-east-1`, and the IP allowlist defaults to `0.0.0.0/0`; production workflows should normally set all three explicitly. When the create response includes an initial password, it is shown only once.

`--query` and `--queries-file` are mutually exclusive. If neither is supplied, `cloud service query` reads SQL from stdin; `--queries-file -` also reads stdin explicitly.

`--query` never reads stdin, so `--query "INSERT INTO trips FORMAT CSV" < data.csv` is refused (exit code `1`) before any request is sent rather than silently inserting nothing. The Query API takes a single request body, so a statement and a separate data stream cannot both be sent; pipe them together instead:

```bash
printf 'INSERT INTO trips FORMAT CSV\n' | cat - data.csv | \
  clickhousectl cloud service query --id <service-id>
```

Only real input counts as a conflict. A redirected file, a closed pipe, `/dev/null` and a pipe that already holds data are all answered immediately, and a pipe that is open but silent is given 250 ms to produce its first byte before the CLI treats stdin as empty and runs the query. So an empty non-terminal stdin, which is what CI runners and coding agents normally have, leaves `--query` working as before, a pipe nobody writes to can never hang the command, and a producer that is merely slow to start still gets caught. The residual is narrow and deliberate: a producer that has written nothing within those 250 ms is indistinguishable from no input at all.

Whatever the source, the SQL must be a single statement. The Query API runs exactly one statement per request, so a multi-statement `.sql` script is rejected by ClickHouse (error 62, `Multi-statements are not allowed`). Run statements one invocation at a time, or put a real client on PATH with `clickhousectl local use latest` and run the script through `clickhouse client` connected to the service.

Private endpoint IDs supplied to `private-endpoint create --endpoint-id` and `service update --add-private-endpoint-id` are format-checked before the request is sent, because adding one registers it for the whole organization and a typo has to be unpicked from both the service and the organization. Each provider uses its own format — AWS a `vpce-` VPC endpoint ID, GCP the numeric Private Service Connect connection ID, Azure the private endpoint Resource ID or `resourceGuid` — and the provider is not known when the flag is parsed, so only provider-independent mistakes are rejected (exit code `2`): empty values, values containing whitespace, and any value carrying `vpce-` that is not exactly a well-formed AWS VPC endpoint ID (`vpce-` plus 8 or 17 lowercase hex characters) — which also catches a pasted VPC endpoint ARN or endpoint service name. Azure Resource IDs (values starting with `/`) are exempt from that check, since an Azure resource may itself be named `vpce-...`. Whether the endpoint actually exists and belongs to you is not validated by the CLI or, currently, by the Cloud API. Removal flags are never format-checked, so an already-registered bogus ID stays removable.

#### Query API auth modes

`cloud service query` is the canonical way to run SQL against a cloud service — over HTTP, with no `clickhouse` binary and no service password required. It works with both credential modes:

- **API key auth** (read + write SQL): when no per-service key is stored, `cloud service query` first uses the authenticated API key directly. This supports services whose Query API endpoint already authorizes that key without requiring permission to create another key. If the key or endpoint is not authorized, the CLI provisions a dedicated API key and binds it to the service. Those generated query credentials, the endpoint ID, exact management API key ID, and provisioning organization ID are stored in `.clickhouse/credentials.json` under `service_query_keys.<service-id>`, alongside any user-level API key. Subsequent queries use that key. The generated key is scoped to a single service, so it can read and write (SELECT, INSERT, DDL) against that service but cannot reach any other service in the org. Pass `--no-auto-enable` to fail instead of provisioning.
- **OAuth** (`cloud auth login`): the query runs as your own identity — the CLI sends your bearer token straight to the Query API, which grants **read-only** SQL access (SELECT and other read statements only; no INSERT, DDL, or other writes). No Query API key is provisioned or stored, and no query endpoint needs to be configured on the service. Use API key auth if you need to write. `--no-auto-enable` has no effect in this mode.

Provisioning happens lazily (rather than at `service create` time) because the endpoint can only be bound once the service has finished provisioning, which can take several minutes — `service create` returns immediately instead of blocking on it.

Provisioning is single-flight for processes using the same project directory: the CLI serializes the create, bind, and credential-save transaction and reuses the result written by the first process. The endpoint upsert API replaces the complete `openApiKeys` list and does not currently support a conditional or idempotent key-binding operation. Provisioning the same service concurrently from different project directories can therefore still lose a binding if both projects read and replace that list at the same time.

Per-service scoping is enforced at the query endpoint binding, which is created with role `sql_console_admin` (read + write inside the bound service only). The API key itself has no org-level roles, so the binding is the only thing that grants it any access. After deleting a service, `cloud service delete` deletes an auto-provisioned key by its stored management and organization IDs, then removes the local record. Legacy records without that metadata remain readable, but service deletion will not guess at a cloud key by name; a partial record with a management ID is retained for manual recovery.

If a stored per-service key is revoked or its endpoint binding changes, a query that receives HTTP 401/403 reports the exact `repair-query-key` command and does not silently provision another key. Repair is an explicit API-key-authenticated write operation. It verifies the stored organization, management key ID, and endpoint ID, replaces only that key ID in the endpoint binding, and preserves every other binding and project credential. Concurrent repairs in the same project reuse the first process's replacement instead of rotating it again. Legacy or incomplete records without exact ownership metadata are refused. If deletion of the superseded key fails after replacement, its exact ID stays in the service record so rerunning the repair command can finish cleanup without provisioning again.

The Query API endpoint does not support conditional updates, so repair reads and rewrites the complete endpoint configuration while replacing the owned key binding. Do not modify the same endpoint concurrently with a repair because an update made after that read can be overwritten. Also wait for a first-use query's provisioning and readiness attempt to finish before running repair: its newly stored key can receive a temporary 401/403 while the endpoint binding converges, and an explicitly started repair can rotate that still-valid key.

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
  --pg-config-file ./pg.json

# Update size, HA, or tags (all flags optional)
clickhousectl cloud postgres update <pg-id> \
  --size m7i.4xlarge \
  --add-tag env=prod --remove-tag legacy

# Delete (works from any state, including running; no stop needed first)
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
clickhousectl cloud postgres promote <replica-id>
clickhousectl cloud postgres promote <replica-id> --wait                    # poll until isPrimary=true
clickhousectl cloud postgres switchover <primary-id>
clickhousectl cloud postgres switchover <primary-id> --wait --wait-timeout 600
```

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

# Update scaling (at least one of --replicas/--cpu-millicores/--memory-gb is required)
clickhousectl cloud clickpipe scale <service-id> <clickpipe-id> \
  --replicas 2 --cpu-millicores 250 --memory-gb 1

# Get/update settings
clickhousectl cloud clickpipe settings get <service-id> <clickpipe-id>
clickhousectl cloud clickpipe settings update <service-id> <clickpipe-id> \
  --streaming-max-insert-wait-ms 10000
```

`settings update` only sends the settings you name on the command line, and it
first reads the pipe to find its source type: settings that the API supports for
one source only — currently the Kafka `kafka_read_committed` setting, which the
CLI preserves rather than exposing as a flag — are sent for Kafka pipes and
omitted for every other source. Object-storage-, streaming- and
ClickHouse-specific settings are validated by the API, so passing (for example)
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

# From a Kafka broker that requires no authentication: omit every auth flag.
# `--auth` is optional; when it is omitted the mechanism is inferred from the
# credential flags you passed (`--username`/`--password` → PLAIN,
# `--access-key-id`/`--secret-key` → IAM_USER, `--iam-role` → IAM_ROLE,
# `--client-certificate`/`--client-key` → MUTUAL_TLS), and no authentication is
# sent when none were passed. Each credential pair must be given in full — half
# a pair (e.g. `--client-certificate` without `--client-key`) is a usage error,
# not a request with no authentication.
clickhousectl cloud clickpipe create kafka <service-id> \
  --name my-kafka-pipe \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow \
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

# From PostgreSQL with a publicly trusted certificate (CDC)
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-pg-pipe \
  --host db.example.com --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --publication-name clickpipes \
  --table-mapping "public.users:public_users" \
  --table-mapping "public.orders:public_orders"

# From PostgreSQL with a private or self-signed CA (CDC)
clickhousectl cloud clickpipe create postgres <service-id> \
  --name my-private-pg-pipe \
  --host 10.0.0.15 --pg-database mydb \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --ca-certificate ./postgres-ca.pem \
  --tls-host postgres.internal.example.com \
  --publication-name clickpipes \
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

#### PostgreSQL ClickPipe prerequisites

TLS and certificate verification are enabled by default. A source that serves a
complete, publicly trusted certificate chain needs neither `--ca-certificate`
nor `--tls-host`. If the source certificate uses a private or self-signed CA,
pass the CA certificate or bundle as PEM with `--ca-certificate <PATH>`; the CLI
reads that file and sends its contents in the create request. Certificate
hostname verification defaults to `--host`. Use `--tls-host <HOSTNAME>` only
when the certificate is issued for a different hostname, such as when `--host`
is an IP address. These options preserve certificate verification; they do not
disable it.

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

PostgreSQL ClickPipes require one or more complete
`--table-mapping schema.table:target_table` values. Ports must be in
`1..=65535`. `--auth IAM_ROLE` requires `--iam-role`; the CLI rejects
`--iam-role` with basic auth rather than silently ignoring it.
`--replication-slot-name` is valid only with `--replication-mode cdc_only`.

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

# Discover schema from a Kafka broker that requires no authentication
clickhousectl cloud clickpipe schema-discover <service-id> kafka \
  --brokers 'broker:9092' --topics events \
  --format JSONEachRow

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

`clickhousectl` auto-detects coding-agent contexts (Claude Code, Cursor, Codex, Gemini CLI, Goose, Devin, and any tool that sets the standard `AGENT` env var) and emits JSON to stdout automatically without setting `--json`. Protocol-oriented commands retain their natural output: `cloud org prometheus` and `cloud service prometheus` always emit raw Prometheus exposition text and silently ignore `--json`, `cloud service query` uses a ClickHouse format such as `JSONEachRow`, and Postgres runtime configuration is JSON already.

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
- the **names** of the flags passed (e.g. `json`, `org-id`) — never flag values
- the **names** of the positional arguments passed (e.g. `name` for `local server stop dev`) — presence only, never the value
- how the invocation ended and its exit code
- the CLI version, OS, and architecture
- whether it ran in CI (`CI` env var)
- whether it ran under a detected coding agent, and if so which one (e.g. `claude-code`)
- when a command fails at runtime, a bounded description of *how* it failed (see below)

There is no install ID, no device ID, and no fingerprinting of any kind. The payload is built from the clap command definitions rather than the raw command line, so leaking an argument value is structurally impossible — the code that builds the event has no access to values at all.

The privacy boundary for positional arguments is exactly the same one as for flags: every recorded name is a string compiled into the binary, so the field's vocabulary is a closed set that cannot carry anything you typed. `clickhousectl local server stop analytics-prod` records `positionals: ["name"]` — the fact that a server was named, not which one. Three further exclusions keep the field honest:

- only arguments you actually passed count, so a value clap filled in from a default (or from the environment), and a name the CLI generated for you, are absent — which is what makes "you named it" and "we picked one" distinguishable
- arguments forwarded to another program are never recorded: everything after `--` for `local server start`, and the trailing arguments of `local client` and `local postgres client`, belong to `clickhouse-server`, `clickhouse-client`, and `psql`
- when a command fails to parse, the unmatched token is still never recorded — only the slot it would have filled

A failed *runtime* invocation may also carry up to six failure-classification fields, so that "exit code 1" stops being the only thing we know about a broken command. Each one is a closed vocabulary defined in the source, and nothing else can ever appear in it:

- `failure_stage` — which stage failed: `sql_input`, `org_resolution`, `service_resolution`, `query_request`, `key_create`, `endpoint_get`, `endpoint_upsert`, `response_stream`
- `failure_kind` — what kind of failure it was: `io`, `transport`, `http_4xx`, `http_5xx`, `sql_error`, `service_stopped`, `timeout`, `rate_limited`, `other`
- `http_status` — the exact HTTP status, and only if it is one of a fixed list of common statuses; anything else is dropped (its class is already in `failure_kind`)
- `retry_bucket` — how many retries the run made, as a bucket (`0`, `1`, `2`, `3_5`, `6_10`, `gt_10`), never an exact count
- `provisioning_state` — how far Query API credential provisioning had got: `bearer`, `stored_key`, `management_key`, `provisioning`, `provisioned`, `refused`
- `duration_bucket` — how long the operation ran before failing, as a bucket (`lt_250ms`, `lt_1s`, `lt_5s`, `lt_30s`, `lt_2m`, `ge_2m`)

The privacy boundary is again structural rather than a filter: these values are fixed strings compiled into the binary (plus one number from an allowlist), and they are set only at the points in the code that own a given failure — a category is never derived from an error message. Your SQL, database and format values, file paths, service and organization IDs, API response text, and credentials therefore have no representation in these fields at all. A field is omitted, never sent as `null`, when it does not apply, and no failure classification is attached to a successful run or to the censored `exec_attempt` handoff described below.

Exactly one event is recorded per invocation. Two commands are special: `local client` and `local postgres client` hand the process over to the native `clickhouse client` or to `psql` with `exec()`, which replaces clickhousectl's process image — same PID, same process group, same terminal, inherited stdin/stdout/stderr — so that Ctrl-C, job control and the program's own exit status or fatal signal reach your shell exactly as if you had run it directly. Because clickhousectl is gone at that point, its event is recorded just before the handover and is explicitly *censored*: the outcome is `exec_attempt` and its exit code is a fixed `0`, which means "the handoff was reached" and never "the native client succeeded". Failures clickhousectl can see for itself — a build that is missing, is not a regular file, or has no execute bit, or a `psql` that is not on `PATH` — are refused before the handover, so they are ordinary failures with the real exit code and a message telling you how to repair the install.

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

Maintainer operation, exact-SHA overrides, stacked-PR policy, and the required
check rollout procedure are documented in
[`.github/CLOUD_INTEGRATION.md`](.github/CLOUD_INTEGRATION.md).

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
