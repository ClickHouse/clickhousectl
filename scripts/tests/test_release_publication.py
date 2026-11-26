import hashlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SCRIPTS = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("release_handoff", SCRIPTS / "fetch-signed-release.py")
handoff = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(handoff)


class HandoffTests(unittest.TestCase):
    def test_only_the_audited_bundle_can_be_extracted(self):
        bundle = io.BytesIO()
        with zipfile.ZipFile(bundle, "w") as output:
            for name in handoff.LIMITS:
                output.writestr(name, name.encode())
        raw = bundle.getvalue()
        url = "https://staging.s3.us-east-1.amazonaws.com/item?X-Amz-Signature=fixture"
        with tempfile.TemporaryDirectory() as temporary:
            dest = Path(temporary) / "notarized"
            with patch.object(handoff.urllib.request, "build_opener") as build:
                with self.assertRaises(ValueError):
                    handoff.fetch(url, "", dest)
                build.assert_not_called()
                build.return_value.open.return_value = io.BytesIO(raw)
                with self.assertRaises(ValueError):
                    handoff.fetch(url, "0" * 64, dest)
                self.assertFalse(dest.exists())
                build.return_value.open.return_value = io.BytesIO(raw)
                handoff.fetch(url, hashlib.sha256(raw).hexdigest(), dest)
            self.assertEqual((dest / "arm64").read_bytes(), b"arm64")


class S3Tests(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory()
        self.addCleanup(work.cleanup)
        self.root = Path(work.name)
        self.assets = self.root / "assets"
        self.assets.mkdir()
        self.remote = self.root / "objects"
        self.remote.mkdir()
        self.log = self.root / "puts"
        fake = self.root / "aws"
        fake.write_text('''#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
    's3api head-object')
        if [[ ! -f "$OBJECTS/${6##*/}" ]]; then
            echo 'An error occurred (404) when calling the HeadObject operation: Not Found' >&2
            exit 254
        fi ;;
    's3 cp') cp "$OBJECTS/${3##*/}" "$4" ;;
    's3api put-object')
        [[ "${11} ${12}" == '--if-none-match *' ]]
        test ! -e "$OBJECTS/${6##*/}"
        printf '%s\n' "$6" >> "$PUTS"
        cp "$8" "$OBJECTS/${6##*/}" ;;
    *) exit 3 ;;
esac
''')
        fake.chmod(0o755)
        self.env = {**os.environ, "PATH": str(self.root) + os.pathsep + os.environ["PATH"],
                    "OBJECTS": str(self.remote), "PUTS": str(self.log), "S3_BUCKET": "fixture",
                    "S3_PREFIX": "clickhousectl", "GITHUB_REF_NAME": "v1.2.3"}
        (self.assets / "a.tar.gz").write_bytes(b"new archive")
        (self.assets / "b.tar.gz").write_bytes(b"authorized archive")
        (self.assets / "SHA256SUMS").write_bytes(b"approved checksums\n")

    def publish(self):
        return subprocess.run(["bash", str(SCRIPTS / "upload-release-assets.sh")], cwd=self.assets,
                              env=self.env, capture_output=True, timeout=5)

    def test_keep_mismatch_stops_before_any_upload(self):
        (self.remote / "b.tar.gz").write_bytes(b"different archive")
        result = self.publish()
        self.assertEqual(result.returncode, 1)
        self.assertFalse(self.log.exists())
        self.assertEqual((self.assets / "b.tar.gz").read_bytes(), b"authorized archive")

    def test_same_bytes_remain_and_missing_objects_are_written(self):
        (self.remote / "b.tar.gz").write_bytes(b"authorized archive")
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(set(self.log.read_text().splitlines()), {"clickhousectl/a.tar.gz",
                         "clickhousectl/clickhousectl-v1.2.3-SHA256SUMS"})
        self.assertEqual((self.remote / "a.tar.gz").read_bytes(), b"new archive")


if __name__ == "__main__":
    unittest.main()
