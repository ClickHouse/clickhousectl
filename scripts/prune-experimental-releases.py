#!/usr/bin/env python3
"""Prune old experimental GitHub pre-releases, keeping only the newest N.

Safety invariants:
- Only non-draft GitHub pre-releases whose tag strictly matches
  `experimental-YYYYMMDD[-HHMM[SS]]-<shortsha>` are ever eligible.
- Standard `vX.Y.Z` releases, drafts, and non-matching pre-releases are never
  touched.
- `--keep-tag` is never deleted even if timestamps are out of order.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from typing import Any

EXPERIMENTAL_TAG_RE = re.compile(
    r"\Aexperimental-[0-9]{8}(?:-[0-9]{4,6})?-[0-9a-f]{7,}\Z"
)


def is_eligible_experimental_release(release: dict[str, Any]) -> bool:
    """Return True only for published experimental pre-releases."""
    if release.get("prerelease") is not True:
        return False
    if release.get("draft") is not False:
        return False
    tag = release.get("tag_name")
    return isinstance(tag, str) and EXPERIMENTAL_TAG_RE.match(tag) is not None


def select_releases_to_prune(
    releases: list[dict[str, Any]],
    *,
    keep: int,
    keep_tag: str | None = None,
) -> list[dict[str, Any]]:
    """Return the eligible experimental releases that exceed `keep`."""
    if keep < 1:
        raise ValueError(f"keep must be >= 1, got {keep}")

    eligible = [r for r in releases if is_eligible_experimental_release(r)]
    eligible.sort(
        key=lambda r: (str(r.get("created_at") or ""), str(r["tag_name"])),
        reverse=True,
    )

    kept_tags: set[str] = set()
    if keep_tag is not None and any(r["tag_name"] == keep_tag for r in eligible):
        kept_tags.add(keep_tag)

    for release in eligible:
        if len(kept_tags) >= keep:
            break
        kept_tags.add(release["tag_name"])

    return [r for r in eligible if r["tag_name"] not in kept_tags]


def fetch_releases(repo: str) -> list[dict[str, Any]]:
    proc = subprocess.run(
        [
            "gh",
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repo}/releases?per_page=100",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    pages = json.loads(proc.stdout)
    releases: list[dict[str, Any]] = []
    for page in pages:
        if isinstance(page, list):
            releases.extend(page)
    return releases


def delete_release(repo: str, tag: str) -> None:
    subprocess.run(
        [
            "gh",
            "release",
            "delete",
            tag,
            "--repo",
            repo,
            "--yes",
            "--cleanup-tag",
        ],
        check=True,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--repo",
        required=True,
        help="GitHub repository in OWNER/REPO form",
    )
    parser.add_argument(
        "--keep",
        type=int,
        default=5,
        help="Number of newest experimental pre-releases to keep (default: 5)",
    )
    parser.add_argument(
        "--keep-tag",
        help="Tag that must never be pruned (e.g. the release just published)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print releases that would be pruned without deleting them",
    )
    args = parser.parse_args(argv)

    if args.keep < 1:
        print(f"error: --keep must be >= 1, got {args.keep}", file=sys.stderr)
        return 2

    releases = fetch_releases(args.repo)
    to_prune = select_releases_to_prune(
        releases,
        keep=args.keep,
        keep_tag=args.keep_tag,
    )

    if not to_prune:
        print(f"No experimental releases to prune (keeping up to {args.keep}).")
        return 0

    for release in to_prune:
        tag = release["tag_name"]
        if args.dry_run:
            print(f"[dry-run] Would prune experimental release {tag}")
        else:
            print(f"Pruning experimental release {tag}")
            delete_release(args.repo, tag)

    return 0


if __name__ == "__main__":
    sys.exit(main())
