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
META_RS = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "src" / "meta.rs"
SPEC_COVERAGE_TEST_RS = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "tests" / "spec_coverage_test.rs"
SNAPSHOT_JSON = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "clickhouse_cloud_openapi.json"
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


def parse_non_openapi_client_methods() -> set[str]:
    """Parse `NON_OPENAPI_CLIENT_METHODS` from spec_coverage_test.rs.

    The Rust test exempts these client methods from operation coverage because
    they intentionally back endpoints outside the control-plane OpenAPI spec
    (e.g. the `queries.clickhouse.cloud` Query API). Re-use the same list here
    so the drift report does not re-flag them.
    """
    if not SPEC_COVERAGE_TEST_RS.exists():
        return set()
    source = SPEC_COVERAGE_TEST_RS.read_text()
    match = re.search(
        r"const\s+NON_OPENAPI_CLIENT_METHODS\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'"([^"]+)"', match.group(1)))


def parse_beta_operations() -> set[str]:
    """Parse `BETA_OPERATIONS` from meta.rs.

    The list mirrors `x-badges` Beta markers in the OpenAPI spec and is the
    consumer-facing declaration of which client methods back beta endpoints.
    """
    if not META_RS.exists():
        return set()
    source = META_RS.read_text()
    match = re.search(
        r"pub\s+const\s+BETA_OPERATIONS\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'"([^"]+)"', match.group(1)))


def spec_beta_operations(spec: dict) -> set[str]:
    """Extract snake_case operationIds that are tagged Beta via `x-badges`."""
    beta = set()
    for path_item in spec.get("paths", {}).values():
        for method, op in path_item.items():
            if method not in HTTP_METHODS or not isinstance(op, dict):
                continue
            badges = op.get("x-badges") or []
            if any(b.get("name") == "Beta" for b in badges):
                beta.add(camel_to_snake(op["operationId"]))
    return beta


def parse_deprecated_fields() -> set[tuple[str, str]]:
    """Parse `DEPRECATED_FIELDS` from meta.rs.

    The list mirrors `deprecated: true` properties on every schema in the spec
    (both request- and response-side). Each field carries a
    `#[cfg(feature = "deprecated-fields")]` marker in models.rs so it is removed
    from the struct by default — consumers can neither read a deprecated
    response field nor send a deprecated request field.
    """
    if not META_RS.exists():
        return set()
    source = META_RS.read_text()
    match = re.search(
        r"pub\s+const\s+DEPRECATED_FIELDS\s*:\s*&\[\(&str,\s*&str\)\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', match.group(1)))


def spec_deprecated_fields(spec: dict) -> set[tuple[str, str]]:
    """Extract (PascalStructName, specFieldName) for `deprecated: true` props on
    every schema — both request-side and response-side."""
    fields = set()
    schemas = spec.get("components", {}).get("schemas", {})
    for spec_name, schema in schemas.items():
        props = schema.get("properties") or {}
        for prop_name, prop in props.items():
            if isinstance(prop, dict) and prop.get("deprecated") is True:
                fields.add((pascalize(spec_name), prop_name))
    return fields


def parse_optionality_exemptions() -> set[tuple[str, str]]:
    """Parse `OPTIONALITY_EXEMPTIONS` from spec_coverage_test.rs.

    The Rust test deliberately diverges from the spec for these (struct, field)
    pairs — the API behaves differently from what the spec declares (e.g.
    rejecting zero-value defaults the spec implies are required). Re-use the
    same list so the drift report stays in sync with the test.
    """
    if not SPEC_COVERAGE_TEST_RS.exists():
        return set()
    source = SPEC_COVERAGE_TEST_RS.read_text()
    match = re.search(
        r"const\s+OPTIONALITY_EXEMPTIONS\s*:\s*&\[\(&str,\s*&str\)\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', match.group(1)))


def parse_extra_field_exemptions() -> set[tuple[str, str]]:
    """Parse `EXTRA_FIELD_EXEMPTIONS` from spec_coverage_test.rs.

    Mirror of `parse_optionality_exemptions`: these (struct, field) pairs are
    fields we intentionally keep in `models.rs` even though the mapped spec
    schema has no such property (code-only/computed fields, or standard
    attributes the upstream spec omits). Re-use the same list so the drift
    report does not re-flag them and stays in sync with the test.
    """
    if not SPEC_COVERAGE_TEST_RS.exists():
        return set()
    source = SPEC_COVERAGE_TEST_RS.read_text()
    match = re.search(
        r"const\s+EXTRA_FIELD_EXEMPTIONS\s*:\s*&\[\(&str,\s*&str\)\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', match.group(1)))


def parse_extra_enum_value_exemptions() -> set[tuple[str, str]]:
    """Parse `EXTRA_ENUM_VALUE_EXEMPTIONS` from spec_coverage_test.rs.

    Mirror of `parse_extra_field_exemptions` for enum values: these
    (RustEnumName, wireValue) pairs are variants we intentionally keep in
    `models.rs` even though the mapped spec enum no longer lists the value.
    Re-use the same list so the drift report stays in sync with the test.
    """
    if not SPEC_COVERAGE_TEST_RS.exists():
        return set()
    source = SPEC_COVERAGE_TEST_RS.read_text()
    match = re.search(
        r"const\s+EXTRA_ENUM_VALUE_EXEMPTIONS\s*:\s*&\[\(&str,\s*&str\)\]\s*=\s*&\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if not match:
        return set()
    return set(re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', match.group(1)))


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


def parse_model_fields(source: str) -> dict[str, dict[str, dict]]:
    """Parse models.rs to extract field optionality and type per struct.

    Returns: { StructName: { specFieldName: {"is_option": bool, "type": str} } }
    """
    result: dict[str, dict[str, dict]] = {}
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
                fields: dict[str, dict] = {}
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
                        fields[spec_name] = {"is_option": is_option, "type": type_str}
                        pending_rename = None

                    i += 1

                result[struct_name] = fields
        i += 1

    return result


def check_field_optionality(
    spec: dict,
    model_fields: dict[str, dict[str, dict]],
    exemptions: set[tuple[str, str]] | None = None,
) -> list[dict]:
    """Compare field optionality between spec and models.rs.

    Returns list of mismatches: [{schema, field, expected, actual}]
    """
    exemptions = exemptions or set()
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

            if (pascal_name, prop_name) in exemptions:
                continue

            is_required = prop_name in required_fields
            is_option = fields[prop_name]["is_option"]

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


def check_snapshot_staleness(live_spec: dict) -> dict:
    """Compare the committed snapshot against the live spec.

    Returns a dict with 'added_ops', 'removed_ops', 'added_schemas',
    'removed_schemas' keys (all sets of names). Empty sets mean no drift.
    """
    result = {"added_ops": set(), "removed_ops": set(), "added_schemas": set(), "removed_schemas": set()}

    if not SNAPSHOT_JSON.exists():
        return result

    try:
        snapshot = json.loads(SNAPSHOT_JSON.read_text())
    except (json.JSONDecodeError, OSError):
        return result

    snap_ops = set(spec_operations(snapshot).keys())
    live_ops = set(spec_operations(live_spec).keys())
    result["added_ops"] = live_ops - snap_ops
    result["removed_ops"] = snap_ops - live_ops

    snap_schemas = set(spec_schema_names(snapshot).keys())
    live_schemas = set(spec_schema_names(live_spec).keys())
    result["added_schemas"] = live_schemas - snap_schemas
    result["removed_schemas"] = snap_schemas - live_schemas

    return result


def check_missing_fields(
    spec: dict,
    model_fields: dict[str, dict[str, dict]],
) -> list[dict]:
    """Find spec properties that have no corresponding Rust struct field.

    Returns list of: [{schema, spec_name, field}]
    """
    missing = []
    schemas = spec.get("components", {}).get("schemas", {})

    for spec_name, schema in schemas.items():
        pascal_name = pascalize(spec_name)
        fields = model_fields.get(pascal_name)
        if fields is None:
            continue

        props = schema.get("properties", {})
        for prop_name in props:
            if prop_name not in fields:
                missing.append({
                    "schema": pascal_name,
                    "spec_name": spec_name,
                    "field": prop_name,
                })

    return missing


def check_extra_fields(
    spec: dict,
    model_fields: dict[str, dict[str, dict]],
    exemptions: set[tuple[str, str]] | None = None,
) -> list[dict]:
    """Find Rust struct fields that have no corresponding spec property.

    The reverse of `check_missing_fields`: catches fields removed from (or never
    present in) the OpenAPI schema but still lingering in `models.rs`. Schemas
    with no/empty `properties` are skipped (composition/marker schemas carry
    their fields elsewhere), matching the Rust test.

    Returns list of: [{schema, spec_name, field}]
    """
    exemptions = exemptions or set()
    extra = []
    schemas = spec.get("components", {}).get("schemas", {})

    for spec_name, schema in schemas.items():
        pascal_name = pascalize(spec_name)
        fields = model_fields.get(pascal_name)
        if fields is None:
            continue

        props = schema.get("properties")
        if not props:
            continue

        for field_name in fields:
            if field_name in props:
                continue
            if (pascal_name, field_name) in exemptions:
                continue
            extra.append({
                "schema": pascal_name,
                "spec_name": spec_name,
                "field": field_name,
            })

    return extra


# ---------------------------------------------------------------------------
# Enum value checking (mirrors spec_coverage_test.rs logic)
# ---------------------------------------------------------------------------


def inner_type(type_str: str) -> str:
    """Strip `Option<`/`Vec<`/`Box<` wrappers down to the innermost type name."""
    type_str = type_str.strip()
    while True:
        for wrapper in ("Option<", "Vec<", "Box<"):
            if type_str.startswith(wrapper):
                type_str = type_str[len(wrapper):]
                if type_str.endswith(">"):
                    type_str = type_str[:-1]
                break
        else:
            return type_str


def parse_enum_variant_values(source: str) -> dict[str, set[str]]:
    """Parse models.rs to extract the wire values of every value enum: enums
    whose non-catch-all variants are all data-free. The wire value is the
    variant's `#[serde(rename = "...")]` if present, else the variant identifier
    itself. Variants marked `#[serde(untagged)]` (the `Unknown(String)`-style
    catch-all) carry no spec value and are skipped — identified by attribute,
    not by name. Enums with data-carrying variants that are not untagged model
    `oneOf` unions, not value enums, and are excluded entirely.

    Returns: { RustEnumName: { wireValue } }
    """
    result: dict[str, set[str]] = {}
    lines = source.splitlines()
    i = 0

    while i < len(lines):
        line = lines[i].lstrip()

        if line.startswith("pub enum "):
            rest = line[len("pub enum "):]
            match = re.match(r"[A-Za-z0-9_]+", rest)
            if match:
                enum_name = match.group(0)
                i += 1
                values: set[str] = set()
                is_value_enum = True
                pending_rename: str | None = None
                pending_untagged = False

                while i < len(lines):
                    line = lines[i].strip()
                    if line == "}":
                        break

                    if line.startswith("#[serde("):
                        m = _RE_RENAME.search(line)
                        if m:
                            pending_rename = m.group(1)
                        if "untagged" in line:
                            pending_untagged = True
                    elif line and not line.startswith("#[") and not line.startswith("//"):
                        m = re.match(r"[A-Za-z0-9_]+", line)
                        if m:
                            variant_name = m.group(0)
                            is_unit = not line[len(variant_name):].rstrip(",").strip()

                            if pending_untagged:
                                pass  # catch-all variant — carries no spec value
                            elif is_unit:
                                values.add(pending_rename or variant_name)
                            else:
                                is_value_enum = False
                            pending_rename = None
                            pending_untagged = False

                    i += 1

                if is_value_enum and values:
                    result[enum_name] = values
        i += 1

    return result


def spec_enum_values(spec: dict) -> dict[tuple[str, str | None], set[str]]:
    """Extract every string-valued `enum` in the spec, keyed by location: a
    named enum schema is `(schemaName, None)`, an inline property enum is
    `(schemaName, propertyName)`. Non-string values (numeric enums) are dropped
    and locations left with no string values are omitted. Enums nested inside
    `items`/`oneOf` are out of scope — models.rs represents those as plain
    collections/unions, not value enums.
    """
    out: dict[tuple[str, str | None], set[str]] = {}
    schemas = spec.get("components", {}).get("schemas", {})

    for spec_name, schema in schemas.items():
        values = _string_enum_values(schema)
        if values:
            out[(spec_name, None)] = values
        props = schema.get("properties") or {}
        for prop_name, prop in props.items():
            if not isinstance(prop, dict):
                continue
            values = _string_enum_values(prop)
            if values:
                out[(spec_name, prop_name)] = values

    return out


def _string_enum_values(node: dict) -> set[str]:
    """The string values of a schema/property's `enum` array, if it has any."""
    enum = node.get("enum")
    if not isinstance(enum, list):
        return set()
    return {v for v in enum if isinstance(v, str)}


def resolve_enum_for_location(
    spec_name: str,
    prop_name: str | None,
    model_fields: dict[str, dict[str, dict]],
    value_enums: dict[str, set[str]],
) -> tuple[str, str | None]:
    """Resolve the Rust type that serializes a spec enum location.

    Named enum schemas map by name (pascalize); inline property enums map via
    the struct field's declared type — the actual serialization path — so the
    mapping is structural and cannot rot the way a naming convention could.

    Returns ("enum", RustEnumName) | ("not_enum", type_name) |
    ("unmapped", None) — unmapped means the struct/field itself is missing,
    which the schema/field coverage checks already report.
    """
    if prop_name is None:
        type_name = pascalize(spec_name)
    else:
        fields = model_fields.get(pascalize(spec_name))
        if fields is None:
            return ("unmapped", None)
        info = fields.get(prop_name)
        if info is None:
            return ("unmapped", None)
        type_name = inner_type(info["type"])

    if type_name in value_enums:
        return ("enum", type_name)
    return ("not_enum", type_name)


def check_missing_enum_values(
    spec: dict,
    model_fields: dict[str, dict[str, dict]],
    value_enums: dict[str, set[str]],
) -> list[dict]:
    """Find spec enum values that no Rust enum variant can represent.

    Catches values added to a spec enum that never made it into models.rs
    (responses fall into the untagged catch-all, and requests can't express
    the value at all). Also flags spec enum locations whose Rust type is not a
    value enum. Returns list of: [{enum, location, value}] (value is None for
    the not-a-value-enum case).
    """
    missing = []

    for (spec_name, prop_name), spec_values in spec_enum_values(spec).items():
        location = f"{spec_name}.{prop_name}" if prop_name else spec_name
        kind, type_name = resolve_enum_for_location(
            spec_name, prop_name, model_fields, value_enums
        )
        if kind == "unmapped":
            continue
        if kind == "not_enum":
            missing.append({"enum": type_name, "location": location, "value": None})
            continue
        for value in spec_values - value_enums[type_name]:
            missing.append({"enum": type_name, "location": location, "value": value})

    return missing


def check_extra_enum_values(
    spec: dict,
    model_fields: dict[str, dict[str, dict]],
    value_enums: dict[str, set[str]],
    exemptions: set[tuple[str, str]] | None = None,
) -> list[dict]:
    """Find Rust enum variants that serialize a value absent from the spec enum.

    The reverse of `check_missing_enum_values`: catches values removed from a
    spec enum but left behind in `models.rs`, which the API rejects on requests.
    Returns list of: [{enum, location, value}]
    """
    exemptions = exemptions or set()
    extra = []

    for (spec_name, prop_name), spec_values in spec_enum_values(spec).items():
        location = f"{spec_name}.{prop_name}" if prop_name else spec_name
        kind, type_name = resolve_enum_for_location(
            spec_name, prop_name, model_fields, value_enums
        )
        if kind != "enum":
            continue  # unmapped/mistyped locations are missing-enum-value findings
        for value in value_enums[type_name] - spec_values:
            if (type_name, value) in exemptions:
                continue
            extra.append({"enum": type_name, "location": location, "value": value})

    return extra


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
    missing_fields: list[dict] | None = None,
    extra_fields: list[dict] | None = None,
    missing_enum_values: list[dict] | None = None,
    extra_enum_values: list[dict] | None = None,
    snapshot_staleness: dict | None = None,
    beta_status_changes: dict | None = None,
    deprecation_changes: dict | None = None,
) -> str:
    snap = snapshot_staleness or {}
    snap_total = sum(len(snap.get(k, set())) for k in ("added_ops", "removed_ops", "added_schemas", "removed_schemas"))

    beta = beta_status_changes or {}
    beta_total = len(beta.get("newly_beta", set())) + len(beta.get("graduated", set()))

    dep = deprecation_changes or {}
    dep_total = len(dep.get("newly_deprecated", set())) + len(dep.get("undeprecated", set()))

    lines = [
        "The live ClickHouse Cloud OpenAPI spec has operations or schemas that the",
        "Rust library (`client.rs` / `models.rs`) does not cover yet.",
        "",
        f"- **Live spec:** `{LIVE_SPEC_URL}`",
        f"- **Client:** `crates/clickhouse-cloud-api/src/client.rs`",
        f"- **Models:** `crates/clickhouse-cloud-api/src/models.rs`",
        f"- **Beta metadata:** `crates/clickhouse-cloud-api/src/meta.rs`",
        "",
        "## Summary",
        "",
        "| Change | Count |",
        "|--------|-------|",
        f"| Missing client methods | {len(missing_ops)} |",
        f"| Extra client methods (not in spec) | {len(extra_ops)} |",
        f"| Missing model types | {len(missing_types)} |",
        f"| Missing struct fields | {len(missing_fields or [])} |",
        f"| Extra struct fields (not in spec) | {len(extra_fields or [])} |",
        f"| Missing enum values | {len(missing_enum_values or [])} |",
        f"| Extra enum values (not in spec) | {len(extra_enum_values or [])} |",
        f"| Field optionality mismatches | {len(field_mismatches or [])} |",
        f"| Beta status changes | {beta_total} |",
        f"| Deprecated output field changes | {dep_total} |",
        f"| Stale snapshot changes | {snap_total} |",
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

    # ---- Missing struct fields ----
    if missing_fields:
        lines += [
            "## Missing Struct Fields",
            "",
            "These properties exist in the OpenAPI spec but have no corresponding",
            "field in the Rust struct. API response data for these fields is silently",
            "dropped during deserialization.",
            "",
            "| Schema | Field |",
            "|--------|-------|",
        ]
        for m in sorted(missing_fields, key=lambda m: (m["schema"], m["field"])):
            lines.append(f"| `{m['schema']}` | `{m['field']}` |")
        lines.append("")

    # ---- Extra struct fields ----
    if extra_fields:
        lines += [
            "## Extra Struct Fields",
            "",
            "These fields exist in a Rust struct but have no corresponding property",
            "in the mapped OpenAPI schema. The field was likely removed from the spec",
            "upstream while the struct field was left behind in `models.rs`. Remove the",
            "field, or — if it is an intentional code-only field — add it to",
            "`EXTRA_FIELD_EXEMPTIONS` in `spec_coverage_test.rs`.",
            "",
            "| Schema | Field |",
            "|--------|-------|",
        ]
        for m in sorted(extra_fields, key=lambda m: (m["schema"], m["field"])):
            lines.append(f"| `{m['schema']}` | `{m['field']}` |")
        lines.append("")

    # ---- Missing enum values ----
    if missing_enum_values:
        lines += [
            "## Missing Enum Values",
            "",
            "These values exist in a spec `enum` but have no corresponding variant",
            "in the mapped Rust enum. Responses carrying them fall into the untagged",
            "`Unknown` catch-all, and requests cannot express them at all. Add the",
            "variant (with a `#[serde(rename = ...)]` where the value isn't a valid",
            "identifier) and its `Display` arm in `models.rs`.",
            "",
            "| Enum | Spec location | Value |",
            "|------|---------------|-------|",
        ]
        for m in sorted(missing_enum_values, key=lambda m: (m["enum"], m["value"] or "")):
            value = f"`{m['value']}`" if m["value"] else "*(type is not a value enum)*"
            lines.append(f"| `{m['enum']}` | `{m['location']}` | {value} |")
        lines.append("")

    # ---- Extra enum values ----
    if extra_enum_values:
        lines += [
            "## Extra Enum Values",
            "",
            "These Rust enum variants serialize a value the mapped spec `enum` no",
            "longer (or never did) allow — the API rejects it on requests (the PubSub",
            "`seekType`/`snapshot` failure mode from #275). Remove the variant and its",
            "`Display` arm, or — if it is an intentional keeper — add it to",
            "`EXTRA_ENUM_VALUE_EXEMPTIONS` in `spec_coverage_test.rs`.",
            "",
            "| Enum | Spec location | Value |",
            "|------|---------------|-------|",
        ]
        for m in sorted(extra_enum_values, key=lambda m: (m["enum"], m["value"])):
            lines.append(f"| `{m['enum']}` | `{m['location']}` | `{m['value']}` |")
        lines.append("")

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

    # ---- Beta status changes ----
    if beta_total > 0:
        lines += [
            "## Beta Status Changes",
            "",
            "The live spec's `x-badges` Beta markers have drifted from",
            "`BETA_OPERATIONS` in `crates/clickhouse-cloud-api/src/meta.rs`.",
            "Consumers of the typed client (including this CLI) read that constant",
            "to render `(Beta)` affordances and gate stability-sensitive code paths.",
            "",
        ]
        if beta.get("newly_beta"):
            lines.append("**Newly Beta in live spec (add to `BETA_OPERATIONS`):**")
            for name in sorted(beta["newly_beta"]):
                lines.append(f"- `{name}`")
            lines.append("")
        if beta.get("graduated"):
            lines.append("**Graduated out of Beta in live spec (remove from `BETA_OPERATIONS`):**")
            for name in sorted(beta["graduated"]):
                lines.append(f"- `{name}`")
            lines.append("")
        lines += [
            "Regenerate the list with:",
            "```bash",
            "python3 scripts/regenerate-beta-lists.py",
            "```",
            "",
        ]

    # ---- Deprecated field changes ----
    if dep_total > 0:
        lines += [
            "## Deprecated Field Changes",
            "",
            "The live spec's `deprecated: true` markers have drifted from",
            "`DEPRECATED_FIELDS` in `crates/clickhouse-cloud-api/src/meta.rs`. Those",
            "fields are removed from the generated structs unless the",
            "`deprecated-fields` Cargo feature is enabled — consumers can neither",
            "read a deprecated response field nor send a deprecated request field.",
            "",
        ]
        if dep.get("newly_deprecated"):
            lines.append("**Newly deprecated fields (add to `DEPRECATED_FIELDS` + mark in `models.rs`):**")
            for struct_name, field in sorted(dep["newly_deprecated"]):
                lines.append(f"- `{struct_name}.{field}`")
            lines.append("")
        if dep.get("undeprecated"):
            lines.append("**No longer deprecated (remove from `DEPRECATED_FIELDS` + drop the marker in `models.rs`):**")
            for struct_name, field in sorted(dep["undeprecated"]):
                lines.append(f"- `{struct_name}.{field}`")
            lines.append("")
        lines += [
            "Regenerate the list with:",
            "```bash",
            "python3 scripts/regenerate-deprecated-fields.py",
            "```",
            "Then add/remove the matching",
            '`#[cfg(feature = "deprecated-fields")]`',
            "marker in `models.rs`.",
            "",
        ]

    # ---- Stale snapshot ----
    if snap_total > 0:
        lines += [
            "## Stale Snapshot",
            "",
            "The committed `clickhouse_cloud_openapi.json` is behind the live spec.",
            "Tests that run against the snapshot may pass even though the library is",
            "missing coverage for new endpoints or schemas.",
            "",
        ]
        if snap.get("added_ops"):
            lines.append("**New operations in live spec (not in snapshot):**")
            for name in sorted(snap["added_ops"]):
                lines.append(f"- `{name}`")
            lines.append("")
        if snap.get("removed_ops"):
            lines.append("**Operations removed from live spec (still in snapshot):**")
            for name in sorted(snap["removed_ops"]):
                lines.append(f"- `{name}`")
            lines.append("")
        if snap.get("added_schemas"):
            lines.append("**New schemas in live spec (not in snapshot):**")
            for name in sorted(snap["added_schemas"]):
                lines.append(f"- `{name}`")
            lines.append("")
        if snap.get("removed_schemas"):
            lines.append("**Schemas removed from live spec (still in snapshot):**")
            for name in sorted(snap["removed_schemas"]):
                lines.append(f"- `{name}`")
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
        "4. Fix any field optionality mismatches by hand-editing `models.rs`",
        "   (flip `T` ↔ `Option<T>` and the matching `skip_serializing_if` attribute).",
        "5. Fix any enum value drift by adding/removing the variant and its",
        "   `Display` arm in `models.rs`.",
        "6. Verify: `cargo test -p clickhouse-cloud-api`",
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

    non_openapi_methods = parse_non_openapi_client_methods()
    missing_op_names = spec_method_names - client_methods
    extra_op_names = client_methods - spec_method_names - non_openapi_methods

    missing_ops = {name: live_ops[name] for name in missing_op_names}

    # Compare schemas: live spec vs model types
    live_schemas = spec_schema_names(live_spec)
    spec_type_names = set(live_schemas.keys())

    missing_type_names = spec_type_names - model_types
    missing_types = {name: live_schemas[name] for name in missing_type_names}

    # Compare fields
    models_source = MODELS_RS.read_text()
    model_fields = parse_model_fields(models_source)
    optionality_exemptions = parse_optionality_exemptions()
    field_mismatches = check_field_optionality(live_spec, model_fields, optionality_exemptions)
    missing_fields = check_missing_fields(live_spec, model_fields)
    extra_field_exemptions = parse_extra_field_exemptions()
    extra_fields = check_extra_fields(live_spec, model_fields, extra_field_exemptions)

    # Compare enum values
    value_enums = parse_enum_variant_values(models_source)
    missing_enum_values = check_missing_enum_values(live_spec, model_fields, value_enums)
    extra_enum_value_exemptions = parse_extra_enum_value_exemptions()
    extra_enum_values = check_extra_enum_values(
        live_spec, model_fields, value_enums, extra_enum_value_exemptions
    )

    # Check committed snapshot staleness
    snapshot_staleness = check_snapshot_staleness(live_spec)
    snap_total = sum(len(v) for v in snapshot_staleness.values())

    # Compare beta status: spec x-badges vs meta.rs BETA_OPERATIONS
    declared_beta = parse_beta_operations()
    live_beta = spec_beta_operations(live_spec)
    beta_status_changes = {
        "newly_beta": live_beta - declared_beta,
        "graduated": declared_beta - live_beta,
    }
    beta_total = sum(len(v) for v in beta_status_changes.values())

    # Compare deprecated fields: spec deprecated:true (any schema) vs
    # meta.rs DEPRECATED_FIELDS
    declared_deprecated = parse_deprecated_fields()
    live_deprecated = spec_deprecated_fields(live_spec)
    deprecation_changes = {
        "newly_deprecated": live_deprecated - declared_deprecated,
        "undeprecated": declared_deprecated - live_deprecated,
    }
    dep_total = sum(len(v) for v in deprecation_changes.values())

    # Report
    total = (
        len(missing_ops)
        + len(extra_op_names)
        + len(missing_types)
        + len(field_mismatches)
        + len(missing_fields)
        + len(extra_fields)
        + len(missing_enum_values)
        + len(extra_enum_values)
        + beta_total
        + dep_total
        + snap_total
    )

    print(f"Live spec:       {len(spec_method_names)} operations, {len(spec_type_names)} schemas", file=sys.stderr)
    print(f"client.rs:       {len(client_methods)} pub async fn methods", file=sys.stderr)
    print(f"models.rs:       {len(model_types)} pub types", file=sys.stderr)
    print(f"---", file=sys.stderr)
    print(f"Missing methods: {len(missing_ops)}", file=sys.stderr)
    print(f"Extra methods:   {len(extra_op_names)}", file=sys.stderr)
    print(f"Missing types:   {len(missing_types)}", file=sys.stderr)
    print(f"Missing fields:  {len(missing_fields)}", file=sys.stderr)
    print(f"Extra fields:    {len(extra_fields)}", file=sys.stderr)
    print(f"Missing enum vals: {len(missing_enum_values)}", file=sys.stderr)
    print(f"Extra enum vals: {len(extra_enum_values)}", file=sys.stderr)
    print(f"Field mismatches:{len(field_mismatches)}", file=sys.stderr)
    print(f"Beta changes:    {beta_total}", file=sys.stderr)
    print(f"Deprecation chg: {dep_total}", file=sys.stderr)
    print(f"Stale snapshot:  {snap_total}", file=sys.stderr)

    if total == 0:
        print("\nNo drift. Library fully covers the live spec.", file=sys.stderr)
        return

    body = build_issue_body(
        missing_ops,
        extra_op_names,
        missing_types,
        live_schemas,
        field_mismatches,
        missing_fields,
        extra_fields,
        missing_enum_values,
        extra_enum_values,
        snapshot_staleness,
        beta_status_changes,
        deprecation_changes,
    )

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
