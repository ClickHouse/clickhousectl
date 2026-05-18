#!/usr/bin/env node
// Downloads the platform-appropriate clickhousectl binary from the matching
// GitHub release and places it next to bin/clickhousectl. Invoked as the
// package's postinstall script.

const fs = require('fs');
const path = require('path');
const https = require('https');
const { URL } = require('url');
const { spawnSync } = require('child_process');

const REPO = 'ClickHouse/clickhousectl';
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

async function main() {
  // npm install with --ignore-scripts will skip this entirely, which is the
  // documented escape hatch for users who don't want network access on install.
  const pkg = require('./package.json');
  const version = pkg.version;
  const target = resolveTarget();
  const archiveName = `clickhousectl-${target}-v${version}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${version}/${archiveName}`;

  const vendorDir = path.join(__dirname, 'vendor');
  fs.mkdirSync(vendorDir, { recursive: true });
  const archivePath = path.join(vendorDir, archiveName);

  console.log(`clickhousectl: downloading ${url}`);
  await download(url, archivePath);

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
}

main().catch((err) => {
  console.error(`clickhousectl install failed: ${err.message}`);
  process.exit(1);
});
