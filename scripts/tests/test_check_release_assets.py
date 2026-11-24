import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import types
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "check-release-assets.py"
spec = importlib.util.spec_from_file_location("check_release_assets", SCRIPT)
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class ReleaseAssetsTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bundle = self.root / "bundle"
        self.bundle.mkdir()
        self.archives = self.root / "archives"
        self.archives.mkdir()
        self.source = {"repository_id": 1150809931, "sha": "a" * 40, "run_id": 12, "run_attempt": 2}
        self.signer = {"repository_id": 1464494971, "sha": "b" * 40, "run_id": 13, "run_attempt": 1}
        self.binaries = {"arm64": b"arm binary", "x86_64": b"intel binary"}
        self.manifest = {"schema": 1, "version": "1.2.3", "source": self.source,
                         "signer": self.signer, "binaries": {a: sha(b) for a, b in self.binaries.items()}}
        self.ref = "refs/tags/v1.2.3"
        self.write_json(self.bundle / "manifest.json", self.manifest)
        self.manifest_sha = sha((self.bundle / "manifest.json").read_bytes())
        self.write_json(self.bundle / "notarization.json", {"schema": 1, "submission_id": "12345678-1234-1234-1234-123456789abc", "status": "Accepted", "manifest_sha256": self.manifest_sha})
        for arch, binary in self.binaries.items():
            (self.bundle / arch).write_bytes(binary)
            self.write_json(self.bundle / f"{arch}-receipt.json", {"schema": 1, "arch": arch, "macos": "15.5", "manifest_sha256": self.manifest_sha, "source": self.source, "signer": self.signer, "binary_sha256": sha(binary), "leaf_sha256": "c" * 64, "gatekeeper": "Notarized Developer ID", "version": "1.2.3"})
        self.args = types.SimpleNamespace(directory=self.bundle, manifest_sha256=self.manifest_sha,
                                          source_sha="a" * 40, signer_sha="b" * 40, leaf_sha256="c" * 64,
                                          ref=self.ref, run_id="12", receipt_run_attempt="2")

    def write_json(self, path, value):
        path.write_text(json.dumps(value), encoding="utf-8")

    def archive(self, target, binary=None, extra=None):
        root = f"clickhousectl-{target}-v1.2.3"
        path = self.archives / f"{root}.tar.gz"
        if binary is None:
            binary = self.binaries["arm64" if target.startswith("aarch64") else "x86_64"]
        with tarfile.open(path, "w:gz") as tar:
            directory = tarfile.TarInfo(root + "/")
            directory.type = tarfile.DIRTYPE
            tar.addfile(directory)
            for name, data in [("clickhousectl", binary), ("LICENSE", b"license")] + (extra or []):
                member = tarfile.TarInfo(root + "/" + name)
                member.size = len(data)
                tar.addfile(member, io.BytesIO(data))
        return path

    def all_archives(self):
        for target in gate.TARGETS:
            self.archive(target)
        return types.SimpleNamespace(directory=self.archives, ref=self.ref, manifest=self.bundle / "manifest.json")

    def test_valid_bundle(self):
        gate.verify_macos(self.args)

    def test_pins_and_identity(self):
        for field, value in [("manifest_sha256", "0" * 64), ("source_sha", "0" * 40),
                             ("signer_sha", "0" * 40), ("leaf_sha256", "0" * 64),
                             ("run_id", "13"), ("receipt_run_attempt", "1"), ("ref", "refs/tags/v1.2.4")]:
            with self.subTest(field=field):
                args = types.SimpleNamespace(**vars(self.args))
                setattr(args, field, value)
                with self.assertRaises(gate.Rejected):
                    gate.verify_macos(args)

    def test_receipt_binding_and_notarization(self):
        path = self.bundle / "arm64-receipt.json"
        original = json.loads(path.read_text())
        for field, value in [("arch", "x86_64"), ("binary_sha256", "0" * 64),
                             ("leaf_sha256", "0" * 64), ("gatekeeper", "accepted"),
                             ("source", {**self.source, "run_attempt": 1}), ("schema", True)]:
            with self.subTest(field=field):
                self.write_json(path, {**original, field: value})
                with self.assertRaises(gate.Rejected):
                    gate.verify_macos(self.args)
        self.write_json(path, original)
        self.write_json(self.bundle / "notarization.json", {"schema": 1, "submission_id": "12345678-1234-1234-1234-123456789abc", "status": "Rejected", "manifest_sha256": self.manifest_sha})
        with self.assertRaises(gate.Rejected):
            gate.verify_macos(self.args)

    def test_binary_and_inventory_tampering(self):
        (self.bundle / "arm64").write_bytes(b"changed")
        with self.assertRaises(gate.Rejected):
            gate.verify_macos(self.args)
        (self.bundle / "arm64").write_bytes(self.binaries["arm64"])
        (self.bundle / "extra").write_bytes(b"x")
        with self.assertRaises(gate.Rejected):
            gate.verify_macos(self.args)

    def test_duplicate_manifest_key(self):
        path = self.bundle / "manifest.json"
        path.write_text('{"schema":1,"schema":1}')
        self.args.manifest_sha256 = sha(path.read_bytes())
        with self.assertRaises(gate.Rejected):
            gate.verify_macos(self.args)

    def test_valid_archives_and_checksums(self):
        args = self.all_archives()
        gate.verify_archives(args)
        entries = sorted(self.archives.iterdir())
        (self.archives / "SHA256SUMS").write_text("".join(f"{sha(p.read_bytes())}  {p.name}\n" for p in entries))
        gate.verify_archives(args)
        (self.archives / "SHA256SUMS").write_text("0" * 64 + "  invalid\n")
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)

    def test_archive_swaps_and_extra_members(self):
        args = self.all_archives()
        target = "aarch64-apple-darwin"
        self.archive(target, self.binaries["x86_64"])
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)
        self.archive(target, extra=[("../evil", b"x")])
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)
        self.archive(target, extra=[("clickhousectl", b"duplicate")])
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)

    def test_archive_inventory(self):
        args = self.all_archives()
        next(self.archives.iterdir()).unlink()
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)
        self.all_archives()
        (self.archives / "extra").write_bytes(b"x")
        with self.assertRaises(gate.Rejected):
            gate.verify_archives(args)


if __name__ == "__main__":
    unittest.main()
