#!/usr/bin/env python3
"""Select live Cloud integration suites for an exact Git revision diff."""

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SUITES = ("service", "postgres", "organization", "clickpipes")
ALL_SUITES = frozenset(SUITES)
NO_SUITES = frozenset()

# Retain mappings when files are deleted or renamed so old diff paths stay classified.
SOURCE_PATH_SUITES = {
    "crates/clickhousectl/src/cloud/clickstack.rs": NO_SUITES,
    "crates/clickhousectl/src/cloud/config.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/client.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/client/activity.rs": frozenset({"organization"}),
    "crates/clickhouse-cloud-api/src/client/api_keys.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/client/backups.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/client/clickpipes.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/client/clickstack.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/client/organizations.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/client/postgres.rs": frozenset(
        {"postgres", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/client/services.rs": frozenset(
        {"service", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/client/udfs.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/convert.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/convert/clickstack.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/convert/postgres.rs": frozenset(
        {"postgres", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/convert/service.rs": frozenset(
        {"service", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/convert/shared.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/error.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/lib.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/meta.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/models/activity.rs": frozenset(
        {"organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/api_keys.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/backups.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models/byoc.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/clickpipes.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/models/clickstack.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models/clickstack_enums.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models/invitations.rs": frozenset(
        {"organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/members.rs": frozenset(
        {"organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/organization_private_endpoints.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/organizations.rs": frozenset(
        {"service", "organization"}
    ),
    "crates/clickhouse-cloud-api/src/models/postgres.rs": frozenset(
        {"postgres", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/models/quotas.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models/rbac.rs": frozenset({"organization"}),
    "crates/clickhouse-cloud-api/src/models/scim.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/models/services.rs": frozenset(
        {"service", "clickpipes"}
    ),
    "crates/clickhouse-cloud-api/src/models/shared.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/src/models/udfs.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/src/serde_helpers.rs": ALL_SUITES,
}

TEST_PATH_SUITES = {
    "crates/clickhouse-cloud-api/tests/clickpipes/driver.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/e2e_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/kafka_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/kinesis_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/mongo_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/mysql_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/postgres_cdc_test.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/tests/clickpipes/postgres_cli_cdc_test.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/tests/clickpipes/postgres_ec2_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/s3_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/smoke_test.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/kafka.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/kinesis.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/mod.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/mongo.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/mongo_user_data.sh.template": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/mysql.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/mysql_user_data.sh.template": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/postgres.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/postgres_user_data.sh.template": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/redpanda_user_data_mtls.sh.template": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/redpanda_user_data_scram_tls.sh.template": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/stages/s3.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/clickpipes/support.rs": frozenset(
        {"clickpipes"}
    ),
    "crates/clickhouse-cloud-api/tests/client_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/common/mod.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/tests/common/support.rs": ALL_SUITES,
    "crates/clickhouse-cloud-api/tests/integration_org_test.rs": frozenset(
        {"organization"}
    ),
    "crates/clickhouse-cloud-api/tests/integration_postgres_test.rs": frozenset(
        {"postgres"}
    ),
    "crates/clickhouse-cloud-api/tests/integration_test.rs": frozenset({"service"}),
    "crates/clickhouse-cloud-api/tests/model_facade_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/models_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/run_query_test.rs": NO_SUITES,
    "crates/clickhouse-cloud-api/tests/service_query_key_cli_test.rs": frozenset(
        {"service"}
    ),
    "crates/clickhouse-cloud-api/tests/spec_coverage_test.rs": NO_SUITES,
}

API_SOURCE_PREFIX = "crates/clickhouse-cloud-api/src/"
API_TEST_PREFIX = "crates/clickhouse-cloud-api/tests/"
API_CRATE_PREFIX = "crates/clickhouse-cloud-api/"
ALWAYS_ALL_PATHS = {
    ".github/workflows/cloud-integration.yml",
    "Cargo.lock",
    "scripts/classify-cloud-integration.py",
    "scripts/tests/test_classify_cloud_integration.py",
}
RUST_CONFIG_PATHS = {
    ".cargo/config",
    ".cargo/config.toml",
    ".rustfmt.toml",
    "clippy.toml",
    "rust-toolchain",
    "rust-toolchain.toml",
    "rustfmt.toml",
}


class DiffFormatError(ValueError):
    """The name-status stream cannot be classified safely."""


@dataclass(frozen=True)
class Selection:
    suites: tuple[str, ...]
    failed_closed: bool = False
    reason: str | None = None


def classify_path(path: str) -> frozenset[str] | None:
    """Return mapped suites, an empty known mapping, or None for unknown API code."""
    if path in SOURCE_PATH_SUITES:
        return SOURCE_PATH_SUITES[path]
    if path in TEST_PATH_SUITES:
        return TEST_PATH_SUITES[path]
    if (
        path.startswith(API_SOURCE_PREFIX)
        or path.startswith(API_TEST_PREFIX)
        or (path.startswith(API_CRATE_PREFIX) and path.endswith(".rs"))
    ):
        return None
    if (
        path in ALWAYS_ALL_PATHS
        or path in RUST_CONFIG_PATHS
        or path == "Cargo.toml"
        or path.endswith("/Cargo.toml")
    ):
        return ALL_SUITES
    return NO_SUITES


def parse_name_status(data: bytes) -> list[tuple[str, tuple[str, ...]]]:
    """Parse `git diff --name-status -z`, rejecting anything unexpected."""
    if not data:
        return []
    fields = data.split(b"\0")
    if fields[-1] != b"":
        raise DiffFormatError("name-status output is not NUL terminated")
    fields.pop()

    records = []
    index = 0
    while index < len(fields):
        try:
            status = fields[index].decode("ascii")
        except UnicodeDecodeError as error:
            raise DiffFormatError("non-ASCII diff status") from error
        index += 1

        if status in {"A", "M", "D"}:
            path_count = 1
        elif (
            len(status) > 1
            and status[0] in {"R", "C"}
            and status[1:].isdigit()
            and 0 <= int(status[1:]) <= 100
        ):
            path_count = 2
        else:
            raise DiffFormatError(f"unsupported diff status: {status!r}")

        if index + path_count > len(fields):
            raise DiffFormatError(f"missing path for diff status {status!r}")
        try:
            paths = tuple(
                field.decode("utf-8") for field in fields[index : index + path_count]
            )
        except UnicodeDecodeError as error:
            raise DiffFormatError("non-UTF-8 diff path") from error
        if any(not path for path in paths):
            raise DiffFormatError(f"empty path for diff status {status!r}")
        records.append((status, paths))
        index += path_count
    return records


def select_records(records: list[tuple[str, tuple[str, ...]]]) -> Selection:
    selected = set()
    for _status, paths in records:
        for path in paths:
            suites = classify_path(path)
            if suites is None:
                return Selection(
                    SUITES,
                    failed_closed=True,
                    reason=f"unknown Cloud API source/test path: {path}",
                )
            unknown_suites = set(suites) - ALL_SUITES
            if unknown_suites:
                return Selection(
                    SUITES,
                    failed_closed=True,
                    reason=f"unknown suite mapping for {path}: {sorted(unknown_suites)}",
                )
            selected.update(suites)
    return Selection(tuple(suite for suite in SUITES if suite in selected))


def select_name_status(data: bytes) -> Selection:
    try:
        return select_records(parse_name_status(data))
    except DiffFormatError as error:
        return Selection(SUITES, failed_closed=True, reason=str(error))


def select_revisions(base_sha: str, head_sha: str) -> Selection:
    command = [
        "git",
        "diff",
        "--name-status",
        "-z",
        "--find-renames",
        "--find-copies",
        "--find-copies-harder",
        f"{base_sha}...{head_sha}",
        "--",
    ]
    try:
        result = subprocess.run(command, cwd=REPO_ROOT, capture_output=True)
    except OSError as error:
        return Selection(SUITES, failed_closed=True, reason=f"git diff failed: {error}")
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", errors="replace").strip()
        return Selection(
            SUITES,
            failed_closed=True,
            reason=f"git diff failed: {detail or f'exit {result.returncode}'}",
        )
    return select_name_status(result.stdout)


def format_suites(suites: tuple[str, ...]) -> str:
    return ",".join(suites) if suites else "none"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base_sha")
    parser.add_argument("head_sha")
    args = parser.parse_args()

    selection = select_revisions(args.base_sha, args.head_sha)
    if selection.failed_closed:
        print(f"warning: {selection.reason}; selecting all suites", file=sys.stderr)
    print(format_suites(selection.suites))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
