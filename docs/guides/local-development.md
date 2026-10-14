# Run ClickHouse and Postgres locally

[All documentation](../README.md)

Run project-local commands from the directory that owns your `.clickhouse/`. The CLI uses the exact current directory; it does not search parents. ClickHouse binaries are shared globally in `~/.clickhouse/`, while each project's server data is isolated.

## Start ClickHouse and run SQL

```bash
clickhousectl local server start dev --version stable
clickhousectl local client dev --query "SELECT version()"
clickhousectl local server list
```

`local init` optionally scaffolds `clickhouse/` and `postgres/` directories for schemas, queries, seed data, and [UDF sources](user-defined-functions.md#scaffold-a-function); it is not required to start a server.

Named starts reuse the server's data. Background starts wait up to 30 seconds for HTTP and TCP readiness; `--no-wait` returns after spawning. Failures point to `.clickhouse/servers/<name>/server.log`. Default ports are HTTP 8123 and TCP 9000, with free ports assigned when occupied. Use `--http-port` and `--tcp-port` to request specific ports.

Without a name, the first server is `default`; if it is already running, another start generates a new name. Use an explicit name for a repeatable workflow.

## Choose a version

| Selector | Meaning |
| --- | --- |
| `latest` | Rolling master build, checked remotely on each install/use/start |
| `stable` | Latest stable release |
| `lts` | Latest long-term support release |
| `26.8` | Latest matching 26.8 release |
| `26.8.1.1760` | Exact version |

```bash
clickhousectl local use stable
clickhousectl local which
clickhousectl local list
clickhousectl local list --remote
```

`local use` installs if needed and sets the default for future server starts. `server start --version` chooses a version for that start without changing the default.

A bare start installs `latest` if no version or default is available, without setting a default. Pin a version or use `local use stable` to avoid tracking master. Reusing `latest` requires a successful remote freshness check; an unavailable server is an error, not a fallback to an unverified build.

## Run SQL with `local client`

Connect to a running server by name. `local client` finds its TCP port and uses the same ClickHouse binary as the server. With no name, it connects to the server named `default`.

```bash
# Open an interactive SQL session
clickhousectl local client dev

# Run a query, or several queries in order
clickhousectl local client dev --query "SELECT version()"
clickhousectl local client dev --query "SELECT 1" --query "SELECT 2"

# Run SQL from files
clickhousectl local client dev --queries-file schema.sql seed.sql
```

Create `schema.sql` and `seed.sql` with your project's SQL before running the file example. `--queries-file` accepts multiple paths or repeated flags. Use either `--query` (`-q`) or `--queries-file` in a command; they cannot be combined. Repeated `--query` flags require ClickHouse 23.9.1.1854 or newer.

To connect by address, use `--host` and/or `--port` instead of a server name. Omitted values default to `localhost` and TCP port `9000`. In this mode, `--version` selects an already installed numeric version, such as `26.8`; it never downloads one or changes the default. Without `--version`, the client uses the default version, or the sole installed version if no default is set. Named connections cannot combine with `--host`, `--port`, or `--version`.

Query output and exit status come from the ClickHouse client, including under `--json` or agent detection. Set the result format in SQL when scripting:

```bash
clickhousectl local client dev --query "SELECT version() FORMAT JSONEachRow"
```

## Use the native ClickHouse binary

Use `clickhousectl` to manage the binary, then run `clickhouse` directly. Select the rolling master build and put the managed binary on your shell's `PATH`:

```bash
clickhousectl local use latest
export PATH="$HOME/.local/bin:$PATH"
clickhouse client --version
```

`local use` installs the selected version if needed, sets it as the default, and creates or updates `~/.local/bin/clickhouse`. Keep the `PATH` export in your shell startup file to use it in future sessions. An existing regular file at that path is preserved; `--no-global` skips creating or updating the symlink.

You can also run the native client against servers managed by `clickhousectl`. Choose a free TCP port, such as `9001`:

```bash
clickhousectl local server start native-demo --tcp-port 9001
clickhouse client --host localhost --port 9001 \
  --query "SELECT version()" --format CSV
clickhousectl local server stop native-demo
```

For an existing server, find its TCP port with `clickhousectl local server list` and pass that port to `clickhouse client`. The native client connects by host and port; `clickhousectl` manages the server's process, configuration, and data. All native client options are available directly.

Run `local use latest` again to check for a newer master build, or select `stable`, `lts`, or an exact version to switch the binary. Switching changes the binary used by subsequent `clickhouse` commands and the default for future server starts; running servers keep their existing version. To match a running server, select its version from `local server list` with `local use <version>`.

## Apply a custom ClickHouse configuration

Managed servers use embedded defaults. Put a partial XML, YAML, or YML configuration in `~/.clickhouse/configs/` and select its filename or unambiguous stem:

```bash
mkdir -p ~/.clickhouse/configs
cat > ~/.clickhouse/configs/analytics.xml <<'XML'
<clickhouse>
    <profiles>
        <default><max_threads>4</max_threads></default>
    </profiles>
</clickhouse>
XML
clickhousectl local server configs
clickhousectl local server stop dev
clickhousectl local server start dev --config analytics
```

The file must be directly inside the configs directory, not an arbitrary path. Use a `<clickhouse>` root in XML; user profiles and query settings can be included there. The server does not automatically load `/etc/clickhouse-server/config.xml` or a separate `users.xml`.

Edit the source file and restart with `--config` again to apply changes. The selection is not remembered: starting without the flag removes the previously copied overlay. Managed data paths and ports override config values, including after ClickHouse reloads its config. Referenced files must be accessible from the server's working directory, `.clickhouse/servers/<name>/data/`. Inspect the merged `preprocessed_configs/config.xml` there, but edit the source overlay rather than that generated file.

## Test executable UDFs

`local udf` deploys executable UDFs from `clickhouse/udfs/` to a local server, from the same directory `cloud udf` uploads. See [User-defined functions](user-defined-functions.md#test-on-a-local-server).

## Add Postgres

Docker must be running. Postgres 17 and 18, including image sub-tags, are supported; new instances default to 18. A random password is generated unless supplied.

```bash
clickhousectl local postgres start app --version 18
clickhousectl local postgres client app --query "SELECT version()"
clickhousectl local postgres client app --queries-file pg-schema.sql
clickhousectl local postgres dotenv app --local
clickhousectl local server list
```

Create `pg-schema.sql` with your Postgres SQL before running the file example. Put `\set ON_ERROR_STOP on` at the top of the file to stop on SQL errors.

Starts wait for `pg_isready`, by default for 60 seconds; `--wait-timeout` accepts 1–600 seconds. `local postgres client` uses host `psql` or the container's `psql`. File input and stdin work with either, but paths inside SQL (`\i`, `\copy`) refer to the container filesystem in Docker mode. `--json` does not change psql output. When both are supplied, `--query` runs before `--queries-file`.

An instance is identified by name and major version. Pass `--version` when more than one major shares a name. Resuming an instance reuses its original port, user, password, database, and environment; new creation flags do not reconfigure it. To recreate, first back up any needed data, then stop and remove the instance before starting again. Direct `--host`/`--port` client selectors cannot combine with managed name/version selectors.

The generated `.env.local` includes the password. Keep it out of version control. Both engines' `dotenv` commands write a file and print an informational preview: use your application's dotenv loader, never `eval` their stdout.

## Stop or remove data

```bash
clickhousectl local server stop dev
clickhousectl local postgres stop app --version 18
```

Stop preserves data. **The following removals permanently delete the selected server's data**, including Postgres container volumes:

```bash
clickhousectl local server remove dev
clickhousectl local postgres remove app --version 18
```

Both require stopped servers. `local server stop-all` stops both engines in the current project. Global ClickHouse operations (`server list --global`, `stop --global`, `stop-all --global`) span projects; confirm the project and name before stopping one. `stop --global --project <project-root>` disambiguates matching names.

`local remove <exact-version>` removes a shared ClickHouse binary, not server data. It refuses a default or a version used by running servers in any project. Its `--force` stops those servers across projects and removes the version even if it is the default.

Without a name, ClickHouse `stop` selects `default`, then the sole server, and is a no-op if none exists. Bare `remove` only selects `default`; use explicit names for cleanup.

If metadata is corrupt, follow the reported recovery guidance. Confirm the running process with `server list --global`, or the Postgres container with Docker, before moving metadata aside. Do not delete state or force-remove an unverified server. A Docker inspection failure is an error, not evidence that Postgres is stopped.
