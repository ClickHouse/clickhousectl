---
name: release
description: Releases a new `clickhousectl` and `clickhouse-cloud-api` version — bumps versions in lockstep, tags, watches the release workflow, then runs the ClickHouse/ClickHouse NightlyUpload workflow that puts the binaries on builds.clickhouse.com. Use when cutting, tagging, or publishing a release, bumping the version, or when `clickhousectl update`, `install.sh` or `npm install` cannot download the latest version.
---

# Release

One tag publishes everything. `.github/workflows/release.yml` runs on a `v*` tag push: build → smoke tests →
**Create Release** (the GitHub release and its `.tar.gz` assets) → crates.io, npm and PyPI in separate jobs.
crates.io uses a token; npm and PyPI use OIDC.

The binaries are served from builds.clickhouse.com, not from GitHub. A separate workflow in another repo copies
them there. A release doesn't trigger it, so the release isn't finished until you run it (step 4).

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

## 4. Upload the binaries to builds.clickhouse.com

As soon as **Create Release** succeeds, run the `NightlyUpload` workflow in ClickHouse/ClickHouse:

```bash
gh workflow run nightly_upload.yml -R ClickHouse/ClickHouse
```

- Why: `clickhousectl update`, `install.sh` and the npm postinstall all read the latest version from the GitHub
  release, then download `clickhousectl-<target>-vX.Y.Z.tar.gz` from `https://builds.clickhouse.com/clickhousectl/`.
  Until the upload runs, every one of them fails with a 404 for the new version.
- What it does: its `Upload clickhousectl` job (`ci/jobs/upload_clickhousectl.py`) copies the four `.tar.gz`
  assets (`aarch64`/`x86_64` × `apple-darwin`/`unknown-linux-musl`) of the latest GitHub release into the
  `clickhouse-builds` bucket, skipping any already there. It fails if one is missing.
- It otherwise runs only on a daily schedule (06:13 UTC), so a release left alone breaks downloads for up to a day.
- Triggering it needs workflow-run access to ClickHouse/ClickHouse. If you don't have it, ask the user to run it.
- Watch it: `gh run list -R ClickHouse/ClickHouse -w nightly_upload.yml -L 1`, then `gh run watch`.

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
- [ ] NightlyUpload run and green
- [ ] All four builds.clickhouse.com archives return 200
- [ ] `clickhousectl update` reaches the new version
```
