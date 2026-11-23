#!/usr/bin/env python3
"""Create a diagnostic request or authenticate its check-run response."""

import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


class DiagnosticError(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise DiagnosticError("Duplicate JSON key")
        result[key] = value
    return result


def load_json(data):
    return json.loads(data, object_pairs_hook=unique_object)


def positive_int(value):
    if not re.fullmatch(r"[1-9][0-9]*", value):
        raise DiagnosticError("Invalid numeric context")
    return int(value)


def context():
    env = os.environ
    if env.get("GITHUB_EVENT_NAME") != "workflow_dispatch" or env.get("GITHUB_REF") != "refs/heads/main":
        raise DiagnosticError("Diagnostic must be dispatched on main")
    if env.get("GITHUB_REPOSITORY_ID") != "1150809931" or env.get("GITHUB_REPOSITORY_OWNER_ID") != "54801242":
        raise DiagnosticError("Unexpected repository")
    sha = env["GITHUB_SHA"]
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise DiagnosticError("Invalid commit")
    event = load_json(Path(env["GITHUB_EVENT_PATH"]).read_text())
    scenario = event.get("inputs", {}).get("scenario")
    if scenario not in ("valid", "mismatched-attempt"):
        raise DiagnosticError("Invalid scenario")
    return {
        "schema": 1,
        "repository_id": 1150809931,
        "source_sha": sha,
        "run_id": positive_int(env["GITHUB_RUN_ID"]),
        "run_attempt": positive_int(env["GITHUB_RUN_ATTEMPT"]),
    }, scenario


def create_request(base, scenario):
    request = dict(base)
    if scenario == "mismatched-attempt":
        request["run_attempt"] += 1
    directory = Path(os.environ["RUNNER_TEMP"]) / "handoff-request"
    directory.mkdir(exist_ok=False)
    (directory / "request.json").write_text(json.dumps(request, separators=(",", ":")))


def same_context(actual, expected):
    return (isinstance(actual, dict) and actual.keys() == expected.keys()
            and all(type(actual[key]) is type(value) and actual[key] == value
                    for key, value in expected.items()))


def verify(base, scenario):
    env = os.environ
    app_id = positive_int(env["HANDOFF_REPLY_APP_ID"])
    artifact_id = positive_int(env["ARTIFACT_ID"])
    digest = env["ARTIFACT_DIGEST"]
    if not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise DiagnosticError("Invalid artifact digest")
    name = f"handoff-diagnostic/{base['run_id']}/{base['run_attempt']}"
    repo = env["GITHUB_REPOSITORY"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        raise DiagnosticError("Invalid repository name")
    query = urllib.parse.urlencode({"check_name": name, "filter": "all", "per_page": 100})
    url = f"{env['GITHUB_API_URL']}/repos/{repo}/commits/{base['source_sha']}/check-runs?{query}"
    request = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {env['GH_TOKEN']}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    })
    opener = urllib.request.build_opener(NoRedirect())
    deadline = time.monotonic() + 1200
    while time.monotonic() < deadline:
        with opener.open(request, timeout=10) as response:
            if 'rel="next"' in response.headers.get("Link", ""):
                raise DiagnosticError("Too many check runs")
            raw = response.read(8 * 1024 * 1024 + 1)
            if len(raw) > 8 * 1024 * 1024:
                raise DiagnosticError("Check-run response exceeds size limit")
            data = load_json(raw)
        runs = data.get("check_runs")
        count = data.get("total_count")
        if type(count) is not int or count < 0 or count > 100 or not isinstance(runs, list) or len(runs) != count:
            raise DiagnosticError("Invalid or excessive check-run response")
        matches = [run for run in runs if isinstance(run, dict)
                   and isinstance(run.get("app"), dict)
                   and type(run["app"].get("id")) is int and run["app"]["id"] == app_id
                   and run.get("name") == name]
        if len(matches) > 1:
            raise DiagnosticError("Ambiguous diagnostic response")
        if matches:
            run = matches[0]
            if run.get("head_sha") != base["source_sha"] or run.get("external_id") != str(artifact_id):
                raise DiagnosticError("Diagnostic response context mismatch")
            if run.get("status") == "completed":
                verdict = "accepted" if scenario == "valid" else "rejected"
                expected = {**base, "artifact_id": artifact_id, "artifact_sha256": digest, "verdict": verdict}
                conclusion = "success" if verdict == "accepted" else "failure"
                output = run.get("output")
                if run.get("conclusion") != conclusion or not isinstance(output, dict) or not isinstance(output.get("summary"), str):
                    raise DiagnosticError("Invalid diagnostic result")
                if not same_context(load_json(output["summary"]), expected):
                    raise DiagnosticError("Diagnostic response context mismatch")
                print("Diagnostic response authenticated")
                return
            if run.get("status") not in ("queued", "in_progress", "waiting", "pending", "requested"):
                raise DiagnosticError("Invalid diagnostic status")
        time.sleep(min(15, max(0, deadline - time.monotonic())))
    raise DiagnosticError("Timed out waiting for diagnostic response")


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ("request", "verify"):
        raise DiagnosticError("Expected request or verify")
    base, scenario = context()
    if sys.argv[1] == "request":
        create_request(base, scenario)
    else:
        verify(base, scenario)


if __name__ == "__main__":
    try:
        main()
    except DiagnosticError as error:
        print(f"Handoff diagnostic failed: {error}", file=sys.stderr)
        sys.exit(1)
    except (KeyError, OSError, ValueError, RecursionError):
        # Do not print API responses, request headers, or untrusted check content.
        print("Handoff diagnostic failed", file=sys.stderr)
        sys.exit(1)
