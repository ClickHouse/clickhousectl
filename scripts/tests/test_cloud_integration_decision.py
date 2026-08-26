import importlib.util
import re
import sys
import unittest
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "cloud-integration-decision.py"
SPEC = importlib.util.spec_from_file_location("cloud_integration_decision", SCRIPT)
decision = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = decision
SPEC.loader.exec_module(decision)

REPO = "ClickHouse/clickhousectl"
HEAD_SHA = "a" * 40
NEW_HEAD_SHA = "b" * 40
BASE_SHA = "c" * 40


def pull_request(*, head_sha=HEAD_SHA, head_repo=REPO, base_sha=BASE_SHA):
    repo = None if head_repo is None else {"full_name": head_repo}
    return {
        "number": 410,
        "state": "open",
        "html_url": "https://github.example/pull/410",
        "head": {
            "sha": head_sha,
            "ref": "feature",
            "repo": repo,
        },
        "base": {"sha": base_sha},
    }


def workflow_run(*, head_sha=HEAD_SHA, conclusion="success"):
    return {
        "id": 1234,
        "html_url": "https://github.example/actions/runs/1234",
        "path": decision.SOURCE_WORKFLOW_PATH,
        "event": "pull_request",
        "status": "completed",
        "conclusion": conclusion,
        "head_sha": head_sha,
        "head_branch": "feature",
        "head_repository": {"full_name": REPO},
        "pull_requests": [
            {
                "number": 410,
                "head": {"sha": head_sha},
                "base": {"sha": BASE_SHA},
            }
        ],
    }


def plan_job(conclusion="success"):
    return {"name": decision.PLAN_JOB_NAME, "conclusion": conclusion}


def live_job(selected=(), conclusion="success"):
    selected = set(selected)
    return {
        "name": decision.LIVE_JOB_NAME,
        "conclusion": conclusion,
        "steps": [
            {
                "name": step,
                "conclusion": "success" if suite in selected else "skipped",
            }
            for step, suite in decision.SUITE_STEPS.items()
        ],
    }


def selection(*suites):
    return decision.classifier.Selection(tuple(suites))


def override_event(*, actor="maintainer", sha=HEAD_SHA, body=None):
    if body is None:
        body = f"{decision.OVERRIDE_COMMAND} {sha} covered by stack run 123"
    return {
        "repository": {"full_name": REPO},
        "issue": {"number": 410},
        "sender": {"login": actor},
        "comment": {
            "body": body,
            "created_at": "2026-08-26T12:00:00Z",
            "html_url": "https://github.example/comment/1",
            "user": {"login": actor},
        },
    }


class GitHubAPITests(unittest.TestCase):
    def setUp(self):
        self.api = decision.GitHubAPI("https://api.github.example", REPO, "token")
        self.result = decision.Decision(
            conclusion="success",
            title="Accepted",
            summary="Exact SHA accepted",
            details_url="https://github.example/run/1",
        )

    def test_find_decision_check_uses_external_id_app_and_latest_id(self):
        external_id = decision.decision_external_id(HEAD_SHA)
        response = {
            "check_runs": [
                {
                    "id": 99,
                    "external_id": "unrelated",
                    "app": {"slug": "github-actions"},
                },
                {
                    "id": 12,
                    "external_id": external_id,
                    "app": {"slug": "github-actions"},
                },
                {
                    "id": 15,
                    "external_id": external_id,
                    "app": {"slug": "github-actions"},
                },
                {
                    "id": 30,
                    "external_id": external_id,
                    "app": {"slug": "other-app"},
                },
            ]
        }
        with mock.patch.object(self.api, "request", return_value=response) as request:
            result = self.api.find_decision_check(HEAD_SHA)

        self.assertEqual(result["id"], 15)
        method, path = request.call_args.args
        self.assertEqual(method, "GET")
        self.assertIn(f"/commits/{HEAD_SHA}/check-runs?", path)
        self.assertIn("check_name=Cloud+integration+decision", path)

    def test_upsert_creates_once_then_updates_the_same_check(self):
        existing = {"id": 42}
        with (
            mock.patch.object(
                self.api, "find_decision_check", side_effect=(None, existing)
            ),
            mock.patch.object(
                self.api,
                "request",
                side_effect=({"id": 42}, {"id": 42}),
            ) as request,
            mock.patch.object(
                decision, "utc_now", return_value="2026-08-26T12:00:00Z"
            ),
        ):
            self.api.upsert_decision(HEAD_SHA, self.result)
            self.api.upsert_decision(HEAD_SHA, self.result)

        create_method, create_path, create_payload = request.call_args_list[0].args
        self.assertEqual(
            (create_method, create_path),
            ("POST", f"/repos/{REPO}/check-runs"),
        )
        self.assertEqual(create_payload["name"], decision.CHECK_NAME)
        self.assertEqual(create_payload["head_sha"], HEAD_SHA)
        self.assertEqual(
            create_payload["external_id"], decision.decision_external_id(HEAD_SHA)
        )

        update_method, update_path, update_payload = request.call_args_list[1].args
        self.assertEqual(
            (update_method, update_path),
            ("PATCH", f"/repos/{REPO}/check-runs/42"),
        )
        self.assertNotIn("name", update_payload)
        self.assertNotIn("head_sha", update_payload)

    def test_create_only_returns_existing_check_without_updating(self):
        existing = {"id": 42, "conclusion": "success"}
        with (
            mock.patch.object(
                self.api, "find_decision_check", return_value=existing
            ),
            mock.patch.object(self.api, "request") as request,
        ):
            result = self.api.upsert_decision(
                HEAD_SHA, self.result, create_only=True
            )

        self.assertIs(result, existing)
        request.assert_not_called()


class ControllerFlowTests(unittest.TestCase):
    def test_initialize_uses_create_only_for_the_exact_head(self):
        api = mock.Mock()
        api.upsert_decision.return_value = {"id": 42}

        with mock.patch("builtins.print"):
            decision.initialize(
                api, {"pull_request": pull_request(head_repo=None)}, REPO
            )

        sha, result = api.upsert_decision.call_args.args
        self.assertEqual(sha, HEAD_SHA)
        self.assertEqual(result.conclusion, "action_required")
        self.assertIn("fork PR", result.summary)
        self.assertTrue(api.upsert_decision.call_args.kwargs["create_only"])

    def test_reconcile_ignores_run_without_one_associated_pull(self):
        api = mock.Mock()
        run = workflow_run()
        run["pull_requests"] = []

        with mock.patch("builtins.print"):
            decision.reconcile(api, {"workflow_run": run}, REPO)

        api.get_pull.assert_not_called()
        api.upsert_decision.assert_not_called()

    def test_reconcile_ignores_deleted_fork_before_loading_jobs(self):
        api = mock.Mock()
        api.get_pull.return_value = pull_request(head_repo=None)

        with mock.patch("builtins.print"):
            decision.reconcile(api, {"workflow_run": workflow_run()}, REPO)

        api.list_run_jobs.assert_not_called()
        api.upsert_decision.assert_not_called()

    def test_reconcile_ignores_run_with_skipped_planner(self):
        api = mock.Mock()
        api.get_pull.return_value = pull_request()
        api.list_run_jobs.return_value = [plan_job("skipped")]

        with mock.patch("builtins.print"):
            decision.reconcile(api, {"workflow_run": workflow_run()}, REPO)

        api.content_blob_sha.assert_not_called()
        api.upsert_decision.assert_not_called()

    def test_reconcile_updates_decision_from_trusted_evidence(self):
        api = mock.Mock()
        api.get_pull.return_value = pull_request()
        api.list_run_jobs.return_value = [plan_job(), live_job(("service",))]
        api.content_blob_sha.side_effect = ("workflow-blob", "workflow-blob")
        api.list_pull_files.return_value = [
            {"status": "modified", "filename": "README.md"}
        ]
        api.upsert_decision.return_value = {"id": 42}

        with (
            mock.patch.object(
                decision,
                "selection_from_pull_files",
                return_value=selection("service"),
            ),
            mock.patch("builtins.print"),
        ):
            decision.reconcile(api, {"workflow_run": workflow_run()}, REPO)

        sha, result = api.upsert_decision.call_args.args
        self.assertEqual(sha, HEAD_SHA)
        self.assertEqual(result.conclusion, "success")
        self.assertEqual(
            api.content_blob_sha.call_args_list,
            [
                mock.call(decision.SOURCE_WORKFLOW_PATH, BASE_SHA),
                mock.call(decision.SOURCE_WORKFLOW_PATH, HEAD_SHA),
            ],
        )

    def test_record_override_updates_check_and_posts_audit_comment(self):
        api = mock.Mock()
        api.collaborator_permission.return_value = {"permission": "maintain"}
        api.get_pull.return_value = pull_request()
        api.upsert_decision.return_value = {
            "id": 42,
            "html_url": "https://github.example/check/42",
        }

        with mock.patch("builtins.print"):
            decision.record_override(api, override_event(), REPO)

        sha, result = api.upsert_decision.call_args.args
        self.assertEqual(sha, HEAD_SHA)
        self.assertEqual(result.conclusion, "success")
        pull_number, comment = api.post_comment.call_args.args
        self.assertEqual(pull_number, 410)
        self.assertIn("override recorded by `@maintainer`", comment)
        self.assertIn("https://github.example/check/42", comment)

    def test_record_override_posts_feedback_when_permission_is_insufficient(self):
        api = mock.Mock()
        api.collaborator_permission.return_value = {"permission": "write"}

        with mock.patch("builtins.print"):
            decision.record_override(api, override_event(actor="writer"), REPO)

        api.get_pull.assert_not_called()
        api.upsert_decision.assert_not_called()
        pull_number, comment = api.post_comment.call_args.args
        self.assertEqual(pull_number, 410)
        self.assertIn("was not recorded", comment)
        self.assertIn("`maintain` or `admin`", comment)
        self.assertIn("No decision check was changed", comment)

    def test_record_override_posts_feedback_for_stale_sha(self):
        api = mock.Mock()
        api.collaborator_permission.return_value = {"permission": "maintain"}
        api.get_pull.return_value = pull_request(head_sha=NEW_HEAD_SHA)

        with mock.patch("builtins.print"):
            decision.record_override(api, override_event(), REPO)

        api.upsert_decision.assert_not_called()
        pull_number, comment = api.post_comment.call_args.args
        self.assertEqual(pull_number, 410)
        self.assertIn("was not recorded", comment)
        self.assertIn("SHA is stale", comment)
        self.assertIn("No decision check was changed", comment)


class CloudIntegrationDecisionTests(unittest.TestCase):
    def evaluate(self, jobs, selected, **kwargs):
        return decision.evaluate_workflow_run(
            kwargs.get("run", workflow_run()),
            kwargs.get("pr", pull_request()),
            jobs,
            REPO,
            kwargs.get("source_unchanged", True),
            selected,
        )

    def test_initial_decision_is_exact_sha_and_action_required(self):
        result = decision.waiting_decision(pull_request(), REPO)
        self.assertEqual(result.conclusion, "action_required")
        self.assertIn(HEAD_SHA, result.summary)
        self.assertIn("run-cloud-integration", result.summary)
        self.assertIn(decision.OVERRIDE_COMMAND, result.summary)

    def test_successful_selected_live_suites_satisfy_decision(self):
        suites = ("service", "clickpipes")
        result = self.evaluate(
            [plan_job(), live_job(suites)], selection(*suites)
        )
        self.assertEqual(result.conclusion, "success")
        self.assertIn(HEAD_SHA, result.summary)
        self.assertIn("`service`", result.summary)
        self.assertIn("`clickpipes`", result.summary)

    def test_success_requires_exact_trusted_suite_selection(self):
        result = self.evaluate(
            [plan_job(), live_job(("service",))],
            selection("service", "postgres"),
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("did not match trusted selection", result.summary)

    def test_failed_live_job_keeps_decision_failing(self):
        result = self.evaluate(
            [plan_job(), live_job(("service",), conclusion="failure")],
            selection("service"),
            run=workflow_run(conclusion="failure"),
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("concluded 'failure'", result.summary)

    def test_no_suite_succeeds_only_when_secret_job_was_skipped(self):
        result = self.evaluate(
            [plan_job(), live_job((), conclusion="skipped")], selection()
        )
        self.assertEqual(result.conclusion, "success")
        self.assertIn("selected no live suites", result.summary)
        self.assertIn("was skipped", result.summary)

    def test_skipped_secret_job_fails_when_a_suite_was_selected(self):
        result = self.evaluate(
            [plan_job(), live_job((), conclusion="skipped")],
            selection("organization"),
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("was skipped although", result.summary)

    def test_unrelated_label_with_skipped_admission_is_ignored(self):
        result = self.evaluate(
            [plan_job("skipped"), live_job((), conclusion="skipped")],
            selection(),
        )
        self.assertIsNone(result)

    def test_stale_run_is_ignored_after_new_head(self):
        result = self.evaluate(
            [plan_job(), live_job(("service",))],
            selection("service"),
            pr=pull_request(head_sha=NEW_HEAD_SHA),
        )
        self.assertIsNone(result)

    def test_run_for_stale_base_revision_is_ignored(self):
        result = self.evaluate(
            [plan_job(), live_job(("service",))],
            selection("service"),
            pr=pull_request(base_sha="d" * 40),
        )
        self.assertIsNone(result)

    def test_fork_run_cannot_satisfy_decision(self):
        run = workflow_run()
        run["head_repository"] = {"full_name": "contributor/fork"}
        result = self.evaluate(
            [plan_job(), live_job(("service",))],
            selection("service"),
            run=run,
            pr=pull_request(head_repo="contributor/fork"),
        )
        self.assertIsNone(result)

    def test_non_pr_and_wrong_workflow_runs_are_ignored(self):
        for key, value in (
            ("event", "workflow_dispatch"),
            ("path", ".github/workflows/other.yml"),
        ):
            run = workflow_run()
            run[key] = value
            with self.subTest(key=key):
                self.assertIsNone(
                    self.evaluate(
                        [plan_job(), live_job(("service",))],
                        selection("service"),
                        run=run,
                    )
                )

    def test_changed_source_workflow_cannot_attest_itself(self):
        result = self.evaluate(
            [plan_job(), live_job(("service",))],
            selection("service"),
            source_unchanged=False,
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("changes the trusted Cloud workflow", result.summary)

    def test_classifier_handles_github_file_records_and_renames(self):
        result = decision.selection_from_pull_files(
            [
                {
                    "status": "renamed",
                    "previous_filename": (
                        "crates/clickhouse-cloud-api/src/models/activity.rs"
                    ),
                    "filename": (
                        "crates/clickhouse-cloud-api/src/models/services.rs"
                    ),
                },
                {"status": "modified", "filename": "README.md"},
            ]
        )
        self.assertEqual(
            result.suites, ("service", "organization", "clickpipes")
        )
        self.assertFalse(result.failed_closed)

    def test_unknown_file_status_fails_closed_to_all_suites(self):
        result = decision.selection_from_pull_files(
            [{"status": "mystery", "filename": "README.md"}]
        )
        self.assertEqual(result.suites, decision.classifier.SUITES)
        self.assertTrue(result.failed_closed)

    def test_override_requires_full_sha_and_reason(self):
        self.assertIsNone(decision.parse_override_command("ordinary comment"))
        for body in (
            decision.OVERRIDE_COMMAND,
            f"{decision.OVERRIDE_COMMAND} abc reason",
            f"{decision.OVERRIDE_COMMAND} {HEAD_SHA}",
            f"{decision.OVERRIDE_COMMAND} {HEAD_SHA}   ",
        ):
            with self.subTest(body=body):
                with self.assertRaises(decision.ControllerError):
                    decision.parse_override_command(body)
        self.assertEqual(
            decision.parse_override_command(
                f"{decision.OVERRIDE_COMMAND} {HEAD_SHA} covered by run 123"
            ),
            (HEAD_SHA, "covered by run 123"),
        )

    def test_only_maintain_and_admin_permissions_can_override(self):
        for permission in ("admin", "maintain"):
            self.assertTrue(decision.permission_allows_override(permission))
        for permission in ("write", "triage", "read", "none"):
            self.assertFalse(decision.permission_allows_override(permission))
        self.assertTrue(
            decision.permission_allows_override(
                {
                    "permission": "write",
                    "user": {"permissions": {"maintain": True}},
                }
            )
        )
        self.assertFalse(
            decision.permission_allows_override(
                {
                    "permission": "write",
                    "user": {"permissions": {"push": True}},
                }
            )
        )

    def test_override_is_bound_to_current_head_and_audited(self):
        event = {
            "repository": {"full_name": REPO},
            "issue": {"number": 410},
            "sender": {"login": "maintainer"},
            "comment": {
                "body": (
                    f"{decision.OVERRIDE_COMMAND} {HEAD_SHA} "
                    "covered by stack run 123"
                ),
                "created_at": "2026-08-26T12:00:00Z",
                "html_url": "https://github.example/comment/1",
                "user": {"login": "maintainer"},
            },
        }
        override = decision.validate_override(
            event, pull_request(), "maintain", REPO
        )
        result = decision.override_decision(override)
        self.assertEqual(result.conclusion, "success")
        self.assertIn("maintainer", result.summary)
        self.assertIn(HEAD_SHA, result.summary)
        self.assertIn("2026-08-26T12:00:00Z", result.summary)
        self.assertIn("covered by stack run 123", result.summary)

        with self.assertRaises(decision.ControllerError):
            decision.validate_override(
                event, pull_request(head_sha=NEW_HEAD_SHA), "maintain", REPO
            )

    def test_unauthorized_override_is_ignored(self):
        event = {
            "repository": {"full_name": REPO},
            "issue": {"number": 410},
            "sender": {"login": "writer"},
            "comment": {
                "body": f"{decision.OVERRIDE_COMMAND} {HEAD_SHA} reason",
                "created_at": "2026-08-26T12:00:00Z",
                "html_url": "https://github.example/comment/1",
                "user": {"login": "writer"},
            },
        }
        self.assertIsNone(
            decision.validate_override(event, pull_request(), "write", REPO)
        )


class CloudIntegrationDecisionWorkflowTests(unittest.TestCase):
    def test_controller_workflow_never_checks_out_pr_code(self):
        workflow_path = (
            Path(__file__).resolve().parents[2]
            / ".github"
            / "workflows"
            / "cloud-integration-decision.yml"
        )
        workflow = workflow_path.read_text(encoding="utf-8")
        self.assertIn("pull_request_target:", workflow)
        self.assertIn("ref: ${{ github.event.repository.default_branch }}", workflow)
        self.assertNotIn("github.event.pull_request.head", workflow)
        self.assertNotIn("github.event.pull_request.base.sha", workflow)
        self.assertEqual(workflow.count("persist-credentials: false"), 3)

        checkout_uses = re.findall(r"actions/checkout@([^\s]+)", workflow)
        self.assertEqual(len(checkout_uses), 3)
        self.assertTrue(
            all(re.fullmatch(r"[0-9a-f]{40}", pin) for pin in checkout_uses)
        )

    def test_controller_has_narrow_explicit_write_permissions(self):
        workflow_path = (
            Path(__file__).resolve().parents[2]
            / ".github"
            / "workflows"
            / "cloud-integration-decision.yml"
        )
        workflow = workflow_path.read_text(encoding="utf-8")
        self.assertIn("actions: read", workflow)
        self.assertIn("contents: read", workflow)
        self.assertIn("checks: write", workflow)
        self.assertIn("issues: write", workflow)
        self.assertIn("pull-requests: read", workflow)
        self.assertNotIn("contents: write", workflow)
        self.assertNotIn("statuses: write", workflow)


if __name__ == "__main__":
    unittest.main()
