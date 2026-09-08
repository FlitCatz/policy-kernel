# Task 8: Advisory CLI and schemas

Scope: Task 8 of `docs/plans/2026-09-04-policy-kernel-implementation.md`.
Base: `7eb7c88`. Branch: `feat/task-8-advisory-cli`.

## Requirements and review checklist

- Local `evaluate-pre` always invokes the existing evaluator in advisory mode.
- `inspect` validates a decision or evidence bundle and preserves its JSON value; no evaluation.
- File and stdin input; one JSON result on stdout; diagnostics on stderr.
- Exit codes: 0 ready, 2 blocked, 3 inconclusive, 64 invalid input.
- No local post-merge evaluation command or remote mutation.
- Five deterministic, sorted schemas cover policy, evidence bundle, sensor profile, and both decisions.
- Schema regeneration and strict field/version rejection are tested.

## Interface review

| Producer / consumer | Finding |
| --- | --- |
| Tasks 2, 6 / Task 8 CLI | Existing typed parser and evaluator are reusable; mode must be overridden before evaluation. |
| Task 7 / Task 8 inspect | Post-merge decisions may be inspected but never locally evaluated. |
| Task 2 / Task 8 schemas | Custom deserializers require explicit schema constraints to match validation. |
| Task 8 internal consistency | Inspection preserves supplied data, including authoritative metadata; it does not issue an authorization. |

## Progress

- Baseline: `rtk cargo test --workspace --locked` — 90 passed, 20 suites.
- CLI: 11 integration tests pass after observed RED failures, including duplicate-field rejection.
- Schemas: 5 integration tests pass after observed RED failures; validation uses pinned test-only `jsonschema` with format checks enabled.
- Full verification: `rtk cargo test --workspace --locked --offline` — 106 passed, 22 suites.
- Static verification: strict Clippy for CLI, xtask, contracts and workspace formatting check pass.
- Regeneration: all five schema SHA-256 hashes unchanged after `rtk cargo run -p xtask --locked --offline -- schema`.
- Review: duplicate-field normalization before typed inspection and missing standard integer bounds were reproduced and fixed. The final bounds patch was reviewed locally after delegated review became unavailable.
- Numeric boundaries: real-validator tests accept u64/i32 extrema and reject adjacent out-of-range values; stricter existing minimum constraints remain intact.
- Scope decision: inspect performs structural validation only and preserves authoritative metadata without authenticating it.
- Scope decision: the generator adds standard bounds for the uint64 and int32 formats currently emitted by these five schemas; future numeric formats require corresponding coverage.
