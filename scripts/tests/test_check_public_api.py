import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
CHECKER = REPOSITORY / "scripts" / "check-public-api.sh"


class PublicApiCheckerTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.docs = self.root / "doc"
        self.schemas = self.root / "schemas"
        (self.docs / "policy_core").mkdir(parents=True)
        (self.docs / "policy_contracts").mkdir()
        self.schemas.mkdir()
        (self.docs / "policy_core" / "index.html").write_text(
            "public policy kernel API", encoding="utf-8"
        )
        (self.docs / "policy_contracts" / "index.html").write_text(
            "LatClaim CodeGraphIndex", encoding="utf-8"
        )
        (self.schemas / "policy.json").write_text("{}\n", encoding="utf-8")

    def tearDown(self):
        self.temp_dir.cleanup()

    def run_checker(self, docs=None, schemas=None, **environment):
        env = os.environ.copy()
        env.update(environment)
        return subprocess.run(
            [
                "bash",
                str(CHECKER),
                str(docs or self.docs),
                str(schemas or self.schemas),
            ],
            text=True,
            capture_output=True,
            env=env,
            check=False,
        )

    def test_clean_api_schema_and_accepted_claim_names_pass(self):
        result = self.run_checker()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_forbidden_exported_name_fails(self):
        (self.docs / "policy_core" / "struct.Legacy.html").write_text(
            "hashline runtime", encoding="utf-8"
        )
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("forbidden public API or schema residue found", result.stderr)
        self.assertIn("forbidden public API or schema residue found", result.stderr)

    def test_forbidden_schema_name_fails(self):
        (self.schemas / "policy.json").write_text(
            '{"description":"gemini-reviewer"}\n', encoding="utf-8"
        )
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)

    def test_missing_generated_docs_fail(self):
        result = self.run_checker(docs=self.root / "missing")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("required public API scan path is missing", result.stderr)

    def test_empty_generated_docs_fail(self):
        (self.docs / "policy_core" / "index.html").unlink()
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)

    def test_hidden_public_api_residue_fails(self):
        (self.docs / "policy_core" / ".hidden.html").write_text("gsd-core")
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)

    def test_search_error_fails_closed(self):
        fake_bin = self.root / "bin"
        fake_bin.mkdir()
        fake_rg = fake_bin / "rg"
        fake_rg.write_text("#!/usr/bin/env bash\nexit 2\n", encoding="utf-8")
        fake_rg.chmod(0o755)
        result = self.run_checker(PATH=f"{fake_bin}:{os.environ['PATH']}")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("public API residue search failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
