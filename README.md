# clickhousectl

`clickhousectl` is the CLI for ClickHouse: local and cloud.

With `clickhousectl` you can:
- Install and manage local ClickHouse versions
- Launch and manage local ClickHouse servers
- Execute queries against ClickHouse servers, or using clickhouse-local
- Setup ClickHouse Cloud and create cloud-managed ClickHouse clusters
- Manage ClickHouse Cloud resources
- Push your local ClickHouse development to cloud

`clickhousectl` helps humans and AI-agents to develop with ClickHouse.

## Installation

### Quick install

```bash
curl -fsSL https://raw.githubusercontent.com/ClickHouse/clickhousectl/main/install.sh | sh
```

The install scripts will download the correct version for your OS and install to `~/.local/bin/clickhousectl`.

### From source

```bash
cargo install --path .
```

## Usage

### Version Management

```bash
# Install a version
clickhousectl install stable          # Latest stable release
clickhousectl install lts             # Latest LTS release
clickhousectl install 25.12           # Latest 25.12.x.x
clickhousectl install 25.12.5.44      # Exact version

# List versions
clickhousectl list                    # Installed versions
clickhousectl list --remote           # Available for download

# Manage default version
clickhousectl use 25.12.5.44          # Exact version
clickhousectl use stable              # Latest stable (installs if needed)
clickhousectl use lts                 # Latest LTS (installs if needed)
clickhousectl use 25.12               # Latest 25.12.x.x (installs if needed)
clickhousectl which                   # Show current default

# Remove a version
clickhousectl remove 25.12.5.44
```

### Project Initialization

```bash
# Initialize a project-local ClickHouse data directory and project scaffold
clickhousectl init
```

This creates two directories:

1. **`.clickhouse/`** — Runtime data directory (git-ignored). Data is scoped by version so switching versions with `clickhousectl use` won't cause compatibility issues. `clickhousectl run server` automatically creates this if needed.

2. **`clickhouse/`** — Project scaffold for organizing your SQL files (meant to be committed):

```
clickhouse/
├── tables/         # Table definitions (CREATE TABLE ...)
│   └── .gitkeep
├── materialized_views/  # Materialized view definitions
│   └── .gitkeep
├── queries/        # Saved queries
│   └── .gitkeep
└── seed/           # Seed data / INSERT statements
    └── .gitkeep
```

The `clickhouse/` scaffold is only created by `clickhousectl init`, not by `clickhousectl run server`.

### Running ClickHouse

```bash
# Quick SQL query (uses clickhouse local)
clickhousectl run --sql "SELECT 1"
clickhousectl run -s "SELECT * FROM system.functions LIMIT 5"

# Run clickhouse local with full options
clickhousectl run local --query "SELECT 1"
clickhousectl run local -- --help

# Run clickhouse client
clickhousectl run client
clickhousectl run client -- --host localhost --query "SHOW DATABASES"

# Run clickhouse server (auto-initializes .clickhouse/ in CWD)
clickhousectl run server
clickhousectl run server -- --config-file=/path/to/config.xml
```

### ClickHouse Cloud

Manage ClickHouse Cloud services via the API.

#### Authentication

The easiest way to authenticate is interactively:
```bash
clickhousectl cloud auth
```

This prompts for your API key and secret, and saves them to `.clickhouse/credentials.json` (project-local, git-ignored).

You can also use environment variables:
```bash
export CLICKHOUSE_CLOUD_API_KEY=your-key
export CLICKHOUSE_CLOUD_API_SECRET=your-secret
```

Or pass credentials directly via flags:
```bash
clickhousectl cloud --api-key KEY --api-secret SECRET ...
```

Credential resolution order: CLI flags > `.clickhouse/credentials.json` > environment variables.

#### Organizations

```bash
clickhousectl cloud org list              # List organizations
clickhousectl cloud org get <org-id>      # Get organization details
```

#### Services

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

# Start/stop a service
clickhousectl cloud service start <service-id>
clickhousectl cloud service stop <service-id>

# Delete a service
clickhousectl cloud service delete <service-id>
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
| `--byoc-id` | BYOC region ID |
| `--compliance-type` | Compliance: hipaa, pci |
| `--profile` | Instance profile (enterprise) |

#### Backups

```bash
clickhousectl cloud backup list <service-id>
clickhousectl cloud backup get <service-id> <backup-id>
```

#### JSON Output

Add `--json` for machine-readable output (useful for AI agents):

```bash
clickhousectl cloud --json service list
clickhousectl cloud --json service get <service-id>
```

## Storage

Versions of the ClickHouse binary are are stored in `~/.clickhouse/`:

```
~/.clickhouse/
├── versions/
│   └── 25.12.5.44/
│       └── clickhouse
└── default
```

## Requirements

- macOS (aarch64, x86_64) or Linux (aarch64, x86_64)
- Binaries are downloaded from [ClickHouse GitHub releases](https://github.com/ClickHouse/ClickHouse/releases)
- Cloud commands require a [ClickHouse Cloud API key](https://clickhouse.com/docs/en/cloud/manage/api)
