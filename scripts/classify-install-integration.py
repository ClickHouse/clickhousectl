#!/usr/bin/env python3
"""Classify paths that can affect live local ClickHouse install checks."""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Keep this exact: local Docker/Postgres changes must not start the live install
# matrix. The static test inventories shared/local Rust sources and every
# subprocess test file, so a new or renamed candidate fails closed until it is
# classified here.
INSTALL_EXACT_PATHS = frozenset(
    {
        ".github/workflows/test-cli.yml",
        ".github/workflows/test-install.yml",
        "Cargo.lock",
        "Cargo.toml",
        "crates/clickhousectl/Cargo.toml",
        "crates/clickhousectl/src/cli.rs",
        "crates/clickhousectl/src/error.rs",
        "crates/clickhousectl/src/http.rs",
        "crates/clickhousectl/src/init.rs",
        "crates/clickhousectl/src/local/cli.rs",
        "crates/clickhousectl/src/local/discovery.rs",
        "crates/clickhousectl/src/local/mod.rs",
        "crates/clickhousectl/src/local/output.rs",
        "crates/clickhousectl/src/local/server.rs",
        "crates/clickhousectl/src/local/symlink.rs",
        "crates/clickhousectl/src/main.rs",
        "crates/clickhousectl/src/paths.rs",
        "crates/clickhousectl/src/user_agent.rs",
        "crates/clickhousectl/tests/local_install_local_first_test.rs",
        "crates/clickhousectl/tests/local_version_error_test.rs",
        "scripts/classify-install-integration.py",
        "scripts/tests/test_classify_install_integration.py",
    }
)

INSTALL_PREFIXES = ("crates/clickhousectl/src/version_manager/",)

# Explicit non-install mappings make the scope reviewable while allowing the
# inventory test to reject an unclassified new shared/local source or test.
NON_INSTALL_EXACT_PATHS = frozenset(
    {
        "crates/clickhousectl/src/dotenv.rs",
        "crates/clickhousectl/src/local/config.rs",
        "crates/clickhousectl/src/local/docker.rs",
        "crates/clickhousectl/src/local/postgres.rs",
        "crates/clickhousectl/src/skills.rs",
        "crates/clickhousectl/src/telemetry.rs",
        "crates/clickhousectl/src/update.rs",
        "crates/clickhousectl/tests/cli_request_shape_test.rs",
        "crates/clickhousectl/tests/local_client_project_scope_errors_test.rs",
        "crates/clickhousectl/tests/local_client_selectors_test.rs",
        "crates/clickhousectl/tests/local_docker_diagnostics_test.rs",
        "crates/clickhousectl/tests/local_docker_pull_progress_test.rs",
        "crates/clickhousectl/tests/local_postgres_readiness_test.rs",
        "crates/clickhousectl/tests/local_postgres_start_validation_test.rs",
        "crates/clickhousectl/tests/local_server_metadata_test.rs",
        "crates/clickhousectl/tests/local_server_name_compatibility_test.rs",
        "crates/clickhousectl/tests/local_server_project_scope_errors_test.rs",
        "crates/clickhousectl/tests/local_server_readiness_test.rs",
        "crates/clickhousectl/tests/local_server_selection_test.rs",
        "crates/clickhousectl/tests/local_server_start_args_test.rs",
        "crates/clickhousectl/tests/local_server_state_machine_test.rs",
        "crates/clickhousectl/tests/local_server_stopped_test.rs",
        "crates/clickhousectl/tests/local_structured_errors_test.rs",
        "crates/clickhousectl/tests/telemetry_test.rs",
    }
)

CANDIDATE_SOURCE_PREFIX = "crates/clickhousectl/src/"
CANDIDATE_TEST_PREFIX = "crates/clickhousectl/tests/"
CLOUD_SOURCE_PREFIX = "crates/clickhousectl/src/cloud/"


def workflow_path_patterns() -> frozenset[str]:
    """Return the exact pull-request path filter represented by this mapping."""
    return INSTALL_EXACT_PATHS | frozenset(f"{prefix}**" for prefix in INSTALL_PREFIXES)


def classify_path(path: str) -> bool | None:
    """Return run/skip, or None for an unclassified installer candidate."""
    if path in INSTALL_EXACT_PATHS or path.startswith(INSTALL_PREFIXES):
        return True
    if path in NON_INSTALL_EXACT_PATHS or path.startswith(CLOUD_SOURCE_PREFIX):
        return False
    if (
        path.endswith(".rs") and path.startswith(CANDIDATE_SOURCE_PREFIX)
    ) or path.startswith(CANDIDATE_TEST_PREFIX):
        return None
    return False
