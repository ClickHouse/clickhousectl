---
name: release
description: Releases a new `clickhousectl` and `clickhouse-cloud-api` version — bumps versions in lockstep, tags, watches the release workflow (which also uploads the binaries to builds.clickhouse.com), then verifies the downloads. Use when cutting, tagging, or publishing a release, bumping the version, or when `clickhousectl update`, `install.sh` or `npm install` cannot download the latest version.
---

# Release

One tag publishes everything. `.github/workflows/release.yml` runs on a `v*` tag push: build → smoke tests →
**Upload to builds.clickhouse.com** → **Create Release** (the GitHub release and its `.tar.gz` assets) →
crates.io, npm and PyPI in separate jobs. crates.io uses a token; npm, PyPI and the S3 upload use OIDC.

The binaries are served from builds.clickhouse.com, not from GitHub. `clickhousectl update`, `install.sh` and the
npm postinstall read the latest version from the GitHub release, then download
`clickhousectl-<target>-vX.Y.Z.tar.gz` from `https://builds.clickhouse.com/clickhousectl/`. The upload runs before
**Create Release** so a new release is never visible without its archives.

## 1. Bump versions in lockstep

Set the same `X.Y.Z` in all of:

- `crates/clickhousectl/Cargo.toml` — `version` and the `clickhouse-cloud-api` dependency `version`
- `crates/clickhouse-cloud-api/Cargo.toml` — `version`
- `npm/package.json` — `version`

`pypi/pyproject.toml` takes its version from `crates/clickhousectl/Cargo.toml` (`dynamic = ["version"]`).
Land the bump on `main` through a PR like any other change.

## 2. Tag and push

Tag the merge commit on `main`, not a branch head:

```bash
git tag vX.Y.Z <merge-commit> && git push origin vX.Y.Z
```

## 3. Watch the release workflow

`gh run watch -R ClickHouse/clickhousectl <run-id>` (find it with `gh run list -R ClickHouse/clickhousectl -w release.yml -L 1`).
Every job must be green. Re-run a failed publish job alone; the crates.io job skips `clickhouse-cloud-api` if that
version is already published.

## 4. If the upload fails

**Upload to builds.clickhouse.com** assumes `arn:aws:iam::542516086801:role/external-s3-upload` through GitHub
OIDC and copies the four `.tar.gz` archives (`aarch64`/`x86_64` × `apple-darwin`/`unknown-linux-musl`) to
`s3://clickhouse-builds/clickhousectl/`. It skips an archive already in the bucket and never overwrites one:
builds.clickhouse.com caches them for days.

- An AWS `AccessDenied` or OIDC error means the role or its trust policy changed. The role is managed in
  ClickHouse/clickhouse-private `tests/ci/terraform/external_s3_upload.tf`; ask the user to raise it with the CI
  owners there.
- Re-run the failed job once fixed; **Create Release** and the publish jobs then continue.
- The `NightlyUpload` workflow in ClickHouse/ClickHouse is the old pull-based copy of the latest GitHub release.
  Don't rely on it.

## 5. Verify

```bash
for t in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
  curl -fsSI "https://builds.clickhouse.com/clickhousectl/clickhousectl-$t-vX.Y.Z.tar.gz" >/dev/null && echo "ok $t" || echo "MISSING $t"
done
```

Then `clickhousectl update` from an older install should report and install `vX.Y.Z`. Check the crates.io, npm
and PyPI pages show the new version.

## Checklist

```
- [ ] Versions bumped in lockstep and merged to main
- [ ] Tag pushed on the merge commit
- [ ] release.yml fully green
- [ ] All four builds.clickhouse.com archives return 200
- [ ] `clickhousectl update` reaches the new version
```
