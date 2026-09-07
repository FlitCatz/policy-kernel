import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/rollback-drill.py"
FIXTURES = ROOT / "tests/fixtures/rollback"


class RollbackDrillTests(unittest.TestCase):
    def run_drill(self, changes=None):
        with tempfile.TemporaryDirectory() as directory:
            paths = {}
            for name in ("baseline", "promoted", "restored", "evidence-before", "evidence-after"):
                value = json.loads((FIXTURES / f"{name}.json").read_text())
                if changes and name in changes:
                    changes[name](value)
                path = Path(directory) / f"{name}.json"
                path.write_text(json.dumps(value))
                paths[name] = path
            before = {name: path.read_bytes() for name, path in paths.items()}
            command = ["python3", str(SCRIPT)]
            for name, path in paths.items():
                command.extend([f"--{name}", str(path)])
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(before, {name: path.read_bytes() for name, path in paths.items()})
            return result

    def test_restores_native_checks_and_preserves_evidence_without_writes(self):
        result = self.run_drill()
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertEqual(output["status"], "passed")
        self.assertEqual(output["scope"], "supplied-snapshots-only")
        self.assertFalse(output["live_protection_verified"])

    def test_rejects_weakened_review_or_other_native_protection(self):
        for field, value in [("required_approving_review_count", 0), ("allow_force_pushes", True)]:
            with self.subTest(field=field):
                result = self.run_drill({"restored": lambda snapshot: snapshot.update({field: value})})
                self.assertEqual(result.returncode, 64, result.stderr)
                self.assertEqual(result.stdout, "")

    def test_rejects_retained_kernel_check_or_missing_native_check(self):
        for checks in [[], [{"context": "policy-kernel/pre-merge", "app_id": 123}]]:
            result = self.run_drill({"restored": lambda snapshot: snapshot.update({"required_checks": checks})})
            self.assertEqual(result.returncode, 64, result.stderr)

    def test_rejects_modified_evidence(self):
        result = self.run_drill({"evidence-after": lambda manifest: manifest["artifacts"][0].update({"sha256": "b" * 64})})
        self.assertEqual(result.returncode, 64, result.stderr)

    def test_rejects_unbound_baseline(self):
        result = self.run_drill({"baseline": lambda snapshot: snapshot.update({"repository": "other/repository"})})
        self.assertEqual(result.returncode, 64, result.stderr)

    def test_rejects_empty_evidence_manifest(self):
        result = self.run_drill({name: lambda manifest: manifest.update({"artifacts": []}) for name in ("evidence-before", "evidence-after")})
        self.assertEqual(result.returncode, 64, result.stderr)


if __name__ == "__main__":
    unittest.main()
