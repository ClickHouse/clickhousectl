import importlib.util
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check-release-versions.py"
SPEC = importlib.util.spec_from_file_location("check_release_versions", SCRIPT)
releases = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = releases
SPEC.loader.exec_module(releases)


def write_tree(
    root: Path,
    *,
    cli="1.2.3",
    dep="1.2.3",
    api="1.2.3",
    npm="1.2.3",
    lock_cli="1.2.3",
    lock_api="1.2.3",
):
    (root / "crates/clickhousectl").mkdir(parents=True)
    (root / "crates/clickhouse-cloud-api").mkdir(parents=True)
    (root / "npm").mkdir()
    (root / releases.CLI_MANIFEST).write_text(
        f'[package]\nname = "clickhousectl"\nversion = "{cli}"\n\n'
        "[dependencies]\n"
        f'clickhouse-cloud-api = {{ version = "{dep}", path = "../clickhouse-cloud-api" }}\n'
    )
    (root / releases.API_MANIFEST).write_text(
        f'[package]\nname = "clickhouse-cloud-api"\nversion = "{api}"\n'
    )
    (root / releases.NPM_MANIFEST).write_text(
        json.dumps({"name": "clickhousectl", "version": npm})
    )
    (root / releases.LOCKFILE).write_text(
        "version = 4\n\n"
        f'[[package]]\nname = "clickhousectl"\nversion = "{lock_cli}"\n\n'
        f'[[package]]\nname = "clickhouse-cloud-api"\nversion = "{lock_api}"\n\n'
        '[[package]]\nname = "clap"\nversion = "9.9.9"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
    )


class ReadVersionsTests(unittest.TestCase):
    def test_reads_every_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_tree(root, dep="1.2.4", lock_api="1.2.5")
            versions = releases.read_versions(root)
        self.assertEqual(
            versions,
            {
                "crates/clickhousectl/Cargo.toml package.version": "1.2.3",
                "crates/clickhousectl/Cargo.toml dependencies.clickhouse-cloud-api.version": "1.2.4",
                "crates/clickhouse-cloud-api/Cargo.toml package.version": "1.2.3",
                "npm/package.json version": "1.2.3",
                "Cargo.lock clickhousectl": "1.2.3",
                "Cargo.lock clickhouse-cloud-api": "1.2.5",
            },
        )


class CheckTests(unittest.TestCase):
    AGREED = {"src-one": "1.2.3", "src-two": "1.2.3"}

    def test_matching_tag_passes(self):
        self.assertEqual(releases.check(self.AGREED, "v1.2.3"), [])

    def test_agreeing_manifests_pass_without_a_tag(self):
        self.assertEqual(releases.check(self.AGREED, None), [])

    def test_tag_names_each_mismatched_source(self):
        errors = releases.check({"src-one": "1.2.3", "src-two": "1.2.4"}, "v1.2.4")
        self.assertEqual(len(errors), 1)
        self.assertIn("src-one", errors[0])
        self.assertIn("1.2.3", errors[0])
        self.assertNotIn("src-two", errors[0])

    def test_tag_must_be_plain_semver(self):
        for tag in ("1.2.3", "v1.2", "v1.2.3-rc.1", "v1.2.3\n", "main"):
            with self.subTest(tag=tag):
                errors = releases.check(self.AGREED, tag)
                self.assertEqual(len(errors), 1)
                self.assertIn(repr(tag), errors[0])

    def test_disagreement_without_a_tag_lists_every_source(self):
        errors = releases.check({"src-one": "1.2.3", "src-two": "1.2.4"}, None)
        self.assertTrue(any("src-one" in e and "1.2.3" in e for e in errors))
        self.assertTrue(any("src-two" in e and "1.2.4" in e for e in errors))

    def test_missing_source_fails(self):
        for tag in (None, "v1.2.3"):
            with self.subTest(tag=tag):
                errors = releases.check({"src-one": "1.2.3", "src-two": None}, tag)
                self.assertEqual(len(errors), 1)
                self.assertIn("src-two", errors[0])


class MainTests(unittest.TestCase):
    def run_main(self, root: Path, *args: str) -> int:
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            return releases.main(["--root", str(root), *args])

    def test_exit_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            agreed = Path(tmp) / "agreed"
            write_tree(agreed)
            missed_bump = Path(tmp) / "missed"
            write_tree(missed_bump, npm="1.2.2")
            self.assertEqual(self.run_main(agreed), 0)
            self.assertEqual(self.run_main(agreed, "--tag", "v1.2.3"), 0)
            self.assertEqual(self.run_main(agreed, "--tag", "v1.2.4"), 1)
            self.assertEqual(self.run_main(missed_bump), 1)


if __name__ == "__main__":
    unittest.main()
