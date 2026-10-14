# Cloud authentication and access

[All documentation](../README.md)

## Choose credentials

| Mode | Scope and capabilities | Storage |
| --- | --- | --- |
| Browser OAuth | User identity, read-only Cloud operations and SQL | `~/.clickhouse/tokens.json`, shared across directories |
| API key | Organization-scoped; writes and reads depend on assigned roles | `.clickhouse/credentials.json` when saved, local to the project |

```bash
clickhousectl cloud auth signup
clickhousectl cloud auth login
clickhousectl cloud auth login --interactive
clickhousectl cloud auth whoami
clickhousectl cloud auth status
```

`signup` opens account creation. Plain `login` opens the OAuth device flow; `--interactive` prompts for an API key and secret. Login checks identity: a rejected key is not saved and exits 4. If the API is unreachable, the key is saved with a warning for offline setup. `whoami` requires no organization selection and supports either credential type.

`auth status` shows configured sources, the active source, and identity verification. It can exit 0 with no active credentials, rejected credentials, or an unavailable identity check; use `whoami` or the intended command to test access. Its remote check times out after five seconds.

For automation, inject `CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` through a secret manager. The CLI does not save environment credentials. For development it also reads `.env` from the exact current directory; keep that file out of version control and restrict access. Flags are available, but command-line secrets can appear in shell history and process listings.

## Credential precedence

The first applicable source wins:

1. `--api-key` and `--api-secret` flags.
2. `.clickhouse/credentials.json`.
3. Exported environment variables.
4. Variables loaded from the current directory's `.env`.
5. Global OAuth tokens.

Supplying only one credential flag blocks fallback. `auth status` identifies inactive sources; `--debug` on a Cloud command prints the selected credential source and API URL to stderr.

`cloud postgres query` accepts **OAuth only**, unlike `cloud service query`. Remove higher-priority API key sources before using that route. Clearing a saved key file alone is insufficient if exported variables or `.env` still supply a key.

## Select an organization and resource

```bash
clickhousectl cloud org list
clickhousectl cloud service list --org-id <org-id>
clickhousectl cloud service get --name analytics --org-id <org-id>
```

Organization-scoped commands auto-select only when credentials reach exactly one organization. Otherwise pass `--org-id` or, where supported, exact `--org-name`.

Most resource commands accept a positional ID or exact `--name`. Name lookup needs list permission within the selected organization or required parent service, and ambiguous matches fail. A child name selects the child, never its parent service. Member and invitation `--email` matching is exact, including case.

## Check permissions before an operation

Every executable Cloud command's `--help` lists bundled API-key permission requirements without making a network request:

```bash
clickhousectl cloud service get --help
clickhousectl cloud service query --help
```

All applicable permissions are needed, including conditional permissions for name lookup, optional flags, and Query API binding. An empty upstream permission list still requires a valid key. Permission help does not inspect your credentials, change OAuth's read-only restriction, or grant SQL privileges.

Schema discovery for ClickPipes requires an API key. Postgres query is OAuth-only. See [Query API behavior](query-api.md) for ClickHouse SQL and [key administration](#create-a-restricted-api-key) for key roles and IP allowlists.

## Create a restricted API key

Find a role appropriate for the intended commands, then create a key with an explicit trusted egress IP/CIDR and expiration:

```bash
clickhousectl cloud org role list --org-id <org-id>
clickhousectl cloud key create --org-id <org-id> --name ci-key \
  --role-id <role-id> --expires-at <future-RFC3339-time> \
  --ip-allow '203.0.113.10/32=CI runner'
```

Replace the example IP. **Omitting `--ip-allow` on key creation denies all network access**, unlike service creation's allow-all default. Use `0.0.0.0/0` only for deliberately unrestricted access.

Save the generated authentication key ID and secret securely; the secret is returned only at creation. The management resource `id` used by `get`, `update`, `delete`, and endpoint bindings differs from the authentication `keyId`/`keySecret`. Pre-hashed key creation does not return a secret.

## Inspect all keys and change access

```bash
clickhousectl cloud key list --all --json
clickhousectl cloud key get <key-id>
clickhousectl cloud key update <key-id> --state disabled
```

Without `--all`, list returns one page. `--limit` is 1–250 and `--cursor` takes the opaque `nextCursor` unchanged. One-page JSON has a `result` array and optional metadata; only an absent `nextCursor` marks the last page (an empty string is still a token). `--all --json` combines pages into one array and emits no partial array on failure; it conflicts with `--cursor`. Name selectors search every page and reject ambiguity.

Omitting expiry, role, or IP flags on update preserves that setting. `--clear-expiry`, `--clear-roles`, and `--clear-ip-allow` deliberately remove it and conflict with the corresponding set flag. Clearing the IP list denies all network access. After disabling/deleting a key, inspect dependent automation and Query API bindings; do not rotate keys automatically to bypass an administrator's restriction.

## Roles and membership

```bash
clickhousectl cloud member list
clickhousectl cloud member update <user-id> --role-id <role-id>
clickhousectl cloud invitation create --email dev@example.com --role-id <role-id>
```

Member/invitation email selectors match exactly, including case. Omitting member role flags preserves assignments; `--clear-roles` removes all and conflicts with `--role-id`.

Custom organization roles are created/updated with `--file` (`-` for stdin). Updates are partial at the root, but supplied `actors` and `policies` replace their complete lists. Only custom roles can be updated/deleted. Policy `allowDeny` is `ALLOW` or `DENY`; SQL `roleV2` values are `sql-console-readonly` or `sql-console-admin`. Inspect `org role get` and prepare a writable request without unrelated response fields.

## Other organization tasks

Use `cloud org --help` for quotas, credit balance, usage, settings, BYOC infrastructure, and Prometheus discovery. BYOC `validate` is a preflight that creates nothing but requires an API key. Creation is asynchronous: find infrastructure IDs in `org get`'s `byocConfig`, then inspect `byoc get` and `byoc progress` before creating a service in it.

BYO-VPC `--vpc-id` needs `--private-subnet-id` and conflicts with `--vpc-cidr-range`. For private endpoint registration, use the provider's endpoint identity, not an endpoint-service name or ARN; registration affects the organization as well as the service. Removing an organization endpoint requires both provider and region to identify it completely. The CLI's format checks do not prove an endpoint exists or belongs to you.

## Log out and clean up keys

```bash
clickhousectl cloud auth logout --oauth
clickhousectl cloud auth logout --api-keys
clickhousectl cloud auth logout
```

The last command clears both saved credential types. It does not unset environment variables or remove `.env` values. Clearing API keys deletes the entire local credentials file, including stored per-service Query API records from earlier versions or explicit repair. Before doing so, delete any CLI-created cloud-side keys you no longer need (`cloud service delete` handles stored-key cleanup, or use `cloud key delete <key-id>`); otherwise their management IDs are lost locally. Review [stored Query API keys](query-api.md#stored-keys-and-deliberate-repair) first.
