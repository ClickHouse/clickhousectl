#!/usr/bin/env python3
"""Maintain the exact-SHA Cloud integration decision check."""

import argparse
import importlib.util
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CHECK_NAME = "Cloud integration decision"
SOURCE_WORKFLOW_PATH = ".github/workflows/cloud-integration.yml"
PLAN_JOB_NAME = "Plan Cloud integration tests"
LIVE_JOB_NAME = "Cloud integration tests"
OVERRIDE_COMMAND = "/cloud-integration-override"
OVERRIDE_RE = re.compile(
    rf"\A{re.escape(OVERRIDE_COMMAND)}\s+([0-9a-fA-F]{{40}})\s+(.+?)\s*\Z",
    re.DOTALL,
)
SUITE_STEPS = {
    "Run cloud integration suite": "service",
    "Run cloud Postgres integration suite": "postgres",
    "Run cloud Org integration suite": "organization",
    "Run ClickPipe Postgres CDC integration test": "clickpipes",
}

CLASSIFIER_PATH = Path(__file__).with_name("classify-cloud-integration.py")
CLASSIFIER_SPEC = importlib.util.spec_from_file_location(
    "cloud_integration_classifier_for_decision", CLASSIFIER_PATH
)
if CLASSIFIER_SPEC is None or CLASSIFIER_SPEC.loader is None:
    raise RuntimeError(f"cannot load classifier from {CLASSIFIER_PATH}")
classifier = importlib.util.module_from_spec(CLASSIFIER_SPEC)
sys.modules[CLASSIFIER_SPEC.name] = classifier
CLASSIFIER_SPEC.loader.exec_module(classifier)


class ControllerError(RuntimeError):
    """The controller cannot safely update a decision."""


class APIError(ControllerError):
    def __init__(self, status: int, message: str):
        super().__init__(f"GitHub API returned {status}: {message}")
        self.status = status


@dataclass(frozen=True)
class Decision:
    conclusion: str
    title: str
    summary: str
    details_url: str


@dataclass(frozen=True)
class Override:
    sha: str
    reason: str
    actor: str
    timestamp: str
    comment_url: str


class GitHubAPI:
    def __init__(self, base_url: str, repository: str, token: str):
        self.base_url = base_url.rstrip("/")
        self.repository = repository
        self.token = token

    def request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> Any:
        data = None
        if payload is not None:
            data = json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            f"{self.base_url}{path}",
            data=data,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
                "User-Agent": "clickhousectl-cloud-integration-decision",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                body = response.read()
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", errors="replace")
            raise APIError(error.code, detail) from error
        except urllib.error.URLError as error:
            raise ControllerError(f"GitHub API request failed: {error}") from error
        return json.loads(body) if body else None

    def get_pull(self, number: int) -> dict[str, Any]:
        return self.request("GET", f"/repos/{self.repository}/pulls/{number}")

    def list_pull_files(self, number: int) -> list[dict[str, Any]]:
        files = []
        for page in range(1, 31):
            batch = self.request(
                "GET",
                f"/repos/{self.repository}/pulls/{number}/files"
                f"?per_page=100&page={page}",
            )
            files.extend(batch)
            if len(batch) < 100:
                return files
        raise ControllerError("PR file list reached GitHub's 3,000-file limit")

    def list_run_jobs(self, run_id: int) -> list[dict[str, Any]]:
        jobs = []
        for page in range(1, 11):
            response = self.request(
                "GET",
                f"/repos/{self.repository}/actions/runs/{run_id}/jobs"
                f"?filter=latest&per_page=100&page={page}",
            )
            batch = response["jobs"]
            jobs.extend(batch)
            if len(batch) < 100:
                return jobs
        raise ControllerError("workflow job list exceeded 1,000 jobs")

    def content_blob_sha(self, path: str, ref: str) -> str | None:
        encoded_path = urllib.parse.quote(path, safe="/")
        query = urllib.parse.urlencode({"ref": ref})
        try:
            response = self.request(
                "GET",
                f"/repos/{self.repository}/contents/{encoded_path}?{query}",
            )
        except APIError as error:
            if error.status == 404:
                return None
            raise
        return response.get("sha") if isinstance(response, dict) else None

    def collaborator_permission(self, actor: str) -> dict[str, Any]:
        encoded_actor = urllib.parse.quote(actor, safe="")
        return self.request(
            "GET",
            f"/repos/{self.repository}/collaborators/{encoded_actor}/permission",
        )

    def post_comment(self, number: int, body: str) -> dict[str, Any]:
        return self.request(
            "POST",
            f"/repos/{self.repository}/issues/{number}/comments",
            {"body": body},
        )

    def find_decision_check(self, sha: str) -> dict[str, Any] | None:
        query = urllib.parse.urlencode(
            {"check_name": CHECK_NAME, "filter": "latest", "per_page": 100}
        )
        response = self.request(
            "GET",
            f"/repos/{self.repository}/commits/{sha}/check-runs?{query}",
        )
        external_id = decision_external_id(sha)
        matches = [
            check
            for check in response["check_runs"]
            if check.get("external_id") == external_id
            and check.get("app", {}).get("slug") == "github-actions"
        ]
        return max(matches, key=lambda check: check["id"]) if matches else None

    def upsert_decision(
        self, sha: str, decision: Decision, *, create_only: bool = False
    ) -> dict[str, Any]:
        existing = self.find_decision_check(sha)
        if existing is not None and create_only:
            return existing

        timestamp = utc_now()
        payload = {
            "status": "completed",
            "conclusion": decision.conclusion,
            "completed_at": timestamp,
            "details_url": decision.details_url,
            "output": {
                "title": decision.title,
                "summary": decision.summary,
            },
        }
        if existing is not None:
            return self.request(
                "PATCH",
                f"/repos/{self.repository}/check-runs/{existing['id']}",
                payload,
            )

        payload.update(
            {
                "name": CHECK_NAME,
                "head_sha": sha,
                "external_id": decision_external_id(sha),
                "started_at": timestamp,
            }
        )
        return self.request(
            "POST", f"/repos/{self.repository}/check-runs", payload
        )


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def decision_external_id(sha: str) -> str:
    return f"cloud-integration-decision:{sha}"


def load_event() -> dict[str, Any]:
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if not event_path:
        raise ControllerError("GITHUB_EVENT_PATH is not set")
    with open(event_path, encoding="utf-8") as event_file:
        return json.load(event_file)


def required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise ControllerError(f"{name} is not set")
    return value


def waiting_decision(pull_request: dict[str, Any], repository: str) -> Decision:
    sha = pull_request["head"]["sha"]
    head_repository = pull_request["head"].get("repo") or {}
    is_fork = head_repository.get("full_name") != repository
    fork_note = (
        " This is a fork PR, so use a trusted same-repository mirror or a "
        "maintainer override; Cloud secrets are not available to the fork."
        if is_fork
        else ""
    )
    summary = (
        f"Head `{sha}` has no Cloud integration decision. Apply the "
        "`run-cloud-integration` label to run the trusted planner and any "
        f"selected live suites.{fork_note}\n\n"
        "Maintainers may instead record a one-shot override by commenting:\n\n"
        f"`{OVERRIDE_COMMAND} {sha} <reason or successful stack-run URL>`\n\n"
        "A decision for another commit does not apply to this SHA."
    )
    return Decision(
        conclusion="action_required",
        title="Cloud integration decision required",
        summary=summary,
        details_url=pull_request["html_url"],
    )


def failed_decision(run: dict[str, Any], reason: str) -> Decision:
    sha = run["head_sha"]
    return Decision(
        conclusion="failure",
        title="Cloud integration decision failed",
        summary=(
            f"Cloud Integration run [{run['id']}]({run['html_url']}) did not "
            f"produce a trusted successful decision for exact head `{sha}`.\n\n"
            f"Reason: {reason}\n\n"
            "Fix or rerun Cloud Integration, or have a maintainer record a "
            "subsequent exact-SHA override."
        ),
        details_url=run["html_url"],
    )


def successful_live_decision(
    run: dict[str, Any], suites: tuple[str, ...]
) -> Decision:
    suite_text = ", ".join(f"`{suite}`" for suite in suites)
    return Decision(
        conclusion="success",
        title="Selected Cloud integration suites passed",
        summary=(
            f"Cloud Integration run [{run['id']}]({run['html_url']}) tested "
            f"exact head `{run['head_sha']}`.\n\n"
            f"Successful suites: {suite_text}."
        ),
        details_url=run["html_url"],
    )


def successful_no_suite_decision(run: dict[str, Any]) -> Decision:
    return Decision(
        conclusion="success",
        title="No live Cloud integration suites selected",
        summary=(
            f"The trusted planner in Cloud Integration run "
            f"[{run['id']}]({run['html_url']}) selected no live suites for "
            f"exact head `{run['head_sha']}`. The environment-bearing "
            f"`{LIVE_JOB_NAME}` job was skipped."
        ),
        details_url=run["html_url"],
    )


def parse_override_command(body: str) -> tuple[str, str] | None:
    if not body.lstrip().startswith(OVERRIDE_COMMAND):
        return None
    match = OVERRIDE_RE.fullmatch(body.strip())
    if match is None:
        raise ControllerError(
            f"override syntax is: {OVERRIDE_COMMAND} <full-head-sha> <reason>"
        )
    reason = match.group(2).strip()
    if not reason:
        raise ControllerError("override reason must not be empty")
    return match.group(1).lower(), reason


def permission_allows_override(permission: str | dict[str, Any]) -> bool:
    if isinstance(permission, str):
        return permission in {"admin", "maintain"}
    effective = permission.get("permission", "none")
    granular = permission.get("user", {}).get("permissions", {})
    return effective in {"admin", "maintain"} or bool(
        granular.get("admin") or granular.get("maintain")
    )


def validate_override(
    event: dict[str, Any],
    pull_request: dict[str, Any],
    permission: str | dict[str, Any],
    repository: str,
) -> Override | None:
    parsed = parse_override_command(event["comment"]["body"])
    if parsed is None:
        return None
    sha, reason = parsed
    actor = event["sender"]["login"]
    if event["comment"]["user"]["login"] != actor:
        raise ControllerError("comment author and event sender do not match")
    if not permission_allows_override(permission):
        return None
    if event["repository"]["full_name"] != repository:
        raise ControllerError("override event repository does not match")
    if pull_request["state"] != "open":
        raise ControllerError("override target PR is not open")
    if pull_request["number"] != event["issue"]["number"]:
        raise ControllerError("override event PR number does not match")
    if pull_request["head"]["sha"].lower() != sha:
        raise ControllerError(
            "override SHA is stale or is not the PR's current full head SHA"
        )
    return Override(
        sha=sha,
        reason=reason,
        actor=actor,
        timestamp=event["comment"]["created_at"],
        comment_url=event["comment"]["html_url"],
    )


def override_decision(override: Override) -> Decision:
    quoted_reason = "\n".join(
        f"> {line}" for line in override.reason.splitlines()
    )
    return Decision(
        conclusion="success",
        title="Maintainer Cloud integration override recorded",
        summary=(
            f"Actor: `@{override.actor}`  \n"
            f"Exact head: `{override.sha}`  \n"
            f"Recorded: `{override.timestamp}`  \n"
            f"Audit comment: {override.comment_url}\n\n"
            f"Reason:\n{quoted_reason}\n\n"
            "This one-shot override does not apply after the PR head changes."
        ),
        details_url=override.comment_url,
    )


def selection_from_pull_files(files: list[dict[str, Any]]) -> Any:
    records = []
    status_map = {
        "added": "A",
        "modified": "M",
        "changed": "M",
        "removed": "D",
    }
    for changed_file in files:
        status = changed_file.get("status")
        filename = changed_file.get("filename")
        if not isinstance(filename, str) or not filename:
            return classifier.Selection(
                classifier.SUITES,
                failed_closed=True,
                reason="GitHub returned a changed file without a filename",
            )
        if status in status_map:
            records.append((status_map[status], (filename,)))
        elif status in {"renamed", "copied"}:
            previous = changed_file.get("previous_filename")
            if not isinstance(previous, str) or not previous:
                return classifier.Selection(
                    classifier.SUITES,
                    failed_closed=True,
                    reason=f"GitHub omitted the previous filename for {filename}",
                )
            marker = "R100" if status == "renamed" else "C100"
            records.append((marker, (previous, filename)))
        else:
            return classifier.Selection(
                classifier.SUITES,
                failed_closed=True,
                reason=f"GitHub returned unsupported file status {status!r}",
            )
    return classifier.select_records(records)


def current_run_is_safe(
    run: dict[str, Any], pull_request: dict[str, Any], repository: str
) -> bool:
    source_prs = run.get("pull_requests") or []
    if len(source_prs) != 1:
        return False
    source_pr = source_prs[0]
    source_head = source_pr.get("head") or {}
    source_base = source_pr.get("base") or {}
    head_repository = run.get("head_repository") or {}
    pull_head = pull_request.get("head") or {}
    pull_head_repository = pull_head.get("repo") or {}
    pull_base = pull_request.get("base") or {}
    return all(
        (
            run.get("path") == SOURCE_WORKFLOW_PATH,
            run.get("event") == "pull_request",
            run.get("status") == "completed",
            head_repository.get("full_name") == repository,
            pull_request.get("state") == "open",
            source_pr.get("number") == pull_request.get("number"),
            source_head.get("sha") == run.get("head_sha"),
            pull_head.get("sha") == run.get("head_sha"),
            pull_head.get("ref") == run.get("head_branch"),
            pull_head_repository.get("full_name") == repository,
            source_base.get("sha") == pull_base.get("sha"),
        )
    )


def one_job(jobs: list[dict[str, Any]], name: str) -> dict[str, Any] | None:
    matches = [job for job in jobs if job.get("name") == name]
    return matches[0] if len(matches) == 1 else None


def successful_suite_steps(
    live_job: dict[str, Any],
) -> tuple[tuple[str, ...], str | None]:
    selected = set()
    steps = live_job.get("steps") or []
    for step_name, suite in SUITE_STEPS.items():
        matches = [step for step in steps if step.get("name") == step_name]
        if len(matches) != 1:
            return (), f"live job did not contain exactly one {step_name!r} step"
        conclusion = matches[0].get("conclusion")
        if conclusion == "success":
            selected.add(suite)
        elif conclusion != "skipped":
            return (), f"{step_name!r} concluded {conclusion!r}"
    return tuple(suite for suite in classifier.SUITES if suite in selected), None


def evaluate_workflow_run(
    run: dict[str, Any],
    pull_request: dict[str, Any],
    jobs: list[dict[str, Any]],
    repository: str,
    source_workflow_unchanged: bool,
    selection: Any | None,
) -> Decision | None:
    if not current_run_is_safe(run, pull_request, repository):
        return None

    plan_job = one_job(jobs, PLAN_JOB_NAME)
    if plan_job is None or plan_job.get("conclusion") == "skipped":
        return None
    if not source_workflow_unchanged:
        return failed_decision(run, "the PR changes the trusted Cloud workflow")
    if plan_job.get("conclusion") != "success":
        return failed_decision(
            run, f"planner job concluded {plan_job.get('conclusion')!r}"
        )
    if selection is None:
        return failed_decision(run, "trusted suite selection was unavailable")

    live_job = one_job(jobs, LIVE_JOB_NAME)
    if live_job is None:
        return failed_decision(run, "the environment-bearing job was missing")

    expected_suites = tuple(selection.suites)
    if live_job.get("conclusion") == "skipped":
        if expected_suites:
            return failed_decision(
                run,
                "the environment-bearing job was skipped although trusted "
                f"selection required {classifier.format_suites(expected_suites)}",
            )
        if run.get("conclusion") != "success":
            return failed_decision(
                run, f"workflow concluded {run.get('conclusion')!r}"
            )
        return successful_no_suite_decision(run)

    if live_job.get("conclusion") != "success":
        return failed_decision(
            run,
            f"environment-bearing job concluded {live_job.get('conclusion')!r}",
        )
    if run.get("conclusion") != "success":
        return failed_decision(
            run, f"workflow concluded {run.get('conclusion')!r}"
        )

    actual_suites, step_error = successful_suite_steps(live_job)
    if step_error is not None:
        return failed_decision(run, step_error)
    if actual_suites != expected_suites or not actual_suites:
        return failed_decision(
            run,
            "successful suite steps did not match trusted selection "
            f"(expected {classifier.format_suites(expected_suites)}, got "
            f"{classifier.format_suites(actual_suites)})",
        )
    return successful_live_decision(run, actual_suites)


def initialize(api: GitHubAPI, event: dict[str, Any], repository: str) -> None:
    pull_request = event["pull_request"]
    decision = waiting_decision(pull_request, repository)
    check = api.upsert_decision(
        pull_request["head"]["sha"], decision, create_only=True
    )
    print(f"Decision check {check['id']} initialized for {pull_request['head']['sha']}")


def reconcile(api: GitHubAPI, event: dict[str, Any], repository: str) -> None:
    run = event["workflow_run"]
    source_prs = run.get("pull_requests") or []
    if len(source_prs) != 1:
        print("Ignoring Cloud run without exactly one associated PR")
        return

    pull_request = api.get_pull(source_prs[0]["number"])
    if not current_run_is_safe(run, pull_request, repository):
        print("Ignoring fork, stale, non-PR, or otherwise unrelated Cloud run")
        return

    jobs = api.list_run_jobs(run["id"])
    plan_job = one_job(jobs, PLAN_JOB_NAME)
    if plan_job is None or plan_job.get("conclusion") == "skipped":
        print("Ignoring Cloud run whose admission/planner job did not run")
        return

    base_sha = pull_request["base"]["sha"]
    head_sha = pull_request["head"]["sha"]
    base_workflow = api.content_blob_sha(SOURCE_WORKFLOW_PATH, base_sha)
    head_workflow = api.content_blob_sha(SOURCE_WORKFLOW_PATH, head_sha)
    source_unchanged = base_workflow is not None and head_workflow == base_workflow

    selection = None
    if source_unchanged and plan_job.get("conclusion") == "success":
        try:
            selection = selection_from_pull_files(
                api.list_pull_files(pull_request["number"])
            )
        except ControllerError as error:
            selection = classifier.Selection(
                classifier.SUITES, failed_closed=True, reason=str(error)
            )

    decision = evaluate_workflow_run(
        run,
        pull_request,
        jobs,
        repository,
        source_unchanged,
        selection,
    )
    if decision is None:
        print("Cloud run did not change the current decision")
        return
    check = api.upsert_decision(head_sha, decision)
    print(
        f"Decision check {check['id']} updated to {decision.conclusion} for {head_sha}"
    )


def record_override(
    api: GitHubAPI, event: dict[str, Any], repository: str
) -> None:
    actor = event["sender"]["login"]
    pull_number = event["issue"]["number"]
    try:
        parsed = parse_override_command(event["comment"]["body"])
    except ControllerError as error:
        post_override_rejection(api, pull_number, actor, str(error))
        return
    if parsed is None:
        print("Ignoring unrelated PR comment")
        return

    permission = api.collaborator_permission(actor)
    if not permission_allows_override(permission):
        level = permission.get("permission", "none")
        post_override_rejection(
            api,
            pull_number,
            actor,
            "override commands require `maintain` or `admin` permission",
        )
        print(f"Ignoring override from @{actor} with {level!r} permission")
        return

    pull_request = api.get_pull(pull_number)
    try:
        override = validate_override(event, pull_request, permission, repository)
    except ControllerError as error:
        post_override_rejection(api, pull_number, actor, str(error))
        return
    if override is None:
        return
    decision = override_decision(override)
    check = api.upsert_decision(override.sha, decision)

    reason_quote = "\n".join(f"> {line}" for line in override.reason.splitlines())
    api.post_comment(
        pull_number,
        (
            f"Cloud integration override recorded by `@{override.actor}`.\n\n"
            f"- Exact head: `{override.sha}`\n"
            f"- Recorded: `{override.timestamp}`\n"
            f"- Decision check: {check['html_url']}\n\n"
            f"Reason:\n{reason_quote}\n\n"
            "This override is one-shot. A new head SHA requires a new decision."
        ),
    )
    print(f"Override recorded in decision check {check['id']} for {override.sha}")


def post_override_rejection(
    api: GitHubAPI, pull_number: int, actor: str, reason: str
) -> None:
    api.post_comment(
        pull_number,
        (
            f"Cloud integration override from `@{actor}` was not recorded.\n\n"
            f"Reason: {reason}.\n\n"
            "No decision check was changed."
        ),
    )
    print(f"Override from @{actor} was not recorded: {reason}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("initialize", "reconcile", "override"))
    args = parser.parse_args()

    repository = required_env("GITHUB_REPOSITORY")
    api = GitHubAPI(
        required_env("GITHUB_API_URL"), repository, required_env("GH_TOKEN")
    )
    event = load_event()

    if args.mode == "initialize":
        initialize(api, event, repository)
    elif args.mode == "reconcile":
        reconcile(api, event, repository)
    else:
        record_override(api, event, repository)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ControllerError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
