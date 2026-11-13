#!/usr/bin/env node
// Downloads the platform-appropriate clickhousectl binary from
// builds.clickhouse.com and places it next to bin/clickhousectl. Invoked as
// the package's postinstall script.
//
// The release workflow embeds the release's SHA256SUMS in the published
// package, so the archive is checked against digests that came through the
// npm registry rather than the server that served it (#1307).

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const https = require('https');
const { URL } = require('url');
const { spawnSync } = require('child_process');

const BUILDS_BASE_URL = 'https://builds.clickhouse.com/clickhousectl';
const PLATFORM_MAP = {
  'linux-x64': 'x86_64-unknown-linux-musl',
  'linux-arm64': 'aarch64-unknown-linux-musl',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
};

function resolveTarget() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];
  if (!target) {
    throw new Error(
      `clickhousectl: unsupported platform/arch "${key}". ` +
      `Supported: ${Object.keys(PLATFORM_MAP).join(', ')}`
    );
  }
  return target;
}

function get(url) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { 'User-Agent': 'clickhousectl-npm-installer' } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        const next = new URL(res.headers.location, url).toString();
        resolve(get(next));
        return;
      }
      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`GET ${url} -> ${res.statusCode}`));
        return;
      }
      resolve(res);
    });
    req.on('error', reject);
  });
}

async function download(url, dest) {
  const res = await get(url);
  await new Promise((resolve, reject) => {
    const out = fs.createWriteStream(dest);
    res.pipe(out);
    out.on('finish', () => out.close(resolve));
    out.on('error', reject);
    res.on('error', reject);
  });
}

// The digest SHA256SUMS lists for `archiveName`, in `sha256sum` output form
// (`<digest>  <name>` or `<digest> *<name>` per line). A missing file or
// entry is an error: the install fails closed.
function expectedSha256(archiveName) {
  const sumsPath = path.join(__dirname, 'SHA256SUMS');
  let sums;
  try {
    sums = fs.readFileSync(sumsPath, 'utf8');
  } catch (err) {
    throw new Error(`cannot read ${sumsPath} to verify the download: ${err.message}`);
  }
  for (const line of sums.split(/\r?\n/)) {
    const match = /^([0-9a-fA-F]{64}) [ *](.+)$/.exec(line);
    if (match && match[2] === archiveName) {
      return match[1].toLowerCase();
    }
  }
  throw new Error(`${sumsPath} has no SHA-256 for ${archiveName}`);
}

function sha256File(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

async function main() {
  // npm install with --ignore-scripts will skip this entirely, which is the
  // documented escape hatch for users who don't want network access on install.
  const pkg = require('./package.json');
  const version = pkg.version;
  const target = resolveTarget();
  const archiveName = `clickhousectl-${target}-v${version}.tar.gz`;
  const url = `${BUILDS_BASE_URL}/${archiveName}`;
  const expected = expectedSha256(archiveName);

  const vendorDir = path.join(__dirname, 'vendor');
  fs.mkdirSync(vendorDir, { recursive: true });
  const archivePath = path.join(vendorDir, archiveName);

  console.log(`clickhousectl: downloading ${url}`);
  await download(url, archivePath);
  const actual = sha256File(archivePath);
  if (actual !== expected) {
    fs.unlinkSync(archivePath);
    throw new Error(
      `${archiveName} does not match SHA256SUMS: expected SHA-256 ${expected}, got ${actual}`
    );
  }

  // The archive contains a single directory `clickhousectl-{target}-v{version}/`
  // with `clickhousectl` inside. --strip-components=1 unpacks the binary
  // directly into vendorDir.
  const extract = spawnSync(
    'tar',
    ['-xzf', archivePath, '-C', vendorDir, '--strip-components=1'],
    { stdio: 'inherit' }
  );
  if (extract.status !== 0) {
    throw new Error(`tar exited with status ${extract.status}`);
  }
  fs.unlinkSync(archivePath);

  const binaryPath = path.join(vendorDir, 'clickhousectl');
  fs.chmodSync(binaryPath, 0o755);
  console.log(`clickhousectl: installed ${binaryPath}`);
  console.log("clickhousectl: for Tab completion, see 'clickhousectl completions --help'");
}

main().catch((err) => {
  console.error(`clickhousectl install failed: ${err.message}`);
  process.exit(1);
});
