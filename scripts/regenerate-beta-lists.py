#!/usr/bin/env python3
"""
Print the BETA_OPERATIONS list for crates/clickhouse-cloud-api/src/meta.rs,
sourced from `x-badges` entries in the committed OpenAPI snapshot.

Run this whenever the snapshot is refreshed. Paste the output into meta.rs.
The `beta_operations_match_spec` test will fail until the constant matches.

Usage:
    python3 scripts/regenerate-beta-lists.py
"""

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "clickhouse_cloud_openapi.json"
HTTP_METHODS = {"get", "put", "post", "delete", "patch", "options", "head", "trace"}


def camel_to_snake(name: str) -> str:
    out = []
    prev = None
    for ch in name:
        if ch.isupper():
            if prev and (prev.islower() or prev.isdigit()):
                out.append("_")
            out.append(ch.lower())
        else:
            out.append(ch)
        prev = ch
    return "".join(out)


def beta_operation_ids(spec: dict) -> list[str]:
    ids = []
    for path_item in spec.get("paths", {}).values():
        for method, op in path_item.items():
            if method not in HTTP_METHODS or not isinstance(op, dict):
                continue
            badges = op.get("x-badges") or []
            if any(b.get("name") == "Beta" for b in badges):
                ids.append(camel_to_snake(op["operationId"]))
    return sorted(set(ids))


def main():
    spec = json.loads(SNAPSHOT.read_text())
    ops = beta_operation_ids(spec)
    print(f"// {len(ops)} beta operations extracted from x-badges in")
    print(f"// crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json")
    print(f"// Regenerate with: python3 scripts/regenerate-beta-lists.py")
    print("pub const BETA_OPERATIONS: &[&str] = &[")
    for op in ops:
        print(f'    "{op}",')
    print("];")


if __name__ == "__main__":
    main()
