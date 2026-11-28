#!/usr/bin/env python3
"""Unit tests for scripts/prune-experimental-releases.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

SCRIPT_PATH = Path(__file__).resolve().parent.parent / "prune-experimental-releases.py"
SPEC = importlib.util.spec_from_file_location("prune_experimental_releases", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def make_release(
    tag: str,
    created_at: str,
    *,
    prerelease: bool = True,
    draft: bool = False,
) -> dict:
    return {
        "tag_name": tag,
        "created_at": created_at,
        "prerelease": prerelease,
        "draft": draft,
    }


class PruneExperimentalReleasesTest(unittest.TestCase):
    def test_keeps_newest_five_and_prunes_older_experimental_releases(self) -> None:
        releases = [
            make_release("experimental-20261101-100000-aaaaaaa", "2026-11-01T10:00:00Z"),
            make_release("experimental-20261103-100000-ccccccc", "2026-11-03T10:00:00Z"),
            make_release("experimental-20261107-100000-1111111", "2026-11-07T10:00:00Z"),
            make_release("experimental-20261102-100000-bbbbbbb", "2026-11-02T10:00:00Z"),
            make_release("experimental-20261106-100000-fffffff", "2026-11-06T10:00:00Z"),
            make_release("experimental-20261104-100000-ddddddd", "2026-11-04T10:00:00Z"),
            make_release("experimental-20261105-100000-eeeeeee", "2026-11-05T10:00:00Z"),
        ]

        pruned = MODULE.select_releases_to_prune(releases, keep=5)
        self.assertEqual(
            [r["tag_name"] for r in pruned],
            [
                "experimental-20261102-100000-bbbbbbb",
                "experimental-20261101-100000-aaaaaaa",
            ],
        )

    def test_never_touches_stable_draft_or_non_experimental_releases(self) -> None:
        releases = [
            make_release("v0.6.1", "2026-11-01T00:00:00Z", prerelease=False),
            make_release("v0.6.0", "2026-10-01T00:00:00Z", prerelease=True),
            make_release("v0.7.0-rc1", "2026-10-02T00:00:00Z", prerelease=True),
            make_release("beta-20261101-abcdef1", "2026-10-03T00:00:00Z", prerelease=True),
            make_release(
                "experimental-20261001-000000-abcdef1",
                "2026-10-01T00:00:00Z",
                prerelease=False,
            ),
            make_release(
                "experimental-20261002-000000-abcdef2",
                "2026-10-02T00:00:00Z",
                prerelease=True,
                draft=True,
            ),
            make_release("experimental-20261101-abcdef1", "2026-11-01T00:00:00Z"),
            make_release("experimental-20261102-abcdef2", "2026-11-02T00:00:00Z"),
        ]

        pruned = MODULE.select_releases_to_prune(releases, keep=1)
        self.assertEqual(
            [r["tag_name"] for r in pruned],
            ["experimental-20261101-abcdef1"],
        )

    def test_keep_tag_is_preserved_even_with_older_timestamp(self) -> None:
        releases = [
            make_release("experimental-20261101-100000-aaaaaaa", "2026-11-01T10:00:00Z"),
            make_release("experimental-20261102-100000-bbbbbbb", "2026-11-02T10:00:00Z"),
            make_release("experimental-20261103-100000-ccccccc", "2026-11-03T10:00:00Z"),
        ]

        pruned = MODULE.select_releases_to_prune(
            releases,
            keep=2,
            keep_tag="experimental-20261101-100000-aaaaaaa",
        )
        self.assertEqual(
            [r["tag_name"] for r in pruned],
            ["experimental-20261102-100000-bbbbbbb"],
        )

    def test_fewer_than_keep_returns_empty(self) -> None:
        releases = [
            make_release("experimental-20261101-100000-aaaaaaa", "2026-11-01T10:00:00Z"),
            make_release("experimental-20261102-100000-bbbbbbb", "2026-11-02T10:00:00Z"),
        ]
        self.assertEqual(MODULE.select_releases_to_prune(releases, keep=5), [])

    def test_rejects_non_positive_keep(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.select_releases_to_prune([], keep=0)


if __name__ == "__main__":
    unittest.main()
