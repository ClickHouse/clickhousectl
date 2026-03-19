# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                          # dev build
cargo build --release                # release build
cargo test                           # run all tests
cargo test test_detect_platform      # run a single test
cargo clippy                         # lint
```

No separate lint CI — just `cargo build` and `cargo test` must pass.

Cross-compilation for aarch64-linux uses `cross` (see `.github/workflows/release.yml`). The crate uses `rustls-tls` instead of OpenSSL to support this.

## Architecture

`clickhousectl` is the official ClickHouse CLI — a version manager + cloud CLI. Two top-level subcommands: `local` and `cloud`.

1. **Local** (`local install|list|use|remove|which|init|client|server`) — version management in `src/version_manager/`, server management in `src/server.rs`, client/init in `main.rs`. Binaries live in `~/.clickhousectl/versions/{version}/clickhouse`, default tracked in `~/.clickhousectl/default`. Project data lives in `.clickhousectl/`.

2. **Cloud** (`cloud org|service|backup`) — handled by `src/cloud/`. `CloudClient` wraps reqwest with Basic or Bearer auth. Commands go through `cloud/commands.rs`, types in `cloud/types.rs`. All cloud commands support `--json` output.

3. **Auth** (`cloud auth login|logout|status|keys`) — authentication subcommand under cloud. OAuth device flow in `src/cloud/auth.rs`, API key management in `cloud/commands.rs`. Tokens stored in `~/.clickhousectl/tokens.json`.

## Adding commands

### New local subcommand

1. Add variant to `LocalCommands` in `src/cli.rs` using clap derive macros
2. Add match arm in `run_local()` in `src/main.rs`
3. Implement handler (in `main.rs` for simple commands, or a dedicated module)

### New cloud subcommand

1. Add variant to the relevant enum in `src/cli.rs` (e.g. `ServiceCommands`)
2. Add match arm in `run_cloud()` in `src/main.rs`
3. Add method to `CloudClient` in `cloud/client.rs`
4. Add request/response types to `cloud/types.rs` — use `#[serde(rename_all = "camelCase")]` (API uses camelCase) and `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
5. Implement handler in `cloud/commands.rs` with the `--json` output pattern:
   ```rust
   if json {
       println!("{}", serde_json::to_string_pretty(&data)?);
   } else {
       println!("Human readable: {}", data.field);
   }
   ```

ClickHouse Cloud OpenAPI spec: https://api.clickhouse.cloud/v1

## Dependencies

Use `cargo add` to add new dependencies (not manual `Cargo.toml` edits). Always use the latest version of packages.

```bash
cargo add serde --features derive    # add with features
cargo add rpassword                  # add latest version
```

## Key details

- CLI is defined with clap derive macros in `src/cli.rs`, dispatched in `src/main.rs`
- `src/paths.rs` handles `~/.clickhousectl/` paths (global install dir); `src/init.rs` handles `.clickhousectl/` paths (project-local data dir)
- `local client` uses `exec()` (process replacement), so code after `cmd.exec()` only runs on failure
- Error types use `thiserror` in `src/error.rs`; cloud module has its own error type wrapped as `Error::Cloud(String)`
- Version resolution (`version_manager/resolve.rs`) handles specs like `stable`, `lts`, `25.12`, or exact `25.12.5.44` — all resolve to an exact version + channel via GitHub API
- Releases are triggered by pushing a version tag (`v0.1.3`), which runs the GitHub Actions workflow

## Git workflow

- **Branch per feature/issue.** When working on a new feature or issue, create a branch and use a PR workflow. Do not commit directly to `main`.
- If the user references a GitHub issue (e.g. "work on issue 3"), use `gh issue view 3` to get the details, then create a branch like `issue-3-short-description`.
- Update `README.md` and any relevant documentation as part of the change — PRs should include doc updates for new or changed functionality.
- Commit to the branch, push, and create a PR with `gh pr create`.
- Releases are done by tagging `main` (e.g. `git tag v0.1.4 && git push origin v0.1.4`), which triggers the GitHub Actions release workflow. Ensure version is updated in Cargo.toml.

## Testing locally

```bash
cargo run -- local install stable
cargo run -- local server start      # starts server in .clickhousectl/servers/default/
cargo run -- local client --query "SELECT 1"
```
