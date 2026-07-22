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
            "schema_version": 2,
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
                },
                {
                    "kind": "enum_values_mismatch",
                    "message": "Color::VALUES does not match enum wire values: missing \"green\"",
                    "rust_item": "models.rs::Color::VALUES",
                    "details": {
                        "enum": "Color",
                        "missing": "green",
                    },
                },
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
        self.assertIn("## Enum VALUES Const Mismatches", body)

    def test_truncate_issue_body_keeps_short_bodies_intact(self):
        body = "short body"
        self.assertEqual(drift.truncate_issue_body(body), body)

    def test_truncate_issue_body_fits_github_limit_and_closes_markdown(self):
        lines = ["<details>", "```json"]
        lines += ['{"filler": %d}' % i for i in range(10_000)]
        lines += ["```", "</details>"]
        body = "\n".join(lines)
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        truncated = drift.truncate_issue_body(body)
        self.assertLessEqual(len(truncated), drift.MAX_ISSUE_BODY_CHARS)
        self.assertTrue(truncated.endswith(drift.TRUNCATION_NOTICE))
        # The cut lands inside the fenced block; both must be re-closed.
        before_notice = truncated[: -len(drift.TRUNCATION_NOTICE)]
        self.assertTrue(before_notice.endswith("```\n</details>"))

    @mock.patch.object(drift.subprocess, "run")
    def test_create_issue_streams_body_over_stdin(self, run):
        body = "x" * (drift.MAX_ISSUE_BODY_CHARS * 3)
        drift.create_issue("title", body)

        run.assert_called_once()
        args, kwargs = run.call_args
        command = args[0]
        self.assertIn("--body-file", command)
        self.assertIn("-", command)
        self.assertNotIn("--body", command)
        self.assertNotIn(body, command)
        self.assertLessEqual(len(kwargs["input"]), drift.MAX_ISSUE_BODY_CHARS)
        self.assertTrue(kwargs["input"].endswith(drift.TRUNCATION_NOTICE))

    @mock.patch.object(drift.subprocess, "run")
    def test_analyzer_subprocess_failure_is_fatal(self, run):
        run.return_value = SimpleNamespace(returncode=1, stdout="", stderr="bad source")
        with self.assertRaisesRegex(RuntimeError, "bad source"):
            drift.run_analyzer({"paths": {}, "components": {"schemas": {}}})

    @mock.patch.object(drift.subprocess, "run")
    def test_analyzer_report_schema_is_validated(self, run):
        run.return_value = SimpleNamespace(
            returncode=0,
            stdout=json.dumps({"schema_version": 3}),
            stderr="",
        )
        with self.assertRaisesRegex(RuntimeError, "schema version"):
            drift.run_analyzer({"paths": {}, "components": {"schemas": {}}})


if __name__ == "__main__":
    unittest.main()
