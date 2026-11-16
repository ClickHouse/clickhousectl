#!/usr/bin/env python3
"""Check that the release manifests carry one version, and that a tag names it.

The release workflow runs this first: on a tag it requires `vX.Y.Z` and that
every manifest says `X.Y.Z`, so a missed bump fails before anything is built or
published. Without a tag (pull requests, workflow_dispatch from a branch) it
requires only that the manifests agree with each other.
"""

import argparse
import json
import os
import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TAG_RE = re.compile(r"\Av([0-9]+\.[0-9]+\.[0-9]+)\Z")

CLI_MANIFEST = "crates/clickhousectl/Cargo.toml"
API_MANIFEST = "crates/clickhouse-cloud-api/Cargo.toml"
NPM_MANIFEST = "npm/package.json"
LOCKFILE = "Cargo.lock"
WORKSPACE_PACKAGES = ("clickhousectl", "clickhouse-cloud-api")


def _load_toml(root: Path, relative: str) -> dict:
    with (root / relative).open("rb") as f:
        return tomllib.load(f)


def read_versions(root: Path) -> dict[str, str | None]:
    """Return each release version source, keyed by a label naming its file."""
    cli = _load_toml(root, CLI_MANIFEST)
    api = _load_toml(root, API_MANIFEST)
    with (root / NPM_MANIFEST).open(encoding="utf-8") as f:
        npm = json.load(f)
    lock = _load_toml(root, LOCKFILE)

    dep = cli.get("dependencies", {}).get("clickhouse-cloud-api")
    dep_version = dep.get("version") if isinstance(dep, dict) else dep
    # Workspace members are the lockfile entries without a registry source.
    locked = {
        package["name"]: package.get("version")
        for package in lock.get("package", [])
        if package.get("name") in WORKSPACE_PACKAGES and "source" not in package
    }

    return {
        f"{CLI_MANIFEST} package.version": cli.get("package", {}).get("version"),
        f"{CLI_MANIFEST} dependencies.clickhouse-cloud-api.version": dep_version,
        f"{API_MANIFEST} package.version": api.get("package", {}).get("version"),
        f"{NPM_MANIFEST} version": npm.get("version"),
        **{f"{LOCKFILE} {name}": locked.get(name) for name in WORKSPACE_PACKAGES},
    }


def check(versions: dict[str, str | None], tag: str | None) -> list[str]:
    """Return one error per disagreement; empty when the release is consistent."""
    errors = [f"{source} is missing" for source, v in versions.items() if not v]
    present = {source: v for source, v in versions.items() if v}

    if tag is not None:
        match = TAG_RE.match(tag)
        if not match:
            return [f"release tag {tag!r} is not vX.Y.Z", *errors]
        expected = match.group(1)
        errors.extend(
            f"{source} is {v}, but the tag is {tag}"
            for source, v in present.items()
            if v != expected
        )
    elif len(set(present.values())) > 1:
        errors.extend(f"{source} is {v}" for source, v in present.items())
        errors.insert(0, "release versions disagree")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--tag", help="release tag (vX.Y.Z) that every manifest version must match"
    )
    parser.add_argument("--root", type=Path, default=REPO_ROOT, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    versions = read_versions(args.root)
    for source, version in versions.items():
        print(f"{source}: {version}")
    errors = check(versions, args.tag)
    prefix = "::error::" if os.environ.get("GITHUB_ACTIONS") == "true" else "error: "
    for error in errors:
        print(f"{prefix}{error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: release versions agree{f' with {args.tag}' if args.tag else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
