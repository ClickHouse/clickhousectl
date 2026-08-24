import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "cloud-integration-decision.py"
REPO_ROOT = SCRIPT.parent.parent
SPEC = importlib.util.spec_from_file_location("cloud_integration_decision", SCRIPT)
decision = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = decision
SPEC.loader.exec_module(decision)

REPOSITORY = "ClickHouse/clickhousectl"
SHA = "1" * 40
NEW_SHA = "2" * 40
TIMESTAMP = "2026-08-24T12:00:00Z"


def pull(sha=SHA, *, head_repository=REPOSITORY, author="contributor", state="open"):
    return {
        "number": 410,
        "state": state,
        "html_url": "https://github.com/ClickHouse/clickhousectl/pull/500",
        "head": {"sha": sha, "repo": {"full_name": head_repository}},
        "user": {"login": author},
    }


def pull_event(action, sha=SHA, *, label=None, actor="maintainer"):
    event = {
        "action": action,
        "number": 410,
        "pull_request": pull(sha),
        "sender": {"login": actor},
    }
    if label is not None:
        event["label"] = {"name": label}
    return event


def workflow_event(
    *,
    sha=SHA,
    conclusion="success",
    source_event="pull_request",
    path=decision.LIVE_WORKFLOW_PATH,
    head_repository=REPOSITORY,
):
    return {
        "action": "completed",
        "workflow_run": {
            "id": 9001,
            "run_attempt": 1,
            "name": decision.LIVE_WORKFLOW_NAME,
            "path": path,
            "event": source_event,
            "status": "completed",
            "conclusion": conclusion,
            "head_sha": sha,
            "head_branch": "feature",
            "head_repository": {"full_name": head_repository},
            "pull_requests": [{"number": 410}],
            "html_url": "https://github.com/ClickHouse/clickhousectl/actions/runs/9001",
        },
    }


def jobs(plan="success", live="success"):
    return [
        {"name": decision.PLAN_JOB_NAME, "conclusion": plan},
        {"name": decision.LIVE_JOB_NAME, "conclusion": live},
    ]


class FakeClient:
    def __init__(
        self,
        *,
        current_pull=None,
        permission=None,
        comments=None,
        run_jobs=None,
        commit_pulls=None,
        head_pulls=None,
    ):
        self.pull = current_pull or pull()
        self.permission = permission or {"permission": "maintain"}
        self.comments = comments or []
        self.run_jobs = run_jobs if run_jobs is not None else jobs()
        self.commit_pulls = commit_pulls or []
        self.head_pulls = head_pulls or []
        self.permission_lookups = []
        self.job_lookups = []
        self.commit_pull_lookups = []
        self.head_pull_lookups = []

    def get_pull(self, number):
        self.pull_number = number
        return self.pull

    def get_permission(self, username):
        self.permission_lookups.append(username)
        return self.permission

    def list_comments(self, number):
        self.comment_pull_number = number
        return self.comments

    def list_run_jobs(self, run_id, attempt):
        self.job_lookups.append((run_id, attempt))
        return self.run_jobs

    def list_commit_pulls(self, sha):
        self.commit_pull_lookups.append(sha)
        return self.commit_pulls

    def list_head_pulls(self, branch):
        self.head_pull_lookups.append(branch)
        return self.head_pulls


class CloudIntegrationDecisionTests(unittest.TestCase):
    def decide(self, event_name, event, client=None):
        return decision.decide(
            event_name,
            event,
            client or FakeClient(),
            REPOSITORY,
            TIMESTAMP,
        )

    def test_new_and_updated_heads_are_pending_on_the_exact_sha(self):
        for action in ("opened", "reopened", "synchronize"):
            with self.subTest(action=action):
                result = self.decide(
                    "pull_request_target",
                    pull_event(action),
                )
                self.assertEqual(result.sha, SHA)
                self.assertEqual(result.status, "in_progress")
                self.assertIsNone(result.conclusion)
                self.assertIn(SHA, result.summary)
                self.assertIn("run-cloud-integration", result.summary)

    def test_stale_pull_event_does_not_touch_the_new_head(self):
        result = self.decide(
            "pull_request_target",
            pull_event("synchronize", SHA),
            FakeClient(current_pull=pull(NEW_SHA)),
        )
        self.assertIsNone(result)

    def test_unrelated_label_is_ignored(self):
        client = FakeClient()
        result = self.decide(
            "pull_request_target",
            pull_event("labeled", label="documentation"),
            client,
        )
        self.assertIsNone(result)
        self.assertEqual(client.permission_lookups, [])

    def test_maintainer_override_records_actor_sha_time_reason_and_comment(self):
        comment_url = "https://github.com/ClickHouse/clickhousectl/pull/500#issuecomment-1"
        client = FakeClient(
            comments=[
                {
                    "user": {"login": "maintainer"},
                    "body": f"{decision.OVERRIDE_COMMAND} {SHA} covered by run 42",
                    "html_url": comment_url,
                }
            ]
        )
        result = self.decide(
            "pull_request_target",
            pull_event("labeled", label=decision.OVERRIDE_LABEL),
            client,
        )
        self.assertEqual(result.sha, SHA)
        self.assertEqual(result.conclusion, "success")
        self.assertIn("@maintainer", result.summary)
        self.assertIn(TIMESTAMP, result.summary)
        self.assertIn("covered by run 42", result.summary)
        self.assertIn(comment_url, result.summary)

    def test_write_permission_cannot_override(self):
        client = FakeClient(permission={"permission": "write"})
        result = self.decide(
            "pull_request_target",
            pull_event("labeled", label=decision.OVERRIDE_LABEL),
            client,
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("only `maintain` or `admin`", result.summary)

    def test_legacy_write_role_with_explicit_maintain_permission_can_override(self):
        client = FakeClient(
            permission={
                "permission": "write",
                "user": {"permissions": {"admin": False, "maintain": True}},
            },
            comments=[
                {
                    "user": {"login": "maintainer"},
                    "body": f"{decision.OVERRIDE_COMMAND} {SHA} reviewed exception",
                    "html_url": "https://github.test/comment",
                }
            ],
        )
        result = self.decide(
            "pull_request_target",
            pull_event("labeled", label=decision.OVERRIDE_LABEL),
            client,
        )
        self.assertEqual(result.conclusion, "success")

    def test_override_requires_same_actor_exact_sha_and_reason(self):
        cases = {
            "different actor": {
                "user": {"login": "other-maintainer"},
                "body": f"{decision.OVERRIDE_COMMAND} {SHA} reason",
            },
            "old sha": {
                "user": {"login": "maintainer"},
                "body": f"{decision.OVERRIDE_COMMAND} {NEW_SHA} reason",
            },
            "missing reason": {
                "user": {"login": "maintainer"},
                "body": f"{decision.OVERRIDE_COMMAND} {SHA}",
            },
        }
        for name, comment in cases.items():
            with self.subTest(name=name):
                result = self.decide(
                    "pull_request_target",
                    pull_event("labeled", label=decision.OVERRIDE_LABEL),
                    FakeClient(comments=[comment]),
                )
                self.assertEqual(result.conclusion, "failure")
                self.assertIn("Remove the label", result.summary)

    def test_override_for_old_label_event_is_not_applied_to_new_head(self):
        client = FakeClient(current_pull=pull(NEW_SHA))
        result = self.decide(
            "pull_request_target",
            pull_event("labeled", SHA, label=decision.OVERRIDE_LABEL),
            client,
        )
        self.assertIsNone(result)

    def test_successful_live_run_passes_only_its_current_exact_sha(self):
        client = FakeClient(run_jobs=jobs("success", "success"))
        result = self.decide("workflow_run", workflow_event(), client)
        self.assertEqual(result.sha, SHA)
        self.assertEqual(result.conclusion, "success")
        self.assertIn("selected live suites passed", result.summary)
        self.assertEqual(client.job_lookups, [(9001, 1)])

    def test_empty_workflow_pull_list_uses_exact_head_commit_metadata(self):
        event = workflow_event()
        event["workflow_run"]["pull_requests"] = []
        client = FakeClient(commit_pulls=[{"number": 410}])
        result = self.decide("workflow_run", event, client)
        self.assertEqual(result.conclusion, "success")
        self.assertEqual(client.commit_pull_lookups, [SHA])

    def test_empty_commit_metadata_falls_back_to_open_head_branch(self):
        event = workflow_event()
        event["workflow_run"]["pull_requests"] = []
        client = FakeClient(head_pulls=[{"number": 410}])
        result = self.decide("workflow_run", event, client)
        self.assertEqual(result.conclusion, "success")
        self.assertEqual(client.head_pull_lookups, ["feature"])

    def test_commit_metadata_does_not_match_inherited_or_ambiguous_prs(self):
        event = workflow_event()
        event["workflow_run"]["pull_requests"] = []
        cases = [
            (pull(NEW_SHA), [{"number": 410}]),
            (pull(), [{"number": 410}, {"number": 411}]),
        ]
        for current, commit_pulls in cases:
            with self.subTest(current=current, commit_pulls=commit_pulls):
                client = FakeClient(
                    current_pull=current,
                    commit_pulls=commit_pulls,
                )
                self.assertIsNone(self.decide("workflow_run", event, client))

    def test_no_suite_plan_passes_without_live_job(self):
        result = self.decide(
            "workflow_run",
            workflow_event(),
            FakeClient(run_jobs=jobs("success", "skipped")),
        )
        self.assertEqual(result.conclusion, "success")
        self.assertIn("environment-bearing job was skipped", result.summary)

    def test_failed_live_run_fails_the_decision(self):
        result = self.decide(
            "workflow_run",
            workflow_event(conclusion="failure"),
            FakeClient(run_jobs=jobs("success", "failure")),
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("Workflow conclusion: `failure`", result.summary)

    def test_failed_planner_fails_the_decision(self):
        result = self.decide(
            "workflow_run",
            workflow_event(conclusion="failure"),
            FakeClient(run_jobs=jobs("failure", "skipped")),
        )
        self.assertEqual(result.conclusion, "failure")
        self.assertIn("Planner conclusion: `failure`", result.summary)

    def test_skipped_admission_from_unrelated_label_is_ignored(self):
        result = self.decide(
            "workflow_run",
            workflow_event(),
            FakeClient(run_jobs=jobs("skipped", "skipped")),
        )
        self.assertIsNone(result)

    def test_missing_or_duplicate_job_evidence_fails(self):
        cases = [
            [],
            [{"name": decision.PLAN_JOB_NAME, "conclusion": "success"}],
            jobs() + [{"name": decision.LIVE_JOB_NAME, "conclusion": "success"}],
        ]
        for run_jobs in cases:
            with self.subTest(run_jobs=run_jobs):
                result = self.decide(
                    "workflow_run",
                    workflow_event(),
                    FakeClient(run_jobs=run_jobs),
                )
                self.assertEqual(result.conclusion, "failure")
                self.assertIn("exactly one planner job", result.summary)

    def test_stale_live_run_cannot_pass_new_head(self):
        result = self.decide(
            "workflow_run",
            workflow_event(sha=SHA),
            FakeClient(current_pull=pull(NEW_SHA)),
        )
        self.assertIsNone(result)

    def test_fork_and_dependabot_live_runs_are_ignored(self):
        cases = [
            (
                workflow_event(head_repository="contributor/clickhousectl"),
                pull(head_repository="contributor/clickhousectl"),
            ),
            (workflow_event(), pull(author="dependabot[bot]")),
        ]
        for event, current in cases:
            with self.subTest(current=current):
                result = self.decide(
                    "workflow_run", event, FakeClient(current_pull=current)
                )
                self.assertIsNone(result)

    def test_manual_scheduled_and_wrong_workflow_runs_are_ignored(self):
        cases = [
            workflow_event(source_event="workflow_dispatch"),
            workflow_event(source_event="schedule"),
            workflow_event(path=".github/workflows/not-cloud.yml"),
        ]
        for event in cases:
            with self.subTest(event=event):
                self.assertIsNone(self.decide("workflow_run", event))

    def test_non_controller_event_is_ignored(self):
        self.assertIsNone(self.decide("push", {}, FakeClient()))

    def test_check_payload_uses_stable_name_and_exact_sha(self):
        client = decision.GitHubClient("https://api.github.test", REPOSITORY, "token")
        result = decision.completed(
            SHA,
            "success",
            "Accepted",
            "Exact SHA accepted",
            "https://github.test/run",
        )
        with mock.patch.object(
            client, "_request", return_value={"html_url": "https://github.test/check"}
        ) as request:
            client.create_check(
                result,
                external_id="cloud-integration-decision:1:1",
                timestamp=TIMESTAMP,
            )

        method, path, payload = request.call_args.args
        self.assertEqual((method, path), ("POST", f"/repos/{REPOSITORY}/check-runs"))
        self.assertEqual(payload["name"], decision.CHECK_NAME)
        self.assertEqual(payload["head_sha"], SHA)
        self.assertEqual(payload["status"], "completed")
        self.assertEqual(payload["conclusion"], "success")


class CloudIntegrationWorkflowSecurityTests(unittest.TestCase):
    def test_controller_checks_out_only_its_trusted_workflow_revision(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "cloud-integration-decision.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("checks: write", workflow)
        self.assertIn("ref: ${{ github.workflow_sha }}", workflow)
        self.assertIn("sparse-checkout: scripts/cloud-integration-decision.py", workflow)
        self.assertNotIn("pull_request.head.sha", workflow)

    def test_live_workflow_keeps_read_only_same_repository_admission(self):
        workflow = (
            REPO_ROOT / ".github" / "workflows" / "cloud-integration.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("pull_request:", workflow)
        self.assertIn("contents: read", workflow)
        self.assertNotIn("checks: write", workflow)
        self.assertIn("github.event.pull_request.head.repo.full_name == github.repository", workflow)
        self.assertIn("github.event.pull_request.user.login != 'dependabot[bot]'", workflow)


if __name__ == "__main__":
    unittest.main()
