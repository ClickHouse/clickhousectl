"""Exercise the publisher with fake downloads and Git; never contact a tap."""

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGETS = (
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
)

# Both external commands are replaced even for failure cases, so tests cannot
# accidentally download a release or publish with real credentials.
FAKE_COMMAND = r'''#!/usr/bin/env python3
import json
import os
import shutil
import sys
from pathlib import Path

args = sys.argv[1:]
command = Path(sys.argv[0]).name
with open(os.environ["PUBLISH_TEST_LOG"], "a") as log:
    log.write(json.dumps([command, args]) + "\n")
if command == "curl":
    url = next(arg for arg in args if arg.startswith("https://"))
    mirror = "builds.clickhouse.com" in url
    if mirror and os.environ.get("MIRROR_MISSING"):
        sys.exit(22)
    content = url.rsplit("/", 1)[1].encode()
    if mirror and os.environ.get("MIRROR_MISMATCH"):
        content += b"different archive"
    Path(args[args.index("-o") + 1]).write_bytes(content)
elif args[0] == "clone":
    import shlex
    ssh = shlex.split(os.environ["GIT_SSH_COMMAND"])
    assert "StrictHostKeyChecking=yes" in ssh
    key = Path(ssh[ssh.index("-i") + 1])
    assert key.stat().st_mode & 0o777 == 0o600
    known_hosts = Path(next(a.split("=", 1)[1] for a in ssh if a.startswith("UserKnownHostsFile=")))
    expected_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl"
    assert known_hosts.read_text().strip() == "github.com " + expected_key
    Path(args[-1]).mkdir()
elif "diff" in args:
    sys.exit(0 if os.environ.get("FORMULA_UNCHANGED") else 1)
elif "push" in args:
    if os.environ.get("PUSH_FAILS"):
        sys.exit(1)
    shutil.copyfile(Path(args[1]) / "Formula/clickhousectl.rb", os.environ["PUBLISH_TEST_FORMULA"])
'''


class HomebrewPublisherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="homebrew test ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "homebrew").mkdir()
        shutil.copyfile(
            ROOT / "homebrew/clickhousectl.rb.tmpl",
            self.root / "homebrew/clickhousectl.rb.tmpl",
        )
        cargo = self.root / "crates/clickhousectl/Cargo.toml"
        cargo.parent.mkdir(parents=True)
        cargo.write_text('[package]\nname = "clickhousectl"\nversion = "0.5.0"\n')
        fake_bin = self.root / "bin"
        fake_bin.mkdir()
        for command in ("curl", "git"):
            path = fake_bin / command
            path.write_text(FAKE_COMMAND)
            path.chmod(0o755)
        self.log = self.root / "commands.jsonl"
        self.formula = self.root / "rendered.rb"
        self.env = {
            **os.environ,
            "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
            "HOMEBREW_TAP_DEPLOY_KEY": "fixture key only",
            "PUBLISH_TEST_LOG": str(self.log),
            "PUBLISH_TEST_FORMULA": str(self.formula),
            "TMPDIR": str(self.root),
        }

    def publish(self, version="0.5.0", **env):
        return subprocess.run(
            ["bash", str(ROOT / "scripts/update-homebrew-formula.sh"), version],
            cwd=self.root,
            env={**self.env, **env},
            capture_output=True,
            text=True,
        )

    def commands(self, command):
        if not self.log.exists():
            return []
        return [
            args
            for name, args in map(json.loads, self.log.read_text().splitlines())
            if name == command
        ]

    def test_publishes_four_verified_archives_and_valid_ruby(self):
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        formula = self.formula.read_text()
        self.assertNotIn("{{", formula)
        for target in TARGETS:
            asset = f"clickhousectl-{target}-v0.5.0.tar.gz"
            digest = hashlib.sha256(asset.encode()).hexdigest()
            self.assertIn(f'https://builds.clickhouse.com/clickhousectl/{asset}', formula)
            self.assertIn(f'sha256 "{digest}"', formula)
        self.assertEqual(len(self.commands("curl")), 8)
        self.assertTrue(any("commit" in args for args in self.commands("git")))
        self.assertTrue(any("push" in args for args in self.commands("git")))
        syntax = subprocess.run(["ruby", "-c", str(self.formula)], capture_output=True, text=True)
        self.assertEqual(syntax.returncode, 0, syntax.stderr)
        self.assertFalse(list(self.root.glob("tmp.*")), "publisher temporary files should be removed")

    def test_mismatched_version_stops_before_download_or_push(self):
        result = self.publish(version="0.5.1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.commands("curl"), [])
        self.assertEqual(self.commands("git"), [])

    def test_missing_key_stops_before_download_or_push(self):
        result = self.publish(HOMEBREW_TAP_DEPLOY_KEY="")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.commands("curl"), [])
        self.assertEqual(self.commands("git"), [])

    def test_missing_mirror_stops_before_tap_changes(self):
        result = self.publish(MIRROR_MISSING="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.commands("git"), [])

    def test_different_mirror_bytes_stop_before_tap_changes(self):
        result = self.publish(MIRROR_MISMATCH="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.commands("git"), [])

    def test_unchanged_formula_is_an_idempotent_success(self):
        result = self.publish(FORMULA_UNCHANGED="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(any("commit" in args for args in self.commands("git")))
        self.assertTrue(any("push" in args for args in self.commands("git")))

    def test_push_failure_is_not_reported_as_success(self):
        result = self.publish(PUSH_FAILS="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.formula.exists())


if __name__ == "__main__":
    unittest.main()
