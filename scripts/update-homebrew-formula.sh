#!/usr/bin/env bash
#
# update-homebrew-formula.sh — render the Homebrew formula template with the
# version and SHA256 hashes for a given release, then commit it to the
# ClickHouse/homebrew-tap repo.
#
# Usage:
#   ./scripts/update-homebrew-formula.sh <version>
#
#   <version>  Release version WITHOUT leading "v" (e.g. 0.3.1).
#
# Environment:
#   HOMEBREW_TAP_DEPLOY_KEY  Private SSH key registered as a deploy key with
#                           write access on ClickHouse/homebrew-tap (required).
#   GITHUB_REPOSITORY        Set by GitHub Actions (e.g. ClickHouse/clickhousectl).
#
# The script:
#   1. Downloads the 4 prebuilt archives from the GitHub release (immediately
#      available, no mirror propagation delay) and computes SHA256 for each.
#   2. Renders homebrew/clickhousectl.rb.tmpl with the version + hashes.
#   3. Clones ClickHouse/homebrew-tap, replaces Formula/clickhousectl.rb,
#      commits and pushes.
#
# The formula's download URLs point at builds.clickhouse.com (same bytes) so
# installs use the same distribution path as install.sh, cargo binstall, npm,
# and pip.
set -euo pipefail

VERSION="${1:?usage: $0 <version> (without leading v)}"
TAG="v${VERSION}"
TAP_REPO="ClickHouse/homebrew-tap"
TEMPLATE="homebrew/clickhousectl.rb.tmpl"

if [ -z "${HOMEBREW_TAP_DEPLOY_KEY:-}" ]; then
  echo "error: HOMEBREW_TAP_DEPLOY_KEY is not set" >&2
  exit 1
fi

# ── 1. Download archives and compute SHA256 ────────────────────────────────
# GitHub release assets are named clickhousectl-{target}-v{version}.tar.gz.
# The bytes are identical to builds.clickhouse.com, so we compute hashes from
# GitHub (available immediately after the release job) and use builds URLs in
# the formula for the canonical distribution path.
GH_BASE="https://github.com/ClickHouse/clickhousectl/releases/download/${TAG}"

declare -A TARGETS=(
  [X86_64_LINUX_MUSL]="x86_64-unknown-linux-musl"
  [AARCH64_LINUX_MUSL]="aarch64-unknown-linux-musl"
  [X86_64_APPLE_DARWIN]="x86_64-apple-darwin"
  [AARCH64_APPLE_DARWIN]="aarch64-apple-darwin"
)

declare -A HASHES

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

for key in "${!TARGETS[@]}"; do
  target="${TARGETS[$key]}"
  asset="clickhousectl-${target}-${TAG}.tar.gz"
  url="${GH_BASE}/${asset}"
  echo "Downloading ${asset}..."
  curl -fsSL "$url" -o "${TMPDIR}/${asset}"
  hash="$(shasum -a 256 "${TMPDIR}/${asset}" | awk '{print $1}')"
  HASHES[$key]="$hash"
  echo "  sha256: ${hash}"
done

# ── 2. Render the template ─────────────────────────────────────────────────
if [ ! -f "$TEMPLATE" ]; then
  echo "error: template not found: ${TEMPLATE}" >&2
  exit 1
fi

RENDERED="${TMPDIR}/clickhousectl.rb"
sed \
  -e "s|{{VERSION}}|${VERSION}|g" \
  -e "s|{{SHA256_X86_64_LINUX_MUSL}}|${HASHES[X86_64_LINUX_MUSL]}|g" \
  -e "s|{{SHA256_AARCH64_LINUX_MUSL}}|${HASHES[AARCH64_LINUX_MUSL]}|g" \
  -e "s|{{SHA256_X86_64_APPLE_DARWIN}}|${HASHES[X86_64_APPLE_DARWIN]}|g" \
  -e "s|{{SHA256_AARCH64_APPLE_DARWIN}}|${HASHES[AARCH64_APPLE_DARWIN]}|g" \
  "$TEMPLATE" > "$RENDERED"

echo "Rendered formula:"
head -5 "$RENDERED"

# ── 3. Clone the tap, update, commit, push ─────────────────────────────────
# Use a deploy key (SSH) for authentication — scoped to the tap repo only,
# not tied to any individual's GitHub account.
TAP_DIR="${TMPDIR}/homebrew-tap"
SSH_KEY="${TMPDIR}/deploy_key"
printf '%s\n' "$HOMEBREW_TAP_DEPLOY_KEY" > "$SSH_KEY"
chmod 600 "$SSH_KEY"

# Ensure known_hosts has GitHub's SSH key fingerprints.
ssh-keyscan -t ed25519 github.com >> "${TMPDIR}/known_hosts" 2>/dev/null

GIT_SSH_COMMAND="ssh -i ${SSH_KEY} -o UserKnownHostsFile=${TMPDIR}/known_hosts -o IdentitiesOnly=yes" \
  git clone "git@github.com:${TAP_REPO}.git" "$TAP_DIR"
mkdir -p "${TAP_DIR}/Formula"
cp "$RENDERED" "${TAP_DIR}/Formula/clickhousectl.rb"

git -C "$TAP_DIR" add Formula/clickhousectl.rb
git -C "$TAP_DIR" -c user.name="github-actions[bot]" \
  -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
  commit -m "clickhousectl ${VERSION}"

GIT_SSH_COMMAND="ssh -i ${SSH_KEY} -o UserKnownHostsFile=${TMPDIR}/known_hosts -o IdentitiesOnly=yes" \
  git -C "$TAP_DIR" push origin HEAD
echo "Pushed clickhousectl ${VERSION} to ${TAP_REPO}"
