#!/usr/bin/env python3
"""
Print the DEPRECATED_OUTPUT_FIELDS list for
crates/clickhouse-cloud-api/src/meta.rs, sourced from `deprecated: true`
properties on response-side schemas in the committed OpenAPI snapshot.

Response-side schemas only: request-side schemas (`*Request`, `*Patch`,
`*Input`) are excluded because callers may still need to send a deprecated
field. Note that `*PatchResponse` ends in "Response" and so counts as a
response schema even though it contains "Patch".

Run this whenever the snapshot is refreshed. Paste the output into meta.rs.
The `deprecated_output_fields_match_spec` test fails until the constant matches,
and `deprecated_output_fields_hidden` fails until each field also carries the
`#[cfg_attr(not(feature = "deprecated-fields"), serde(skip_serializing))]`
marker in models.rs.

Usage:
    python3 scripts/regenerate-deprecated-fields.py
"""

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "clickhouse_cloud_openapi.json"


def pascalize(name: str) -> str:
    out = []
    upper_next = True
    for ch in name:
        if ch.isalnum():
            out.append(ch.upper() if upper_next else ch)
            upper_next = False
        else:
            upper_next = True
    return "".join(out)


def is_request_side_schema(name: str) -> bool:
    return name.endswith("Request") or name.endswith("Patch") or name.endswith("Input")


def deprecated_output_fields(spec: dict) -> list[tuple[str, str]]:
    fields = []
    schemas = spec.get("components", {}).get("schemas", {})
    for spec_name, schema in schemas.items():
        if is_request_side_schema(spec_name):
            continue
        props = schema.get("properties") or {}
        for prop_name, prop in props.items():
            if isinstance(prop, dict) and prop.get("deprecated") is True:
                fields.append((pascalize(spec_name), prop_name))
    return sorted(set(fields))


def main():
    spec = json.loads(SNAPSHOT.read_text())
    fields = deprecated_output_fields(spec)
    print(f"// {len(fields)} deprecated response-side fields extracted from")
    print(f"// crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json")
    print(f"// Regenerate with: python3 scripts/regenerate-deprecated-fields.py")
    print("pub const DEPRECATED_OUTPUT_FIELDS: &[(&str, &str)] = &[")
    for struct_name, field in fields:
        print(f'    ("{struct_name}", "{field}"),')
    print("];")


if __name__ == "__main__":
    main()
