import importlib.util
import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "check-openapi-drift.py"
SPEC = importlib.util.spec_from_file_location("check_openapi_drift", SCRIPT)
drift = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = drift
SPEC.loader.exec_module(drift)


class DriftScriptTests(unittest.TestCase):
    def test_groups_findings_and_renders_spec_snippets(self):
        report = {
            "schema_version": 1,
            "findings": [
                {
                    "kind": "missing_client_method",
                    "message": "missing",
                    "spec_pointer": "/paths/~1widgets/get",
                    "rust_item": "client.rs::Client::list_widgets",
                    "details": {
                        "method_name": "list_widgets",
                        "operation_id": "listWidgets",
                        "method": "GET",
                        "path": "/widgets",
                        "summary": "List widgets",
                    },
                }
            ],
            "unsupported_enum_constraints": [
                {
                    "spec_pointer": "/components/schemas/Widget/properties/state",
                    "rust_item": "models.rs::Widget::state",
                    "reason": "Rust type String is not an enum",
                    "acknowledged": True,
                }
            ],
        }
        spec = {
            "paths": {
                "/widgets": {
                    "get": {"operationId": "listWidgets", "summary": "List widgets"}
                }
            }
        }

        self.assertEqual(
            drift.findings_by_kind(report)["missing_client_method"][0]["message"],
            "missing",
        )
        body = drift.build_issue_body(report, spec)
        self.assertIn("## Missing Client Methods", body)
        self.assertIn('"operationId": "listWidgets"', body)
        self.assertIn("## Acknowledged Unsupported Enum Constraints", body)

    @mock.patch.object(drift.subprocess, "run")
    def test_analyzer_subprocess_failure_is_fatal(self, run):
        run.return_value = SimpleNamespace(returncode=1, stdout="", stderr="bad source")
        with self.assertRaisesRegex(RuntimeError, "bad source"):
            drift.run_analyzer({"paths": {}, "components": {"schemas": {}}})

    @mock.patch.object(drift.subprocess, "run")
    def test_analyzer_report_schema_is_validated(self, run):
        run.return_value = SimpleNamespace(
            returncode=0,
            stdout=json.dumps({"schema_version": 2}),
            stderr="",
        )
        with self.assertRaisesRegex(RuntimeError, "schema version"):
            drift.run_analyzer({"paths": {}, "components": {"schemas": {}}})


if __name__ == "__main__":
    unittest.main()
