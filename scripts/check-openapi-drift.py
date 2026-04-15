#!/usr/bin/env python3
"""
Check the live ClickHouse Cloud OpenAPI spec against the Rust library code
(client.rs and models.rs). Creates a GitHub issue when the live API has
operations or schemas that the library doesn't cover yet.

This mirrors the logic in spec_coverage_test.rs but runs against the live
spec and produces actionable GitHub issues instead of test failures.

Usage:
    python scripts/check-openapi-drift.py              # check and create issue
    python scripts/check-openapi-drift.py --dry-run     # check and print report only

Requires: gh CLI (authenticated), Python 3.8+
"""

import argparse
import importlib.util
import json
import os
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CLIENT_RS = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "src" / "client.rs"
MODELS_RS = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "src" / "models.rs"
LIVE_SPEC_URL = os.environ.get(
    "CLICKHOUSE_OPENAPI_SPEC_URL", "https://api.clickhouse.cloud/v1"
)
ISSUE_LABEL = "openapi-drift"
HTTP_METHODS = {"get", "put", "post", "delete", "patch", "options", "head", "trace"}

# Import field requirement resolution logic from sibling script
_spec = importlib.util.spec_from_file_location(
    "resolve_field_requirements",
    Path(__file__).resolve().parent / "resolve-field-requirements.py",
)
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)
resolve_required_fields = _mod.resolve_required_fields

# Regex to match serde rename value
_RE_RENAME = re.compile(r'rename\s*=\s*"([^"]+)"')
# Regex to match a field line (supports r#ident raw identifiers)
_RE_FIELD = re.compile(r'^\s*pub\s+(?:r#)?(\w+)\s*:\s*(.+?),\s*$')


def fetch_live_spec() -> dict | None:
    """Fetch the live OpenAPI spec. Returns None on network failure."""
    try:
        req = urllib.request.Request(LIVE_SPEC_URL, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.URLError:
        pass

    # Fallback: use curl (handles system CA certs better on some platforms)
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


# ---------------------------------------------------------------------------
# Rust source parsing (mirrors spec_coverage_test.rs logic)
# ---------------------------------------------------------------------------


def public_items(source: str, prefix: str) -> set[str]:
    """Extract identifier names from lines starting with `prefix` in Rust source.

    Mirrors the Rust test's public_items() + identifier_prefix() functions.
    """
    names = set()
    for line in source.splitlines():
        stripped = line.lstrip()
        if stripped.startswith(prefix):
            rest = stripped[len(prefix):]
            # Extract the identifier (alphanumeric + underscore)
            match = re.match(r"[A-Za-z0-9_]+", rest)
            if match:
                names.add(match.group(0))
    return names


def client_method_names() -> set[str]:
    """Extract all pub async fn names from client.rs."""
    source = CLIENT_RS.read_text()
    return public_items(source, "pub async fn ")


def model_type_names() -> set[str]:
    """Extract all pub struct/enum/type names from models.rs."""
    source = MODELS_RS.read_text()
    return (
        public_items(source, "pub struct ")
        | public_items(source, "pub enum ")
        | public_items(source, "pub type ")
    )


# ---------------------------------------------------------------------------
# Spec parsing (mirrors spec_coverage_test.rs logic)
# ---------------------------------------------------------------------------


def camel_to_snake(name: str) -> str:
    """Convert camelCase to snake_case, matching the Rust test convention."""
    result = []
    prev = None
    for ch in name:
        if ch.isupper():
            if prev and (prev.islower() or prev.isdigit()):
                result.append("_")
            result.append(ch.lower())
        else:
            result.append(ch)
        prev = ch
    return "".join(result)


def pascalize(name: str) -> str:
    """Convert to PascalCase, matching the Rust test convention."""
    result = []
    upper_next = True
    for ch in name:
        if ch.isalnum():
            result.append(ch.upper() if upper_next else ch)
            upper_next = False
        else:
            upper_next = True
    return "".join(result)


def spec_operations(spec: dict) -> dict[str, dict]:
    """Extract operations as {snake_case_id: {camel_id, method, path, summary, operation}}.

    Keys are snake_case to match client method names.
    """
    ops = {}
    for path, path_item in spec.get("paths", {}).items():
        for method, operation in path_item.items():
            if method in HTTP_METHODS and isinstance(operation, dict):
                camel_id = operation.get("operationId", f"{method}_{path}")
                snake_id = camel_to_snake(camel_id)
                ops[snake_id] = {
                    "camel_id": camel_id,
                    "method": method.upper(),
                    "path": path,
                    "summary": operation.get("summary", ""),
                    "operation": operation,
                }
    return ops


def spec_schema_names(spec: dict) -> dict[str, dict]:
    """Extract schemas as {PascalCase_name: {spec_name, schema}}.

    Keys are PascalCase to match Rust type names.
    """
    schemas = {}
    for spec_name, schema in spec.get("components", {}).get("schemas", {}).items():
        pascal_name = pascalize(spec_name)
        schemas[pascal_name] = {
            "spec_name": spec_name,
            "schema": schema,
        }
    return schemas


def collect_refs(value, refs=None) -> set[str]:
    """Recursively collect all $ref schema names from a JSON value."""
    if refs is None:
        refs = set()
    if isinstance(value, dict):
        ref = value.get("$ref", "")
        if ref.startswith("#/components/schemas/"):
            refs.add(ref.split("/")[-1])
        for v in value.values():
            collect_refs(v, refs)
    elif isinstance(value, list):
        for item in value:
            collect_refs(item, refs)
    return refs


# ---------------------------------------------------------------------------
# Field-level optionality checking
# ---------------------------------------------------------------------------


def parse_model_fields(source: str) -> dict[str, dict[str, bool]]:
    """Parse models.rs to extract field optionality per struct.

    Returns: { StructName: { specFieldName: is_option } }
    """
    result: dict[str, dict[str, bool]] = {}
    lines = source.splitlines()
    i = 0

    while i < len(lines):
        line = lines[i].lstrip()

        if line.startswith("pub struct "):
            rest = line[len("pub struct "):]
            match = re.match(r"[A-Za-z0-9_]+", rest)
            if match:
                struct_name = match.group(0)
                i += 1
                fields: dict[str, bool] = {}
                pending_rename: str | None = None

                while i < len(lines):
                    line = lines[i].strip()
                    if line == "}":
                        break

                    # Extract rename from serde attribute
                    if line.startswith("#[serde("):
                        m = _RE_RENAME.search(line)
                        if m:
                            pending_rename = m.group(1)

                    # Extract field definition
                    m = _RE_FIELD.match(lines[i])
                    if m:
                        rust_name = m.group(1)
                        type_str = m.group(2).strip()
                        is_option = type_str.startswith("Option<")
                        spec_name = pending_rename or rust_name
                        fields[spec_name] = is_option
                        pending_rename = None

                    i += 1

                result[struct_name] = fields
        i += 1

    return result


def check_field_optionality(
    spec: dict,
    model_fields: dict[str, dict[str, bool]],
) -> list[dict]:
    """Compare field optionality between spec and models.rs.

    Returns list of mismatches: [{schema, field, expected, actual}]
    """
    mismatches = []
    schemas = spec.get("components", {}).get("schemas", {})

    for spec_name, schema in schemas.items():
        pascal_name = pascalize(spec_name)
        fields = model_fields.get(pascal_name)
        if fields is None:
            continue

        props = schema.get("properties", {})
        if not props:
            continue

        required_fields, _method = resolve_required_fields(spec_name, schema)

        for prop_name in props:
            if prop_name not in fields:
                continue

            is_required = prop_name in required_fields
            is_option = fields[prop_name]

            if is_required and is_option:
                mismatches.append({
                    "schema": pascal_name,
                    "spec_name": spec_name,
                    "field": prop_name,
                    "expected": "required (T)",
                    "actual": "Option<T>",
                })
            elif not is_required and not is_option:
                mismatches.append({
                    "schema": pascal_name,
                    "spec_name": spec_name,
                    "field": prop_name,
                    "expected": "optional (Option<T>)",
                    "actual": "T",
                })

    return mismatches


# ---------------------------------------------------------------------------
# GitHub helpers
# ---------------------------------------------------------------------------


def ensure_label_exists():
    """Create the issue label if it doesn't exist yet."""
    subprocess.run(
        [
            "gh", "label", "create", ISSUE_LABEL,
            "--description", "Automated: live OpenAPI spec has operations/schemas not covered by the Rust library",
            "--color", "D93F0B",
            "--force",
        ],
        capture_output=True,
    )


def open_drift_issues() -> list[dict]:
    """Return open issues with the drift label."""
    result = subprocess.run(
        [
            "gh", "issue", "list",
            "--label", ISSUE_LABEL,
            "--state", "open",
            "--json", "number,title",
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return json.loads(result.stdout)


def create_issue(title: str, body: str):
    """Create a GitHub issue."""
    subprocess.run(
        ["gh", "issue", "create", "--title", title, "--body", body, "--label", ISSUE_LABEL],
        check=True,
    )


# ---------------------------------------------------------------------------
# Issue body generation
# ---------------------------------------------------------------------------


def build_issue_body(
    missing_ops: dict[str, dict],
    extra_ops: set[str],
    missing_types: dict[str, dict],
    all_spec_schemas: dict[str, dict],
    field_mismatches: list[dict] | None = None,
) -> str:
    lines = [
        "The live ClickHouse Cloud OpenAPI spec has operations or schemas that the",
        "Rust library (`client.rs` / `models.rs`) does not cover yet.",
        "",
        f"- **Live spec:** `{LIVE_SPEC_URL}`",
        f"- **Client:** `crates/clickhouse-cloud-api/src/client.rs`",
        f"- **Models:** `crates/clickhouse-cloud-api/src/models.rs`",
        "",
        "## Summary",
        "",
        "| Change | Count |",
        "|--------|-------|",
        f"| Missing client methods | {len(missing_ops)} |",
        f"| Extra client methods (not in spec) | {len(extra_ops)} |",
        f"| Missing model types | {len(missing_types)} |",
        f"| Field optionality mismatches | {len(field_mismatches or [])} |",
        "",
    ]

    # ---- Missing client methods ----
    if missing_ops:
        lines += [
            "## Missing Client Methods",
            "",
            "The live spec has these operations but `client.rs` has no matching `pub async fn`.",
            "",
        ]
        for snake_id in sorted(missing_ops):
            op = missing_ops[snake_id]
            lines.append(f"### `{snake_id}`")
            lines.append("")
            lines.append(f"**{op['method']}** `{op['path']}`")
            if op["summary"]:
                lines.append(f"> {op['summary']}")
            lines.append("")

            # Show which schemas this operation references
            refs = collect_refs(op["operation"])
            missing_refs = {
                pascalize(r) for r in refs
            } & set(missing_types)
            if missing_refs:
                lines.append(
                    "References missing types: "
                    + ", ".join(f"`{r}`" for r in sorted(missing_refs))
                )
                lines.append("")

            lines += [
                "<details>",
                "<summary>Operation spec JSON</summary>",
                "",
                "```json",
                json.dumps(op["operation"], indent=2),
                "```",
                "</details>",
                "",
            ]

    # ---- Extra client methods ----
    if extra_ops:
        lines += [
            "## Extra Client Methods",
            "",
            "These `pub async fn` methods exist in `client.rs` but have no matching",
            "operation in the live spec. They may have been removed from the API.",
            "",
        ]
        for name in sorted(extra_ops):
            lines.append(f"- `{name}`")
        lines.append("")

    # ---- Missing model types ----
    if missing_types:
        lines += [
            "## Missing Model Types",
            "",
            "The live spec defines these schemas but `models.rs` has no matching",
            "`pub struct`, `pub enum`, or `pub type`.",
            "",
        ]
        for pascal_name in sorted(missing_types):
            info = missing_types[pascal_name]
            lines += [
                f"### `{pascal_name}` (spec name: `{info['spec_name']}`)",
                "",
                "<details>",
                "<summary>Schema JSON</summary>",
                "",
                "```json",
                json.dumps(info["schema"], indent=2),
                "```",
                "</details>",
                "",
            ]

    # ---- Field optionality mismatches ----
    if field_mismatches:
        lines += [
            "## Field Optionality Mismatches",
            "",
            "These fields have incorrect `Option<T>` vs `T` types in `models.rs`.",
            "Required non-nullable fields should be `T`; optional or nullable fields",
            "should be `Option<T>`.",
            "",
            "| Schema | Field | Expected | Actual |",
            "|--------|-------|----------|--------|",
        ]
        for m in sorted(field_mismatches, key=lambda m: (m["schema"], m["field"])):
            lines.append(
                f"| `{m['schema']}` | `{m['field']}` | {m['expected']} | {m['actual']} |"
            )
        lines.append("")

    # ---- Implementation guide ----
    lines += [
        "## Implementation Guide",
        "",
        "1. Update the checked-in spec:",
        "   ```bash",
        f"   curl -s {LIVE_SPEC_URL} -o crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json",
        "   ```",
        "2. Add missing types to `crates/clickhouse-cloud-api/src/models.rs`",
        "   - Use `#[serde(rename_all = \"camelCase\")]` on structs",
        "   - Use `#[serde(skip_serializing_if = \"Option::is_none\")]` for optional fields",
        "   - Enums should derive `Default` and include an `#[serde(other)]` `Unknown` variant",
        "3. Add missing methods to `crates/clickhouse-cloud-api/src/client.rs`",
        "4. Fix field optionality:",
        "   ```bash",
        "   python scripts/update-models-optionality.py",
        "   ```",
        "5. Verify: `cargo test -p clickhouse-cloud-api`",
        "",
    ]

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the report but don't create a GitHub issue",
    )
    args = parser.parse_args()

    # Fetch live spec
    print("Fetching live OpenAPI spec...", file=sys.stderr)
    live_spec = fetch_live_spec()
    if live_spec is None:
        print(
            f"WARNING: Could not reach {LIVE_SPEC_URL} — skipping drift check.",
            file=sys.stderr,
        )
        return

    # Parse Rust source
    print("Parsing client.rs and models.rs...", file=sys.stderr)
    client_methods = client_method_names()
    model_types = model_type_names()

    # Compare operations: live spec vs client methods
    live_ops = spec_operations(live_spec)
    spec_method_names = set(live_ops.keys())

    missing_op_names = spec_method_names - client_methods
    extra_op_names = client_methods - spec_method_names

    missing_ops = {name: live_ops[name] for name in missing_op_names}

    # Compare schemas: live spec vs model types
    live_schemas = spec_schema_names(live_spec)
    spec_type_names = set(live_schemas.keys())

    missing_type_names = spec_type_names - model_types
    missing_types = {name: live_schemas[name] for name in missing_type_names}

    # Compare field optionality
    models_source = MODELS_RS.read_text()
    model_fields = parse_model_fields(models_source)
    field_mismatches = check_field_optionality(live_spec, model_fields)

    # Report
    total = len(missing_ops) + len(extra_op_names) + len(missing_types) + len(field_mismatches)

    print(f"Live spec:       {len(spec_method_names)} operations, {len(spec_type_names)} schemas", file=sys.stderr)
    print(f"client.rs:       {len(client_methods)} pub async fn methods", file=sys.stderr)
    print(f"models.rs:       {len(model_types)} pub types", file=sys.stderr)
    print(f"---", file=sys.stderr)
    print(f"Missing methods: {len(missing_ops)}", file=sys.stderr)
    print(f"Extra methods:   {len(extra_op_names)}", file=sys.stderr)
    print(f"Missing types:   {len(missing_types)}", file=sys.stderr)
    print(f"Field mismatches:{len(field_mismatches)}", file=sys.stderr)

    if total == 0:
        print("\nNo drift. Library fully covers the live spec.", file=sys.stderr)
        return

    body = build_issue_body(missing_ops, extra_op_names, missing_types, live_schemas, field_mismatches)

    if args.dry_run:
        print("\n--- Issue body (dry run) ---\n", file=sys.stderr)
        print(body)
        return

    # Check for existing open issue
    existing = open_drift_issues()
    if existing:
        nums = ", ".join(f"#{i['number']}" for i in existing)
        print(
            f"Open drift issue(s) already exist ({nums}). "
            "Skipping — close them to allow a fresh issue.",
            file=sys.stderr,
        )
        return

    title = f"OpenAPI drift: {total} gap{'s' if total != 1 else ''} between live spec and library"
    ensure_label_exists()
    print(f"\nCreating issue: {title}", file=sys.stderr)
    create_issue(title, body)
    print("Done.", file=sys.stderr)


if __name__ == "__main__":
    main()
