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

`signup` opens account creation. Plain `login` opens the OAuth device flow; in JSON mode its code and URL go to stderr, so stdout holds only the login report. `--interactive` prompts for an API key and secret; it needs a TTY and cannot be combined with `--api-key` or `--api-secret` (both exit 2). Login checks identity: a rejected key is not saved and exits 4. If the API is unreachable, the key is saved with a warning for offline setup. `whoami` requires no organization selection and supports either credential type.

`auth status` shows configured sources, the active source, and identity verification. It can exit 0 with no active credentials, rejected credentials, or an unavailable identity check; use `whoami` or the intended command to test access. Its remote check times out after five seconds.

If a saved OAuth login cannot be read or uses an older format without complete authorization configuration, `auth status` shows OAuth as `Login required`, not `Not configured`, and explains the problem below the table with the command to log in again. JSON output includes this explanation in `oauth_warning`, separate from any identity-verification `warning`; it is shown even when an API key is active. Run `clickhousectl cloud auth login` to log in again. Without a higher-priority API key source, Cloud commands exit 4 (`auth_required` in JSON) with an explicit re-login message. The unusable file is left unchanged; the CLI does not reconstruct its authorization configuration from built-in defaults or environment variables.

For automation, inject `CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` through a secret manager. The CLI does not save environment credentials. For development it also reads `.env` from the exact current directory; keep that file out of version control and restrict access. Flags are available, but command-line secrets can appear in shell history and process listings.

## Credential precedence

The first applicable source wins:

1. `--api-key` and `--api-secret` flags.
2. `.clickhouse/credentials.json`.
3. Exported environment variables.
4. Variables loaded from the current directory's `.env`.
5. Global OAuth tokens.

Supplying only one credential flag blocks fallback. Cloud commands skip OAuth token refresh when a higher-priority API key source wins, leaving saved OAuth tokens unchanged. When OAuth is selected, expired or near-expiry tokens are refreshed before the API request. If the login has no refresh token or the refresh fails, including when the authorization server is unreachable, the saved login is kept: `auth status` shows OAuth as `Expired`, and the command exits 4 (`auth_required` in JSON) with a message to run `clickhousectl cloud auth login`. `auth status` identifies inactive sources; `--debug` on a Cloud command prints the selected credential source and API URL to stderr.

A saved API key works only against the API it was saved for. When `auth login --api-key` verifies a key against a non-production API (`--url`), that API is saved with the key as `api_url`; a key with no `api_url` is for production. Later commands without `--url` use the saved API, print a note naming it to stderr, and derive the Query API host for `cloud service query` from it unless `CLICKHOUSE_CLOUD_QUERY_HOST` is set. A `--url` naming any other API exits 4 without sending the key or falling back to environment or OAuth credentials: pass the saved URL, or save a key for that API. `auth status` shows the API each saved source uses. Flag and environment keys carry no URL; they use `--url` or production.

`cloud access roles --debug` and `cloud access request --debug` report their OAuth credential source and resolved API URL on stderr, including with `--json`. These experimental commands use OAuth even when higher-priority API keys are configured; the diagnostics describe the identity actually used for access negotiation.

`cloud postgres query` accepts **OAuth only**, unlike `cloud service query`. Remove higher-priority API key sources before using that route. Clearing a saved key file alone is insufficient if exported variables or `.env` still supply a key.

## Select an organization and resource

```bash
clickhousectl cloud org list
clickhousectl cloud service list --org-id <org-id>
clickhousectl cloud service get --name analytics --org-id <org-id>
```

Organization-scoped commands auto-select only when credentials reach exactly one organization. Otherwise pass `--org-id` or, where supported, exact `--org-name`. When credentials reach several organizations and neither is given, the command exits 2 as a usage error and lists each organization's ID and name. Organization names need not be unique: when `--org-name` matches more than one, the command exits 1 and lists the matching IDs to pass to `--org-id`.

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

## Experimental: negotiate a scoped API key

> **Experimental.** `cloud access` is compiled only into builds with the `experimental-access-negotiation` feature. Published binaries (install script, npm, `cargo install clickhousectl`) do not include it and report `unrecognized subcommand 'access'`. Build it from a checkout:
>
> ```bash
> cargo install --path crates/clickhousectl --features experimental-access-negotiation
> ```
>
> The command surface and behavior may change without notice.

### Discover assignable roles

```bash
clickhousectl cloud access roles
clickhousectl cloud access roles --org <organization-id> --json
```

Discovery uses your refreshed OAuth user identity even when an API key is saved, with the same organization selectors and endpoint precedence as `cloud access request`. Human output identifies the selected organization and lists role IDs/names with policy effects and resource scopes. `--json` returns exactly the complete role array, including all ALLOW/DENY policies and optional SQL tags; an eligible empty menu is `[]`. SQL grant/revoke strings and unknown values remain opaque. Incomplete required discovery data fails before any menu is printed, rather than implying empty permissions or falling back to the administrative catalog. Discovery describes current authority, not a guarantee of later approval or a role recommendation.

### Request a scoped API key

`cloud access request` obtains a scoped API key through the control plane's PKCE approval handshake, so an agent can get a narrowly-scoped key that a human approves in a browser rather than reusing a long-lived one.

```bash
clickhousectl cloud access request
clickhousectl cloud access request --key-name agent-key --purpose "CI deploy"
clickhousectl cloud access request --org <organization-id> --perms role:<role-uuid>,role:<another-role-uuid>
clickhousectl cloud access request --json
```

Use IDs from `cloud access roles` to propose roles with `--perms role:<UUID>` (repeat the flag or separate tokens with commas). UUIDs are normalized to lowercase and deduplicated by identity in first-seen order; at most 100 distinct roles are allowed. Empty tokens, whitespace, wrong prefixes and malformed UUIDs fail before authentication or network access. Role names, raw permissions and resource overrides are not supported. Omit `--perms` for manual browser selection; an explicitly empty value is invalid. A proposal is not a grant: the server validates the whole proposal atomically, and the human's eventual selection may differ. The CLI neither filters rejected proposals nor retries a subset.

The command requires your OAuth user identity from `cloud auth login` and refreshes it when needed; API keys from flags, saved credentials, or environment variables are never used for negotiation. Missing or expired identity that cannot be refreshed fails with authentication exit code 4 and login guidance. It runs the authorization code flow with PKCE: it starts a loopback redirect server on `127.0.0.1` (or `[::1]` when IPv4 loopback is unavailable), generates a PKCE verifier/challenge and a CSRF `state`, files the request, and opens the approval URL (always printed in case the browser does not open). After you approve, the browser redirects to the CLI with a one-time authorization code. The CLI exchanges that code, the PKCE verifier and the same redirect URI for the issued key, and stores the key id and secret in `.clickhouse/credentials.json`. The secret is never printed or placed in a URL. A redirect with a missing or wrong `state` is refused and the wait continues. If no loopback listener can be started, the command fails.

The code arrives only on the browser redirect, so the browser must run on this machine. The CLI does not read the request status while waiting: only the redirect or the deadline ends the wait. A denial in the approval page redirects back with `error=access_denied` and ends the wait at once; a denial or cancellation that never reaches this browser waits until the deadline. Pending approval waits until the server pending deadline (ten minutes after filing) or two minutes by default, whichever comes first; the error says which one ended the wait. See [approval timeout configuration](#approval-timeout).

The approval URL and progress always go to stderr, including with `--json`; JSON stdout contains only the final success object. Denial, expiry and authentication failures never replace saved credentials. A rejected authorization code or PKCE verifier (HTTP 400 or 401 from the exchange) exits 1 with the server's message; it does not ask you to log in again. Each access HTTP operation is bounded to 30 seconds. Filing and the exchange are not retried: a timeout may leave a filed request or a completed server exchange without local credentials.

Successful requests report the organization and request IDs, the server-saved proposal, the actual roles returned by the exchange, and when the key expires (issued keys are short-lived). JSON retains `status`, `keyId`, `key`, and `credentialsPath`, and adds `requestId`, `organizationId`, `requestedRoleIds`, `grantedRoleIds`, and `keyExpiresAt` (epoch milliseconds, or `null` when the server sends none). With no proposal, `requestedRoleIds` is `[]` (manual selection); the human may trim or add roles, so actual grants need not match the proposal. Missing or invalid server role arrays fail before local credential installation; older servers without this reporting contract are unsupported.

The API URL comes from `--url` when provided, otherwise the API URL saved with your OAuth login. Token records without a saved API URL are rejected; run `cloud auth login` again. Local development requires an explicit local URL at login or through `--url`; there is no implicit localhost default. Other Cloud commands retain their normal credential precedence, so a saved negotiated key remains available for subsequent operations. Optional `--purpose` (1–200 characters, shown to the approver) and `--key-name` (1–100 characters) are sent with the request; the calling agent is identified by the CLI User-Agent and agent-session headers.

Access organization selection uses your OAuth user's complete, paginated membership list, not the saved API key's organization. A single membership is selected automatically; zero or multiple memberships require action (no remembered default or prompt). Find IDs with `cloud org list --json` and select `cloud access request --org <ID>` (`--org-id` remains supported), or use exact `--org-name`. Conflicting selectors, including across command depths, are usage errors.

### Approval timeout

`cloud access request` waits up to **two minutes** for human approval. Override it with `--timeout-seconds` or `CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS` (positive whole seconds, at most 4294967295):

```bash
clickhousectl cloud access request --org-id <org-id> --timeout-seconds 600

export CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS=600
clickhousectl cloud access request --org-id <org-id>
```

Precedence is flag, environment, then the 120-second default. This setting is not read from `.env` (the CLI uses that file only for credential configuration). Invalid values fail with usage exit code 2 before authentication or filing a request. The CLI reports the configured wait on stderr.

An earlier server pending deadline still wins; increasing this setting cannot extend server expiry. The error names the deadline that ended the wait: "access request expired" for the server deadline, "approval wait timed out after N seconds" for this setting. The authorization code arrives only on the browser redirect, and the CLI never reads the request status. The 30-second bound per HTTP operation is unchanged. This setting does not extend OAuth token validity or the issued key lifetime.

## Inspect all keys and change access

```bash
clickhousectl cloud key list --all --json
clickhousectl cloud key get <key-id>
clickhousectl cloud key update <key-id> --state disabled
```

Without `--all`, list returns one page. `--limit` is at least 1 and `--cursor` takes the opaque `nextCursor` unchanged. One-page JSON has a `result` array and optional metadata; only an absent `nextCursor` marks the last page (an empty string is still a token). `--all --json` combines pages into one array and emits no partial array on failure; it conflicts with `--cursor`. Name selectors search every page and reject ambiguity.

Omitting expiry, role, or IP flags on update preserves that setting. `--clear-expiry`, `--clear-roles`, and `--clear-ip-allow` deliberately remove it and conflict with the corresponding set flag. Clearing the IP list denies all network access. After disabling/deleting a key, inspect dependent automation and Query API bindings; do not rotate keys automatically to bypass an administrator's restriction.

## Roles and membership

```bash
clickhousectl cloud member list
clickhousectl cloud member update <user-id> --role-id <role-id>
clickhousectl cloud invitation create --email dev@example.com --role-id <role-id>
```

Member/invitation email selectors match exactly, including case. Omitting member role flags preserves assignments; `--clear-roles` removes all and conflicts with `--role-id`.

Custom organization roles are created/updated with `--file` (`-` for stdin). Updates are partial at the root, but supplied `actors` and `policies` replace their complete lists. Only custom roles can be updated/deleted. Policy `allowDeny` is `ALLOW` or `DENY`; SQL `roleV2` values are `sql-console-readonly` or `sql-console-admin`. Inspect `org role get` and prepare a writable request without unrelated response fields.

## Read organization notifications

```bash
clickhousectl cloud org notification-types --org-id <org-id>
clickhousectl cloud org notifications --org-id <org-id> --severity warning \
  --from-date 2026-10-01T00:00:00Z --limit 100 --json
clickhousectl cloud org notifications --org-id <org-id> --cursor '<nextCursor>' --json
```

Notifications are Beta and support OAuth reads. Each invocation returns one page; JSON contains `result` and optional `limit`/`nextCursor`. Pass the cursor unchanged until it is absent; an empty string is still a cursor. Keep the same filters across pages. `--limit` accepts 1–250; omission uses the server default of 100. Results are retained for 180 days and include only resources the caller can access.

Use `--type` with a name from `notification-types`, `--severity`, `--resource-id`, or `--resource-type` to narrow results. A service ID also matches its ClickPipes unless a resource kind narrows it. `--from-date` and `--to-date` are inclusive RFC 3339 timestamps. Human output shows a table and the next cursor as a JSON string; JSON preserves notification descriptions and resource names too.

## Other organization tasks

Use `cloud org --help` for quotas, credit balance, notifications, notification types, usage, settings, BYOC infrastructure, and Prometheus discovery. BYOC `validate` is a preflight that creates nothing but requires an API key. Creation is asynchronous: find infrastructure IDs in `org get`'s `byocConfig`, then inspect `byoc get` and `byoc progress` before creating a service in it.

`byoc update` changes only the flags you pass. Its `--tag` flags replace the infrastructure's whole tag set, so read the current set with `byoc tags` first; `byoc private-endpoint-config` shows the private link endpoint service to connect to:

```bash
clickhousectl cloud org byoc tags <byoc-id>
clickhousectl cloud org byoc update <byoc-id> --tag team=data --tag env=prod
clickhousectl cloud org byoc update <byoc-id> --enable-private-link true
clickhousectl cloud org byoc private-endpoint-config <byoc-id>
```

BYO-VPC `--vpc-id` needs `--private-subnet-id` and conflicts with `--vpc-cidr-range`. For private endpoint registration, use the provider's endpoint identity, not an endpoint-service name or ARN; registration affects the organization as well as the service. Removing an organization endpoint requires both provider and region to identify it completely. The CLI's format checks do not prove an endpoint exists or belongs to you.

## Log out and clean up keys

```bash
clickhousectl cloud auth logout --oauth
clickhousectl cloud auth logout --api-keys
clickhousectl cloud auth logout
```

The last command clears both saved credential types. It does not unset environment variables or remove `.env` values. Clearing API keys deletes the entire local credentials file, including stored per-service Query API records from earlier versions or explicit repair. Before doing so, delete any CLI-created cloud-side keys you no longer need (`cloud service delete` handles stored-key cleanup, or use `cloud key delete <key-id>`); otherwise their management IDs are lost locally. Review [stored Query API keys](query-api.md#stored-keys-and-deliberate-repair) first.
