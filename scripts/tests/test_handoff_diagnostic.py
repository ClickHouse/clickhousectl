import importlib.util
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("handoff_diagnostic", Path(__file__).parents[1] / "handoff-diagnostic.py")
diagnostic = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diagnostic)

BASE = {"schema": 1, "repository_id": 1150809931, "source_sha": "a" * 40, "run_id": 42, "run_attempt": 2}
ENV = {"HANDOFF_REPLY_APP_ID": "123", "ARTIFACT_ID": "456", "ARTIFACT_DIGEST": "b" * 64,
       "GITHUB_REPOSITORY": "ClickHouse/clickhousectl", "GITHUB_API_URL": "https://api.github.com", "GH_TOKEN": "token"}


def check(verdict="accepted"):
    return {"app": {"id": 123}, "name": "handoff-diagnostic/42/2", "head_sha": "a" * 40,
            "external_id": "456", "status": "completed", "conclusion": "success" if verdict == "accepted" else "failure",
            "output": {"summary": json.dumps({**BASE, "artifact_id": 456, "artifact_sha256": "b" * 64, "verdict": verdict})}}


class Response:
    headers = {}

    def __init__(self, runs):
        self.runs = runs

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def read(self, _limit):
        return json.dumps({"total_count": len(self.runs), "check_runs": self.runs}).encode()


class VerifyTest(unittest.TestCase):
    def run_response(self, runs, scenario="valid"):
        with patch.dict(os.environ, ENV), patch.object(diagnostic.urllib.request.OpenerDirector, "open", return_value=Response(runs)):
            diagnostic.verify(BASE, scenario)

    def test_authenticated_results(self):
        self.run_response([check()])
        self.run_response([check("rejected")], "mismatched-attempt")

    def test_duplicate_pinned_app_is_ambiguous(self):
        with self.assertRaises(diagnostic.DiagnosticError):
            self.run_response([check(), check()])

    def test_old_attempt_in_summary_is_rejected(self):
        run = check()
        summary = json.loads(run["output"]["summary"])
        summary["run_attempt"] = 1
        run["output"]["summary"] = json.dumps(summary)
        with self.assertRaises(diagnostic.DiagnosticError):
            self.run_response([run])

    def test_negative_requires_rejected_result(self):
        with self.assertRaises(diagnostic.DiagnosticError):
            self.run_response([check()], "mismatched-attempt")

    def test_duplicate_summary_keys_rejected(self):
        run = check()
        run["output"]["summary"] = '{"schema":1,"schema":1}'
        with self.assertRaises(diagnostic.DiagnosticError):
            self.run_response([run])

    def test_wrong_external_id_rejected(self):
        run = check()
        run["external_id"] = "457"
        with self.assertRaises(diagnostic.DiagnosticError):
            self.run_response([run])


if __name__ == "__main__":
    unittest.main()
