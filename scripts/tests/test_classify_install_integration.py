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


class InstallIntegrationClassifierTests(unittest.TestCase):
    def test_path_mapping(self):
        cases = {
            "crates/clickhousectl/src/version_manager/lock.rs": classifier.INSTALL,
            "crates/clickhousectl/src/version_manager/network.rs": classifier.INSTALL,
            "crates/clickhousectl/src/local/mod.rs": classifier.INSTALL,
            "crates/clickhousectl/src/local/postgres.rs": classifier.NO_INSTALL,
            "crates/clickhousectl/src/cloud/client.rs": classifier.NO_INSTALL,
            "crates/clickhousectl/tests/local_version_error_test.rs": classifier.INSTALL,
            "crates/clickhousectl/tests/local_docker_pull_progress_test.rs": classifier.NO_INSTALL,
            "Cargo.lock": classifier.INSTALL,
            "crates/clickhousectl/src/version_manager/new_source.rs": None,
            "crates/clickhousectl/src/local/new_postgres_helper.rs": None,
            "crates/clickhousectl/tests/new_install_test.rs": None,
            "crates/clickhousectl/build.rs": None,
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.assertEqual(classifier.classify_path(path), expected)

    def test_every_non_cloud_cli_source_has_an_explicit_mapping(self):
        crate_root = classifier.REPO_ROOT / "crates" / "clickhousectl"
        actual = {
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in crate_root.rglob("*.rs")
            if path.relative_to(crate_root).parts[0] != "tests"
            and not path.relative_to(crate_root).as_posix().startswith("src/cloud/")
        }
        self.assertFalse(actual - set(classifier.SOURCE_PATH_INSTALL))

    def test_current_source_mapping_values(self):
        expected_install = {
            "crates/clickhousectl/src/cli.rs",
            "crates/clickhousectl/src/error.rs",
            "crates/clickhousectl/src/http.rs",
            "crates/clickhousectl/src/local/cli.rs",
            "crates/clickhousectl/src/local/discovery.rs",
            "crates/clickhousectl/src/local/mod.rs",
            "crates/clickhousectl/src/local/output.rs",
            "crates/clickhousectl/src/local/server.rs",
            "crates/clickhousectl/src/local/symlink.rs",
            "crates/clickhousectl/src/main.rs",
            "crates/clickhousectl/src/paths.rs",
            "crates/clickhousectl/src/update.rs",
            "crates/clickhousectl/src/user_agent.rs",
            "crates/clickhousectl/src/version_manager/download.rs",
            "crates/clickhousectl/src/version_manager/install.rs",
            "crates/clickhousectl/src/version_manager/list.rs",
            "crates/clickhousectl/src/version_manager/lock.rs",
            "crates/clickhousectl/src/version_manager/master.rs",
            "crates/clickhousectl/src/version_manager/mod.rs",
            "crates/clickhousectl/src/version_manager/network.rs",
            "crates/clickhousectl/src/version_manager/platform.rs",
            "crates/clickhousectl/src/version_manager/resolve.rs",
            "crates/clickhousectl/src/version_manager/spec.rs",
        }
        expected_no_install = {
            "crates/clickhousectl/src/dotenv.rs",
            "crates/clickhousectl/src/init.rs",
            "crates/clickhousectl/src/local/config.rs",
            "crates/clickhousectl/src/local/docker.rs",
            "crates/clickhousectl/src/local/postgres.rs",
            "crates/clickhousectl/src/skills.rs",
            "crates/clickhousectl/src/telemetry.rs",
        }
        self.assertEqual(
            {path for path, selected in classifier.SOURCE_PATH_INSTALL.items() if selected},
            expected_install,
        )
        self.assertEqual(
            {path for path, selected in classifier.SOURCE_PATH_INSTALL.items() if not selected},
            expected_no_install,
        )

    def test_every_cli_subprocess_test_has_an_explicit_mapping(self):
        test_root = classifier.REPO_ROOT / "crates" / "clickhousectl" / "tests"
        actual = {
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in test_root.rglob("*")
            if path.is_file()
        }
        self.assertFalse(actual - set(classifier.TEST_PATH_INSTALL))

    def test_current_test_mapping_values(self):
        expected_install = {
            "crates/clickhousectl/tests/local_install_local_first_test.rs",
            "crates/clickhousectl/tests/local_server_start_args_test.rs",
            "crates/clickhousectl/tests/local_version_error_test.rs",
            "crates/clickhousectl/tests/telemetry_test.rs",
        }
        expected_no_install = {
            "crates/clickhousectl/tests/cli_request_shape_test.rs",
            "crates/clickhousectl/tests/local_client_project_scope_test.rs",
            "crates/clickhousectl/tests/local_client_selectors_test.rs",
            "crates/clickhousectl/tests/local_docker_pull_progress_test.rs",
            "crates/clickhousectl/tests/local_postgres_diagnostics_test.rs",
            "crates/clickhousectl/tests/local_postgres_dotenv_lock_test.rs",
            "crates/clickhousectl/tests/local_postgres_help_test.rs",
            "crates/clickhousectl/tests/local_postgres_preflight_test.rs",
            "crates/clickhousectl/tests/local_postgres_readiness_test.rs",
            "crates/clickhousectl/tests/local_postgres_start_lock_test.rs",
            "crates/clickhousectl/tests/local_server_metadata_test.rs",
            "crates/clickhousectl/tests/local_server_project_scope_test.rs",
            "crates/clickhousectl/tests/local_server_readiness_test.rs",
            "crates/clickhousectl/tests/local_server_selection_test.rs",
            "crates/clickhousectl/tests/local_server_state_machine_test.rs",
            "crates/clickhousectl/tests/local_server_stopped_test.rs",
            "crates/clickhousectl/tests/local_structured_errors_test.rs",
            "crates/clickhousectl/tests/snapshots/local_postgres_help.txt",
            "crates/clickhousectl/tests/snapshots/local_postgres_start_help.txt",
        }
        self.assertEqual(
            {path for path, selected in classifier.TEST_PATH_INSTALL.items() if selected},
            expected_install,
        )
        self.assertEqual(
            {path for path, selected in classifier.TEST_PATH_INSTALL.items() if not selected},
            expected_no_install,
        )

    def test_workflow_filter_matches_exact_install_mapping(self):
        workflow = classifier.REPO_ROOT / ".github" / "workflows" / "test-install.yml"
        paths = []
        in_paths = False
        for line in workflow.read_text().splitlines():
            if line == "    paths:":
                in_paths = True
                continue
            if in_paths and line.startswith("      - "):
                paths.append(json.loads(line.removeprefix("      - ")))
            elif in_paths and not paths and line.lstrip().startswith("#"):
                continue
            elif in_paths:
                break

        self.assertEqual(len(paths), len(set(paths)), "workflow path filter has duplicates")
        self.assertFalse(
            {path for path in paths if "*" in path},
            "install workflow paths must be exact",
        )
        self.assertEqual(set(paths), set(classifier.INSTALL_PATHS))

    def test_broad_cli_workflow_runs_fail_closed_inventory(self):
        command = (
            "python3 -m unittest discover -s scripts/tests "
            "-p 'test_classify_install_integration.py'"
        )
        for workflow_name in ("test-cli.yml", "test-install.yml"):
            workflow = (
                classifier.REPO_ROOT / ".github" / "workflows" / workflow_name
            ).read_text()
            with self.subTest(workflow=workflow_name):
                self.assertIn(command, workflow)


if __name__ == "__main__":
    unittest.main()
