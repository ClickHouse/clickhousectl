#!/usr/bin/env python3
"""Check the live ClickHouse Cloud OpenAPI spec with the shared Rust analyzer.

Python owns network access, human-readable GitHub issue rendering, and issue
orchestration. Rust source parsing and all comparison semantics live in the
private clickhouse-openapi-analyzer workspace crate.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from collections import Counter, defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
API_ROOT = REPO_ROOT / "crates" / "clickhouse-cloud-api"
RUST_SOURCE_ROOT = API_ROOT / "src"
SNAPSHOT_JSON = API_ROOT / "clickhouse_cloud_openapi.json"
LIVE_SPEC_URL = os.environ.get(
    "CLICKHOUSE_OPENAPI_SPEC_URL", "https://api.clickhouse.cloud/v1"
)
ISSUE_LABEL = "openapi-drift"
GENERATED_ISSUE_MARKER = "<!-- clickhousectl-openapi-drift -->"
REPORT_INTRO = (
    "The live ClickHouse Cloud OpenAPI spec has drifted from the Rust API library."
)
# GitHub rejects issue bodies and comments over 65,536 characters.
MAX_ISSUE_BODY_CHARS = 65536
CONTINUATION_NOTICE = "\n---\n\n**Report continues in the next comment.**\n"
CONTINUATION_HEADER = "**Drift report, continued from above.**\n\n---\n\n"
INCOMPLETE_REPORT_PREFIX = "**This drift report is incomplete.**"
CLEAN_ISSUE_COMMENT = (
    "The automated drift check now reports no actionable drift. Closing this issue "
    "because its report is no longer current."
)
GITHUB_ACTIONS_LOGINS = {"github-actions", "github-actions[bot]", "app/github-actions"}


def fetch_live_spec() -> dict | None:
    """Fetch the live spec, falling back to curl for system-CA compatibility."""
    try:
        request = urllib.request.Request(
            LIVE_SPEC_URL, headers={"Accept": "application/json"}
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read())
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError):
        pass

    try:
        result = subprocess.run(
            ["curl", "-sf", LIVE_SPEC_URL],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 0:
            return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        pass
    return None


def run_analyzer(spec: dict) -> dict:
    """Run the canonical Rust analyzer and return its serialized DriftReport."""
    with tempfile.TemporaryDirectory() as spec_dir:
        spec_path = Path(spec_dir) / "spec.json"
        spec_path.write_text(json.dumps(spec))
        command = [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "-p",
            "clickhouse-openapi-analyzer",
            "--bin",
            "openapi-drift-analyzer",
            "--",
            "--spec",
            str(spec_path),
            "--snapshot",
            str(SNAPSHOT_JSON),
            "--source-root",
            str(RUST_SOURCE_ROOT),
        ]
        result = subprocess.run(
            command,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
    if result.returncode != 0:
        detail = result.stderr.strip() or "analyzer exited without an error message"
        raise RuntimeError(f"OpenAPI analyzer failed: {detail}")
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("OpenAPI analyzer emitted invalid JSON") from error
    if report.get("schema_version") != 4:
        raise RuntimeError(
            f"Unsupported DriftReport schema version: {report.get('schema_version')!r}"
        )
    return report


def findings_by_kind(report: dict) -> dict[str, list[dict]]:
    grouped = defaultdict(list)
    for finding in report.get("findings", []):
        grouped[finding["kind"]].append(finding)
    return dict(grouped)


def resolve_json_pointer(document, pointer: str):
    """Resolve an RFC 6901 pointer for issue snippets."""
    value = document
    if not pointer:
        return value
    for raw_part in pointer.lstrip("/").split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def ensure_label_exists():
    subprocess.run(
        [
            "gh",
            "label",
            "create",
            ISSUE_LABEL,
            "--description",
            "Automated: live OpenAPI spec differs from the Rust library",
            "--color",
            "D93F0B",
            "--force",
        ],
        capture_output=True,
        text=True,
        check=True,
    )


def open_drift_issues() -> list[dict]:
    result = subprocess.run(
        [
            "gh",
            "issue",
            "list",
            "--label",
            ISSUE_LABEL,
            "--state",
            "open",
            "--json",
            "number,title,body,url,author",
            "--limit",
            "100",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    try:
        issues = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("GitHub returned invalid JSON while listing drift issues") from error
    if not isinstance(issues, list):
        raise RuntimeError("GitHub returned an invalid drift issue list")
    return issues


def issue_comments(issue_number: int) -> list[dict]:
    result = subprocess.run(
        [
            "gh",
            "api",
            "--paginate",
            "--slurp",
            f"repos/{{owner}}/{{repo}}/issues/{issue_number}/comments?per_page=100",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    try:
        pages = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("GitHub returned invalid JSON while listing issue comments") from error
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise RuntimeError("GitHub returned an invalid issue comment list")
    return [comment for page in pages for comment in page]


def is_bot_generated_comment(comment: dict) -> bool:
    user = comment.get("user") or {}
    body = comment.get("body") or ""
    return user.get("login") in GITHUB_ACTIONS_LOGINS and (
        body.startswith(CONTINUATION_HEADER) or body.startswith(INCOMPLETE_REPORT_PREFIX)
    )


def is_generated_drift_issue(issue: dict) -> bool:
    body = issue.get("body") or ""
    if body.startswith(GENERATED_ISSUE_MARKER):
        return True

    # Migrate the bot-authored issue created before generated issues had a marker.
    author = issue.get("author") or {}
    return author.get("login") in GITHUB_ACTIONS_LOGINS and body.startswith(REPORT_INTRO)


def edit_issue(issue_url: str, title: str, body: str):
    subprocess.run(
        ["gh", "issue", "edit", issue_url, "--title", title, "--body-file", "-"],
        input=body,
        text=True,
        check=True,
        capture_output=True,
    )


def edit_comment(comment_id: int, body: str):
    subprocess.run(
        [
            "gh",
            "api",
            "--method",
            "PATCH",
            f"repos/{{owner}}/{{repo}}/issues/comments/{comment_id}",
            "--input",
            "-",
        ],
        input=json.dumps({"body": body}),
        text=True,
        check=True,
        capture_output=True,
    )


def delete_comment(comment_id: int):
    subprocess.run(
        [
            "gh",
            "api",
            "--method",
            "DELETE",
            f"repos/{{owner}}/{{repo}}/issues/comments/{comment_id}",
        ],
        text=True,
        check=True,
        capture_output=True,
    )


def close_issue(issue_url: str):
    subprocess.run(
        [
            "gh",
            "issue",
            "close",
            issue_url,
            "--reason",
            "completed",
            "--comment",
            CLEAN_ISSUE_COMMENT,
        ],
        text=True,
        check=True,
        capture_output=True,
    )


def split_issue_body(body: str) -> list[str]:
    """Split an oversized body into GitHub-sized chunks; overflow goes to comments.

    Chunks break at line boundaries, preferring the last point that is outside
    any code fence or <details> block so no block straddles a chunk boundary.
    If a single block exceeds the budget, it is cut mid-block and re-closed to
    keep the chunk's markdown well-formed.
    """
    if len(body) <= MAX_ISSUE_BODY_CHARS:
        return [body]
    closers = "```\n</details>\n"
    lines = body.splitlines()
    chunks = []
    start = 0
    while start < len(lines):
        header = CONTINUATION_HEADER if chunks else ""
        budget = (
            MAX_ISSUE_BODY_CHARS - len(header) - len(CONTINUATION_NOTICE) - len(closers)
        )
        used = 0
        in_fence = False
        in_details = False
        clean_cut = None
        end = start
        while end < len(lines):
            line = lines[end]
            if used + len(line) + 1 > budget:
                break
            used += len(line) + 1
            end += 1
            stripped = line.strip()
            if stripped.startswith("```"):
                in_fence = not in_fence
            elif stripped == "<details>":
                in_details = True
            elif stripped == "</details>":
                in_details = False
            if not in_fence and not in_details:
                clean_cut = end
        if end == len(lines):
            chunks.append(header + "\n".join(lines[start:]))
            break
        if clean_cut is not None and clean_cut > start:
            kept = lines[start:clean_cut]
            start = clean_cut
        elif end > start:
            # A single block exceeds the budget; cut mid-block and re-close.
            kept = lines[start:end]
            if in_fence:
                kept.append("```")
            if in_details:
                kept.append("</details>")
            start = end
        else:
            # A single line alone exceeds the budget; hard-truncate it so the
            # emitted chunk is guaranteed to fit, leaving a visible marker.
            marker = "… [line truncated]"
            cut = max(budget - len(marker) - len(closers), 0)
            kept = [lines[start][:cut] + marker]
            if in_fence:
                kept.append("```")
            if in_details:
                kept.append("</details>")
            start += 1
        # Only advertise a continuation when more lines actually remain; the
        # forced branches above can consume the final line of the body.
        notice = CONTINUATION_NOTICE if start < len(lines) else ""
        chunks.append(header + "\n".join(kept) + notice)
    return chunks


def post_continuation_comment(issue_url: str, body: str):
    try:
        subprocess.run(
            ["gh", "issue", "comment", issue_url, "--body-file", "-"],
            input=body,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError:
        # Retry once; transient GitHub failures are common.
        try:
            subprocess.run(
                ["gh", "issue", "comment", issue_url, "--body-file", "-"],
                input=body,
                text=True,
                check=True,
            )
        except subprocess.CalledProcessError:
            # Never leave an issue that silently looks complete after a failed
            # continuation update. The next successful sync removes this notice.
            fallback = (
                f"{INCOMPLETE_REPORT_PREFIX} A continuation comment failed to post "
                "after a retry; check the workflow logs and re-run the drift check "
                "to regenerate the full report.\n"
            )
            try:
                subprocess.run(
                    ["gh", "issue", "comment", issue_url, "--body-file", "-"],
                    input=fallback,
                    text=True,
                    check=False,
                    capture_output=True,
                )
            except Exception:  # noqa: BLE001 - the fallback must never mask the failure
                pass
            print(
                f"ERROR: failed to post a continuation comment to {issue_url}; "
                "the drift report is incomplete.",
                file=sys.stderr,
            )
            raise


def create_issue(title: str, body: str):
    chunks = split_issue_body(body)
    # The body can exceed the kernel's per-argument size limit; feed it via stdin.
    result = subprocess.run(
        ["gh", "issue", "create", "--title", title, "--body-file", "-", "--label", ISSUE_LABEL],
        input=chunks[0],
        text=True,
        check=True,
        capture_output=True,
    )
    issue_url = result.stdout.strip().splitlines()[-1]
    print(issue_url, file=sys.stderr)
    for chunk in chunks[1:]:
        post_continuation_comment(issue_url, chunk)


def sync_drift_issue(title: str | None, body: str | None) -> str:
    """Synchronize the one open generated drift issue with the latest report."""
    existing = open_drift_issues()
    if len(existing) > 1:
        numbers = ", ".join(f"#{issue.get('number', 'unknown')}" for issue in existing)
        raise RuntimeError(
            f"Multiple open {ISSUE_LABEL} issues exist ({numbers}); refusing to choose one"
        )

    issue = existing[0] if existing else None
    if issue is not None and not is_generated_drift_issue(issue):
        raise RuntimeError(
            f"Open {ISSUE_LABEL} issue #{issue.get('number', 'unknown')} is not generated "
            "by this script; refusing to modify it"
        )

    if title is None or body is None:
        if title is not None or body is not None:
            raise ValueError("title and body must either both be set or both be None")
        if issue is None:
            print("No open drift issue to close.", file=sys.stderr)
            return "clean"
        print(f"Closing clean drift issue #{issue['number']}.", file=sys.stderr)
        close_issue(issue["url"])
        return "closed"

    generated_body = f"{GENERATED_ISSUE_MARKER}\n\n{body}"
    chunks = split_issue_body(generated_body)
    if issue is None:
        ensure_label_exists()
        print(f"Creating issue: {title}", file=sys.stderr)
        create_issue(title, generated_body)
        return "created"

    comments = issue_comments(issue["number"])
    managed = [comment for comment in comments if is_bot_generated_comment(comment)]
    for comment in managed:
        if not isinstance(comment.get("id"), int):
            raise RuntimeError("GitHub returned a generated issue comment without a numeric ID")

    continuations = [
        comment
        for comment in managed
        if (comment.get("body") or "").startswith(CONTINUATION_HEADER)
    ]
    incomplete_notices = [
        comment
        for comment in managed
        if (comment.get("body") or "").startswith(INCOMPLETE_REPORT_PREFIX)
    ]
    desired_continuations = chunks[1:]
    unchanged = (
        issue.get("title") == title
        and issue.get("body") == chunks[0]
        and [comment.get("body") for comment in continuations] == desired_continuations
        and not incomplete_notices
    )
    if unchanged:
        print(f"Open drift issue #{issue['number']} is already current.", file=sys.stderr)
        return "unchanged"

    print(f"Updating drift issue #{issue['number']}: {title}", file=sys.stderr)
    if issue.get("title") != title or issue.get("body") != chunks[0]:
        edit_issue(issue["url"], title, chunks[0])

    shared_count = min(len(continuations), len(desired_continuations))
    for index in range(shared_count):
        if continuations[index].get("body") != desired_continuations[index]:
            edit_comment(continuations[index]["id"], desired_continuations[index])
    for continuation in desired_continuations[shared_count:]:
        post_continuation_comment(issue["url"], continuation)
    for continuation in continuations[shared_count:]:
        delete_comment(continuation["id"])
    for notice in incomplete_notices:
        delete_comment(notice["id"])
    return "updated"


def build_issue_body(report: dict, live_spec: dict) -> str:
    grouped = findings_by_kind(report)
    counts = Counter(finding["kind"] for finding in report.get("findings", []))
    unsupported = report.get("unsupported_enum_constraints", [])
    acknowledged = [item for item in unsupported if item.get("acknowledged")]

    def total(*kinds):
        return sum(counts[kind] for kind in kinds)

    lines = [
        REPORT_INTRO,
        "The comparison was produced by the shared `syn`-based analyzer.",
        "",
        f"- **Live spec:** `{LIVE_SPEC_URL}`",
        "- **Client:** `crates/clickhouse-cloud-api/src/client.rs`",
        "- **Models:** `crates/clickhouse-cloud-api/src/models.rs`",
        "- **Analyzer:** `crates/clickhouse-openapi-analyzer`",
        "",
        "## Summary",
        "",
        "| Change | Count |",
        "|--------|-------|",
        f"| Missing client methods | {counts['missing_client_method']} |",
        f"| Extra client methods | {counts['extra_client_method']} |",
        f"| Missing model types | {counts['missing_model_type']} |",
        f"| Missing schema definitions | {counts['missing_schema_definition']} |",
        f"| Missing struct fields | {counts['missing_struct_field']} |",
        f"| Extra struct fields | {counts['extra_struct_field']} |",
        f"| Missing enum values | {counts['missing_enum_value']} |",
        f"| Extra enum values | {counts['extra_enum_value']} |",
        f"| Enum VALUES const mismatches | {counts['enum_values_mismatch']} |",
        f"| Additional properties mismatches | {counts['additional_properties_mismatch']} |",
        f"| Field optionality mismatches | {counts['field_optionality_mismatch']} |",
        f"| Beta status changes | {total('newly_beta_operation', 'graduated_beta_operation')} |",
        f"| Deprecated-field changes | {total('newly_deprecated_field', 'undeprecated_field', 'missing_deprecated_marker', 'stray_deprecated_marker')} |",
        f"| Stale snapshot changes | {total('snapshot_added_operation', 'snapshot_removed_operation', 'snapshot_added_schema', 'snapshot_removed_schema')} |",
        f"| Stale exemptions | {counts['stale_exemption']} |",
        f"| New unsupported enum constraints | {counts['unsupported_enum_constraint']} |",
        f"| Acknowledged unsupported enum constraints | {len(acknowledged)} |",
        "",
    ]

    if grouped.get("missing_client_method"):
        lines += ["## Missing Client Methods", ""]
        for finding in grouped["missing_client_method"]:
            details = finding.get("details", {})
            lines += [
                f"### `{details.get('method_name', 'unknown')}`",
                "",
                f"**{details.get('method', '')}** `{details.get('path', '')}`",
            ]
            if details.get("summary"):
                lines.append(f"> {details['summary']}")
            try:
                fragment = resolve_json_pointer(live_spec, finding["spec_pointer"])
            except (KeyError, IndexError, ValueError, TypeError):
                fragment = None
            if fragment is not None:
                lines += [
                    "",
                    "<details>",
                    "<summary>Operation spec JSON</summary>",
                    "",
                    "```json",
                    json.dumps(fragment, indent=2),
                    "```",
                    "</details>",
                ]
            lines.append("")

    simple_sections = [
        ("extra_client_method", "Extra Client Methods"),
        ("missing_schema_definition", "Missing Schema Definitions"),
        ("missing_struct_field", "Missing Struct Fields"),
        ("extra_struct_field", "Extra Struct Fields"),
        ("missing_enum_value", "Missing Enum Values"),
        ("extra_enum_value", "Extra Enum Values"),
        ("enum_values_mismatch", "Enum VALUES Const Mismatches"),
        ("field_optionality_mismatch", "Field Optionality Mismatches"),
        ("additional_properties_mismatch", "Additional Properties Mismatches"),
        ("newly_beta_operation", "Newly Beta Operations"),
        ("graduated_beta_operation", "Graduated Beta Operations"),
        ("newly_deprecated_field", "Newly Deprecated Fields"),
        ("undeprecated_field", "No Longer Deprecated Fields"),
        ("missing_deprecated_marker", "Missing Deprecated-Field Markers"),
        ("stray_deprecated_marker", "Stray Deprecated-Field Markers"),
        ("snapshot_added_operation", "New Operations Missing From Snapshot"),
        ("snapshot_removed_operation", "Removed Operations Still In Snapshot"),
        ("snapshot_added_schema", "New Schemas Missing From Snapshot"),
        ("snapshot_removed_schema", "Removed Schemas Still In Snapshot"),
        ("stale_exemption", "Stale Exemptions"),
        ("unsupported_enum_constraint", "Unsupported Enum Constraints"),
    ]
    for kind, title in simple_sections:
        findings = grouped.get(kind, [])
        if not findings:
            continue
        lines += [f"## {title}", ""]
        for finding in findings:
            location = finding.get("spec_pointer") or finding.get("rust_item") or "unknown"
            lines.append(f"- `{location}` — {finding['message']}")
        lines.append("")

    if grouped.get("missing_model_type"):
        lines += ["## Missing Model Types", ""]
        for finding in grouped["missing_model_type"]:
            details = finding.get("details", {})
            lines += [
                f"### `{details.get('rust_type', 'unknown')}` (spec: `{details.get('schema', 'unknown')}`)",
                "",
            ]
            try:
                fragment = resolve_json_pointer(live_spec, finding["spec_pointer"])
            except (KeyError, IndexError, ValueError, TypeError):
                fragment = None
            if fragment is not None:
                lines += [
                    "<details>",
                    "<summary>Schema JSON</summary>",
                    "",
                    "```json",
                    json.dumps(fragment, indent=2),
                    "```",
                    "</details>",
                    "",
                ]

    if acknowledged:
        lines += [
            "## Acknowledged Unsupported Enum Constraints",
            "",
            "These locations are inventoried but cannot yet be compared to a typed Rust value enum.",
            "They do not count as drift; new or stale locations do.",
            "",
        ]
        for item in acknowledged:
            rust_item = f" (`{item['rust_item']}`)" if item.get("rust_item") else ""
            lines.append(f"- `{item['spec_pointer']}`{rust_item} — {item['reason']}")
        lines.append("")

    lines += [
        "## Implementation Guide",
        "",
        "1. Replace `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json` with this same live document; do not hand-edit it.",
        "2. Follow each finding's `spec_pointer` and `rust_item` to update `client.rs`, `models.rs`, or `meta.rs`.",
        "3. Regenerate beta/deprecation metadata when applicable and add focused model/client tests.",
        "4. Edit `crates/clickhouse-openapi-analyzer/src/config.rs` only for a deliberate, documented divergence. New unsupported acknowledgements require a tracking issue.",
        "5. Run the analyzer and Cloud API tests, Clippy, Python renderer tests, and this dry run again; see `AGENTS.md` for the exact commands.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dry-run", action="store_true", help="Print the issue body without creating an issue"
    )
    args = parser.parse_args()

    print("Fetching live OpenAPI spec...", file=sys.stderr)
    live_spec = fetch_live_spec()
    if live_spec is None:
        print(f"WARNING: Could not reach {LIVE_SPEC_URL} — skipping drift check.", file=sys.stderr)
        return

    print("Running shared Rust drift analyzer...", file=sys.stderr)
    try:
        report = run_analyzer(live_spec)
    except RuntimeError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(1) from error

    total = len(report.get("findings", []))
    acknowledged = sum(
        1
        for item in report.get("unsupported_enum_constraints", [])
        if item.get("acknowledged")
    )
    print(f"Actionable drift: {total}", file=sys.stderr)
    print(f"Acknowledged unsupported enum constraints: {acknowledged}", file=sys.stderr)
    if total == 0:
        print("No actionable drift. Library fully covers the live spec.", file=sys.stderr)
        if args.dry_run:
            return
        title = None
        body = None
    else:
        title = (
            f"OpenAPI drift: {total} gap{'s' if total != 1 else ''} "
            "between live spec and library"
        )
        body = build_issue_body(report, live_spec)
        if args.dry_run:
            print(body)
            return

    try:
        sync_drift_issue(title, body)
    except (RuntimeError, subprocess.CalledProcessError) as error:
        detail = getattr(error, "stderr", None)
        message = detail.strip() if isinstance(detail, str) and detail.strip() else str(error)
        print(f"ERROR: Could not synchronize the OpenAPI drift issue: {message}", file=sys.stderr)
        raise SystemExit(1) from error
    print("Done.", file=sys.stderr)


if __name__ == "__main__":
    main()
