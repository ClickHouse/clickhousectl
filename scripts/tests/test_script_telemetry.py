import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
POSTGRES_INTEGRATION_SCRIPT = REPO_ROOT / "scripts" / "test-postgres-integration.sh"


class ScriptTelemetryTests(unittest.TestCase):
    def test_postgres_integration_cli_processes_inherit_do_not_track(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp = Path(temp_dir)
            log = temp / "do-not-track.log"
            cli = self._write_executable(
                temp / "clickhousectl",
                '#!/bin/sh\nprintf \'%s\\n\' "${DO_NOT_TRACK-unset}" >> "$DNT_LOG"\nexit 1\n',
            )
            self._write_executable(temp / "docker", "#!/bin/sh\nexit 0\n")
            self._write_executable(temp / "jq", "#!/bin/sh\nexit 0\n")

            env = os.environ.copy()
            env["DO_NOT_TRACK"] = "0"
            env["DNT_LOG"] = str(log)
            env["PATH"] = f"{temp}{os.pathsep}{env['PATH']}"
            result = subprocess.run(
                ["bash", str(POSTGRES_INTEGRATION_SCRIPT), str(cli)],
                env=env,
                capture_output=True,
                text=True,
                timeout=10,
            )

            self.assertNotEqual(result.returncode, 0, "the fake CLI should fail the cases")
            inherited_values = log.read_text().splitlines()
            self.assertTrue(inherited_values, "the integration script did not invoke the CLI")
            self.assertEqual(set(inherited_values), {"1"})

    @staticmethod
    def _write_executable(path: Path, contents: str) -> Path:
        path.write_text(contents)
        path.chmod(0o755)
        return path


if __name__ == "__main__":
    unittest.main()
