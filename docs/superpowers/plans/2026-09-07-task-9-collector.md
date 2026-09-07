# Task 9 Bounded Evidence Collector Implementation Plan

> Execute inline with the existing approved task 9 scope; use TDD and review the
> resulting change before completing the branch.

**Goal:** Collect bounded execution evidence without making gate decisions.

**Architecture:** Shared contracts gain an explicit truncation status. The
collector owns validated requests, detailed digest-bound records, Unix process
management and a JSON CLI. The core remains free of process and filesystem I/O.

**Tech Stack:** Rust 1.95, serde, policy-contracts, policy-core canonical hashing,
rustix safe Unix process and descriptor APIs.

**Spec:** `docs/superpowers/specs/2026-09-07-task-9-collector-design.md`

## Constraints

- Work in `/private/tmp/policy-kernel-task9`; leave task 8's worktree untouched.
- No shell invocation, inherited environment, retry, model selection or trust upgrade.
- Bound streams, elapsed execution time and artifact reads; preserve byte logs.
- Main and probe share the request timeout; all incomplete outcomes fail closed.

## 1. Shared status

- [x] Extend `crates/policy-core/tests/pre_merge.rs` table with `truncated`, expecting
  `PreMergeStatus::Inconclusive` and `PK-EXEC-TRUNCATED`.
- [x] Run `cargo test -p policy-core --test pre_merge missing_and_non_success` and
  confirm deserialization fails on the missing enum variant.
- [x] Add `Truncated` in `policy-contracts/src/evidence.rs` and the evaluator arm.
- [x] Rerun the focused test and verify success.

## 2. Collector library

- [x] Write `crates/evidence-collector/tests/collector.rs` with a real compiled
  fixture executable and JSON requests. Assert timeout/truncation/nonzero/spawn
  failure are non-success, limits hold, environment/cwd/argv are honored, version
  probe is bounded, and descendants cannot retain pipes indefinitely.
- [x] Run `cargo test -p evidence-collector --test collector` to observe missing API.
- [x] Implement request/result types and validation in `src/lib.rs`, bounded Unix
  execution in `src/process.rs`, and bounded artifact capture in `src/artifacts.rs`.
- [x] Add crate-local dependencies and run the tests until all behaviors pass.

## 3. CLI and integration documentation

- [x] Add process-level CLI tests for file/stdin JSON, local trust, invalid input,
  non-success exit and one-object stdout.
- [x] Observe the existing empty CLI failing those assertions.
- [x] Implement `evidence-collector <request.json|->` with exit codes 0 success,
  2 non-success and 64 invalid input; diagnostics go to stderr.
- [x] Add success/timeout/truncated JSON fixtures and integration documentation
  describing runner responsibility, record verification and replaceable reviewers.

## 4. Review and finish

- [x] Run `cargo test --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check`.
- [x] Request an independent review of process cleanup, digest/trust boundaries,
  tests and the task 8 schema integration note; resolve actionable findings.
- [x] Commit the verified task 9 change on its feature branch and report location,
  verification results and the shared enum change to the user.
