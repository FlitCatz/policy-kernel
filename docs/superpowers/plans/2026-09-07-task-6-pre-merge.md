# Task 6 Pre-merge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Produce deterministic pre-merge readiness from all required evidence.

**Architecture:** Extend the existing input with explicit evaluation context and
bind its profile into policy. Compose Tasks 3–5 and keep digest projection separate
from gate validation.

**Tech Stack:** Rust 1.95.0, existing serde/serde_json and SHA-256 dependencies.

**Spec:** docs/specs/2026-09-07-pre-merge-evaluator-design.md

## Global Constraints

- No filesystem, environment, clocks, Git, networking or subprocesses in core.
- Blocked > Inconclusive > ReadyToMerge; advisory cannot authorize post-merge.
- Preserve legacy schema-1 fixture round trips; missing context fails closed.
- Do not mutate or rehash historical input bundles.

### Task 1: Contract and aggregate tests

Files: `crates/policy-contracts/src/{decision,github,policy}.rs`,
`crates/policy-core/tests/pre_merge.rs`, `tests/fixtures/pre-merge/*.json`.

- [x] Write static fixtures and tests asserting ReadyToMerge only for complete
  authoritative input, Inconclusive for absent agent-ready and wrong head, and
  advisory conversion rejection. Add gate mutation tests with literal expected
  statuses/reason codes and compatibility coverage.
- [x] Run `cargo test -p policy-core --test pre_merge --locked`; observe the
  missing evaluate_pre_merge API before implementing it.
- [x] Add `PreMergeContext`, `AgentReadyObservation`, optional context/profile
  binding fields; leave existing deserialization strict and legacy values intact.

### Task 2: Pure aggregation and digest projection

Files: `crates/policy-core/src/{pre_merge,pre_merge_digest,lib}.rs`.

Interface: `evaluate_pre_merge(&PreMergeEvaluationInput) -> PreMergeDecision`.
Export `pre_merge_policy_digest(&PolicySnapshot)` and
`pre_merge_evidence_digest(&EvidenceBundle)` for trusted producers.

- [x] Implement the gates and the normalized digest envelope from the spec.
  Preserve raw integrity validation and all reasons. Supply policy-required IDs
  through a local profile clone to the existing change-aware sensor path; no
  changes to the sensor module or its public behavior are necessary.
- [x] Run the aggregate tests; require every gate mutation to deny readiness and
  Blocked precedence to retain other gate reasons.
- [x] Add and run 100-repeat, permutation and changed-input digest tests, including
  the waiver target with a non-circular digest and a changed waiver reason.

### Task 3: Integration verification and handoff

- [x] Run `cargo test --workspace --locked`, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, and
  `git diff --check`.
- [x] Review all changes against the spec, fix issues with regression tests, and
  record validation and limitations in the main implementation plan.
- [x] Commit with `feat: evaluate pre-merge release readiness`.

Validation: 21 pre-merge integration tests and 78 workspace tests pass. Formatting,
strict workspace Clippy and whitespace checks pass. A read-only reviewer identified
fractional freshness truncation; the regression first failed, then passed with
full-duration comparison, and the reviewer verified the focused fix.
