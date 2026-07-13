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
CLIENT_RS = API_ROOT / "src" / "client.rs"
MODELS_RS = API_ROOT / "src" / "models.rs"
META_RS = API_ROOT / "src" / "meta.rs"
SNAPSHOT_JSON = API_ROOT / "clickhouse_cloud_openapi.json"
LIVE_SPEC_URL = os.environ.get(
    "CLICKHOUSE_OPENAPI_SPEC_URL", "https://api.clickhouse.cloud/v1"
)
ISSUE_LABEL = "openapi-drift"


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
            "--client",
            str(CLIENT_RS),
            "--models",
            str(MODELS_RS),
            "--meta",
            str(META_RS),
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
    if report.get("schema_version") != 1:
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
            "number,title",
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return json.loads(result.stdout)


def create_issue(title: str, body: str):
    subprocess.run(
        ["gh", "issue", "create", "--title", title, "--body", body, "--label", ISSUE_LABEL],
        check=True,
    )


def build_issue_body(report: dict, live_spec: dict) -> str:
    grouped = findings_by_kind(report)
    counts = Counter(finding["kind"] for finding in report.get("findings", []))
    unsupported = report.get("unsupported_enum_constraints", [])
    acknowledged = [item for item in unsupported if item.get("acknowledged")]

    def total(*kinds):
        return sum(counts[kind] for kind in kinds)

    lines = [
        "The live ClickHouse Cloud OpenAPI spec has drifted from the Rust API library.",
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
        ("field_optionality_mismatch", "Field Optionality Mismatches"),
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
        return

    body = build_issue_body(report, live_spec)
    if args.dry_run:
        print(body)
        return

    existing = open_drift_issues()
    if existing:
        numbers = ", ".join(f"#{issue['number']}" for issue in existing)
        print(f"Open drift issue(s) already exist ({numbers}); skipping.", file=sys.stderr)
        return

    title = f"OpenAPI drift: {total} gap{'s' if total != 1 else ''} between live spec and library"
    ensure_label_exists()
    print(f"Creating issue: {title}", file=sys.stderr)
    create_issue(title, body)
    print("Done.", file=sys.stderr)


if __name__ == "__main__":
    main()
