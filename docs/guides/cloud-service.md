# Create and operate a Cloud service

[All documentation](../README.md)

## Create and connect

Use [API key authentication](../reference/authentication.md) with the permissions listed by each command's help. Choose the organization explicitly when you have access to more than one. Replace the example CIDR with a trusted client network.

```bash
clickhousectl cloud service create --org-id <org-id> --name analytics \
  --provider aws --region us-east-1 --ip-allow 203.0.113.10/32
clickhousectl cloud service get <service-id> --org-id <org-id>
```

Save the initial password from creation; it is shown once. Omitting `--provider` selects AWS, omitting `--region` selects `us-east-1`, and **omitting `--ip-allow` allows all IPs (`0.0.0.0/0`)**. Set these deliberately. Entries accept `IP_OR_CIDR=DESCRIPTION`; quote descriptions containing spaces.

`--region` accepts only the regions the Cloud API lists. For an internal or private region that isn't listed, such as `sa-east-1`, pass `--custom-region sa-east-1` instead. The CLI sends it unchecked and Cloud validates it.

Creation acknowledges the request before provisioning finishes. Repeat `get` until the service is `running`, then query it:

```bash
clickhousectl cloud service query <service-id> --org-id <org-id> \
  --query "SELECT version()"
```

The HTTP Query API needs no local ClickHouse binary or service password. OAuth gives read-only SQL. API-key queries may automatically bind the caller's key to a Query API endpoint; new endpoints use `sql_console_admin`. Pass `--no-auto-enable` to require an existing binding. See [SQL and Query API behavior](../reference/query-api.md) before automating writes, using bulk input, or retrying a timed-out query.

### Write connection settings to `.env`

The password is returned only once, so write it to your application's `.env` at creation with `--dotenv`. The password then goes only to the file, not to the terminal or JSON output. Without a file name it writes `.env` in the current directory.

```bash
clickhousectl cloud service create --org-id <org-id> --name analytics \
  --ip-allow 203.0.113.10/32 --dotenv .env.local
```

For an existing service, `service dotenv` writes the same settings without a password (`.env`, or `.env.local` with `--local`):

```bash
clickhousectl cloud service dotenv <service-id> --org-id <org-id> --local
```

Both write `CLICKHOUSE_HOST`, `CLICKHOUSE_PORT` (the native TLS port, as in `local clickhouse dotenv`), `CLICKHOUSE_HTTP_PORT`, `CLICKHOUSE_USER` (`default`) and `CLICKHOUSE_SECURE=true`; `create --dotenv` adds `CLICKHOUSE_PASSWORD`. An existing file is edited in place: only those keys are replaced, so a password written at creation survives a later `service dotenv`. If the file can't be written, `create` refuses before creating the service; if a write fails after creation, it prints the password as usual and exits 1. The file holds a credential: keep it out of version control.

## Change capacity and settings

Inspect the service and its available profiles before choosing memory and replica counts:

```bash
clickhousectl cloud service profile list --region us-east-1
clickhousectl cloud service get <service-id>
clickhousectl cloud service scale <service-id> \
  --min-replica-memory-gb 8 --max-replica-memory-gb 32 --num-replicas 2
clickhousectl cloud service get <service-id>
```

Cloud validates counts against the organization and warehouse limits. Horizontal autoscaling requires its organization feature; use `scale --help` for the separate minimum/maximum replica controls. BYOC services use `profile list --byoc-id` and the returned profile's exact memory size; see `cloud org byoc --help` for preflight and provisioning.

Enterprise and BYOC organizations can change the instance profile with `scale --profile`. Send a standard profile on its own, without memory flags; a BYOC profile needs `--min-replica-memory-gb` and `--max-replica-memory-gb` both equal to its memory size. While a scaling-schedule window is active, the new profile applies once the window ends.

```bash
clickhousectl cloud service scale <service-id> --profile v1-highmem-s
```

Discover service setting names and types before changing them:

```bash
clickhousectl cloud service settings schema <service-id>
clickhousectl cloud service settings set <service-id> --setting 'enable_analyzer=1'
clickhousectl cloud service settings get <service-id> enable_analyzer
```

Settings changes affect only supplied names. `--file` accepts the settings map itself, without a `settings` wrapper; `-` reads stdin. String values in `--setting NAME=JSON_VALUE` need JSON quotes. Numeric literals must be integers; use a JSON string for decimal/exponent values only if that setting accepts one. `settings unset` resets a name to its platform default.

For recurring capacity changes, `scaling-schedule set --file` replaces the complete entry list. Read `get --json`, retain all desired entries, and copy only writable fields into the request. Remove response-only `id`, `isActiveNow`, `activeEntryId`, and `baseConfig`. `{"entries":[]}` clears the list. The base configuration is managed by `service scale` and applies outside scheduled windows; deleting a schedule restores it if a window is active.

Schedule hours are UTC, start-inclusive and end-exclusive; `24` means midnight, and an end before the start creates an overnight window. Weekdays use Sunday `0` through Saturday `6`. Upgrade windows use the same weekday numbering, last six hours, and start at UTC hour `0`, `6`, `12`, or `18`. Set both day and hour when replacing a window. Only primary services can change it; secondaries inherit it.

Replica aware (sticky) routing is beta and set per protocol; an omitted flag leaves that protocol unchanged. Updating it requires the organization feature. Once HTTP routing is on, every HTTP client must send the `X-ClickHouse-Replica-Tag` header, so update clients before enabling it:

```bash
clickhousectl cloud service replica-aware-routing get <service-id>
clickhousectl cloud service replica-aware-routing update <service-id> --native-enabled true
```

Service tag and IP updates have explicit add/remove flags. Add and remove tags in separate calls. Unmatched removals exit 0 with a warning; verify the resulting service rather than relying on exit 0 to prove a match.

## Let users sign in with a JWT

JWT authentication providers are beta and need the Enterprise tier. A service accepts tokens from up to five providers. A token holder signs in as the `default` user and receives the ClickHouse roles listed in the token's roles claim. Cloud fetches the JWKS URL when you create or update a provider, so it must be a public `https://` URL that serves at least one RS256 key.

```bash
clickhousectl cloud service jwt-provider create <service-id> corp-sso \
  --issuer https://auth.example.com --audience analytics \
  --jwks-url https://auth.example.com/.well-known/jwks.json \
  --roles-claim clickhouse:roles
clickhousectl cloud service jwt-provider list <service-id>
```

Provider names use lowercase letters, digits and hyphens. Repeating `create` with the same name and configuration succeeds without a change; the same name with a different configuration fails, so change an existing provider with `update`.

**`update` replaces the whole configuration.** Pass `--issuer`, `--audience` and `--jwks-url` every time, and pass `--roles-claim` again to keep it: omitting it removes the stored roles claim. Read the current values first:

```bash
clickhousectl cloud service jwt-provider get <service-id> corp-sso
clickhousectl cloud service jwt-provider update <service-id> corp-sso \
  --issuer https://auth.example.com --audience analytics \
  --jwks-url https://auth.example.com/.well-known/jwks-v2.json \
  --roles-claim clickhouse:roles
```

`jwt-provider delete <service-id> <name>` removes a provider, and the service stops accepting its tokens. With `--name <service-name>` in place of the service ID, the provider name is the only positional argument.

## Back up and restore

```bash
clickhousectl cloud backup list <service-id>
clickhousectl cloud backup get <service-id> <backup-id>
clickhousectl cloud service create --name restored-analytics \
  --provider aws --region us-east-1 --ip-allow 203.0.113.10/32 \
  --backup-id <backup-id>
```

Restoration creates a new service. Save any returned credentials and wait for `running` before querying it. Service snapshots (`service snapshot`) are a separate beta feature from backups.

Use `service backup-config` to inspect or update retention and scheduling. Retention is in whole days, expressed as 24–1080 hours. A start time requires a 24- or 48-hour backup period, either already stored or set in the same update. `--clear-backup-start-time` removes that restriction and can be combined with a new period.

For a bring-your-own backup bucket, `cloud backup bucket create/update --file` takes provider-specific JSON. Keep credentials in a restricted file or stdin. Provider names are exactly `AWS`, `GCP`, or `AZURE`:

| Provider | Required fields alongside `bucketProvider` |
| --- | --- |
| AWS | `bucketPath` (`https://<bucket>.s3.<region>.amazonaws.com/<prefix>`), `iamRoleArn`, `iamRoleSessionName` (optional on update); `iamRoleExternalId` is optional |
| GCP | `bucketPath`, `accessKeyId`, `secretAccessKey` |
| Azure | `containerName`, `connectionString` |

Updates still require the provider's complete required fields, including GCP/Azure credentials. There is one backup-bucket resource per service, so no bucket ID is needed. For private-preview TDE restoration from your own bucket, pass the backup's `encryption_config.json` unchanged with `--backup-encryption-config` (or `-` for stdin).

`iamRoleExternalId` is the external ID required by the AWS role's trust policy. Send it only if your organization has the external-ID feature; otherwise omit it, because the server rejects the field (even an empty string). On create, omitting it or setting it to `null` sends no external ID. On update, omit it to preserve the stored ID, set a string to replace it, or set it to `null` to clear it. For example, clear it through stdin while supplying the other required update fields:

```bash
printf '%s' '{"bucketProvider":"AWS","bucketPath":"https://backups.s3.us-east-1.amazonaws.com/prefix","iamRoleArn":"arn:aws:iam::123456789012:role/backups","iamRoleExternalId":null}' \
  | clickhousectl cloud backup bucket update <service-id> --file -
```

## Stop and delete

```bash
clickhousectl cloud service stop <service-id>
clickhousectl cloud service get <service-id>
```

Start and stop return when accepted. Confirm current state with `get`; `service wake` handles an idle service, while a stopped service needs `service start`.

**Deletion permanently removes the service and its data.** Once stopped, delete only the intended service:

```bash
clickhousectl cloud service delete <service-id>
```

`delete --force` stops a running service and polls until the stop completes before deleting it, which can take minutes. It is not a general conflict override; inspect other conflicts before retrying. Deletion also cleans up stored per-service Query API keys when exact ownership metadata is available. A cleanup failure can occur after the service is gone; retain the local credentials file and inspect the reported key IDs instead of assuming nothing happened.
