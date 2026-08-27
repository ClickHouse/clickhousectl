import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "classify-cloud-integration.py"
SPEC = importlib.util.spec_from_file_location("classify_cloud_integration", SCRIPT)
classifier = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = classifier
SPEC.loader.exec_module(classifier)


class CloudIntegrationClassifierTests(unittest.TestCase):
    def test_path_mapping(self):
        cases = {
            "crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json": classifier.NO_SUITES,
            "crates/clickhouse-cloud-api/README.md": classifier.NO_SUITES,
            "crates/clickhousectl/src/cloud/client.rs": classifier.NO_SUITES,
            "Cargo.lock": classifier.ALL_SUITES,
            "crates/clickhousectl/Cargo.toml": classifier.ALL_SUITES,
            ".cargo/config.toml": classifier.ALL_SUITES,
            ".github/workflows/cloud-integration.yml": classifier.ALL_SUITES,
            "scripts/classify-cloud-integration.py": classifier.ALL_SUITES,
            "crates/clickhouse-cloud-api/src/new_domain.rs": None,
            "crates/clickhouse-cloud-api/src/template.txt": None,
            "crates/clickhouse-cloud-api/tests/new_live_test.rs": None,
            "crates/clickhouse-cloud-api/tests/clickpipes/postgres_cdc_test.rs": frozenset(
                {"clickpipes"}
            ),
            "crates/clickhouse-cloud-api/tests/clickpipes/postgres_cli_cdc_test.rs": frozenset(
                {"clickpipes"}
            ),
            "crates/clickhouse-cloud-api/build.rs": None,
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.assertEqual(classifier.classify_path(path), expected)

    def test_every_cloud_api_source_file_has_an_explicit_mapping(self):
        crate_root = classifier.REPO_ROOT / "crates" / "clickhouse-cloud-api"
        actual = {
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in crate_root.rglob("*.rs")
            if path.relative_to(crate_root).parts[0] != "tests"
        }
        self.assertFalse(actual - set(classifier.SOURCE_PATH_SUITES))

    def test_current_source_mapping_values(self):
        expected = {
            classifier.ALL_SUITES: {
                "crates/clickhouse-cloud-api/src/client.rs",
                "crates/clickhouse-cloud-api/src/convert.rs",
                "crates/clickhouse-cloud-api/src/convert/shared.rs",
                "crates/clickhouse-cloud-api/src/error.rs",
                "crates/clickhouse-cloud-api/src/lib.rs",
                "crates/clickhouse-cloud-api/src/models.rs",
                "crates/clickhouse-cloud-api/src/models/shared.rs",
                "crates/clickhouse-cloud-api/src/serde_helpers.rs",
            },
            frozenset({"service", "clickpipes"}): {
                "crates/clickhouse-cloud-api/src/client/services.rs",
                "crates/clickhouse-cloud-api/src/convert/service.rs",
                "crates/clickhouse-cloud-api/src/models/services.rs",
            },
            frozenset({"postgres", "clickpipes"}): {
                "crates/clickhouse-cloud-api/src/client/postgres.rs",
                "crates/clickhouse-cloud-api/src/convert/postgres.rs",
                "crates/clickhouse-cloud-api/src/models/postgres.rs",
            },
            frozenset({"clickpipes"}): {
                "crates/clickhouse-cloud-api/src/client/clickpipes.rs",
                "crates/clickhouse-cloud-api/src/models/clickpipes.rs",
            },
            frozenset({"service", "organization"}): {
                "crates/clickhouse-cloud-api/src/client/api_keys.rs",
                "crates/clickhouse-cloud-api/src/client/organizations.rs",
                "crates/clickhouse-cloud-api/src/models/api_keys.rs",
                "crates/clickhouse-cloud-api/src/models/byoc.rs",
                "crates/clickhouse-cloud-api/src/models/organization_private_endpoints.rs",
                "crates/clickhouse-cloud-api/src/models/organizations.rs",
            },
            frozenset({"organization"}): {
                "crates/clickhouse-cloud-api/src/client/activity.rs",
                "crates/clickhouse-cloud-api/src/models/activity.rs",
                "crates/clickhouse-cloud-api/src/models/invitations.rs",
                "crates/clickhouse-cloud-api/src/models/members.rs",
                "crates/clickhouse-cloud-api/src/models/rbac.rs",
            },
            classifier.NO_SUITES: {
                "crates/clickhouse-cloud-api/src/client/backups.rs",
                "crates/clickhouse-cloud-api/src/client/clickstack.rs",
                "crates/clickhouse-cloud-api/src/client/udfs.rs",
                "crates/clickhouse-cloud-api/src/convert/clickstack.rs",
                "crates/clickhouse-cloud-api/src/meta.rs",
                "crates/clickhouse-cloud-api/src/models/backups.rs",
                "crates/clickhouse-cloud-api/src/models/clickstack.rs",
                "crates/clickhouse-cloud-api/src/models/clickstack_enums.rs",
                "crates/clickhouse-cloud-api/src/models/quotas.rs",
                "crates/clickhouse-cloud-api/src/models/scim.rs",
                "crates/clickhouse-cloud-api/src/models/udfs.rs",
            },
        }
        source_root = (
            classifier.REPO_ROOT / "crates" / "clickhouse-cloud-api" / "src"
        )
        actual = {}
        for path in source_root.rglob("*.rs"):
            relative = path.relative_to(classifier.REPO_ROOT).as_posix()
            actual.setdefault(classifier.SOURCE_PATH_SUITES[relative], set()).add(
                relative
            )
        self.assertEqual(actual, expected)

    def test_every_current_cloud_api_test_file_has_an_explicit_mapping(self):
        test_root = (
            classifier.REPO_ROOT / "crates" / "clickhouse-cloud-api" / "tests"
        )
        actual = {
            path.relative_to(classifier.REPO_ROOT).as_posix()
            for path in test_root.rglob("*")
            if path.is_file()
        }
        self.assertFalse(actual - set(classifier.TEST_PATH_SUITES))

    def test_current_test_mapping_values(self):
        expected = {
            classifier.ALL_SUITES: {
                "crates/clickhouse-cloud-api/tests/common/mod.rs",
                "crates/clickhouse-cloud-api/tests/common/support.rs",
            },
            frozenset({"service"}): {
                "crates/clickhouse-cloud-api/tests/integration_test.rs"
            },
            frozenset({"postgres"}): {
                "crates/clickhouse-cloud-api/tests/integration_postgres_test.rs"
            },
            frozenset({"organization"}): {
                "crates/clickhouse-cloud-api/tests/integration_org_test.rs"
            },
            frozenset({"clickpipes"}): {
                "crates/clickhouse-cloud-api/tests/clickpipes/postgres_cli_cdc_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/smoke_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/support.rs",
            },
            classifier.NO_SUITES: {
                "crates/clickhouse-cloud-api/tests/clickpipes/driver.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/e2e_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/kafka_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/kinesis_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/mongo_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/mysql_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/postgres_ec2_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/s3_test.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/kafka.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/kinesis.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/mod.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/mongo.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/mongo_user_data.sh.template",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/mysql.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/mysql_user_data.sh.template",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/postgres.rs",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/postgres_user_data.sh.template",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/redpanda_user_data_mtls.sh.template",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/redpanda_user_data_scram_tls.sh.template",
                "crates/clickhouse-cloud-api/tests/clickpipes/stages/s3.rs",
                "crates/clickhouse-cloud-api/tests/client_test.rs",
                "crates/clickhouse-cloud-api/tests/model_facade_test.rs",
                "crates/clickhouse-cloud-api/tests/models_test.rs",
                "crates/clickhouse-cloud-api/tests/run_query_test.rs",
                "crates/clickhouse-cloud-api/tests/spec_coverage_test.rs",
            },
        }
        test_root = (
            classifier.REPO_ROOT / "crates" / "clickhouse-cloud-api" / "tests"
        )
        actual = {}
        for path in test_root.rglob("*"):
            if path.is_file():
                relative = path.relative_to(classifier.REPO_ROOT).as_posix()
                actual.setdefault(classifier.TEST_PATH_SUITES[relative], set()).add(
                    relative
                )
        self.assertEqual(actual, expected)

    def test_parses_add_modify_delete_rename_and_copy_records(self):
        data = (
            b"A\0added\0M\0modified\0D\0deleted\0"
            b"R100\0rename-old\0rename-new\0C75\0copy-old\0copy-new\0"
        )
        self.assertEqual(
            classifier.parse_name_status(data),
            [
                ("A", ("added",)),
                ("M", ("modified",)),
                ("D", ("deleted",)),
                ("R100", ("rename-old", "rename-new")),
                ("C75", ("copy-old", "copy-new")),
            ],
        )

    def test_rename_and_copy_union_both_paths_in_canonical_order(self):
        cases = [
            (
                "R100",
                "crates/clickhouse-cloud-api/src/client/services.rs",
                "crates/clickhouse-cloud-api/src/models/activity.rs",
                ("service", "organization", "clickpipes"),
            ),
            (
                "C100",
                "crates/clickhouse-cloud-api/src/models/activity.rs",
                "crates/clickhouse-cloud-api/src/models/postgres.rs",
                ("postgres", "organization", "clickpipes"),
            ),
        ]
        for status, old_path, new_path, expected in cases:
            with self.subTest(status=status):
                selection = classifier.select_records(
                    [(status, (old_path, new_path))]
                )
                self.assertEqual(selection.suites, expected)

    def test_retained_known_none_deletion_and_rename_stay_none(self):
        historical = (
            "crates/clickhouse-cloud-api/tests/integration_clickpipe_s3_test.rs"
        )
        cases = [
            [("D", (historical,))],
            [
                (
                    "R100",
                    (
                        historical,
                        "crates/clickhouse-cloud-api/tests/clickpipes/s3_test.rs",
                    ),
                )
            ],
        ]
        with mock.patch.dict(
            classifier.TEST_PATH_SUITES,
            {historical: classifier.NO_SUITES},
        ):
            for records in cases:
                with self.subTest(records=records):
                    selection = classifier.select_records(records)
                    self.assertEqual(selection.suites, ())
                    self.assertFalse(selection.failed_closed)

    def test_known_none_is_distinct_from_unknown(self):
        known = classifier.select_records(
            [("M", ("crates/clickhouse-cloud-api/src/meta.rs",))]
        )
        unknown = classifier.select_records(
            [("M", ("crates/clickhouse-cloud-api/src/new_domain.rs",))]
        )
        self.assertEqual(known.suites, ())
        self.assertFalse(known.failed_closed)
        self.assertEqual(unknown.suites, classifier.SUITES)
        self.assertTrue(unknown.failed_closed)

    def test_unknown_suite_token_in_mapping_fails_closed(self):
        path = "crates/clickhouse-cloud-api/src/meta.rs"
        with mock.patch.dict(
            classifier.SOURCE_PATH_SUITES,
            {path: frozenset({"future-suite"})},
        ):
            selection = classifier.select_records([("M", (path,))])
        self.assertEqual(selection.suites, classifier.SUITES)
        self.assertTrue(selection.failed_closed)
        self.assertIn("future-suite", selection.reason)

    def test_malformed_or_unsupported_records_fail_closed(self):
        cases = {
            "not NUL terminated": b"M\0path",
            "rename missing destination": b"R100\0old\0",
            "unsupported status": b"T\0path\0",
            "malformed rename score": b"Rbad\0old\0new\0",
            "empty path": b"A\0\0",
            "non-UTF-8 path": b"M\0\xff\0",
        }
        for name, data in cases.items():
            with self.subTest(name=name):
                selection = classifier.select_name_status(data)
                self.assertEqual(selection.suites, classifier.SUITES)
                self.assertTrue(selection.failed_closed)

    @mock.patch.object(classifier.subprocess, "run")
    def test_diffs_from_merge_base_with_rename_and_copy_detection(self, run):
        run.return_value = subprocess.CompletedProcess(
            args=[],
            returncode=0,
            stdout=b"M\0crates/clickhouse-cloud-api/src/meta.rs\0",
            stderr=b"",
        )
        selection = classifier.select_revisions("base-sha", "head-sha")
        self.assertEqual(selection.suites, ())
        run.assert_called_once_with(
            [
                "git",
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                "--find-copies",
                "--find-copies-harder",
                "base-sha...head-sha",
                "--",
            ],
            cwd=classifier.REPO_ROOT,
            capture_output=True,
        )

    def test_stale_base_changes_are_excluded(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            repo = Path(temp_dir)

            def git(*args):
                return subprocess.run(
                    ["git", *args],
                    cwd=repo,
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip()

            git("init", "-b", "main")
            git("config", "user.name", "Cloud Integration Test")
            git("config", "user.email", "cloud-integration-test@example.com")
            git("commit", "--allow-empty", "-m", "initial")

            git("checkout", "-b", "feature")
            source_root = repo / "crates" / "clickhouse-cloud-api" / "src"
            source_root.mkdir(parents=True)
            (source_root / "meta.rs").write_text("feature\n")
            git("add", ".")
            git("commit", "-m", "feature")
            head_sha = git("rev-parse", "HEAD")

            git("checkout", "main")
            source_root.mkdir(parents=True)
            (source_root / "services.rs").write_text("base advance\n")
            git("add", ".")
            git("commit", "-m", "advance base")
            base_sha = git("rev-parse", "HEAD")

            with mock.patch.object(classifier, "REPO_ROOT", repo):
                selection = classifier.select_revisions(base_sha, head_sha)

        self.assertEqual(selection.suites, ())
        self.assertFalse(selection.failed_closed)

    def test_git_failures_fail_closed(self):
        cases = [
            OSError("git unavailable"),
            subprocess.CompletedProcess(
                args=[], returncode=128, stdout=b"", stderr=b"bad revision"
            ),
        ]
        for result in cases:
            with self.subTest(result=result):
                with mock.patch.object(classifier.subprocess, "run") as run:
                    if isinstance(result, Exception):
                        run.side_effect = result
                    else:
                        run.return_value = result
                    selection = classifier.select_revisions("base", "head")
                self.assertEqual(selection.suites, classifier.SUITES)
                self.assertTrue(selection.failed_closed)

    def test_formats_none_and_canonical_suites(self):
        self.assertEqual(classifier.format_suites(()), "none")
        self.assertEqual(
            classifier.format_suites(classifier.SUITES),
            "service,postgres,organization,clickpipes",
        )


if __name__ == "__main__":
    unittest.main()
