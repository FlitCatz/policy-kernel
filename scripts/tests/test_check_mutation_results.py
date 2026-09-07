import json
from pathlib import Path
import subprocess
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
CHECKER = REPOSITORY / "scripts" / "check-mutation-results.py"
FILES = [
    "crates/policy-core/src/canonical.rs",
    "crates/policy-core/src/provenance.rs",
    "crates/policy-core/src/reviews.rs",
    "crates/policy-core/src/pre_merge.rs",
    "crates/policy-core/src/post_merge.rs",
]


def valid_results():
    outcomes = [
        {
            "scenario": "Baseline",
            "summary": "Success",
            "log_path": "log/baseline.log",
            "diff_path": None,
            "phase_results": [],
        }
    ]
    for index, file_name in enumerate(FILES):
        outcomes.append(
            {
                "scenario": {
                    "Mutant": {
                        "name": f"{file_name}:{index + 1}: replace expression",
                        "package": "policy-core",
                        "file": file_name,
                        "function": None,
                        "span": {},
                        "replacement": "false",
                        "genre": "FnValue",
                    }
                },
                "summary": "CaughtMutant",
                "log_path": f"log/mutant-{index}.log",
                "diff_path": f"diff/mutant-{index}.diff",
                "phase_results": [],
            }
        )
    return {
        "outcomes": outcomes,
        "total_mutants": 5,
        "missed": 0,
        "caught": 5,
        "timeout": 0,
        "unviable": 0,
        "success": 0,
        "start_time": "2026-09-07T00:00:00Z",
        "end_time": "2026-09-07T00:01:00Z",
        "cargo_mutants_version": "27.1.0",
    }


class MutationResultCheckerTests(unittest.TestCase):
    def run_checker(self, payload=None, raw=None, inventory=None, omit_inventory=False):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "outcomes.json"
            if raw is not None:
                path.write_text(raw, encoding="utf-8")
            elif payload is not None:
                path.write_text(json.dumps(payload), encoding="utf-8")
            if not omit_inventory and payload is not None:
                if inventory is None:
                    inventory = [item["scenario"]["Mutant"] for item in payload["outcomes"] if isinstance(item["scenario"], dict)]
                (Path(temp_dir) / "mutants.json").write_text(json.dumps(inventory), encoding="utf-8")
            return subprocess.run(
                ["python3", str(CHECKER), str(path)],
                text=True,
                capture_output=True,
                check=False,
            )

    def test_complete_results_with_all_mutants_caught_pass(self):
        result = self.run_checker(valid_results())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("5 mutants across 5 authorization-critical files", result.stdout)

    def test_missing_generated_inventory_fails(self):
        result = self.run_checker(valid_results(), omit_inventory=True)
        self.assertNotEqual(result.returncode, 0)

    def test_unexecuted_inventory_entry_fails_even_with_consistent_counters(self):
        payload = valid_results()
        inventory = [item["scenario"]["Mutant"] for item in payload["outcomes"][1:]]
        inventory.append(dict(inventory[0], name="additional unexecuted mutant"))
        result = self.run_checker(payload, inventory=inventory)
        self.assertNotEqual(result.returncode, 0)

    def test_duplicate_mutant_outcomes_cannot_inflate_coverage(self):
        payload = valid_results()
        payload["outcomes"].append(payload["outcomes"][1])
        payload["total_mutants"] += 1
        payload["caught"] += 1
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)

    def test_missed_mutant_fails(self):
        payload = valid_results()
        payload["missed"] = 1
        payload["caught"] = 4
        payload["outcomes"][1]["summary"] = "MissedMutant"
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missed mutants", result.stderr)

    def test_timeout_fails(self):
        payload = valid_results()
        payload["timeout"] = 1
        payload["caught"] = 4
        payload["outcomes"][1]["summary"] = "Timeout"
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("timed-out mutants", result.stderr)

    def test_empty_mutation_coverage_fails(self):
        payload = valid_results()
        payload["outcomes"] = payload["outcomes"][:1]
        payload["total_mutants"] = 0
        payload["caught"] = 0
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no mutants were exercised", result.stderr)

    def test_missing_required_file_coverage_fails(self):
        payload = valid_results()
        payload["outcomes"] = payload["outcomes"][:-1]
        payload["total_mutants"] = 4
        payload["caught"] = 4
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing mutation coverage", result.stderr)

    def test_mutant_outside_exact_file_set_fails(self):
        payload = valid_results()
        payload["outcomes"][1]["scenario"]["Mutant"]["file"] = (
            "crates/policy-core/src/lib.rs"
        )
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unexpected mutated files", result.stderr)

    def test_baseline_failure_fails(self):
        payload = valid_results()
        payload["outcomes"][0]["summary"] = "Failure"
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("baseline did not succeed", result.stderr)

    def test_incomplete_counters_fail(self):
        payload = valid_results()
        payload["total_mutants"] = 6
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("counter mismatch", result.stderr)

    def test_malformed_json_fails(self):
        result = self.run_checker(raw="{not-json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid outcomes JSON", result.stderr)

    def test_wrong_cargo_mutants_version_fails(self):
        payload = valid_results()
        payload["cargo_mutants_version"] = "27.0.0"
        result = self.run_checker(payload)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unexpected cargo-mutants version", result.stderr)

    def test_missing_results_file_fails(self):
        result = subprocess.run(
            ["python3", str(CHECKER), "/definitely/missing/outcomes.json"],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot read outcomes JSON", result.stderr)


if __name__ == "__main__":
    unittest.main()
