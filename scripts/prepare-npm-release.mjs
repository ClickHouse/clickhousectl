// Usage: node scripts/prepare-npm-release.mjs <version> <dir-of-binaries>
//
// Expects <dir>/<triple>/clickhousectl  for each triple below.
// Rewrites every package.json under npm/ to <version> and copies binaries into
// npm/platforms/<triple>/bin/clickhousectl with mode 0755.

import { readFileSync, writeFileSync, copyFileSync, mkdirSync, chmodSync, existsSync } from "node:fs";
import { join } from "node:path";

const [, , version, binDir] = process.argv;
if (!version || !binDir) {
  console.error("usage: prepare-npm-release.mjs <version> <binDir>");
  process.exit(2);
}
if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`invalid semver: ${version}`);
  process.exit(2);
}

const TRIPLES = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"];
const root = "npm";

function rewrite(pkgPath, mut) {
  const j = JSON.parse(readFileSync(pkgPath, "utf8"));
  mut(j);
  writeFileSync(pkgPath, JSON.stringify(j, null, 2) + "\n");
}

// Wrapper
rewrite(join(root, "clickhousectl/package.json"), (j) => {
  j.version = version;
  for (const t of TRIPLES) {
    j.optionalDependencies[`@clickhouse/clickhousectl-${t}`] = version;
  }
});

// Platforms
for (const t of TRIPLES) {
  const dir = join(root, "platforms", t);
  rewrite(join(dir, "package.json"), (j) => { j.version = version; });

  const src = join(binDir, t, "clickhousectl");
  if (!existsSync(src)) {
    console.error(`missing binary: ${src}`);
    process.exit(1);
  }
  const dst = join(dir, "bin");
  mkdirSync(dst, { recursive: true });
  copyFileSync(src, join(dst, "clickhousectl"));
  chmodSync(join(dst, "clickhousectl"), 0o755);
}

console.log(`prepared npm packages at version ${version}`);
