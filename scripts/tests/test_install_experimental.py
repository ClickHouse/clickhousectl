#!/usr/bin/env python3
"""End-to-end tests for scripts/install-experimental.sh against a local server."""

from __future__ import annotations

import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import io
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import threading
import unittest

SCRIPT_PATH = Path(__file__).resolve().parent.parent / "install-experimental.sh"


def host_target() -> str:
    sys_name = platform.system()
    if sys_name == "Linux":
        os_target = "unknown-linux-musl"
    elif sys_name == "Darwin":
        os_target = "apple-darwin"
    else:
        raise unittest.SkipTest(f"unsupported OS {sys_name}")

    machine = platform.machine()
    if machine in ("x86_64", "amd64"):
        arch_target = "x86_64"
    elif machine in ("aarch64", "arm64"):
        arch_target = "aarch64"
    else:
        raise unittest.SkipTest(f"unsupported arch {machine}")

    return f"{arch_target}-{os_target}"


def build_archive(tag: str, target: str, payload: bytes) -> tuple[str, bytes, str]:
    archive_name = f"clickhousectl-{target}-{tag}.tar.gz"
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tf:
        info = tarfile.TarInfo(name=f"clickhousectl-{target}-{tag}/clickhousectl")
        info.size = len(payload)
        info.mode = 0o755
        tf.addfile(info, io.BytesIO(payload))
    data = buf.getvalue()
    digest = hashlib.sha256(data).hexdigest()
    return archive_name, data, digest


class MockGitHubServer:
    def __init__(self, routes: dict[str, bytes]) -> None:
        self.routes = routes

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self_handler) -> None:
                path = self_handler.path.split("?", 1)[0]
                body = routes.get(path)
                if body is None:
                    self_handler.send_response(404)
                    self_handler.end_headers()
                    return
                self_handler.send_response(200)
                self_handler.send_header("Content-Length", str(len(body)))
                self_handler.end_headers()
                self_handler.wfile.write(body)

            def log_message(self_handler, format: str, *args: object) -> None:
                pass

        self.server = HTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        host, port = self.server.server_address
        return f"http://{host}:{port}"

    def __enter__(self) -> MockGitHubServer:
        self.thread.start()
        return self

    def __exit__(self, *args: object) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)


class InstallExperimentalScriptTest(unittest.TestCase):
    def setUp(self) -> None:
        self.target = host_target()

    def run_installer(
        self,
        base_url: str,
        install_dir: Path,
        *args: str,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.pop("CLICKHOUSECTL_EXPERIMENTAL_TAG", None)
        env["CLICKHOUSECTL_GITHUB_URL"] = base_url
        env["CLICKHOUSECTL_GITHUB_API_URL"] = base_url
        env["CLICKHOUSECTL_INSTALL_DIR"] = str(install_dir)
        return subprocess.run(
            ["sh", str(SCRIPT_PATH), *args],
            env=env,
            capture_output=True,
            text=True,
        )

    def test_installs_latest_experimental_prerelease_ignoring_stable_and_drafts(self) -> None:
        newer_tag = "experimental-20261128-120000-bbbbbbb"
        older_tag = "experimental-20261120-120000-aaaaaaa"
        payload = b"#!/bin/sh\necho clickhousectl 0.6.1-experimental.20261128.bbbbbbb\n"
        archive_name, archive_bytes, digest = build_archive(newer_tag, self.target, payload)

        releases = [
            {
                "tag_name": "v0.6.2",
                "prerelease": False,
                "draft": False,
                "created_at": "2026-11-29T00:00:00Z",
            },
            {
                "tag_name": "experimental-20261130-120000-ccccccc",
                "prerelease": True,
                "draft": True,
                "created_at": "2026-11-30T00:00:00Z",
            },
            {
                "tag_name": older_tag,
                "prerelease": True,
                "draft": False,
                "created_at": "2026-11-20T12:00:00Z",
            },
            {
                "tag_name": newer_tag,
                "prerelease": True,
                "draft": False,
                "created_at": "2026-11-28T12:00:00Z",
            },
        ]
        routes = {
            "/repos/ClickHouse/clickhousectl/releases": json.dumps(releases).encode(),
            f"/ClickHouse/clickhousectl/releases/download/{newer_tag}/{archive_name}": archive_bytes,
            f"/ClickHouse/clickhousectl/releases/download/{newer_tag}/SHA256SUMS": (
                f"{digest}  {archive_name}\n".encode()
            ),
        }

        with tempfile.TemporaryDirectory() as tmp, MockGitHubServer(routes) as srv:
            install_dir = Path(tmp) / "bin"
            proc = self.run_installer(srv.url, install_dir)
            self.assertEqual(proc.returncode, 0, f"stdout={proc.stdout}\nstderr={proc.stderr}")

            binary = install_dir / "clickhousectl"
            alias = install_dir / "chctl"
            self.assertEqual(binary.read_bytes(), payload)
            self.assertTrue(alias.is_symlink())
            self.assertEqual(alias.resolve(), binary.resolve())

    def test_rejects_tampered_archive_and_installs_nothing(self) -> None:
        tag = "experimental-20261128-120000-deadbee"
        archive_name, archive_bytes, _ = build_archive(tag, self.target, b"tampered")
        bad_digest = "0" * 64
        routes = {
            f"/ClickHouse/clickhousectl/releases/download/{tag}/{archive_name}": archive_bytes,
            f"/ClickHouse/clickhousectl/releases/download/{tag}/SHA256SUMS": (
                f"{bad_digest}  {archive_name}\n".encode()
            ),
        }

        with tempfile.TemporaryDirectory() as tmp, MockGitHubServer(routes) as srv:
            install_dir = Path(tmp) / "bin"
            proc = self.run_installer(srv.url, install_dir, "--tag", tag)
            self.assertNotEqual(proc.returncode, 0)
            self.assertFalse((install_dir / "clickhousectl").exists())

    def test_rejects_stable_v_tag(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            install_dir = Path(tmp) / "bin"
            proc = self.run_installer("http://127.0.0.1:1", install_dir, "--tag", "v0.6.1")
            self.assertNotEqual(proc.returncode, 0)
            self.assertFalse((install_dir / "clickhousectl").exists())


if __name__ == "__main__":
    unittest.main()
