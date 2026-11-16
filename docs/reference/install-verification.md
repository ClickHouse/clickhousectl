# Release verification

Every release publishes a `SHA256SUMS` file and a GitHub build provenance attestation for its four `.tar.gz` archives. The archives are served from builds.clickhouse.com; `SHA256SUMS` is read from the GitHub release, so the server that delivers an archive cannot also vouch for it.

## What each install path checks

| Path | Check | On failure |
| --- | --- | --- |
| Install script (`curl -fsSL https://clickhouse.com/cli \| sh`) | The archive against the GitHub release's `SHA256SUMS`, with `sha256sum` or `shasum -a 256` | Exits 1 and installs nothing, including when `SHA256SUMS` or both tools are missing |
| `clickhousectl update` | The archive against the GitHub release's `SHA256SUMS`, before extracting it | Exits 1 and leaves the installed binary unchanged; `--json` reports `verification_failed` for a missing entry or a different digest |
| npm | The archive against the `SHA256SUMS` embedded in the npm package at publish time | The postinstall script fails, so `npm install` fails |
| pip, uv, pipx | No separate download: the binary is inside the wheel that PyPI serves | Handled by the package manager |
| `cargo install` | Builds from the crates.io source, which Cargo checks against the index checksum | Handled by Cargo |
| cargo-binstall | Nothing: binstall downloads the archive from builds.clickhouse.com without a checksum | |

Releases before v0.6.0 have no `SHA256SUMS`, so `update` and the install script refuse to install them.

## Verify a download yourself

Download an archive and the release's `SHA256SUMS`, then check the digest:

```bash
VERSION=vX.Y.Z
TARGET=aarch64-apple-darwin   # or x86_64-apple-darwin, aarch64-unknown-linux-musl, x86_64-unknown-linux-musl
curl -fsSLO "https://builds.clickhouse.com/clickhousectl/clickhousectl-${TARGET}-${VERSION}.tar.gz"
curl -fsSLO "https://github.com/ClickHouse/clickhousectl/releases/download/${VERSION}/SHA256SUMS"
sha256sum -c --ignore-missing SHA256SUMS   # macOS: shasum -a 256 -c --ignore-missing SHA256SUMS
```

To check that the archive was built by this repository's release workflow, verify its attestation with the [GitHub CLI](https://cli.github.com/):

```bash
gh attestation verify "clickhousectl-${TARGET}-${VERSION}.tar.gz" -R ClickHouse/clickhousectl
```

The command exits 0 only when GitHub holds a provenance attestation for that exact digest, signed for a workflow run in `ClickHouse/clickhousectl`.
