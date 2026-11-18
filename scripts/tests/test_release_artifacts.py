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
        return copy.deepcopy(self.objects[int(path.rsplit("/", 1)[1])])

    def create(self):
        result = p.receipt(self.directory, SOURCE, REF, RUN, 3, BUILDER, self.inputs)
        self.signed = {
            path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in self.directory.iterdir()
        }
        return result

    def command(self, args):
        if args[1] == "--version":
            return "gh version 2.102.0 (2026-01-01)"
        path = Path(args[3])
        if path.name not in self.signed:
            raise p.ProvenanceError("unsigned artifact")
        evidence = attestation(path, self.signed[path.name])
        if hasattr(self, "certificate_override"):
            evidence["verificationResult"]["signature"]["certificate"].update(
                self.certificate_override
            )
        return json.dumps([evidence])

    def verify(self):
        with patch.object(p, "command", side_effect=self.command):
            return p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])

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
        file = self.directory / self.objects[1]["name"]
        original = file.read_bytes()
        file.write_bytes(original + b"tampered")
        with self.assertRaisesRegex(p.ProvenanceError, "artifact content mismatch"):
            self.verify()
        file.write_bytes(original)
        self.objects[1]["digest"] = self.objects[2]["digest"]
        with self.assertRaisesRegex(
            p.ProvenanceError, "identity, expiration or digest"
        ):
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
        with self.assertRaisesRegex(p.ProvenanceError, "exactly two artifacts"):
            self.create()
        extra.unlink()
        file = self.directory / self.objects[1]["name"]
        original = file.read_bytes()
        file.unlink()
        with self.assertRaisesRegex(p.ProvenanceError, "exactly two artifacts"):
            self.create()
        file.write_bytes(original)
        self.inputs["x86_64-apple-darwin"] = self.inputs["aarch64-apple-darwin"]
        with self.assertRaisesRegex(p.ProvenanceError, "IDs must be distinct"):
            self.create()

    def test_changed_receipt_rejected_before_metadata(self):
        self.create()
        file = self.directory / f"macos-build-receipt-{RUN}-3.json"
        obj = json.loads(file.read_text())
        obj["targets"] = "hostile"
        file.write_text(json.dumps(obj))
        with self.assertRaisesRegex(p.ProvenanceError, "no attestation matches"):
            self.verify()

    def test_unsigned_receipt_rejected_before_metadata(self):
        self.create()
        self.signed.pop(f"macos-build-receipt-{RUN}-3.json")
        with self.assertRaisesRegex(p.ProvenanceError, "unsigned artifact"):
            self.verify()

    def test_unapproved_builder(self):
        self.create()
        file = self.directory / f"macos-build-receipt-{RUN}-3.json"
        obj = json.loads(file.read_text())
        obj["builder_sha"] = "c" * 40
        file.write_text(json.dumps(obj))
        with self.assertRaisesRegex(p.ProvenanceError, "unapproved builder"):
            self.verify()

    def test_receipt_decoder_recursion_is_invalid(self):
        self.create()
        # Native decoder nesting limits differ across Python versions.
        with patch.object(p.json, "loads", side_effect=RecursionError("receipt nesting")):
            with self.assertRaisesRegex(p.ProvenanceError, "invalid receipt"):
                self.verify()

    def test_verify_inventory_rejections(self):
        self.create()
        file = self.directory / self.objects[1]["name"]
        original = file.read_bytes()
        extra = self.directory / "extra"
        extra.write_text("extra")
        with self.assertRaisesRegex(p.ProvenanceError, "two artifacts and one receipt"):
            self.verify()
        extra.unlink()
        file.unlink()
        with self.assertRaisesRegex(p.ProvenanceError, "two artifacts and one receipt"):
            self.verify()
        file.symlink_to(self.directory / self.objects[2]["name"])
        with self.assertRaisesRegex(p.ProvenanceError, "not regular"):
            self.verify()
        file.unlink()
        file.write_bytes(original)
        extra = self.directory / f"macos-build-receipt-{RUN}-4.json"
        extra.write_text("{}")
        with self.assertRaisesRegex(p.ProvenanceError, "two artifacts and one receipt"):
            self.verify()

    def test_verify_rejects_bad_api_objects(self):
        self.create()
        cases = [
            ("run", lambda obj: obj["workflow_run"].update(id=99), "run or source"),
            (
                "source",
                lambda obj: obj["workflow_run"].update(head_sha="c" * 40),
                "run or source",
            ),
            ("id", lambda obj: obj.update(id=99), "identity, expiration or digest"),
            (
                "repository",
                lambda obj: obj.update(repository_id=99),
                "repository mismatch",
            ),
            ("name", lambda obj: obj.update(name="foreign"), "name mismatch"),
            (
                "expired",
                lambda obj: obj.update(expired=True),
                "identity, expiration or digest",
            ),
        ]
        original = copy.deepcopy(self.objects[1])
        for label, mutate, reason in cases:
            with self.subTest(label=label):
                self.objects[1] = copy.deepcopy(original)
                mutate(self.objects[1])
                with self.assertRaisesRegex(p.ProvenanceError, reason):
                    self.verify()

    def test_verify_duplicate_id_in_authenticated_receipt(self):
        self.create()
        file = self.directory / f"macos-build-receipt-{RUN}-3.json"
        obj = json.loads(file.read_text())
        obj["targets"]["x86_64-apple-darwin"]["artifact_id"] = 1
        file.write_text(json.dumps(obj))
        self.signed[file.name] = hashlib.sha256(file.read_bytes()).hexdigest()
        with self.assertRaisesRegex(p.ProvenanceError, "duplicate or invalid artifact"):
            self.verify()

    def test_verify_rejects_attested_wrong_builder(self):
        self.create()
        self.certificate_override = {"buildSignerDigest": "c" * 40}
        with self.assertRaisesRegex(p.ProvenanceError, "no attestation matches"):
            self.verify()

    def test_verify_rejects_swapped_authenticated_digest(self):
        self.create()
        file = self.directory / f"macos-build-receipt-{RUN}-3.json"
        obj = json.loads(file.read_text())
        swapped = self.inputs["x86_64-apple-darwin"][1]
        obj["targets"]["aarch64-apple-darwin"]["sha256"] = swapped
        self.objects[1]["digest"] = "sha256:" + swapped
        file.write_text(json.dumps(obj))
        self.signed[file.name] = hashlib.sha256(file.read_bytes()).hexdigest()
        with self.assertRaisesRegex(p.ProvenanceError, "artifact content mismatch"):
            self.verify()

    def test_old_gh_rejected_before_crypto(self):
        self.create()
        with (
            patch.object(p, "command", return_value="gh version 2.83.2 (2025-01-01)"),
            patch.object(p, "verify_attestation") as check,
        ):
            with self.assertRaises(p.ProvenanceError):
                p.verify(self.directory, SOURCE, REF, RUN, [BUILDER])
            check.assert_not_called()


def attestation(path, digest):
    identity = "https://github.com/" + p.WORKFLOW + "@" + BUILDER
    cert = {
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
    return {
        "verificationResult": {
            "signature": {"certificate": cert},
            "verifiedTimestamps": [{"type": "Tlog"}],
            "statement": {
                "predicateType": p.PREDICATE,
                "subject": [{"name": path.name, "digest": {"sha256": digest}}],
            },
        }
    }


class AttestationTests(unittest.TestCase):
    def setUp(self):
        self.path = Path("artifact")
        self.digest = "d" * 64
        self.result = attestation(self.path, self.digest)

    def check(self, results):
        with patch.object(p, "command", return_value=json.dumps(results)):
            return p.verify_attestation(
                self.path, BUILDER, SOURCE, REF, RUN, self.digest
            )

    def test_valid_and_other_run_also_present(self):
        other = copy.deepcopy(self.result)
        other["verificationResult"]["signature"]["certificate"]["runInvocationURI"] = (
            f"https://github.com/{p.REPO}/actions/runs/999/attempts/1"
        )
        trusted = self.check([other, self.result])
        self.assertEqual(trusted, self.result["verificationResult"])
        self.assertEqual(
            trusted["statement"]["subject"][0]["digest"]["sha256"], self.digest
        )

    def test_foreign_and_malformed_certificate_rejected(self):
        for key, value in (
            ("subjectAlternativeName", "foreign"),
            ("buildSignerURI", "foreign"),
            ("buildSignerDigest", "c" * 40),
            ("sourceRepositoryURI", "https://github.com/foreign/repo"),
            ("sourceRepositoryDigest", "c" * 40),
            ("sourceRepositoryIdentifier", "1"),
            ("sourceRepositoryOwnerIdentifier", "1"),
            ("sourceRepositoryRef", REF.upper()),
            ("buildTrigger", "pull_request"),
            ("runnerEnvironment", "self-hosted"),
            ("runInvocationURI", "foreign"),
            (
                "runInvocationURI",
                f"https://github.com/{p.REPO}/actions/runs/{RUN}/attempts/0",
            ),
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

    def test_predicate_and_subject_identity_required(self):
        mutations = (
            ("predicate", lambda s: s.update(predicateType="foreign")),
            ("name", lambda s: s["subject"][0].update(name="foreign")),
            ("digest", lambda s: s["subject"][0]["digest"].update(sha256="e" * 64)),
        )
        for label, mutate in mutations:
            with self.subTest(label=label):
                result = copy.deepcopy(self.result)
                mutate(result["verificationResult"]["statement"])
                with self.assertRaises(p.ProvenanceError):
                    self.check([result])


if __name__ == "__main__":
    unittest.main()
