#!/usr/bin/env python3
"""Classify paths that require the live local install integration workflow."""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INSTALL = True
NO_INSTALL = False

# Retain mappings when files are deleted or renamed so old diff paths remain
# classified. The inventory test requires every current non-Cloud CLI Rust file
# to have an explicit decision.
SOURCE_PATH_INSTALL = {
    "crates/clickhousectl/src/cli.rs": INSTALL,
    "crates/clickhousectl/src/dotenv.rs": NO_INSTALL,
    "crates/clickhousectl/src/error.rs": INSTALL,
    "crates/clickhousectl/src/http.rs": INSTALL,
    "crates/clickhousectl/src/init.rs": NO_INSTALL,
    "crates/clickhousectl/src/local/cli.rs": INSTALL,
    "crates/clickhousectl/src/local/config.rs": NO_INSTALL,
    "crates/clickhousectl/src/local/discovery.rs": INSTALL,
    "crates/clickhousectl/src/local/docker.rs": NO_INSTALL,
    "crates/clickhousectl/src/local/mod.rs": INSTALL,
    "crates/clickhousectl/src/local/output.rs": INSTALL,
    "crates/clickhousectl/src/local/postgres.rs": NO_INSTALL,
    "crates/clickhousectl/src/local/server.rs": INSTALL,
    "crates/clickhousectl/src/local/symlink.rs": INSTALL,
    "crates/clickhousectl/src/main.rs": INSTALL,
    "crates/clickhousectl/src/paths.rs": INSTALL,
    "crates/clickhousectl/src/skills.rs": NO_INSTALL,
    "crates/clickhousectl/src/telemetry.rs": NO_INSTALL,
    "crates/clickhousectl/src/update.rs": INSTALL,
    "crates/clickhousectl/src/user_agent.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/download.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/install.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/list.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/lock.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/master.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/mod.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/network.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/platform.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/resolve.rs": INSTALL,
    "crates/clickhousectl/src/version_manager/spec.rs": INSTALL,
}

TEST_PATH_INSTALL = {
    "crates/clickhousectl/tests/cli_request_shape_test.rs": NO_INSTALL,
    "crates/clickhousectl/tests/local_docker_pull_progress_test.rs": NO_INSTALL,
    "crates/clickhousectl/tests/local_install_local_first_test.rs": INSTALL,
    "crates/clickhousectl/tests/local_server_readiness_test.rs": NO_INSTALL,
    "crates/clickhousectl/tests/local_server_start_args_test.rs": INSTALL,
    "crates/clickhousectl/tests/local_server_stopped_test.rs": NO_INSTALL,
    "crates/clickhousectl/tests/local_version_error_test.rs": INSTALL,
    "crates/clickhousectl/tests/telemetry_test.rs": INSTALL,
}

ALWAYS_INSTALL_PATHS = {
    ".cargo/config",
    ".cargo/config.toml",
    ".github/workflows/test-cli.yml",
    ".github/workflows/test-install.yml",
    "Cargo.lock",
    "Cargo.toml",
    "crates/clickhousectl/Cargo.toml",
    "rust-toolchain",
    "rust-toolchain.toml",
    "scripts/classify-install-integration.py",
    "scripts/tests/test_classify_install_integration.py",
}

INSTALL_PATHS = (
    frozenset(path for path, selected in SOURCE_PATH_INSTALL.items() if selected)
    | frozenset(path for path, selected in TEST_PATH_INSTALL.items() if selected)
    | frozenset(ALWAYS_INSTALL_PATHS)
)

CLI_CRATE_PREFIX = "crates/clickhousectl/"
CLI_TEST_PREFIX = f"{CLI_CRATE_PREFIX}tests/"
CLOUD_SOURCE_PREFIX = f"{CLI_CRATE_PREFIX}src/cloud/"


def classify_path(path: str) -> bool | None:
    """Return a decision, or None for an unclassified local CLI source/test path."""
    if path in SOURCE_PATH_INSTALL:
        return SOURCE_PATH_INSTALL[path]
    if path in TEST_PATH_INSTALL:
        return TEST_PATH_INSTALL[path]
    if path in ALWAYS_INSTALL_PATHS:
        return INSTALL
    if path.startswith(CLI_TEST_PREFIX):
        return None
    if (
        path.startswith(CLI_CRATE_PREFIX)
        and path.endswith(".rs")
        and not path.startswith(CLOUD_SOURCE_PREFIX)
    ):
        return None
    return NO_INSTALL
