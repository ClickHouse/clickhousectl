#!/bin/sh
set -eu

REPO="ClickHouse/clickhousectl"
GITHUB_URL="${CLICKHOUSECTL_GITHUB_URL:-https://github.com}"
GITHUB_API_URL="${CLICKHOUSECTL_GITHUB_API_URL:-https://api.github.com}"
INSTALL_DIR="${CLICKHOUSECTL_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="clickhousectl"
TAG="${CLICKHOUSECTL_EXPERIMENTAL_TAG:-}"

usage() {
  cat <<'EOF'
Install an experimental build of clickhousectl from GitHub Releases.

Usage:
  install-experimental.sh [--tag <experimental-tag>]

Options:
  --tag <tag>   Install a specific experimental pre-release tag instead of the latest
  -h, --help    Show this help message

Environment:
  CLICKHOUSECTL_EXPERIMENTAL_TAG  Same as --tag
  CLICKHOUSECTL_INSTALL_DIR       Install directory (default: $HOME/.local/bin)
  GH_TOKEN / GITHUB_TOKEN         Optional GitHub token to avoid API rate limits
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --tag)
      if [ $# -lt 2 ]; then
        echo "Error: --tag requires a value" >&2
        exit 1
      fi
      TAG="$2"
      shift 2
      ;;
    --tag=*)
      TAG="${1#--tag=}"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Linux)  OS_TARGET="unknown-linux-musl" ;;
  Darwin) OS_TARGET="apple-darwin" ;;
  *)
    echo "Error: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)  ARCH_TARGET="x86_64" ;;
  aarch64|arm64) ARCH_TARGET="aarch64" ;;
  *)
    echo "Error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"
echo "Detected platform: $TARGET"

is_experimental_tag() {
  printf '%s\n' "$1" | grep -Eq '^experimental-[0-9]{8}(-[0-9]{4,6})?-[0-9a-f]{7,}$'
}

resolve_latest_experimental_tag() {
  releases_json="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$releases_json" <<'PY'
import json, re, sys

pattern = re.compile(r"\Aexperimental-[0-9]{8}(?:-[0-9]{4,6})?-[0-9a-f]{7,}\Z")
with open(sys.argv[1], encoding="utf-8") as f:
    releases = json.load(f)
if not isinstance(releases, list):
    sys.exit(1)
candidates = [
    r for r in releases
    if isinstance(r, dict)
    and r.get("prerelease") is True
    and r.get("draft") is False
    and isinstance(r.get("tag_name"), str)
    and pattern.match(r["tag_name"])
]
candidates.sort(
    key=lambda r: (str(r.get("created_at") or ""), r["tag_name"]),
    reverse=True,
)
if not candidates:
    sys.exit(1)
print(candidates[0]["tag_name"])
PY
    return $?
  fi

  if command -v jq >/dev/null 2>&1; then
    jq -er '
      [ .[]
        | select(.prerelease == true and .draft == false)
        | select((.tag_name // "") | test("^experimental-[0-9]{8}(-[0-9]{4,6})?-[0-9a-f]{7,}$"))
      ]
      | sort_by([(.created_at // ""), .tag_name])
      | reverse
      | .[0].tag_name // empty
    ' "$releases_json"
    return $?
  fi

  if command -v node >/dev/null 2>&1; then
    node -e '
      const fs = require("fs");
      const re = /^experimental-[0-9]{8}(?:-[0-9]{4,6})?-[0-9a-f]{7,}$/;
      const releases = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
      if (!Array.isArray(releases)) process.exit(1);
      const candidates = releases
        .filter((r) => r && r.prerelease === true && r.draft === false && typeof r.tag_name === "string" && re.test(r.tag_name))
        .sort((a, b) => String(b.created_at || "").localeCompare(String(a.created_at || "")) || b.tag_name.localeCompare(a.tag_name));
      if (!candidates.length) process.exit(1);
      console.log(candidates[0].tag_name);
    ' "$releases_json"
    return $?
  fi

  echo "Error: python3, jq, or node is required to resolve the latest experimental release (or pass --tag <tag>)" >&2
  return 1
}

mkdir -p "$INSTALL_DIR"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if [ -z "$TAG" ]; then
  echo "Fetching latest experimental release from GitHub..."
  AUTH_TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
  RELEASES_URL="${GITHUB_API_URL}/repos/${REPO}/releases?per_page=30"
  if [ -n "$AUTH_TOKEN" ]; then
    curl -fsSL \
      -H "Accept: application/vnd.github+json" \
      -H "Authorization: Bearer ${AUTH_TOKEN}" \
      "$RELEASES_URL" -o "${TMPDIR}/releases.json"
  else
    curl -fsSL \
      -H "Accept: application/vnd.github+json" \
      "$RELEASES_URL" -o "${TMPDIR}/releases.json"
  fi
  TAG="$(resolve_latest_experimental_tag "${TMPDIR}/releases.json" || true)"
  if [ -z "$TAG" ]; then
    echo "Error: could not find an experimental pre-release in ${REPO}" >&2
    exit 1
  fi
fi

if ! is_experimental_tag "$TAG"; then
  echo "Error: '${TAG}' is not a valid experimental release tag (expected experimental-YYYYMMDD[-HHMMSS]-<sha>)" >&2
  exit 1
fi
echo "Experimental release: $TAG"

ARCHIVE_NAME="clickhousectl-${TARGET}-${TAG}.tar.gz"
RELEASE_BASE="${GITHUB_URL}/${REPO}/releases/download/${TAG}"
DOWNLOAD_URL="${RELEASE_BASE}/${ARCHIVE_NAME}"
SUMS_URL="${RELEASE_BASE}/SHA256SUMS"

echo "Downloading ${DOWNLOAD_URL}..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ARCHIVE_NAME}"

echo "Verifying against ${SUMS_URL}..."
if ! curl -fsSL "$SUMS_URL" -o "${TMPDIR}/SHA256SUMS"; then
  echo "Error: could not download ${SUMS_URL}" >&2
  exit 1
fi
EXPECTED="$(awk -v f="$ARCHIVE_NAME" '($2 == f || $2 == "*" f) && $1 ~ /^[0-9a-fA-F]+$/ && length($1) == 64 { print tolower($1); exit }' "${TMPDIR}/SHA256SUMS")"
if [ -z "$EXPECTED" ]; then
  echo "Error: ${SUMS_URL} has no SHA-256 for ${ARCHIVE_NAME}" >&2
  exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "${TMPDIR}/${ARCHIVE_NAME}" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "${TMPDIR}/${ARCHIVE_NAME}" | awk '{ print $1 }')"
else
  echo "Error: sha256sum or shasum is required to verify the download" >&2
  exit 1
fi
if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "Error: ${ARCHIVE_NAME} does not match SHA256SUMS (expected ${EXPECTED}, got ${ACTUAL})" >&2
  exit 1
fi

tar -xzf "${TMPDIR}/${ARCHIVE_NAME}" -C "$TMPDIR"
mv "${TMPDIR}/clickhousectl-${TARGET}-${TAG}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Create chctl alias (symlink)
ln -sf "${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/chctl"

echo "Installed experimental ${BINARY_NAME} (${TAG}) to ${INSTALL_DIR}/${BINARY_NAME}"
echo "Created alias: chctl -> ${BINARY_NAME}"
echo ""
echo "NOTE: This is an experimental build. Self-update ('clickhousectl update') is disabled."
echo "To return to the latest stable release, run:"
echo "  curl -fsSL https://clickhouse.com/cli | sh"

# Check if install dir is in PATH
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "NOTE: ${INSTALL_DIR} is not in your PATH."
    echo "Add it by running:"
    echo ""
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
    echo "You may want to add that line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    ;;
esac
