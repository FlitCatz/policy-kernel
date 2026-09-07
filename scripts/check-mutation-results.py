#!/usr/bin/env python3
"""Fail closed unless cargo-mutants produced complete promotion evidence."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import NoReturn


EXPECTED_VERSION = "27.1.0"
EXPECTED_FILES = {
    "crates/policy-core/src/canonical.rs",
    "crates/policy-core/src/provenance.rs",
    "crates/policy-core/src/reviews.rs",
    "crates/policy-core/src/pre_merge.rs",
    "crates/policy-core/src/post_merge.rs",
}
COUNTERS = ("missed", "caught", "timeout", "unviable", "success")
SUMMARY_TO_COUNTER = {
    "MissedMutant": "missed",
    "CaughtMutant": "caught",
    "Timeout": "timeout",
    "Unviable": "unviable",
    "Success": "success",
}


def fail(message: str) -> NoReturn:
    print(f"mutation promotion gate failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_results(path: Path) -> dict:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read outcomes JSON: {error}")
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as error:
        fail(f"invalid outcomes JSON: {error}")
    if not isinstance(payload, dict):
        fail("outcomes JSON root must be an object")
    return payload


def require_count(payload: dict, key: str) -> int:
    value = payload.get(key)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        fail(f"{key} must be a non-negative integer")
    return value


def mutant_file(scenario: object) -> str | None:
    if scenario == "Baseline":
        return None
    if not isinstance(scenario, dict) or set(scenario) != {"Mutant"}:
        fail("scenario must be Baseline or a Mutant object")
    mutant = scenario["Mutant"]
    if not isinstance(mutant, dict):
        fail("Mutant scenario payload must be an object")
    if mutant.get("package") != "policy-core":
        fail("every mutation must belong to policy-core")
    file_name = mutant.get("file")
    if not isinstance(file_name, str) or not file_name:
        fail("every Mutant scenario must name its source file")
    return file_name.replace("\\", "/").removeprefix("./")


def check(path: Path) -> None:
    payload = load_results(path)
    if payload.get("cargo_mutants_version") != EXPECTED_VERSION:
        fail(
            "unexpected cargo-mutants version: "
            f"expected {EXPECTED_VERSION}, got {payload.get('cargo_mutants_version')!r}"
        )
    if not isinstance(payload.get("end_time"), str) or not payload["end_time"]:
        fail("outcomes are incomplete because end_time is absent")

    total_mutants = require_count(payload, "total_mutants")
    counts = {key: require_count(payload, key) for key in COUNTERS}
    outcomes = payload.get("outcomes")
    if not isinstance(outcomes, list):
        fail("outcomes must be an array")

    baseline_summaries: list[str] = []
    mutant_summaries: list[str] = []
    mutated_files: set[str] = set()
    for outcome in outcomes:
        if not isinstance(outcome, dict):
            fail("each outcome must be an object")
        summary = outcome.get("summary")
        if not isinstance(summary, str):
            fail("each outcome must have a string summary")
        file_name = mutant_file(outcome.get("scenario"))
        if file_name is None:
            baseline_summaries.append(summary)
        else:
            mutant_summaries.append(summary)
            mutated_files.add(file_name)

    if baseline_summaries != ["Success"]:
        fail(f"baseline did not succeed exactly once: {baseline_summaries!r}")
    if total_mutants == 0:
        fail("no mutants were exercised")
    if len(mutant_summaries) != total_mutants:
        fail(
            "counter mismatch: total_mutants does not equal the number of mutant outcomes"
        )
    if sum(counts.values()) != total_mutants:
        fail("counter mismatch: outcome counters do not sum to total_mutants")

    observed_counts = {key: 0 for key in COUNTERS}
    for summary in mutant_summaries:
        counter = SUMMARY_TO_COUNTER.get(summary)
        if counter is None:
            fail(f"unclassified mutation outcome: {summary!r}")
        observed_counts[counter] += 1
    if observed_counts != counts:
        fail(
            "counter mismatch: summary counts differ from top-level counters "
            f"({observed_counts!r} != {counts!r})"
        )
    if counts["missed"] != 0:
        fail(f"found {counts['missed']} missed mutants")
    if counts["timeout"] != 0:
        fail(f"found {counts['timeout']} timed-out mutants")
    if counts["success"] != 0:
        fail(f"found {counts['success']} unclassified successful mutant outcomes")
    if counts["caught"] == 0:
        fail("no viable mutant was caught")

    missing_files = EXPECTED_FILES - mutated_files
    unexpected_files = mutated_files - EXPECTED_FILES
    if unexpected_files:
        fail(f"unexpected mutated files: {', '.join(sorted(unexpected_files))}")
    if missing_files:
        fail(f"missing mutation coverage for: {', '.join(sorted(missing_files))}")

    # Compare against the tool's full generated inventory, not only counters of
    # completed outcomes: interrupted runs can have internally consistent totals.
    try:
        inventory = json.loads(path.with_name("mutants.json").read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        fail(f"cannot read generated mutant inventory: {error}")
    if not isinstance(inventory, list):
        fail("generated mutant inventory must be an array")

    def identity(mutant: object) -> str:
        keys = ("name", "package", "file", "function", "span", "replacement", "genre")
        if not isinstance(mutant, dict) or not all(key in mutant for key in keys):
            fail("mutant identity is incomplete")
        if not isinstance(mutant["name"], str) or not mutant["name"]:
            fail("mutant name must be nonempty")
        return json.dumps({key: mutant[key] for key in keys}, sort_keys=True)

    expected = [identity(item) for item in inventory]
    observed = [identity(item["scenario"]["Mutant"]) for item in outcomes if item["scenario"] != "Baseline"]
    if len(set(expected)) != len(expected) or len(set(observed)) != len(observed):
        fail("duplicate mutation identity in inventory or outcomes")
    if set(expected) != set(observed):
        fail("incomplete mutation coverage: outcomes differ from generated inventory")

    print(
        "mutation promotion gate passed: "
        f"{total_mutants} mutants across {len(mutated_files)} authorization-critical files; "
        f"{counts['caught']} caught, {counts['unviable']} unviable"
    )


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(
            "usage: check-mutation-results.py PATH/TO/outcomes.json",
            file=sys.stderr,
        )
        return 2
    check(Path(argv[1]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
