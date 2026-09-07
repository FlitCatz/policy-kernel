# Task 11 release gates execution record

> **For agentic workers:** Use superpowers:subagent-driven-development or executing-plans. This refines Task 11 of the existing implementation plan; it does not authorize promotion.

**Goal:** Exercise release authorization adversarially, enforce complete CI and
mutation evidence, and document an auditable promotion/rollback boundary.

**Architecture:** Test real contract/core APIs using frozen input fixtures and
properties. Shell/Python verification helpers stay outside production crates.
GitHub authentication and check publication remain Task 10 responsibilities.

**Tech Stack:** Rust 1.95.0, existing serde/serde_yaml/proptest pins, Python standard
library, cargo-mutants 27.1.0, cargo-deny 0.20.2, cargo-audit 0.22.2.

**Spec:** [Extraction design](../../specs/2026-09-04-policy-kernel-extraction-design.md),
sections 7–9 and 12–16; [original Task 11](../../plans/2026-09-04-policy-kernel-implementation.md).

## Constraints and integration

- Use isolated `feat/task-11-release-gates`; never merge/push or change protection.
- Include Task 8 and Task 9 completed commits. Record Task 10 integration separately
  once its author provides a completed commit; never copy ongoing uncommitted work.
- Preserve source provenance and all contributor/authority semantics. No provider
  runtime, ambient core I/O or mutation exemptions.
- An unavailable tool, missing review or unperformed live observation stays
  inconclusive. No fabricated shadow samples, live rollback or independent review.
- Only a complete nonempty mutation run with zero missed and zero timeouts passes;
  unviable mutants are distinct from killed ones.

## Work units and verification

- [x] Compose Task 8/9 and run `cargo test --offline --workspace --locked`.
  The initial run caught missing `truncated` in generated evidence schema; after
  regeneration the baseline was 119 passing tests.
- [x] Tests: create `adversarial_release_gate.rs`, `legacy_cases.rs`, properties and
  full typed legacy fixtures. Assert exact statuses/reasons for every matrix row;
  an omitted row must fail coverage. Record changed semantics with spec sections.
  Run the named integration targets and preserve failure/success evidence.
- [ ] Mutation regressions: use surviving mutants from the initial five-file run
  as failing evidence; add public-behavior regression tests. Do not exclude hard
  mutations. Rerun the complete candidate after fixes and count every outcome.
- [x] CI and scripts: test failure paths before implementing residue and mutation
  result checks. Missing inputs/tools, malformed JSON, zero tests/mutants or
  incomplete results fail. Pin actions by verified immutable SHA. Run the full
  workspace checks, audits and clean-target build in CI.
- [x] Rollback drill: test that the supplied native protection snapshot and evidence
  inventory are restored unchanged; reject weakened reviews, missing native
  checks, retained kernel check, mismatched repository and altered evidence.
  Positive test failed with missing script, then six tests passed. The script has
  no network calls and makes no writes.
- [x] Document trust, sample thresholds, candidate/policy binding, review
  independence and owner-only promotion/rollback. A live baseline API read returned
  HTTP401; committed rollback fixtures are synthetic and labeled accordingly.
- [ ] Integrate completed Task 10; run full fmt/check/clippy/test/schema/deny/audit/
  residue/clean-build/mutation evidence and freeze a candidate SHA.
- [ ] Perform engineering review and fix findings. Separately collect two
  contributor-independent provider reviews before promotion; missing external
  review prevents any release-ready claim.

## Decisions

The historical PK-WAIVE-001 matrix row expected an expired waiver to yield
INCONCLUSIVE/PK-WAIVER-INVALID. The implemented review gate and approved severity
rules leave its High finding unwaived, yielding BLOCKED/PK-WAIVER-EXPIRED. Preserve
that rule and correct the stale expectation with an explicit changed-by-design
entry. Historical sensor shorthand fixtures must be replaced in matrix execution
with complete typed inputs; do not claim the old application was executed.

The selected tool pins fill an omission in the original provenance record. Version
sources and normal dependency boundaries are recorded in `docs/provenance.md`.

## Discovered correctness changes

Nanosecond boundary tests reproduced truncation in direct provenance, review
evidence and waiver issuance freshness checks. All three now compare exact
durations, reject future evidence and preserve the inclusive maximum age. The
post-merge future/stale branch was made explicit so the equal-time boundary is
reachable to tests; its old nested comparison had an equivalent mutation. No
mutants were skipped or excluded.

The local mutation baseline found 33 missed mutants. Regression tests now cover
individual policy clauses, binding-chain diagnostics, knowledge head/source
independence, equality timestamps and positive waiver lifetime. Full candidate
results must still be checked through the inventory-aware outcome gate.

Parallel engineering reviewers became unavailable with a workspace-credit error.
Controller verification continued locally. This is not independent-provider
review evidence; the required external review gate remains outstanding.
