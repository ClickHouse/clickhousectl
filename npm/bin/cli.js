#!/usr/bin/env node
// Thin shim that spawns the platform-specific clickhousectl binary that
// install.js downloaded into ../vendor at npm install time. Node cannot exec(),
// so the shim stays alive as the child's parent and must not let terminal
// signals tear it down before the child: Ctrl-C and Ctrl-\ reach the whole
// foreground process group, so the child alone decides what they mean (an
// interactive client keeps running). SIGTERM and SIGHUP are usually sent to the
// shim alone, so they are forwarded. The shim then exits with the child's status.

const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const binaryPath = path.join(__dirname, '..', 'vendor', 'clickhousectl');

if (!fs.existsSync(binaryPath)) {
  console.error(
    `clickhousectl: binary not found at ${binaryPath}.\n` +
    `The postinstall step may have been skipped (e.g. npm --ignore-scripts) ` +
    `or failed. Reinstall with scripts enabled, or download manually from ` +
    `https://builds.clickhouse.com/clickhousectl/`
  );
  process.exit(1);
}

// The install channel marker tells `clickhousectl update` that npm owns this
// binary, so it prints npm's upgrade command instead of replacing the file.
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: { ...process.env, CLICKHOUSECTL_INSTALL_CHANNEL: 'npm' },
});

const IGNORED_SIGNALS = ['SIGINT', 'SIGQUIT'];
const FORWARDED_SIGNALS = ['SIGTERM', 'SIGHUP'];
const handled = [...IGNORED_SIGNALS, ...FORWARDED_SIGNALS];

for (const signal of IGNORED_SIGNALS) {
  process.on(signal, () => {});
}
for (const signal of FORWARDED_SIGNALS) {
  process.on(signal, () => {
    try {
      child.kill(signal);
    } catch {
      // The child has already exited; its exit handler sets our status.
    }
  });
}

child.on('error', (err) => {
  console.error(`clickhousectl: failed to start the binary: ${err.message}`);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    // Die by the same signal so callers see it. Drop our own handler first, or
    // the re-raise would be swallowed and the shim would exit 0.
    for (const s of handled) {
      process.removeAllListeners(s);
    }
    try {
      process.kill(process.pid, signal);
    } catch {
      // Fall through to the conventional 128 + signal number status.
    }
    process.exit(128 + (os.constants.signals[signal] ?? 0));
  } else {
    process.exit(code ?? 1);
  }
});
