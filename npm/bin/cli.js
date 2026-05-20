#!/usr/bin/env node
// Thin shim that execs the platform-specific clickhousectl binary that
// install.js downloaded into ../vendor at npm install time.

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const binaryName = process.platform === 'win32' ? 'clickhousectl.exe' : 'clickhousectl';
const binaryPath = path.join(__dirname, '..', 'vendor', binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(
    `clickhousectl: binary not found at ${binaryPath}.\n` +
    `The postinstall step may have been skipped (e.g. npm --ignore-scripts) ` +
    `or failed. Reinstall with scripts enabled, or download manually from ` +
    `https://builds.clickhouse.com/clickhousectl/`
  );
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), { stdio: 'inherit' });
child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 1);
  }
});
