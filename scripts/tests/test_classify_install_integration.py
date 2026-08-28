import importlib.util
import json
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "classify-install-integration.py"
SPEC = importlib.util.spec_from_file_location("classify_install_integration", SCRIPT)
classifier = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = classifier
SPEC.loader.exec_module(classifier)


def pull_request_paths(workflow: Path) -> set[str]:
    """Read the simple pull_request.paths sequence without a YAML dependency."""
    in_pull_request = False
    in_paths = False
    paths = set()
    for line in workflow.read_text().splitlines():
        if line == "  pull_request:":
            in_pull_request = True
            continue
        if in_pull_request and line == "    paths:":
            in_paths = True
            continue
        if in_paths and line.startswith("      - "):
            paths.add(json.loads(line.removeprefix("      - ")))
            continue
        if in_paths and line.strip():
            break
        if in_pull_request and line and not line.startswith("    "):
            break
    return paths


class InstallIntegrationClassifierTests(unittest.TestCase):
    def test_positive_and_negative_paths(self):
        cases = {
            "crates/clickhousectl/src/version_manager/future.rs": True,
            "crates/clickhousectl/src/local/mod.rs": True,
            "crates/clickhousectl/src/http.rs": True,
            "crates/clickhousectl/tests/local_install_local_first_test.rs": True,
            "Cargo.lock": True,
            "scripts/classify-install-integration.py": True,
            "crates/clickhousectl/src/local/postgres.rs": False,
            "crates/clickhousectl/src/local/docker.rs": False,
            "crates/clickhousectl/tests/local_client_selectors_test.rs": False,
            "crates/clickhousectl/tests/local_docker_pull_progress_test.rs": False,
            "crates/clickhousectl/tests/local_postgres_readiness_test.rs": False,
            "crates/clickhousectl/tests/local_server_metadata_test.rs": False,
            "crates/clickhousectl/src/cloud/services.rs": False,
            "README.md": False,
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.assertEqual(classifier.classify_path(path), expected)

    def test_new_or_renamed_candidates_are_unknown(self):
        for path in (
            "crates/clickhousectl/src/new_shared.rs",
            "crates/clickhousectl/src/local/new_installer_helper.rs",
            "crates/clickhousectl/tests/new_install_subprocess_test.rs",
            "crates/clickhousectl/tests/snapshots/new_install.snap",
        ):
            with self.subTest(path=path):
                self.assertIsNone(classifier.classify_path(path))

    def test_current_cli_sources_and_subprocess_tests_are_classified(self):
        crate = classifier.REPO_ROOT / "crates" / "clickhousectl"
        candidates = [*sorted((crate / "src").rglob("*.rs"))]
        candidates.extend(
            path for path in sorted((crate / "tests").rglob("*")) if path.is_file()
        )
        unknown = [
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in candidates
            if classifier.classify_path(
                path.relative_to(classifier.REPO_ROOT).as_posix()
            )
            is None
        ]
        self.assertEqual(unknown, [])

    def test_exact_path_mappings(self):
        self.assertEqual(
            classifier.INSTALL_EXACT_PATHS,
            frozenset(
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
            ),
        )
        self.assertEqual(
            classifier.NON_INSTALL_EXACT_PATHS,
            frozenset(
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
                    "crates/clickhousectl/tests/local_init_json_test.rs",
                    "crates/clickhousectl/tests/local_postgres_readiness_test.rs",
                    "crates/clickhousectl/tests/local_postgres_start_validation_test.rs",
                    "crates/clickhousectl/tests/local_remove_default_test.rs",
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
            ),
        )

    def test_workflow_filter_exactly_matches_classifier(self):
        workflow = classifier.REPO_ROOT / ".github" / "workflows" / "test-install.yml"
        self.assertEqual(
            pull_request_paths(workflow), classifier.workflow_path_patterns()
        )

    def test_install_mapping_check_runs_in_install_and_broad_cli_ci(self):
        command = "python3 scripts/tests/test_classify_install_integration.py"
        for name in ("test-install.yml", "test-cli.yml"):
            workflow = classifier.REPO_ROOT / ".github" / "workflows" / name
            with self.subTest(workflow=name):
                self.assertIn(command, workflow.read_text())


if __name__ == "__main__":
    unittest.main()
