# clickhousectl

> **Beta:** `clickhousectl` is currently in beta. Features and behavior may change.

`clickhousectl` is the CLI for ClickHouse: local and cloud.

With `clickhousectl` you can:
- Install and manage local ClickHouse versions
- Launch and manage local ClickHouse servers
- Execute queries against ClickHouse servers
- Setup ClickHouse Cloud and create cloud-managed ClickHouse clusters
- Manage ClickHouse Cloud resources
- Install the official ClickHouse agent skills into supported coding agents
- Push your local ClickHouse development to cloud

`clickhousectl` helps humans and AI-agents to develop with ClickHouse.

## Installation

### Quick install

```bash
curl https://clickhouse.com/cli | sh
```

The install script will download the correct version for your OS and install to `~/.local/bin/clickhousectl`. A `chctl` alias is also created automatically for convenience.

### From source

```bash
cargo install --path .
```

## Local

### Installing and managing ClickHouse versions

`clickhousectl` downloads ClickHouse binaries from [GitHub releases](https://github.com/ClickHouse/ClickHouse/releases).

```bash
# Install a version
clickhousectl local install stable          # Latest stable release
clickhousectl local install lts             # Latest LTS release
clickhousectl local install 25.12           # Latest 25.12.x.x
clickhousectl local install 25.12.5.44      # Exact version

# List versions
clickhousectl local list                    # Installed versions
clickhousectl local list --remote           # Available for download

# Manage default version
clickhousectl local use stable              # Latest stable (installs if needed)
clickhousectl local use lts                 # Latest LTS (installs if needed)
clickhousectl local use 25.12               # Latest 25.12.x.x (installs if needed)
clickhousectl local use 25.12.5.44          # Exact version
clickhousectl local which                   # Show current default

# Remove a version
clickhousectl local remove 25.12.5.44
```

#### ClickHouse binary storage

ClickHouse binaries are stored in a global repository, so they can be used by multiple projects without duplicating storage. Binaries are stored in `~/.clickhouse/`:

```
~/.clickhouse/
├── versions/
│   └── 25.12.5.44/
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
```

### Running queries

```bash
# Connect to a running server with clickhouse-client
clickhousectl local client                           # Connects to "default" server
clickhousectl local client --name dev                # Connects to "dev" server
clickhousectl local client --query "SHOW DATABASES"  # Run a query
clickhousectl local client --queries-file schema.sql # Run queries from a file
clickhousectl local client --host remote-host --port 9000  # Connect to a specific host/port
```

### Creating and managing ClickHouse servers

Start and manage ClickHouse server instances. Each server gets its own isolated data directory at `.clickhouse/servers/<name>/data/`.

```bash
# Start a server (runs in background by default)
clickhousectl local server start                          # Named "default"
clickhousectl local server start --name dev               # Named "dev"
clickhousectl local server start --version stable         # Use a specific version (installs if needed, doesn't change default)
clickhousectl local server start --foreground             # Run in foreground (-F / --fg)
clickhousectl local server start --http-port 8124 --tcp-port 9001  # Explicit ports
clickhousectl local server start -- --config-file=/path/to/config.xml

# List all servers (running and stopped)
clickhousectl local server list

# Stop servers
clickhousectl local server stop default                   # Stop by name
clickhousectl local server stop-all                       # Stop all running servers

# Remove a stopped server and its data
clickhousectl local server remove test
```

**Server naming:** Without `--name`, the first server is called "default". If "default" is already running, a random name is generated (e.g. "bold-crane"). Use `--name` for stable identities you can start/stop repeatedly.

**Ports:** Defaults are HTTP 8123 and TCP 9000. If these are already in use, free ports are automatically assigned and shown in the output. Use `--http-port` and `--tcp-port` to set explicit ports.

#### Project-local data directory

All server data lives inside `.clickhouse/` in your project directory:

```
.clickhouse/
├── .gitignore              # auto-created, ignores everything
├── credentials.json        # cloud API credentials (if configured)
└── servers/
    ├── default/
    │   └── data/           # ClickHouse data files for "default" server
    └── dev/
        └── data/           # ClickHouse data files for "dev" server
```

Each named server has its own data directory, so servers are fully isolated from each other. Data persists between restarts — stop and start a server by name to pick up where you left off. Use `clickhousectl local server remove <name>` to permanently delete a server's data.

## Authentication

Authenticate to ClickHouse Cloud using OAuth (browser-based) or API keys. OAuth provides **read-only** access; API keys provide full **read/write** access.

### OAuth login (read-only)

```bash
clickhousectl cloud auth login
```

This opens your browser for authentication via the OAuth device flow. Tokens are saved to `.clickhouse/tokens.json` (project-local).

> **Note:** OAuth tokens provide **read-only** access. You can list and inspect resources (organizations, services, backups, etc.) but cannot create, modify, or delete them. For write operations, use API key authentication.

### API key/secret (required for write operations)

```bash
# Non-interactive (CI-friendly)
clickhousectl cloud auth login --api-key YOUR_KEY --api-secret YOUR_SECRET

# Interactive prompt
clickhousectl cloud auth login --interactive
```

Credentials are saved to `.clickhouse/credentials.json` (project-local).

You can also use environment variables:
```bash
export CLICKHOUSE_CLOUD_API_KEY=your-key
export CLICKHOUSE_CLOUD_API_SECRET=your-secret
```

Or pass credentials directly via flags on any command:
```bash
clickhousectl cloud --api-key KEY --api-secret SECRET ...
```

Learn how to [create API keys](https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl).

### Auth status and logout

```bash
clickhousectl cloud auth status    # Show current auth state (including read-only/read-write labels)
clickhousectl cloud auth logout    # Clear all saved credentials (credentials.json & tokens.json)
```

Credential resolution order: CLI flags > `.clickhouse/credentials.json` > environment variables > OAuth tokens.

## Cloud

Manage ClickHouse Cloud services via the API.

### Organizations

```bash
clickhousectl cloud org list              # List organizations
clickhousectl cloud org get <org-id>      # Get organization details
clickhousectl cloud org update <org-id> --name "Renamed Org"
clickhousectl cloud org update <org-id> \
  --remove-private-endpoint pe-1,cloud-provider=aws,region=us-east-1 \
  --enable-core-dumps false
clickhousectl cloud org prometheus <org-id> --filtered-metrics true
clickhousectl cloud org usage <org-id> \
  --from-date 2024-01-01 \
  --to-date 2024-01-31
```

### Services

```bash
# List services
clickhousectl cloud service list

# Get service details
clickhousectl cloud service get <service-id>

# Create a service (minimal)
clickhousectl cloud service create --name my-service

# Create with scaling options
clickhousectl cloud service create --name my-service \
  --provider aws \
  --region us-east-1 \
  --min-replica-memory-gb 8 \
  --max-replica-memory-gb 32 \
  --num-replicas 2

# Create with specific IP allowlist
clickhousectl cloud service create --name my-service \
  --ip-allow 10.0.0.0/8 \
  --ip-allow 192.168.1.0/24

# Create from backup
clickhousectl cloud service create --name restored-service --backup-id <backup-uuid>

# Create with release channel
clickhousectl cloud service create --name my-service --release-channel fast

# Create with GA request-only extras
clickhousectl cloud service create --name my-service \
  --tag env=prod \
  --enable-endpoint mysql \
  --private-preview-terms-checked \
  --enable-core-dumps true

# Start/stop a service
clickhousectl cloud service start <service-id>
clickhousectl cloud service stop <service-id>

# Connect to a cloud service with clickhouse-client
clickhousectl cloud service client --name my-service --password secret
clickhousectl cloud service client --id <service-id> -q "SELECT 1" --password secret

# Use CLICKHOUSE_PASSWORD env var (recommended for scripts/agents)
CLICKHOUSE_PASSWORD=secret clickhousectl cloud service client --name my-service -q "SELECT count() FROM system.tables"

# Use a local client version instead of auto-downloading the matching one
clickhousectl cloud service client --name my-service --allow-mismatched-client-version

# Update service metadata and patches
clickhousectl cloud service update <service-id> \
  --name my-renamed-service \
  --add-ip-allow 10.0.0.0/8 \
  --remove-ip-allow 0.0.0.0/0 \
  --add-private-endpoint-id pe-1 \
  --release-channel fast \
  --enable-endpoint mysql \
  --add-tag env=staging \
  --transparent-data-encryption-key-id tde-key-1 \
  --enable-core-dumps false

# Update replica scaling
clickhousectl cloud service scale <service-id> \
  --min-replica-memory-gb 24 \
  --max-replica-memory-gb 48 \
  --num-replicas 3 \
  --idle-scaling true \
  --idle-timeout-minutes 10

# Reset password with generated credentials
clickhousectl cloud service reset-password <service-id>

# Reset password with precomputed hashes
clickhousectl cloud service reset-password <service-id> \
  --new-password-hash <base64-sha256-hash> \
  --new-double-sha1-hash <mysql-double-sha1-hash>

# Query endpoint management
clickhousectl cloud service query-endpoint get <service-id>
clickhousectl cloud service query-endpoint create <service-id> \
  --role admin \
  --open-api-key key-1 \
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

**Service Create Options:**
| Option | Description |
|--------|-------------|
| `--name` | Service name (required) |
| `--provider` | Cloud provider: aws, gcp, azure (default: aws) |
| `--region` | Region (default: us-east-1) |
| `--min-replica-memory-gb` | Min memory per replica in GB (8-356, multiple of 4) |
| `--max-replica-memory-gb` | Max memory per replica in GB (8-356, multiple of 4) |
| `--num-replicas` | Number of replicas (1-20) |
| `--idle-scaling` | Allow scale to zero (default: true) |
| `--idle-timeout-minutes` | Min idle timeout in minutes (>= 5) |
| `--ip-allow` | IP CIDR to allow (repeatable, default: 0.0.0.0/0) |
| `--backup-id` | Backup ID to restore from |
| `--release-channel` | Release channel: slow, default, fast |
| `--data-warehouse-id` | Data warehouse ID (for read replicas) |
| `--readonly` | Make service read-only |
| `--encryption-key` | Customer disk encryption key |
| `--encryption-role` | Role ARN for disk encryption |
| `--enable-tde` | Enable Transparent Data Encryption |
| `--compliance-type` | Compliance: hipaa, pci |
| `--profile` | Instance profile (enterprise) |
| `--tag` | Attach a GA service tag (`key` or `key=value`) |
| `--enable-endpoint` / `--disable-endpoint` | Toggle GA service endpoints (currently `mysql`) |
| `--private-preview-terms-checked` | Accept private preview terms when required |
| `--enable-core-dumps` | Enable or disable service core dump collection |

### Backups

```bash
clickhousectl cloud backup list <service-id>
clickhousectl cloud backup get <service-id> <backup-id>
```

### Members

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
clickhousectl cloud key create --name ci-key --role-id <role-id> --ip-allow 10.0.0.0/8
clickhousectl cloud key create --name prehashed-key \
  --hash-key-id <hash> \
  --hash-key-id-suffix <suffix> \
  --hash-key-secret <hash>
clickhousectl cloud key update <key-id> \
  --name renamed-key \
  --expires-at 2025-12-31T00:00:00Z \
  --state disabled \
  --ip-allow 0.0.0.0/0
clickhousectl cloud key delete <key-id>
```

### Activity

```bash
clickhousectl cloud activity list --from-date 2024-01-01 --to-date 2024-12-31
clickhousectl cloud activity get <activity-id>
```

### JSON output

Use the `--json` flag to print JSON-formatted responses.

```bash
clickhousectl cloud --json service list
clickhousectl cloud --json service get <service-id>
```

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

The CLI also checks for updates in the background (at most once per 24 hours) and displays a notice when a newer version is available.

## Cloud integration testing

Cloud commands are tested against a real ClickHouse Cloud workspace. All changes to Cloud commands must pass CI testing before merge. CI tests are under [`tests/cloud_cli.rs`](tests/cloud_cli.rs).

Required environment variables:

```bash
export CLICKHOUSE_CLOUD_API_KEY=...
export CLICKHOUSE_CLOUD_API_SECRET=...
export CLICKHOUSE_CLOUD_TEST_ORG_ID=...
export CLICKHOUSE_CLOUD_TEST_PROVIDER=aws
export CLICKHOUSE_CLOUD_TEST_REGION=us-east-1
```

Run the CI test:

```bash
CLICKHOUSECTL_BIN=target/debug/clickhousectl \
cargo test --test cloud_cli cloud_service_crud_lifecycle -- --ignored --nocapture --test-threads=1
```

By default, any failed check fails the run. To keep going after `non-blocking` capability failures and collect them in a summary at the end, set:

```bash
export CONTINUE_ON_NON_BLOCKING_FAILURES=1
```

## Requirements

- macOS (aarch64, x86_64) or Linux (aarch64, x86_64)
- Cloud commands require a [ClickHouse Cloud API key](https://clickhouse.com/docs/en/cloud/manage/api)