#!/usr/bin/env python3
"""
Resolve required vs optional fields for each schema in the ClickHouse Cloud
OpenAPI spec. Handles two conventions:

1. Schemas with a standard OpenAPI `required` array (newer/beta endpoints)
2. Schemas without `required` — optional fields have descriptions starting
   with "Optional" (legacy GA endpoints)

A subset of schemas (PARTIAL_REQUIRED_SCHEMAS) have a `required` array that
only lists newly-added fields; older fields on the same schema still rely on
the description heuristic. For those, the description heuristic also applies
as a fallback for fields absent from `required[]`.

PATCH request schemas are always treated as all-optional (partial update
semantics).

Nullable fields (type: ["string", "null"] or oneOf with null) remain optional
even if the schema says they're required, because the API can return null.

Usage:
    python scripts/resolve-field-requirements.py                    # pretty-print manifest
    python scripts/resolve-field-requirements.py -o manifest.json   # write to file
    python scripts/resolve-field-requirements.py --summary          # print summary stats
"""

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SPEC_PATH = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "clickhouse_cloud_openapi.json"


def is_field_nullable(field_schema: dict) -> bool:
    """Check if a field allows null values."""
    # type: ["string", "null"]
    ftype = field_schema.get("type", "")
    if isinstance(ftype, list) and "null" in ftype:
        return True
    # oneOf/anyOf with a null variant
    for key in ("oneOf", "anyOf"):
        variants = field_schema.get(key, [])
        if any(v.get("type") == "null" for v in variants):
            return True
    return False


def is_patch_schema(schema_name: str) -> bool:
    """PATCH request schemas have all-optional fields (partial update semantics)."""
    return "Patch" in schema_name and schema_name.endswith("Request")


# Schemas where the spec's `required` array lists only newly-added fields;
# older fields on the same schema still rely on the description heuristic.
# Adding a schema here makes its resolution use the union of `required[]`
# and the description heuristic instead of treating `required[]` as exclusive.
#
# Remove an entry once the spec is corrected upstream (the `required[]` array
# becomes comprehensive for the schema). The Rust test mirrors this list in
# `tests/spec_coverage_test.rs` and asserts on stale entries.
PARTIAL_REQUIRED_SCHEMAS: set[str] = {
    "Service",
    "ServiceScalingPatchResponse",
}


def resolve_required_fields(schema_name: str, schema: dict) -> tuple[set[str], str]:
    """Returns (set of required non-nullable field names, resolution method).

    Resolution strategy:
    1. PATCH request schemas -> all fields optional.
    2. If schema is in PARTIAL_REQUIRED_SCHEMAS -> required = (required[] ∪ description heuristic).
    3. If schema has a `required` array -> use it (standard OpenAPI).
    4. Otherwise -> field is required if description does NOT start with "Optional".

    In all cases, nullable fields are excluded from the required set.
    """
    props = schema.get("properties", {})

    if is_patch_schema(schema_name):
        return set(), "patch_schema"

    if schema_name in PARTIAL_REQUIRED_SCHEMAS:
        required_names = set(schema.get("required", []))
        for name, field in props.items():
            desc = field.get("description", "") or ""
            if not desc.startswith("Optional"):
                required_names.add(name)
        method = "partial_required"
    elif "required" in schema:
        required_names = set(schema["required"])
        method = "required_array"
    else:
        required_names = set()
        for name, field in props.items():
            desc = field.get("description", "") or ""
            if not desc.startswith("Optional"):
                required_names.add(name)
        method = "description_heuristic"

    # Exclude nullable fields — they must remain Option<T> in Rust
    required_non_nullable = {
        name for name in required_names
        if name in props and not is_field_nullable(props[name])
    }

    return required_non_nullable, method


def build_manifest(spec: dict) -> dict:
    """Build the full manifest mapping schema name -> field requirements."""
    schemas = spec.get("components", {}).get("schemas", {})
    manifest = {}

    for schema_name, schema in sorted(schemas.items()):
        props = schema.get("properties", {})
        if not props:
            continue

        required_fields, method = resolve_required_fields(schema_name, schema)
        all_fields = set(props.keys())
        optional_fields = all_fields - required_fields

        manifest[schema_name] = {
            "required": sorted(required_fields),
            "optional": sorted(optional_fields),
            "resolution_method": method,
        }

    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-o", "--output", help="Write manifest to file instead of stdout")
    parser.add_argument("--summary", action="store_true", help="Print summary stats")
    parser.add_argument(
        "--spec",
        default=str(SPEC_PATH),
        help=f"Path to OpenAPI spec JSON (default: {SPEC_PATH})",
    )
    args = parser.parse_args()

    with open(args.spec) as f:
        spec = json.load(f)

    manifest = build_manifest(spec)

    if args.summary:
        total_required = sum(len(v["required"]) for v in manifest.values())
        total_optional = sum(len(v["optional"]) for v in manifest.values())
        by_method = {}
        for v in manifest.values():
            m = v["resolution_method"]
            by_method[m] = by_method.get(m, 0) + 1

        print(f"Schemas:          {len(manifest)}", file=sys.stderr)
        print(f"Total required:   {total_required}", file=sys.stderr)
        print(f"Total optional:   {total_optional}", file=sys.stderr)
        print(f"By method:", file=sys.stderr)
        for m, count in sorted(by_method.items()):
            print(f"  {m}: {count}", file=sys.stderr)
        return

    output = json.dumps(manifest, indent=2)

    if args.output:
        Path(args.output).write_text(output + "\n")
        print(f"Wrote manifest to {args.output}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
