import base64
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / "release_artifacts.py"
spec = importlib.util.spec_from_file_location("release_artifacts", MODULE)
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)

SOURCE = "a" * 40
BUILDER = "b" * 40
REF = "refs/tags/v1.2.3"
RUN = 42


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory = Path(self.tmp.name)
        self.objects = {}
        self.inputs = {}
        for n, (target, cpu) in enumerate(p.TARGETS.items(), 1):
            name = f"clickhousectl-{target}-{RUN}-{n}"
            data = b"\xcf\xfa\xed\xfe" + struct.pack("<I", cpu) + bytes(24) + bytes([n])
            (self.directory / name).write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            self.inputs[target] = (n, digest)
            self.objects[n] = {
                "id": n,
                "name": name,
                "digest": "sha256:" + digest,
                "expired": False,
                "workflow_run": {"id": RUN, "head_sha": SOURCE},
            }
        self.api = patch.object(p, "github_api", side_effect=self.github_api)
        self.api.start()
        self.addCleanup(self.api.stop)

    def github_api(self, path, *, paginate=False):
        if "/contents/Cargo.lock?" in path:
            return {"encoding": "base64", "content": base64.b64encode(b"lock").decode()}
        return copy.deepcopy(self.objects[int(path.rsplit("/", 1)[1])])

    def create(self):
        return p.receipt(self.directory, SOURCE, REF, RUN, 3, BUILDER, self.inputs)

    def verify(self):
        with (
            patch.object(p, "gh_version"),
            patch.object(p, "verify_attestation") as attestation,
        ):
            result = p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])
            self.assertEqual(attestation.call_count, 3)
            return result

    def test_valid_cross_attempt_receipt_and_verify(self):
        receipt = self.create()
        self.assertEqual(receipt["run_attempt"], 3)
        self.assertTrue(
            receipt["targets"]["aarch64-apple-darwin"]["name"].endswith("-1")
        )
        self.assertTrue(
            receipt["targets"]["x86_64-apple-darwin"]["name"].endswith("-2")
        )
        self.assertEqual(self.verify(), receipt)

    def test_tamper_and_digest_swap(self):
        self.create()
        name = self.objects[1]["name"]
        (self.directory / name).write_bytes(b"tampered")
        with self.assertRaises(p.ProvenanceError):
            self.verify()
        self.objects[1]["digest"] = self.objects[2]["digest"]
        with self.assertRaises(p.ProvenanceError):
            self.verify()

    def test_wrong_run_source_id_and_repository(self):
        for key, value in (("id", 99), ("head_sha", "c" * 40)):
            with self.subTest(key=key):
                old = self.objects[1]["workflow_run"][key]
                self.objects[1]["workflow_run"][key] = value
                with self.assertRaises(p.ProvenanceError):
                    self.create()
                self.objects[1]["workflow_run"][key] = old
        self.objects[1]["id"] = 22
        with self.assertRaises(p.ProvenanceError):
            self.create()
        self.objects[1]["id"] = 1
        self.objects[1]["workflow_run"]["repository_id"] = 22
        with self.assertRaises(p.ProvenanceError):
            self.create()

    def test_symlink_oversize_and_bad_architecture(self):
        file = self.directory / self.objects[1]["name"]
        original = file.read_bytes()
        file.unlink()
        file.symlink_to(self.directory / self.objects[2]["name"])
        with self.assertRaises(p.ProvenanceError):
            self.create()
        file.unlink()
        file.write_bytes(original)
        with patch.object(p, "MAX_BINARY", 10):
            with self.assertRaises(p.ProvenanceError):
                self.create()
        file.write_bytes(
            b"\xcf\xfa\xed\xfe"
            + struct.pack("<I", p.TARGETS["x86_64-apple-darwin"])
            + bytes(24)
        )
        with self.assertRaises(p.ProvenanceError):
            self.create()

    def test_extra_missing_and_duplicate(self):
        extra = self.directory / "extra"
        extra.write_text("extra")
        with self.assertRaises(p.ProvenanceError):
            self.create()
        extra.unlink()
        (self.directory / self.objects[1]["name"]).unlink()
        with self.assertRaises(p.ProvenanceError):
            self.create()
        self.inputs["x86_64-apple-darwin"] = self.inputs["aarch64-apple-darwin"]
        with self.assertRaises(p.ProvenanceError):
            self.create()

    def test_receipt_untrusted_before_crypto(self):
        self.create()
        file = self.directory / f"macos-build-receipt-{RUN}-3.json"
        obj = json.loads(file.read_text())
        obj["targets"] = "hostile"
        file.write_text(json.dumps(obj))
        with (
            patch.object(p, "gh_version"),
            patch.object(
                p,
                "verify_attestation",
                side_effect=p.ProvenanceError("invalid signature"),
            ) as check,
        ):
            with self.assertRaises(p.ProvenanceError):
                p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])
            self.assertEqual(check.call_count, 1)
        obj["builder_sha"] = "c" * 40
        file.write_text(json.dumps(obj))
        with (
            patch.object(p, "gh_version"),
            patch.object(p, "verify_attestation") as check,
        ):
            with self.assertRaises(p.ProvenanceError):
                p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])
            check.assert_not_called()

    def test_old_gh_rejected_before_crypto(self):
        self.create()
        with (
            patch.object(p, "command", return_value="gh version 2.83.2 (2025-01-01)"),
            patch.object(p, "verify_attestation") as check,
        ):
            with self.assertRaises(p.ProvenanceError):
                p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])
            check.assert_not_called()


class AttestationTests(unittest.TestCase):
    def setUp(self):
        self.path = Path("artifact")
        self.digest = "d" * 64
        identity = "https://github.com/" + p.WORKFLOW + "@" + BUILDER
        self.cert = {
            "subjectAlternativeName": identity,
            "buildSignerURI": identity,
            "buildSignerDigest": BUILDER,
            "sourceRepositoryURI": "https://github.com/" + p.REPO,
            "sourceRepositoryIdentifier": str(p.REPO_ID),
            "sourceRepositoryOwnerIdentifier": str(p.OWNER_ID),
            "sourceRepositoryDigest": SOURCE,
            "sourceRepositoryRef": REF,
            "buildTrigger": "push",
            "runnerEnvironment": "github-hosted",
            "runInvocationURI": f"https://github.com/{p.REPO}/actions/runs/{RUN}/attempts/2",
        }
        self.result = {
            "verificationResult": {
                "signature": {"certificate": self.cert},
                "verifiedTimestamps": [{"type": "Tlog"}],
                "statement": {
                    "predicateType": p.PREDICATE,
                    "subject": [
                        {"name": "another", "digest": {"sha256": self.digest}},
                        {"name": self.path.name, "digest": {"sha256": self.digest}},
                    ],
                },
            }
        }

    def check(self, results):
        with patch.object(p, "command", return_value=json.dumps(results)) as command:
            p.verify_attestation(self.path, BUILDER, SOURCE, REF, RUN, self.digest)
            args = command.call_args.args[0]
            self.assertIn("--cert-identity", args)
            self.assertNotIn("--signer-workflow", args)
            self.assertNotIn("--bundle", args)

    def test_valid_and_other_run_also_present(self):
        other = copy.deepcopy(self.result)
        other["verificationResult"]["signature"]["certificate"]["runInvocationURI"] = (
            f"https://github.com/{p.REPO}/actions/runs/999/attempts/1"
        )
        self.check([other, self.result])

    def test_foreign_and_malformed_certificate_rejected(self):
        for key, value in (
            ("subjectAlternativeName", "foreign"),
            ("buildSignerURI", "foreign"),
            ("sourceRepositoryIdentifier", "1"),
            ("sourceRepositoryOwnerIdentifier", "1"),
            ("sourceRepositoryRef", REF.upper()),
            ("buildTrigger", "pull_request"),
            ("runnerEnvironment", "self-hosted"),
            ("runInvocationURI", "foreign"),
        ):
            with self.subTest(key=key):
                result = copy.deepcopy(self.result)
                result["verificationResult"]["signature"]["certificate"][key] = value
                with self.assertRaises(p.ProvenanceError):
                    self.check([result])
        with self.assertRaises(p.ProvenanceError):
            self.check([{}])

    def test_subject_and_timestamp_required(self):
        for key in ("subject", "verifiedTimestamps"):
            result = copy.deepcopy(self.result)
            if key == "subject":
                result["verificationResult"]["statement"][key] = []
            else:
                result["verificationResult"][key] = []
            with self.assertRaises(p.ProvenanceError):
                self.check([result])


if __name__ == "__main__":
    unittest.main()
