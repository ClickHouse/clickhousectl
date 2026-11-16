#!/bin/sh
set -eu

REPO="ClickHouse/clickhousectl"
BUILDS_URL="https://builds.clickhouse.com/clickhousectl"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="clickhousectl"

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

# Get the latest release tag from latest.txt, which the release workflow
# publishes. builds.clickhouse.com caches every path for days, so a unique
# query string fetches it fresh. If it is unavailable, fall back to GitHub's
# release redirect, which avoids the rate-limited REST API.
is_tag() { printf '%s\n' "$1" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; }
echo "Fetching latest release..."
LATEST="$(curl -fsSL "${BUILDS_URL}/latest.txt?t=$(date +%s)" 2>/dev/null | tr -d '[:space:]' || true)"
if ! is_tag "$LATEST"; then
  LATEST="$(curl -fsSI "https://github.com/${REPO}/releases/latest" 2>/dev/null | tr -d '\r' | sed -n 's#^[Ll]ocation:.*/releases/tag/##p' || true)"
fi

if ! is_tag "$LATEST"; then
  echo "Error: could not determine latest release" >&2
  exit 1
fi
echo "Latest release: $LATEST"

# Download archive
ARCHIVE_NAME="clickhousectl-${TARGET}-${LATEST}.tar.gz"
DOWNLOAD_URL="${BUILDS_URL}/${ARCHIVE_NAME}"
echo "Downloading ${DOWNLOAD_URL}..."

mkdir -p "$INSTALL_DIR"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ARCHIVE_NAME}"

# Verify the archive against the release's SHA256SUMS. It is read from the
# GitHub release, not builds.clickhouse.com, so the server that served the
# archive cannot also vouch for it. Any failure stops the install.
SUMS_URL="https://github.com/${REPO}/releases/download/${LATEST}/SHA256SUMS"
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
mv "${TMPDIR}/clickhousectl-${TARGET}-${LATEST}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"

# Create chctl alias (symlink)
ln -sf "${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/chctl"
echo "Created alias: chctl -> ${BINARY_NAME}"
echo "Tab completion: run 'clickhousectl completions --help' for the line to add to your shell startup"

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
