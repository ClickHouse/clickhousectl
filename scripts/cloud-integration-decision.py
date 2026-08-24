#!/usr/bin/env python3
"""Publish the exact-SHA Cloud integration decision check for a pull request."""

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
LIVE_WORKFLOW_NAME = "Cloud Integration"
LIVE_WORKFLOW_PATH = ".github/workflows/cloud-integration.yml"
PLAN_JOB_NAME = "Plan Cloud integration tests"
LIVE_JOB_NAME = "Cloud integration tests"
OVERRIDE_LABEL = "skip-cloud-integration"
OVERRIDE_COMMAND = "/skip-cloud-integration"
MAINTAINER_PERMISSIONS = frozenset({"admin", "maintain"})
SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
OVERRIDE_PATTERN = re.compile(
    rf"\A{re.escape(OVERRIDE_COMMAND)}\s+([0-9a-fA-F]{{40}})\s+(.+)\Z",
    re.DOTALL,
)


@dataclass(frozen=True)
class Decision:
    sha: str
    status: str
    title: str
    summary: str
    details_url: str
    conclusion: str | None = None


class GitHubApiError(RuntimeError):
    pass


class GitHubClient:
    def __init__(self, api_url: str, repository: str, token: str):
        self.api_url = api_url.rstrip("/")
        self.repository = repository
        self.token = token

    def _request(
        self, method: str, path: str, payload: dict[str, Any] | None = None
    ) -> Any:
        body = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(
            f"{self.api_url}{path}",
            data=body,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "X-GitHub-Api-Version": "2022-11-28",
                "Content-Type": "application/json",
                "User-Agent": "clickhousectl-cloud-integration-decision",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", errors="replace")
            raise GitHubApiError(
                f"GitHub API {method} {path} failed ({error.code}): {detail}"
            ) from error

    def get_pull(self, number: int) -> dict[str, Any]:
        return self._request("GET", f"/repos/{self.repository}/pulls/{number}")

    def get_permission(self, username: str) -> dict[str, Any]:
        quoted_username = urllib.parse.quote(username, safe="")
        return self._request(
            "GET",
            f"/repos/{self.repository}/collaborators/{quoted_username}/permission",
        )

    def list_comments(self, number: int) -> list[dict[str, Any]]:
        return self._paginate(
            f"/repos/{self.repository}/issues/{number}/comments", list
        )

    def list_commit_pulls(self, sha: str) -> list[dict[str, Any]]:
        return self._paginate(
            f"/repos/{self.repository}/commits/{sha}/pulls", list
        )

    def list_head_pulls(self, branch: str) -> list[dict[str, Any]]:
        owner = self.repository.split("/", maxsplit=1)[0]
        head = urllib.parse.urlencode({"state": "open", "head": f"{owner}:{branch}"})
        return self._paginate(f"/repos/{self.repository}/pulls?{head}", list)

    def list_run_jobs(self, run_id: int, attempt: int) -> list[dict[str, Any]]:
        path = (
            f"/repos/{self.repository}/actions/runs/{run_id}/attempts/{attempt}/jobs"
        )
        return self._paginate(path, lambda response: response["jobs"])

    def _paginate(self, path: str, items: Any) -> list[dict[str, Any]]:
        results = []
        page = 1
        separator = "&" if "?" in path else "?"
        while True:
            response = self._request(
                "GET", f"{path}{separator}per_page=100&page={page}"
            )
            current = items(response)
            results.extend(current)
            if len(current) < 100:
                return results
            page += 1

    def create_check(
        self, decision: Decision, *, external_id: str, timestamp: str
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "name": CHECK_NAME,
            "head_sha": decision.sha,
            "status": decision.status,
            "external_id": external_id,
            "details_url": decision.details_url,
            "output": {
                "title": decision.title,
                "summary": decision.summary,
            },
        }
        if decision.status == "completed":
            payload["conclusion"] = decision.conclusion
            payload["completed_at"] = timestamp
        else:
            payload["started_at"] = timestamp
        return self._request(
            "POST", f"/repos/{self.repository}/check-runs", payload
        )


def exact_sha(value: Any) -> str | None:
    if isinstance(value, str) and SHA_PATTERN.fullmatch(value.lower()):
        return value.lower()
    return None


def nested(data: dict[str, Any], *keys: str) -> Any:
    value: Any = data
    for key in keys:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def completed(
    sha: str,
    conclusion: str,
    title: str,
    summary: str,
    details_url: str,
) -> Decision:
    return Decision(
        sha=sha,
        status="completed",
        conclusion=conclusion,
        title=title,
        summary=summary,
        details_url=details_url,
    )


def pending_decision(pull: dict[str, Any]) -> Decision | None:
    sha = exact_sha(nested(pull, "head", "sha"))
    if sha is None:
        return None
    number = pull.get("number")
    summary = (
        f"No Cloud integration decision is recorded for `{sha}`.\n\n"
        f"Apply `{OVERRIDE_LABEL}` only after a maintainer comments:\n\n"
        f"`{OVERRIDE_COMMAND} {sha} <reason or top-stack run URL>`\n\n"
        "Otherwise, remove and reapply `run-cloud-integration` to test this exact "
        "head. A decision from an older head does not apply."
    )
    return Decision(
        sha=sha,
        status="in_progress",
        title="Cloud integration decision required",
        summary=summary,
        details_url=pull.get("html_url", f"https://github.com/pull/{number}"),
    )


def current_pull(
    client: GitHubClient, number: int, expected_sha: str
) -> dict[str, Any] | None:
    pull = client.get_pull(number)
    if pull.get("state") != "open":
        return None
    if exact_sha(nested(pull, "head", "sha")) != expected_sha:
        return None
    return pull


def current_run_pull(
    client: GitHubClient, run: dict[str, Any], run_sha: str
) -> dict[str, Any] | None:
    numbers = {
        pull_ref.get("number")
        for pull_ref in run.get("pull_requests", [])
        if isinstance(pull_ref, dict) and isinstance(pull_ref.get("number"), int)
    }
    if not numbers:
        numbers = {
            pull_ref.get("number")
            for pull_ref in client.list_commit_pulls(run_sha)
            if isinstance(pull_ref, dict)
            and isinstance(pull_ref.get("number"), int)
        }
    head_branch = run.get("head_branch")
    if not numbers and isinstance(head_branch, str) and head_branch:
        numbers = {
            pull_ref.get("number")
            for pull_ref in client.list_head_pulls(head_branch)
            if isinstance(pull_ref, dict)
            and isinstance(pull_ref.get("number"), int)
        }

    matches = []
    for number in numbers:
        pull = current_pull(client, number, run_sha)
        if pull is not None:
            matches.append(pull)
    return matches[0] if len(matches) == 1 else None


def parse_override(body: Any) -> tuple[str, str] | None:
    if not isinstance(body, str):
        return None
    match = OVERRIDE_PATTERN.fullmatch(body.strip())
    if match is None:
        return None
    sha = exact_sha(match.group(1))
    reason = match.group(2).strip()
    if sha is None or not reason:
        return None
    return sha, reason[:4000]


def find_override_reason(
    comments: list[dict[str, Any]], actor: str, sha: str
) -> tuple[str, str] | None:
    for comment in reversed(comments):
        if nested(comment, "user", "login") != actor:
            continue
        parsed = parse_override(comment.get("body"))
        if parsed is not None and parsed[0] == sha:
            return parsed[1], comment.get("html_url", "")
    return None


def quote_reason(reason: str) -> str:
    return "\n".join(f"> {line}" if line else ">" for line in reason.splitlines())


def maintainer_permission(permission: dict[str, Any]) -> bool:
    role = permission.get("permission")
    explicit = nested(permission, "user", "permissions")
    return role in MAINTAINER_PERMISSIONS or (
        isinstance(explicit, dict)
        and (explicit.get("admin") is True or explicit.get("maintain") is True)
    )


def decide_pull_request_target(
    event: dict[str, Any], client: GitHubClient, timestamp: str
) -> Decision | None:
    action = event.get("action")
    event_pull = event.get("pull_request")
    if not isinstance(event_pull, dict):
        return None
    sha = exact_sha(nested(event_pull, "head", "sha"))
    number = event.get("number")
    if sha is None or not isinstance(number, int):
        return None

    pull = current_pull(client, number, sha)
    if pull is None:
        return None
    if action in {"opened", "synchronize"}:
        return pending_decision(pull)
    if action != "labeled" or nested(event, "label", "name") != OVERRIDE_LABEL:
        return None

    actor = nested(event, "sender", "login")
    if not isinstance(actor, str):
        return completed(
            sha,
            "failure",
            "Cloud integration override rejected",
            "The label event did not identify an actor. Remove the override label.",
            pull.get("html_url", ""),
        )
    permission = client.get_permission(actor)
    if not maintainer_permission(permission):
        role = permission.get("permission")
        return completed(
            sha,
            "failure",
            "Cloud integration override rejected",
            f"`@{actor}` has `{role or 'unknown'}` repository permission; "
            "only `maintain` or `admin` may override. Remove the override label.",
            pull.get("html_url", ""),
        )

    override = find_override_reason(client.list_comments(number), actor, sha)
    if override is None:
        return completed(
            sha,
            "failure",
            "Cloud integration override rejected",
            f"Before applying `{OVERRIDE_LABEL}`, `@{actor}` must comment "
            f"`{OVERRIDE_COMMAND} {sha} <reason or top-stack run URL>`. Remove "
            "the label before trying again.",
            pull.get("html_url", ""),
        )

    reason, comment_url = override
    summary = (
        f"Maintainer override recorded for exact head `{sha}`.\n\n"
        f"Actor: `@{actor}`  \n"
        f"Recorded: `{timestamp}`  \n"
        f"Audit comment: {comment_url}\n\n"
        f"Reason:\n{quote_reason(reason)}\n\n"
        "This one-shot override does not apply to a later head SHA. Remove and "
        f"reapply `{OVERRIDE_LABEL}` with a new exact-SHA comment after any push."
    )
    return completed(
        sha,
        "success",
        "Cloud integration override recorded",
        summary,
        comment_url or pull.get("html_url", ""),
    )


def unique_job(
    jobs: list[dict[str, Any]], name: str
) -> dict[str, Any] | None:
    matches = [job for job in jobs if job.get("name") == name]
    return matches[0] if len(matches) == 1 else None


def run_failure(
    sha: str, run: dict[str, Any], title: str, detail: str
) -> Decision:
    run_url = run.get("html_url", "")
    summary = (
        f"Cloud Integration did not produce an acceptable decision for exact head "
        f"`{sha}`.\n\n{detail}\n\nRun: {run_url}\n\n"
        "Fix and rerun `run-cloud-integration`, or record a subsequent maintainer "
        "override for this exact SHA."
    )
    return completed(sha, "failure", title, summary, run_url)


def decide_workflow_run(
    event: dict[str, Any], client: GitHubClient, repository: str
) -> Decision | None:
    run = event.get("workflow_run")
    if not isinstance(run, dict):
        return None
    if (
        event.get("action") != "completed"
        or run.get("name") != LIVE_WORKFLOW_NAME
        or run.get("path") != LIVE_WORKFLOW_PATH
        or run.get("event") != "pull_request"
        or run.get("status") != "completed"
    ):
        return None

    run_sha = exact_sha(run.get("head_sha"))
    pull_refs = run.get("pull_requests")
    if run_sha is None or not isinstance(pull_refs, list):
        return None
    pull = current_run_pull(client, run, run_sha)
    if pull is None:
        return None
    if (
        nested(pull, "head", "repo", "full_name") != repository
        or nested(run, "head_repository", "full_name") != repository
        or nested(pull, "user", "login") == "dependabot[bot]"
    ):
        return None

    run_id = run.get("id")
    attempt = run.get("run_attempt")
    if not isinstance(run_id, int) or not isinstance(attempt, int):
        return run_failure(
            run_sha,
            run,
            "Cloud integration evidence is incomplete",
            "The completed run did not identify its run ID and attempt.",
        )
    jobs = client.list_run_jobs(run_id, attempt)
    plan = unique_job(jobs, PLAN_JOB_NAME)
    live = unique_job(jobs, LIVE_JOB_NAME)

    # An unrelated label, fork, or Dependabot event skips admission. It must not
    # replace the head's pending decision with a successful check.
    if plan is not None and plan.get("conclusion") == "skipped":
        return None
    if plan is None or live is None:
        return run_failure(
            run_sha,
            run,
            "Cloud integration evidence is incomplete",
            "The trusted controller could not find exactly one planner job and "
            "one live-test job in this workflow attempt.",
        )
    if plan.get("conclusion") != "success":
        return run_failure(
            run_sha,
            run,
            "Cloud integration planning failed",
            f"Planner conclusion: `{plan.get('conclusion') or 'unknown'}`.",
        )
    if run.get("conclusion") != "success":
        return run_failure(
            run_sha,
            run,
            "Cloud integration tests failed",
            f"Workflow conclusion: `{run.get('conclusion') or 'unknown'}`; live "
            f"job conclusion: `{live.get('conclusion') or 'unknown'}`.",
        )

    run_url = run.get("html_url", "")
    live_conclusion = live.get("conclusion")
    if live_conclusion == "success":
        return completed(
            run_sha,
            "success",
            "Cloud integration tests passed",
            f"The selected live suites passed for exact head `{run_sha}`.\n\n"
            f"Run: {run_url}",
            run_url,
        )
    if live_conclusion == "skipped":
        return completed(
            run_sha,
            "success",
            "No live Cloud suites selected",
            f"The planner completed for exact head `{run_sha}` and selected no "
            "relevant live suites. The environment-bearing job was skipped.\n\n"
            f"Run: {run_url}",
            run_url,
        )
    return run_failure(
        run_sha,
        run,
        "Cloud integration tests failed",
        f"Live job conclusion: `{live_conclusion or 'unknown'}`.",
    )


def decide(
    event_name: str,
    event: dict[str, Any],
    client: GitHubClient,
    repository: str,
    timestamp: str,
) -> Decision | None:
    if event_name == "pull_request_target":
        return decide_pull_request_target(event, client, timestamp)
    if event_name == "workflow_run":
        return decide_workflow_run(event, client, repository)
    return None


def required_environment(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"{name} is required")
    return value


def main() -> int:
    event_name = required_environment("GITHUB_EVENT_NAME")
    event_path = Path(required_environment("GITHUB_EVENT_PATH"))
    repository = required_environment("GITHUB_REPOSITORY")
    token = required_environment("GITHUB_TOKEN")
    api_url = required_environment("GITHUB_API_URL")
    run_id = required_environment("GITHUB_RUN_ID")
    run_attempt = required_environment("GITHUB_RUN_ATTEMPT")
    timestamp = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    with event_path.open(encoding="utf-8") as event_file:
        event = json.load(event_file)

    client = GitHubClient(api_url, repository, token)
    decision = decide(event_name, event, client, repository, timestamp)
    if decision is None:
        print("Event did not produce a Cloud integration decision")
        return 0
    response = client.create_check(
        decision,
        external_id=f"cloud-integration-decision:{run_id}:{run_attempt}",
        timestamp=timestamp,
    )
    print(
        f"Published {decision.status}/{decision.conclusion or 'pending'} "
        f"for {decision.sha}: {response.get('html_url', '')}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (GitHubApiError, OSError, RuntimeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
