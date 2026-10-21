#!/usr/bin/env python3
"""Launch a Cursor Cloud Agent to remediate an OpenAPI drift issue.

The nightly drift workflow runs this after `check-openapi-drift.py` opens a new
drift issue. The agent follows the repository's `openapi-drift-remediation`
skill and opens draft pull requests for review.

The Cursor API key is read only from the `CURSOR_API_KEY` environment variable.
It is never accepted as an argument, never printed, and is redacted from any
API response text that is echoed on failure.
"""

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import uuid

CURSOR_API_URL = "https://api.cursor.com/v1/agents"
API_KEY_ENV = "CURSOR_API_KEY"
REQUEST_TIMEOUT_SECONDS = 60
MAX_ATTEMPTS = 3
RETRY_DELAY_SECONDS = 5
MAX_ECHOED_BODY_CHARS = 500
ISSUE_URL_PATTERN = re.compile(
    r"^(?P<repo>https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)/issues/(?P<number>[0-9]+)$"
)
REF_PATTERN = re.compile(r"^[A-Za-z0-9._/-]+$")

PROMPT_TEMPLATE = """\
Remediate the ClickHouse Cloud OpenAPI drift reported in GitHub issue {issue_url}.

The nightly OpenAPI drift workflow started this run automatically. Nobody is
watching it, so do not stop to ask questions.

- Read the whole issue, including its "Drift report, continued" comments.
- Follow the `openapi-drift-remediation` skill
  (`.agents/skills/openapi-drift-remediation/SKILL.md`) and the root and
  `crates/clickhouse-cloud-api` `AGENTS.md` files. They define the API and CLI
  pull request split, the tests and the gates.
- Where the skill says to check the plan with the user, decide yourself. Put
  the plan, and any decision you were unsure about, at the top of the first API
  pull request description.
- Open every pull request as a draft that references the issue. Only the last
  API pull request may say it fixes the issue.
- If you cannot open GitHub issues yourself, list the issues the skill asks for
  under a "Follow-up issues" heading in the relevant pull request description.
- Do not apply labels (including `run-cloud-integration`), merge, or close
  anything.
"""


class LaunchError(RuntimeError):
    """The agent could not be launched."""


def parse_issue_url(issue_url: str) -> tuple[str, int]:
    match = ISSUE_URL_PATTERN.fullmatch(issue_url)
    if match is None:
        raise LaunchError(f"not a GitHub issue URL: {issue_url!r}")
    return match["repo"], int(match["number"])


def agent_id_for_issue(issue_url: str) -> str:
    """Derive a stable agent ID so each drift issue launches at most one agent."""
    return f"bc-{uuid.uuid5(uuid.NAMESPACE_URL, issue_url)}"


def parse_model_params(raw: str) -> list[dict]:
    try:
        params = json.loads(raw)
    except json.JSONDecodeError as error:
        raise LaunchError(f"model params must be a JSON array: {error}") from error
    if not isinstance(params, list) or not all(
        isinstance(item, dict)
        and set(item) == {"id", "value"}
        and all(isinstance(item[key], str) for key in item)
        for item in params
    ):
        raise LaunchError('model params must look like [{"id": "...", "value": "..."}]')
    return params


def build_request_body(
    issue_url: str,
    repo_url: str,
    starting_ref: str,
    model: str = "",
    model_params: str = "",
    environment: str = "",
) -> dict:
    issue_repo, issue_number = parse_issue_url(issue_url)
    if issue_repo != repo_url:
        raise LaunchError(f"issue {issue_url} does not belong to {repo_url}")
    if not REF_PATTERN.fullmatch(starting_ref):
        raise LaunchError(f"invalid starting ref: {starting_ref!r}")

    body = {
        "agentId": agent_id_for_issue(issue_url),
        "name": f"OpenAPI drift remediation #{issue_number}",
        "prompt": {"text": PROMPT_TEMPLATE.format(issue_url=issue_url)},
        "autoCreatePR": False,
    }
    if environment:
        body["env"] = {"type": "cloud", "name": environment}
    else:
        body["repos"] = [{"url": repo_url, "startingRef": starting_ref}]
    if model:
        body["model"] = {"id": model}
        if model_params:
            body["model"]["params"] = parse_model_params(model_params)
    elif model_params:
        raise LaunchError("model params require a model")
    return body


def redact(text: str, secret: str) -> str:
    return text.replace(secret, "[REDACTED]") if secret else text


def post_agent(body: dict, api_key: str) -> tuple[int, dict | None, str]:
    """POST the create-agent request; return (status, parsed JSON, raw text)."""
    request = urllib.request.Request(
        CURSOR_API_URL,
        data=json.dumps(body).encode(),
        method="POST",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "Accept": "application/json",
            "User-Agent": "clickhousectl-openapi-drift",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
            status, raw = response.status, response.read().decode(errors="replace")
    except urllib.error.HTTPError as error:
        status, raw = error.code, error.read().decode(errors="replace")
    try:
        parsed = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        parsed = None
    return status, parsed if isinstance(parsed, dict) else None, raw


def launch(body: dict, api_key: str, sleep=time.sleep) -> str:
    """Create the agent and return its dashboard URL."""
    agent_url = f"https://cursor.com/agents/{body['agentId']}"
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            status, parsed, raw = post_agent(body, api_key)
        except (urllib.error.URLError, TimeoutError) as error:
            status, parsed, raw = None, None, redact(str(error), api_key)

        if status is not None and 200 <= status < 300:
            return ((parsed or {}).get("agent") or {}).get("url") or agent_url
        if status == 409:
            # The deterministic agentId already exists: an earlier attempt or run
            # launched the agent for this issue.
            print(f"An agent already exists for this issue: {agent_url}", file=sys.stderr)
            return agent_url
        retryable = status is None or status == 429 or status >= 500
        detail = redact(raw, api_key)[:MAX_ECHOED_BODY_CHARS]
        label = f"HTTP {status}" if status is not None else "network error"
        if not retryable or attempt == MAX_ATTEMPTS:
            raise LaunchError(f"Cursor API request failed ({label}): {detail}")
        print(
            f"Cursor API request failed ({label}); retrying ({attempt}/{MAX_ATTEMPTS})...",
            file=sys.stderr,
        )
        sleep(RETRY_DELAY_SECONDS * attempt)
    raise AssertionError("unreachable")


def write_step_summary(text: str):
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if path:
        with open(path, "a", encoding="utf-8") as handle:
            handle.write(text + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--issue-url", required=True, help="Drift issue to remediate")
    parser.add_argument("--repo-url", required=True, help="https://github.com/OWNER/REPO")
    parser.add_argument("--starting-ref", default="main", help="Branch the agent starts from")
    parser.add_argument("--model", default="", help="Cursor model ID from GET /v1/models")
    parser.add_argument("--model-params", default="", help="JSON array of model params")
    parser.add_argument(
        "--environment", default="", help="Named Cursor cloud environment (replaces --repo-url as the target)"
    )
    parser.add_argument(
        "--require-key",
        action="store_true",
        help=f"Fail instead of skipping when {API_KEY_ENV} is not set",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Print the request body without calling the API"
    )
    args = parser.parse_args(argv)

    try:
        body = build_request_body(
            args.issue_url,
            args.repo_url,
            args.starting_ref,
            model=args.model,
            model_params=args.model_params,
            environment=args.environment,
        )
    except LaunchError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    if args.dry_run:
        print(json.dumps(body, indent=2))
        return 0

    api_key = os.environ.get(API_KEY_ENV, "").strip()
    if not api_key:
        message = f"{API_KEY_ENV} is not set; not launching a drift remediation agent."
        if args.require_key:
            print(f"ERROR: {message}", file=sys.stderr)
            return 1
        print(f"::warning::{message}")
        return 0

    try:
        agent_url = launch(body, api_key)
    except LaunchError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"Drift remediation agent: {agent_url}", file=sys.stderr)
    write_step_summary(f"Launched a Cursor drift remediation agent for {args.issue_url}: {agent_url}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
