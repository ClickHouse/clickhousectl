#!/usr/bin/env python3
"""
Print the DEPRECATED_FIELDS list for
crates/clickhouse-cloud-api/src/meta.rs, sourced from `deprecated: true`
properties on every schema in the committed OpenAPI snapshot.

Both request- and response-side schemas are included: deprecated fields are
removed from the generated structs entirely unless the `deprecated-fields`
Cargo feature is enabled, so callers can neither read a deprecated response
field nor send a deprecated request field by default.

Run this whenever the snapshot is refreshed. Paste the output into meta.rs.
The shared OpenAPI analyzer reports drift until the constant matches and each
field also carries the `#[cfg(feature = "deprecated-fields")]` marker in
models.rs.

Struct names come from spec schema names alone. A schema modeled as both a
request and a response type is split into `{Name}` and `{Name}Response` in
models.rs, and the analyzer expects one entry per Rust type, so re-add the
`{Name}Response` entries by hand after pasting (see meta.rs). Deriving them
here would mean parsing models.rs, which belongs to the analyzer, not Python.

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


def deprecated_fields(spec: dict) -> list[tuple[str, str]]:
    fields = []
    schemas = spec.get("components", {}).get("schemas", {})
    for spec_name, schema in schemas.items():
        props = schema.get("properties") or {}
        for prop_name, prop in props.items():
            if isinstance(prop, dict) and prop.get("deprecated") is True:
                fields.append((pascalize(spec_name), prop_name))
    return sorted(set(fields))


def main():
    spec = json.loads(SNAPSHOT.read_text())
    fields = deprecated_fields(spec)
    print(f"// {len(fields)} deprecated fields extracted from")
    print(f"// crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json")
    print(f"// Regenerate with: python3 scripts/regenerate-deprecated-fields.py")
    print("pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[")
    for struct_name, field in fields:
        print(f'    ("{struct_name}", "{field}"),')
    print("];")


if __name__ == "__main__":
    main()
