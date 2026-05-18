#!/usr/bin/env node
"use strict";
const { spawn } = require("child_process");
const { platform, arch } = process;

const triple =
  platform === "darwin" && arch === "arm64" ? "darwin-arm64" :
  platform === "darwin" && arch === "x64"   ? "darwin-x64"   :
  platform === "linux"  && arch === "arm64" ? "linux-arm64"  :
  platform === "linux"  && arch === "x64"   ? "linux-x64"    :
  null;

if (!triple) {
  console.error(
    `clickhousectl: unsupported platform ${platform}-${arch}. ` +
    `Supported: macOS (arm64, x64), Linux (arm64, x64).`
  );
  process.exit(1);
}

const pkg = `@clickhouse/clickhousectl-${triple}`;
let binPath;
try {
  binPath = require.resolve(`${pkg}/bin/clickhousectl`);
} catch {
  console.error(
    `clickhousectl: missing platform package ${pkg}.\n` +
    `This usually means npm was run with --no-optional / --ignore-optional, ` +
    `or the lockfile predates this install. Try: npm install --include=optional`
  );
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });

for (const sig of ["SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT"]) {
  process.on(sig, () => { try { child.kill(sig); } catch {} });
}
child.on("exit", (code, signal) => {
  if (signal) {
    try { process.kill(process.pid, signal); } catch { process.exit(1); }
  } else {
    process.exit(code ?? 1);
  }
});
