import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
POSTGRES_SCRIPT = REPO_ROOT / "scripts" / "test-postgres-integration.sh"
EXPECTED_CLI_TEST_SCRIPTS = {"scripts/test-postgres-integration.sh"}
CLI_REFERENCE = re.compile(r"clickhousectl|\bchctl\b|\$(?:\{)?(?:CLICKHOUSECTL|CTL)")
OPT_OUT_EXPORT = re.compile(
    r"^export[ \t]+DO_NOT_TRACK=(?:1|'1'|\"1\")[ \t]*(?:#.*)?$", re.MULTILINE
)


def executable_cli_test_scripts():
    tracked = subprocess.run(
        ["git", "ls-files", "--stage", "-z"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    scripts = {}
    for entry in tracked.split("\0"):
        if not entry:
            continue
        metadata, relative = entry.split("\t", 1)
        mode = metadata.split(" ", 1)[0]
        path = Path(relative)
        name = path.name.lower()
        parts = {part.lower() for part in path.parts[:-1]}
        is_test_or_evaluation = (
            name.startswith(("test-", "test_", "eval-", "eval_"))
            or parts.intersection(
                {"test", "tests", "evaluation", "evaluations", "eval", "evals"}
            )
        )
        if mode != "100755" or not is_test_or_evaluation:
            continue

        content = (REPO_ROOT / path).read_text()
        if CLI_REFERENCE.search(content):
            scripts[path.as_posix()] = content
    return scripts


class ScriptTelemetryTests(unittest.TestCase):
    def test_executable_cli_test_scripts_export_opt_out(self):
        scripts = executable_cli_test_scripts()
        self.assertEqual(set(scripts), EXPECTED_CLI_TEST_SCRIPTS)
        for path, content in scripts.items():
            with self.subTest(path=path):
                self.assertRegex(content, OPT_OUT_EXPORT)

    def test_postgres_script_cli_processes_inherit_opt_out(self):
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            bin_dir = temp / "bin"
            bin_dir.mkdir()
            observed = temp / "observed"

            fake_cli = bin_dir / "clickhousectl"
            fake_cli.write_text(
                '#!/bin/sh\nprintf \'%s\\n\' "${DO_NOT_TRACK-unset}" '
                '>> "$OBSERVED_DNT"\nexit 1\n'
            )
            fake_cli.chmod(0o755)
            for dependency in ("docker", "jq"):
                fake = bin_dir / dependency
                fake.write_text("#!/bin/sh\nexit 0\n")
                fake.chmod(0o755)

            env = os.environ.copy()
            env.pop("DO_NOT_TRACK", None)
            env["OBSERVED_DNT"] = str(observed)
            env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
            result = subprocess.run(
                [POSTGRES_SCRIPT, fake_cli],
                cwd=REPO_ROOT,
                env=env,
                capture_output=True,
                text=True,
                timeout=20,
            )

            values = observed.read_text().splitlines() if observed.exists() else []
            self.assertTrue(values, result.stdout + result.stderr)
            self.assertEqual(set(values), {"1"})


if __name__ == "__main__":
    unittest.main()
