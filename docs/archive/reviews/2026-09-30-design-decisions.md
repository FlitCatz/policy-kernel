# Design decision changes — 2026-09-30

This record covers four local commits based on
`c8b74b91933f7843dca2288b9aed725bcf560290`, whose hosted Release verification
and Task 4 sensors succeeded. It is local evidence on macOS ARM64 with Rust
1.95.0 and cargo-mutants 27.1.0. It is not a hosted required check, a protected
merge or a production promotion. The items were previously left as "deliberately
unchanged" in the [codebase review remediation](2026-09-30-codebase-review-remediation.md)
record; that earlier record remains a statement of its own time.

## 1. Configuration binding independent of checkout location

`ConfigurationBinding` schema 2 adds the declared checkout-relative
`component_path`. The policy-bound `configuration_digest` now excludes the
absolute `component_root`, so a profile prepared locally binds identically on a
CI runner. Same-machine change detection still compares the full binding.
Profiles adopted from schema-1 bindings need one re-adoption. A regression test
prepares the same project in two locations and compares digests and bound
sensor profiles.

## 2. Execution and adequacy evidence bound to the head

`ExecutionEvidence` and `TestAdequacyEvidence` gain an optional `head_sha`,
omitted when absent, so legacy evidence keeps its exact serialization, digests
and contract version. `validate_bundle` rejects a present head that differs from
the manifest (`PK-PROV-BINDING-MISMATCH`). Authoritative provenance requires it
on every execution and on test adequacy (`PK-PROV-BINDING-MISSING`); advisory
evaluation accepts its absence. The collector and project verification fill it.

Authoritative fixtures now carry the head, with only affected child digests
resealed; deliberately invalid digests stay invalid. The ready-fixture digest
vectors were recomputed with jq canonicalization and `shasum`, independently of
the production projection and hashing helpers. The same method reproduced the
previous evidence vector `04d6090f…` from the previous fixture. New vectors:
evidence `efd6c523…` and decision `64929621…`. The EvidenceBundle JSON Schema
was regenerated.

## 3. Merge protection cross-checked against GitHub

Post-merge reads `GET /repos/{owner}/{repo}/branches/{branch}`, which requires
only read access, and fails closed unless its `protected` flag equals the merge
receipt's claim. The observation is taken at evaluation time: it detects a
receipt claiming protection GitHub does not report, but cannot prove protection
at the historical merge instant or which rules applied. The admin-only
protection-details endpoint remains unused because it returns HTTP 403 for this
repository.

## 4. Segment-aware path globs

Sensor and knowledge path globs use `literal_separator`, following GitHub
Actions `paths` semantics: `*` and `?` stay within one segment and `**` crosses
directories. `src/*.rs` no longer matches `src/nested/lib.rs`. All checked-in
profiles and templates use `**` patterns and are unaffected. Hand-authored
policies relying on a cross-segment `*` must switch to `**`.

## Verification

For each item: `cargo fmt --all --check`, strict workspace Clippy and
`cargo test --workspace --locked` passed; the final state passed 394 tests with 5
ignored and not run. `xtask schema` output matches the checked-in schemas.

The mutation contract was reconciled after items 2 and 4: `provenance.rs` spans
were remapped from a regenerated `cargo mutants --list`, source hashes refreshed,
and item 4 added a typed replacement for the new `path_glob` helper (a
match-everything `**` matcher instead of an uncompilable default). Strict full
suites passed:

- after item 2: core 396 native + 13 replacements, controller 142 + 3, workflow
  source 8 — 562 of 562 caught;
- after item 4: core 396 native + 14 replacements, controller 142 + 3, workflow
  source 8 — 563 of 563 caught;

with zero unviable, missed, timed-out or unmapped entries in each passing run.
The first item-4 run failed on the then-unrepaired `path_glob` default form; it
is not counted.

These observations apply only to this change set and environment. They do not
authenticate a later commit or establish hosted verification.
