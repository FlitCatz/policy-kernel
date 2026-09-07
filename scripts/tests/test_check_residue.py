import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
CHECKER = REPOSITORY / "scripts" / "check-residue.sh"


class ResidueCheckerTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name) / "repo"
        (self.root / "crates" / "policy-core" / "src").mkdir(parents=True)
        (self.root / "schemas").mkdir()
        (self.root / "action").mkdir()
        (self.root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
        (self.root / "crates" / "policy-core" / "src" / "lib.rs").write_text(
            "pub enum KnowledgeKind { LatClaim, CodeGraphIndex }\n",
            encoding="utf-8",
        )
        (self.root / "schemas" / "policy.json").write_text("{}\n", encoding="utf-8")
        (self.root / "action" / "action.yml").write_text("name: gate\n", encoding="utf-8")

        self.bin_dir = Path(self.temp_dir.name) / "bin"
        self.bin_dir.mkdir()
        self.real_rg = shutil.which("rg")
        if self.real_rg is None:
            self.skipTest("rg is required to exercise the residue checker")
        self._write_executable(
            "rg",
            f'#!/usr/bin/env bash\nif [[ "${{RG_FAIL:-0}}" == 1 ]]; then exit 2; fi\nexec "{self.real_rg}" "$@"\n',
        )
        self._write_executable(
            "cargo",
            "#!/usr/bin/env bash\n"
            "[[ ${1:-} == tree ]] || exit 64\n"
            "[[ ${CARGO_TREE_FAIL:-0} != 1 ]] || exit 7\n"
            "printf '%s\\n' \"${FAKE_CARGO_TREE:-policy-core v0.1.0}\"\n",
        )

    def tearDown(self):
        self.temp_dir.cleanup()

    def _write_executable(self, name, contents):
        path = self.bin_dir / name
        path.write_text(contents, encoding="utf-8")
        path.chmod(0o755)

    def run_checker(self, **environment):
        env = os.environ.copy()
        env["PATH"] = f"{self.bin_dir}:{env['PATH']}"
        env.update(environment)
        return subprocess.run(
            ["bash", str(CHECKER), str(self.root)],
            text=True,
            capture_output=True,
            env=env,
            check=False,
        )

    def test_clean_runtime_and_accepted_claim_names_pass(self):
        result = self.run_checker()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_forbidden_runtime_residue_fails(self):
        (self.root / "action" / "action.yml").write_text(
            "name: old gsd-core bridge\n", encoding="utf-8"
        )
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("forbidden runtime residue found", result.stderr)

    def test_forbidden_policy_core_dependency_fails(self):
        result = self.run_checker(
            FAKE_CARGO_TREE="policy-core v0.1.0\n└── tokio v1.0.0"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("forbidden policy-core dependency found", result.stderr)

    def test_missing_required_scan_path_fails(self):
        (self.root / "action" / "action.yml").unlink()
        (self.root / "action").rmdir()
        result = self.run_checker()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("required residue scan path is missing", result.stderr)

    def test_search_error_fails_closed(self):
        result = self.run_checker(RG_FAIL="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("residue search failed", result.stderr)

    def test_dependency_tree_error_fails_closed(self):
        result = self.run_checker(CARGO_TREE_FAIL="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cargo tree failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
