# Codebase review remediation — 2026-09-30

This record covers an uncommitted change set based on
`91ee415a1b4a823f9f8bbffae3851442110de46e`. It is local evidence on macOS ARM64
with Rust 1.95.0 and cargo-mutants 27.1.0. It is not a hosted required check,
a protected merge or a production promotion.

## Defects corrected

1. **Not-applicable exemption matched any changed path.** A required sensor with
   a `docs/**` `not_applicable_when` condition was accepted as not applicable for
   the change `["docs/readme.md", "src/lib.rs"]`. A direct `policy-core` probe
   returned `Passed` with no reasons. An exemption now requires matching risks and
   every changed path matched by one of its globs; an empty change cannot satisfy
   a path condition. `required_when` keeps any-path semantics. All checked-in
   profiles and bootstrap templates declare empty exemption lists, and project
   verification rejects nonempty ones, so the change narrows only hand-authored
   policy profiles.
2. **An extra approval could weaken a review result.** A probe showed that a
   blocked report whose High finding had a valid waiver evaluated as `Approved`,
   while adding an independent approving report produced `Inconclusive`
   (`PK-REVIEW-CONFLICT`). A blocked report whose Medium, High and Critical
   findings are all validly waived no longer counts as contradicting an approval.
   Unwaived, expired, partially waived and low-only blocked reports still do.
3. **Reasonless non-ready gates were ignored.** Pre-merge `Gates::extend` skipped
   an `Inconclusive` status without reasons. No current evaluator produced one,
   but the rule would have failed open. Every non-ready gate now lowers the status,
   and a reasonless gate records `PK-PRE-GATE-UNEXPLAINED`. Valid provenance and
   knowledge results are routed through an explicit empty-means-valid method.
4. **Signed bytes depended on serde_json feature selection.** Controller and
   `policy-ci` canonical bytes now come from `policy_core::canonical_json`, which
   sorts keys explicitly and rejects floating-point numbers. The bytes equal the
   previous output under the workspace's current serde_json features and are the
   `canonical_sha256` preimage.
5. **Knowledge inventory keys used `Debug`.** The controller now spells
   `LatClaim` and `CodeGraphIndex` explicitly, unchanged from the documented
   inventory format.
6. **`policy-ci` JSON inputs could follow symlinks or block on FIFOs.** They now
   use the controller's bounded no-follow, non-blocking reader.
7. **Report parser identity was ambiguous.** The report format documentation
   now states that `parser_version` names the parser baseline, not the producer,
   and records that Playwright 1.63.0 was exercised through schema 4.

## Deliberately unchanged

- `ConfigurationBinding` still hashes the absolute component cwd, as documented in
  the project verification integration guide. Changing it would alter every
  prepared result binding and requires an owner decision.
- `ExecutionEvidence` and `TestAdequacyEvidence` still have no head field; adding
  one is a schema-version change.
- The merge receipt's `protected` and `via_merge_queue` observations remain
  trusted-runner assertions while protection reads are unavailable.
- Glob matching keeps globset defaults (`*` may cross `/`); changing it would
  alter existing policy semantics.
- Parser identifiers and the retained `undefined/` observation directory are not
  renamed.

## Mutation contract reconciliation

`verification/mutation-contract.json` was updated for the edited authorization
sources: the SHA-256 values of `canonical.rs`, `pre_merge.rs`, `reviews.rs` and
`controller.rs`; the `evaluate_reviews` span (end line 231 → 232, function end
232 → 233); and the `evaluate_pre_merge` typed replacement, which remains the
current body with only the status forced to `ReadyToMerge`. A regenerated
`cargo mutants --list` matched each of the twelve contract originals exactly once.
The resulting contract SHA-256 is
`89958ebdc0e5d40024fc1e2b145ae22a60b07d69f716736a66e6a434ed65b97a`.

A targeted run over the changed functions first found two missed mutants:
`&&` → `||` in `Gates::extend`, which would let a later inconclusive gate downgrade
an earlier block, and `==` → `!=` in the approval predicate. Regression assertions
were added and the targeted rerun caught 42 mutants; its one unviable form is the
contract-repaired `evaluate_reviews` default replacement.

## Local verification

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked
  -- -D warnings` passed.
- `cargo test --workspace --locked` passed 383 tests; 5 were ignored and not run.
- `run-mutation-repairs` caught all 12 typed replacements.
- `run-mutation-suite target/review-remediation-full` followed by
  `check-mutation-suite` passed: core 313 native + 9 replacements, controller
  142 native + 3 replacements, workflow source 8 native. All 475 executed mutants
  were caught, with zero unviable, missed, timed-out or unmapped entries. The run
  recorded the dirty tree on head `91ee415`; the edited source hashes matched the
  contract afterward.
- Rust 1.85.0 was not installed locally, so the minimum-version build was not run
  here.

These observations apply only to this change set and environment. They do not
authenticate a later commit or establish hosted verification.
