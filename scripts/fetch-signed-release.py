#!/usr/bin/env python3
"""Fetch the approved release bundle without a private-repository credential."""

import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
import urllib.parse
import urllib.request
import zipfile

MAX_BUNDLE = 300 * 1024 * 1024
LIMITS = {"arm64": 144 * 1024 * 1024, "x86_64": 144 * 1024 * 1024,
          **dict.fromkeys(("manifest.json", "notarization.json", "arm64-receipt.json",
                          "x86_64-receipt.json"), 64 * 1024)}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def fetch(url, sha256, destination):
    parsed = urllib.parse.urlsplit(url)
    if (parsed.scheme != "https" or parsed.username or parsed.password or parsed.fragment
            or parsed.port is not None or not re.fullmatch(
                r"[a-z0-9][a-z0-9.-]*\.s3\.[a-z0-9-]+\.amazonaws\.com", parsed.hostname or "")
            or not parsed.path or not parsed.query or not re.fullmatch(r"[a-f0-9]{64}", sha256)
            or destination.exists() or destination.is_symlink()):
        raise ValueError("invalid handoff configuration")
    opener = urllib.request.build_opener(NoRedirect())
    with tempfile.TemporaryDirectory(dir=destination.parent) as temporary:
        root = Path(temporary)
        bundle = root / "bundle.zip"
        digest = hashlib.sha256()
        size = 0
        with opener.open(url, timeout=30) as response, bundle.open("xb") as output:
            while chunk := response.read(1024 * 1024):
                size += len(chunk)
                if size > MAX_BUNDLE:
                    raise ValueError("handoff exceeds byte limit")
                digest.update(chunk)
                output.write(chunk)
        if digest.hexdigest() != sha256:
            raise ValueError("handoff digest mismatch")
        result = root / "result"
        result.mkdir()
        with zipfile.ZipFile(bundle) as archive:
            entries = archive.infolist()
            if len(entries) != len(LIMITS) or {entry.filename for entry in entries} != LIMITS.keys():
                raise ValueError("unexpected handoff inventory")
            for entry in entries:
                mode = stat.S_IFMT(entry.external_attr >> 16)
                if mode not in (0, stat.S_IFREG) or not 0 < entry.file_size <= LIMITS[entry.filename]:
                    raise ValueError("unexpected handoff member")
                with archive.open(entry) as source, (result / entry.filename).open("xb") as output:
                    shutil.copyfileobj(source, output, 1024 * 1024)
        result.rename(destination)


def main():
    if len(sys.argv) != 2:
        return 2
    try:
        fetch(os.environ["MACOS_RELEASE_BUNDLE_URL"], os.environ["MACOS_RELEASE_BUNDLE_SHA256"], Path(sys.argv[1]))
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, RuntimeError):
        # URLs and storage responses can contain temporary credentials.
        print("Signed release handoff rejected", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
