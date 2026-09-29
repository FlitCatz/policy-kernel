# TJERP Consumer Compatibility Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline with TDD and a fresh whole-branch review.

**Goal:** Verify the existing TJERP toolchain/layout through an explicitly adopted contract.
**Architecture:** Profile schema 3 pairs with selection schema 2. Optional extension fields are absent in legacy serialization. Exact producer contracts, suite bindings and bounded MSBuild inheritance extend existing collection/bootstrap boundaries.
**Tech Stack:** Rust, serde/schemars, .NET 11 RC1/VSTest 17.14.1, Playwright 1.63.0.
**Spec:** [Approved design](../specs/2026-09-28-tjerp-consumer-compatibility-design.md).

## Global constraints

- Preserve old schema meaning and frozen bootstrap assets 1.0.0–1.1.0.
- SDK 11.0.100-rc.1.26425.128 and Playwright 1.63.0 are exact selections, not broad compatibility claims.
- Direct evidence remains LocalUntrusted; Rust owns all orchestration.
- Native success and failure cases are required before claiming verified support.

## Review focus

- Downgrading schema rejects extension fields (Task 1).
- SDK drift stops before execution (Task 1).
- Nested or implicit MSBuild overrides cannot evade binding (Task 2).
- Duplicate test identities cannot satisfy independent roles (Task 3).
- New bootstrap receipts preserve native ownership and old recovery semantics (Task 4).

## Tasks

1. **Producer/runtime contracts** — selection.rs, profile.rs, new consumer.rs, tests/consumer.rs.
   Introduce ProducerContract, ConsumerContract and RuntimePolicy. Add failing JSON tests for exact RC/Playwright probes, schema pairing, malformed probes and legacy serialization; run `cargo test --locked -p project-verification --test consumer`, implement, then run the package suite.
2. **Composition/framework inputs** — consumer.rs, selection.rs, run.rs, paths.rs, consumer tests.
   Explicit SuiteBinding maps role/sensor/graph. Test combined test graphs, missing/duplicate roles, props inheritance, imports, nested overrides and stale digests before implementing binding/layout checks.
3. **Aggregate/schema validation** — consumer.rs, run.rs, profile.rs, consumer tests.
   Test missing/overlapping suite evidence and schema agreement. Validate complete suite results before returning RunResult, preserving sensor failure semantics.
4. **Custom bootstrap** — templates.rs, plan.rs, main.rs, bootstrap/tests/consumer.rs.
   Release 1.2.0 is schema-3 custom-only. Test plan/apply/uninstall and wrong schema/release before implementation. Frozen inventories stay unchanged.
5. **Native acceptance/docs** — examples/tjerp-compatible, acceptance/tests/native_tjerp.rs, paired READMEs, integrations, task/status and lat navigation.
   Synthetic fixture mirrors inherited net11.0 and combined filtered tests. Run actual producers, audits, missing/empty/failing reports and stale inputs. Record exact results; run workspace tests, fmt, clippy, schemas/residue/links and fresh review.

## Ledger

- 2026-09-28: Existing project-verification/bootstrap baseline passed. Producer tests observed RED (unsupported schema/field), then GREEN (3 tests). Combined graph tests observed RED (missing Integration graph), implementation written; result not recovered.
- 2026-09-29: Temporary worktree disappeared between turns. Restored branch and reconstructed uncommitted edits from tool history; rerun verification before relying on results. Keep recovery checkpoints in local git.
- Execution is inline under the user's explicit approval to change the code. No additional permission round for ordinary implementation.
