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
        ):
            with self.subTest(path=path):
                self.assertIsNone(classifier.classify_path(path))

    def test_unknown_candidate_fails_closed(self):
        run, unknown = classifier.classify_paths(
            ["README.md", "crates/clickhousectl/src/local/new_helper.rs"]
        )
        self.assertTrue(run)
        self.assertEqual(
            unknown, ("crates/clickhousectl/src/local/new_helper.rs",)
        )

    def test_current_cli_sources_and_subprocess_tests_are_classified(self):
        crate = classifier.REPO_ROOT / "crates" / "clickhousectl"
        candidates = [*sorted((crate / "src").rglob("*.rs"))]
        candidates.extend(sorted((crate / "tests").rglob("*.rs")))
        unknown = [
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in candidates
            if classifier.classify_path(
                path.relative_to(classifier.REPO_ROOT).as_posix()
            )
            is None
        ]
        self.assertEqual(unknown, [])

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
