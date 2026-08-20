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
            "schema_version": 3,
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

    def test_sync_creates_issue_when_none_is_open(self):
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[]),
            mock.patch.object(drift, "ensure_label_exists") as ensure_label,
            mock.patch.object(drift, "create_issue") as create_issue,
        ):
            action = drift.sync_drift_issue("new title", "new body")

        self.assertEqual(action, "created")
        ensure_label.assert_called_once_with()
        create_issue.assert_called_once_with("new title", "new body")

    def test_sync_updates_issue_and_only_reconciles_bot_generated_comments(self):
        issue = {
            "number": 42,
            "title": "old title",
            "body": "old body",
            "url": "https://github.com/ClickHouse/clickhousectl/issues/42",
        }
        old_first = drift.CONTINUATION_HEADER + "old first"
        stale_second = drift.CONTINUATION_HEADER + "stale second"
        desired = drift.CONTINUATION_HEADER + "new first"
        comments = [
            {
                "id": 101,
                "user": {"login": "github-actions[bot]"},
                "body": old_first,
            },
            {
                "id": 201,
                "user": {"login": "alice"},
                "body": stale_second,
            },
            {
                "id": 202,
                "user": {"login": "another-bot[bot]"},
                "body": stale_second,
            },
            {
                "id": 102,
                "user": {"login": "github-actions[bot]"},
                "body": stale_second,
            },
            {
                "id": 103,
                "user": {"login": "github-actions[bot]"},
                "body": drift.INCOMPLETE_REPORT_PREFIX + " retry failed",
            },
            {
                "id": 104,
                "user": {"login": "github-actions[bot]"},
                "body": "Unrelated automation comment",
            },
        ]
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[issue]),
            mock.patch.object(drift, "issue_comments", return_value=comments),
            mock.patch.object(drift, "split_issue_body", return_value=["new body", desired]),
            mock.patch.object(drift, "edit_issue") as edit_issue,
            mock.patch.object(drift, "edit_comment") as edit_comment,
            mock.patch.object(drift, "delete_comment") as delete_comment,
            mock.patch.object(drift, "post_continuation_comment") as post_comment,
        ):
            action = drift.sync_drift_issue("new title", "rendered body")

        self.assertEqual(action, "updated")
        edit_issue.assert_called_once_with(issue["url"], "new title", "new body")
        edit_comment.assert_called_once_with(101, desired)
        self.assertEqual(delete_comment.call_args_list, [mock.call(102), mock.call(103)])
        post_comment.assert_not_called()

    def test_sync_adds_new_continuation_comments_on_update(self):
        issue = {
            "number": 42,
            "title": "current title",
            "body": "current body",
            "url": "https://github.com/ClickHouse/clickhousectl/issues/42",
        }
        desired = drift.CONTINUATION_HEADER + "new continuation"
        comments = [
            {
                "id": 201,
                "user": {"login": "alice"},
                "body": "Keep this human comment",
            }
        ]
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[issue]),
            mock.patch.object(drift, "issue_comments", return_value=comments),
            mock.patch.object(
                drift, "split_issue_body", return_value=["current body", desired]
            ),
            mock.patch.object(drift, "edit_issue") as edit_issue,
            mock.patch.object(drift, "post_continuation_comment") as post_comment,
        ):
            action = drift.sync_drift_issue("current title", "rendered body")

        self.assertEqual(action, "updated")
        edit_issue.assert_not_called()
        post_comment.assert_called_once_with(issue["url"], desired)

    def test_sync_leaves_unchanged_report_and_human_comments_untouched(self):
        desired = drift.CONTINUATION_HEADER + "current continuation"
        issue = {
            "number": 42,
            "title": "current title",
            "body": "current body",
            "url": "https://github.com/ClickHouse/clickhousectl/issues/42",
        }
        comments = [
            {
                "id": 101,
                "user": {"login": "github-actions[bot]"},
                "body": desired,
            },
            {
                "id": 201,
                "user": {"login": "alice"},
                "body": "Keep this human comment",
            },
        ]
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[issue]),
            mock.patch.object(drift, "issue_comments", return_value=comments),
            mock.patch.object(
                drift, "split_issue_body", return_value=["current body", desired]
            ),
            mock.patch.object(drift, "edit_issue") as edit_issue,
            mock.patch.object(drift, "edit_comment") as edit_comment,
            mock.patch.object(drift, "delete_comment") as delete_comment,
            mock.patch.object(drift, "post_continuation_comment") as post_comment,
            mock.patch.object(drift, "close_issue") as close_issue,
        ):
            action = drift.sync_drift_issue("current title", "rendered body")

        self.assertEqual(action, "unchanged")
        edit_issue.assert_not_called()
        edit_comment.assert_not_called()
        delete_comment.assert_not_called()
        post_comment.assert_not_called()
        close_issue.assert_not_called()

    def test_sync_closes_open_issue_for_clean_report(self):
        issue = {
            "number": 42,
            "title": "old title",
            "body": "old body",
            "url": "https://github.com/ClickHouse/clickhousectl/issues/42",
        }
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[issue]),
            mock.patch.object(drift, "issue_comments") as issue_comments,
            mock.patch.object(drift, "close_issue") as close_issue,
        ):
            action = drift.sync_drift_issue(None, None)

        self.assertEqual(action, "closed")
        close_issue.assert_called_once_with(issue["url"])
        issue_comments.assert_not_called()

    def test_sync_clean_report_without_open_issue_is_noop(self):
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[]),
            mock.patch.object(drift, "close_issue") as close_issue,
        ):
            action = drift.sync_drift_issue(None, None)

        self.assertEqual(action, "clean")
        close_issue.assert_not_called()

    def test_sync_refuses_multiple_open_drift_issues(self):
        issues = [
            {"number": 41, "url": "https://example.test/41"},
            {"number": 42, "url": "https://example.test/42"},
        ]
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=issues),
            mock.patch.object(drift, "create_issue") as create_issue,
            mock.patch.object(drift, "close_issue") as close_issue,
        ):
            with self.assertRaisesRegex(RuntimeError, "Multiple open"):
                drift.sync_drift_issue("title", "body")

        create_issue.assert_not_called()
        close_issue.assert_not_called()

    def test_sync_propagates_lookup_failure_without_creating_issue(self):
        error = drift.subprocess.CalledProcessError(1, ["gh", "issue", "list"])
        with (
            mock.patch.object(drift, "open_drift_issues", side_effect=error),
            mock.patch.object(drift, "create_issue") as create_issue,
        ):
            with self.assertRaises(drift.subprocess.CalledProcessError):
                drift.sync_drift_issue("title", "body")

        create_issue.assert_not_called()

    def test_sync_propagates_update_failure_without_touching_comments(self):
        issue = {
            "number": 42,
            "title": "old title",
            "body": "old body",
            "url": "https://github.com/ClickHouse/clickhousectl/issues/42",
        }
        error = drift.subprocess.CalledProcessError(1, ["gh", "issue", "edit"])
        with (
            mock.patch.object(drift, "open_drift_issues", return_value=[issue]),
            mock.patch.object(drift, "issue_comments", return_value=[]),
            mock.patch.object(drift, "edit_issue", side_effect=error),
            mock.patch.object(drift, "edit_comment") as edit_comment,
            mock.patch.object(drift, "delete_comment") as delete_comment,
            mock.patch.object(drift, "post_continuation_comment") as post_comment,
        ):
            with self.assertRaises(drift.subprocess.CalledProcessError):
                drift.sync_drift_issue("new title", "new body")

        edit_comment.assert_not_called()
        delete_comment.assert_not_called()
        post_comment.assert_not_called()

    @mock.patch.object(drift.subprocess, "run")
    def test_open_issue_lookup_fails_closed(self, run):
        run.side_effect = drift.subprocess.CalledProcessError(
            1, ["gh", "issue", "list"], stderr="lookup failed"
        )

        with self.assertRaises(drift.subprocess.CalledProcessError):
            drift.open_drift_issues()

        self.assertTrue(run.call_args.kwargs["check"])

    @mock.patch.object(drift.subprocess, "run")
    def test_issue_comment_lookup_flattens_all_pages(self, run):
        comments = [
            {"id": 1, "body": "first"},
            {"id": 2, "body": "second"},
        ]
        run.return_value = SimpleNamespace(
            returncode=0,
            stdout=json.dumps([[comments[0]], [comments[1]]]),
            stderr="",
        )

        self.assertEqual(drift.issue_comments(42), comments)

        command = run.call_args.args[0]
        self.assertEqual(command[:2], ["gh", "api"])
        self.assertIn("--paginate", command)
        self.assertIn("--slurp", command)
        self.assertTrue(run.call_args.kwargs["check"])

    def test_dry_run_never_queries_or_mutates_github(self):
        reports = [
            {"schema_version": 3, "findings": []},
            {"schema_version": 3, "findings": [{"kind": "missing_struct_field"}]},
        ]
        for report in reports:
            with self.subTest(findings=len(report["findings"])):
                with (
                    mock.patch.object(sys, "argv", [str(SCRIPT), "--dry-run"]),
                    mock.patch.object(drift, "fetch_live_spec", return_value={}),
                    mock.patch.object(drift, "run_analyzer", return_value=report),
                    mock.patch.object(drift, "build_issue_body", return_value="body"),
                    mock.patch.object(drift, "sync_drift_issue") as sync_issue,
                    mock.patch("builtins.print"),
                ):
                    drift.main()

                sync_issue.assert_not_called()

    def test_main_exits_nonzero_when_synchronization_fails(self):
        report = {"schema_version": 3, "findings": []}
        with (
            mock.patch.object(sys, "argv", [str(SCRIPT)]),
            mock.patch.object(drift, "fetch_live_spec", return_value={}),
            mock.patch.object(drift, "run_analyzer", return_value=report),
            mock.patch.object(
                drift, "sync_drift_issue", side_effect=RuntimeError("lookup failed")
            ),
            mock.patch("builtins.print"),
        ):
            with self.assertRaises(SystemExit) as raised:
                drift.main()

        self.assertEqual(raised.exception.code, 1)

    @mock.patch.object(drift.subprocess, "run")
    def test_close_issue_marks_report_completed(self, run):
        run.return_value = SimpleNamespace(returncode=0, stdout="", stderr="")

        drift.close_issue("https://github.com/ClickHouse/clickhousectl/issues/42")

        command = run.call_args.args[0]
        self.assertEqual(command[:3], ["gh", "issue", "close"])
        self.assertIn("completed", command)
        self.assertIn(drift.CLEAN_ISSUE_COMMENT, command)
        self.assertTrue(run.call_args.kwargs["check"])

    def test_split_issue_body_keeps_short_bodies_intact(self):
        body = "short body"
        self.assertEqual(drift.split_issue_body(body), [body])

    def test_split_issue_body_breaks_between_blocks_and_loses_nothing(self):
        # Many small self-contained sections: every chunk boundary should land
        # between blocks, never inside a fence or <details> block.
        section = "\n".join(
            ["## Section", "", "<details>", "```json"]
            + ['{"filler": "%s"}' % ("x" * 60) for _ in range(50)]
            + ["```", "</details>", ""]
        )
        body = "\n".join(section for _ in range(60))
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        chunks = drift.split_issue_body(body)
        self.assertGreater(len(chunks), 1)
        for chunk in chunks:
            self.assertLessEqual(len(chunk), drift.MAX_ISSUE_BODY_CHARS)
            self.assertEqual(chunk.count("```") % 2, 0)
            self.assertEqual(chunk.count("<details>"), chunk.count("</details>"))
        for chunk in chunks[:-1]:
            self.assertTrue(chunk.endswith(drift.CONTINUATION_NOTICE))
        for chunk in chunks[1:]:
            self.assertTrue(chunk.startswith(drift.CONTINUATION_HEADER))
        # No content is dropped: every original line survives in some chunk.
        rejoined = "\n".join(chunks)
        for line in body.splitlines():
            self.assertIn(line, rejoined)

    def test_split_issue_body_recloses_oversized_single_block(self):
        lines = ["<details>", "```json"]
        lines += ['{"filler": %d}' % i for i in range(10_000)]
        lines += ["```", "</details>"]
        body = "\n".join(lines)
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        chunks = drift.split_issue_body(body)
        self.assertGreater(len(chunks), 1)
        for chunk in chunks:
            self.assertLessEqual(len(chunk), drift.MAX_ISSUE_BODY_CHARS)
        # The first cut lands inside the fenced block; both must be re-closed.
        first = chunks[0][: -len(drift.CONTINUATION_NOTICE)]
        self.assertTrue(first.endswith("```\n</details>"))
        rejoined = "\n".join(chunks)
        for line in lines:
            self.assertIn(line, rejoined)

    def test_split_issue_body_truncates_oversized_single_line(self):
        # A single line longer than the whole budget cannot break at a line
        # boundary, so it must be hard-truncated to keep every chunk in bounds.
        huge = "x" * (drift.MAX_ISSUE_BODY_CHARS + 5_000)
        body = "\n".join(["intro line", huge, "tail line"])
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        chunks = drift.split_issue_body(body)
        self.assertGreater(len(chunks), 1)
        for chunk in chunks:
            self.assertLessEqual(len(chunk), drift.MAX_ISSUE_BODY_CHARS)
        self.assertIn("… [line truncated]", "\n".join(chunks))

    def test_split_issue_body_no_notice_when_final_line_is_oversized(self):
        # The forced truncation branch consumes the last line here; the final
        # chunk must not falsely advertise that the report continues.
        huge = "y" * (drift.MAX_ISSUE_BODY_CHARS + 5_000)
        body = "\n".join(["intro line", huge])
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        chunks = drift.split_issue_body(body)
        self.assertGreater(len(chunks), 1)
        for chunk in chunks:
            self.assertLessEqual(len(chunk), drift.MAX_ISSUE_BODY_CHARS)
        self.assertIn("… [line truncated]", chunks[-1])
        self.assertFalse(chunks[-1].endswith(drift.CONTINUATION_NOTICE))

    @mock.patch.object(drift.subprocess, "run")
    def test_create_issue_streams_body_and_overflows_into_comments(self, run):
        run.return_value = SimpleNamespace(
            returncode=0,
            stdout="https://github.com/ClickHouse/clickhousectl/issues/999\n",
            stderr="",
        )
        body = "\n".join("line %d" % i for i in range(20_000))
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)
        drift.create_issue("title", body)

        self.assertGreater(run.call_count, 1)
        create_args, create_kwargs = run.call_args_list[0]
        self.assertEqual(create_args[0][:3], ["gh", "issue", "create"])
        self.assertIn("--body-file", create_args[0])
        self.assertNotIn("--body", create_args[0])
        self.assertLessEqual(len(create_kwargs["input"]), drift.MAX_ISSUE_BODY_CHARS)

        seen = create_kwargs["input"]
        for comment_args, comment_kwargs in run.call_args_list[1:]:
            self.assertEqual(comment_args[0][:3], ["gh", "issue", "comment"])
            self.assertIn(
                "https://github.com/ClickHouse/clickhousectl/issues/999",
                comment_args[0],
            )
            self.assertLessEqual(len(comment_kwargs["input"]), drift.MAX_ISSUE_BODY_CHARS)
            seen += comment_kwargs["input"]
        # Nothing was dropped across the issue body and its comments.
        for line in body.splitlines():
            self.assertIn(line, seen)

    @mock.patch.object(drift.subprocess, "run")
    def test_create_issue_raises_and_flags_when_comment_fails_twice(self, run):
        def fake_run(cmd, *args, **kwargs):
            if cmd[:3] == ["gh", "issue", "create"]:
                return SimpleNamespace(
                    returncode=0,
                    stdout="https://github.com/ClickHouse/clickhousectl/issues/999\n",
                    stderr="",
                )
            # The fallback truncation comment must succeed (best effort).
            if "incomplete" in kwargs.get("input", ""):
                return SimpleNamespace(returncode=0, stdout="", stderr="")
            # Every real continuation chunk fails, including the retry.
            raise drift.subprocess.CalledProcessError(1, cmd)

        run.side_effect = fake_run
        body = "\n".join("line %d" % i for i in range(20_000))
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        with self.assertRaises(drift.subprocess.CalledProcessError):
            drift.create_issue("title", body)

        comment_inputs = [
            c.kwargs.get("input", "")
            for c in run.call_args_list
            if c.args[0][:3] == ["gh", "issue", "comment"]
        ]
        non_fallback = [i for i in comment_inputs if "incomplete" not in i]
        fallback = [i for i in comment_inputs if "incomplete" in i]
        # The failing chunk was attempted twice (initial + one retry)...
        self.assertEqual(len(non_fallback), 2)
        self.assertEqual(non_fallback[0], non_fallback[1])
        # ...then a best-effort truncation notice was posted before raising.
        self.assertEqual(len(fallback), 1)

    @mock.patch.object(drift.subprocess, "run")
    def test_create_issue_recovers_from_transient_comment_failure(self, run):
        failed_once = set()

        def fake_run(cmd, *args, **kwargs):
            if cmd[:3] == ["gh", "issue", "create"]:
                return SimpleNamespace(
                    returncode=0,
                    stdout="https://github.com/ClickHouse/clickhousectl/issues/999\n",
                    stderr="",
                )
            chunk = kwargs.get("input", "")
            # Fail each chunk's first attempt, then succeed on the retry.
            if chunk not in failed_once:
                failed_once.add(chunk)
                raise drift.subprocess.CalledProcessError(1, cmd)
            return SimpleNamespace(returncode=0, stdout="", stderr="")

        run.side_effect = fake_run
        body = "\n".join("line %d" % i for i in range(20_000))
        self.assertGreater(len(body), drift.MAX_ISSUE_BODY_CHARS)

        # Transient failures must not raise or leave a truncation notice.
        drift.create_issue("title", body)
        comment_inputs = [
            c.kwargs.get("input", "")
            for c in run.call_args_list
            if c.args[0][:3] == ["gh", "issue", "comment"]
        ]
        self.assertTrue(comment_inputs)
        self.assertFalse(any("incomplete" in i for i in comment_inputs))

    @mock.patch.object(drift.subprocess, "run")
    def test_analyzer_receives_the_rust_source_tree(self, run):
        run.return_value = SimpleNamespace(
            returncode=0,
            stdout=json.dumps({"schema_version": 3, "findings": []}),
            stderr="",
        )

        drift.run_analyzer({"paths": {}, "components": {"schemas": {}}})

        command = run.call_args.args[0]
        self.assertIn("--source-root", command)
        source_root_index = command.index("--source-root") + 1
        self.assertEqual(command[source_root_index], str(drift.RUST_SOURCE_ROOT))
        self.assertNotIn("--client", command)
        self.assertNotIn("--models", command)
        self.assertNotIn("--meta", command)

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
