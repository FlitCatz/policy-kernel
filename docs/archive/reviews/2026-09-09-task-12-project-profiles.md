# Task 12 execution record

Date: 2026-09-09.
Base: `0c20c73`; branch: `feat/task-12-project-profiles`.
Status: `implemented_unverified` for 12A–12C. Task 12 verification remains open.

The user rolled back the GSD transition and requested Task 12 implementation.
The starting worktree was clean. The active work is based on the existing
profile/bootstrap design and Task 1–5 impact review. Task 1–11 implementation
is retained. No GSD setup is required.

## Decisions

- Use a local feature branch in the existing checkout. Branch creation required
  a sandbox escalation; it succeeded. No remote push, merge or tool installation.
- Start with the missing policy-bound result requirement: the current core accepts
  three absent count fields as a process-only sensor. A normalizer alone cannot
  enforce a required test suite independently of submitted evidence.
- Version SensorProfile separately so stored v1 profile bytes remain unchanged.
  A v2 profile must explicitly classify every sensor; changing the profile changes
  the policy-bound digest. Other envelope versions and trust semantics stay intact.
- Use one bounded implementation agent for this contract slice while the parent
  records the design and checks integration dependencies. No per-step test loops.

## Verification

- Implemented SensorProfile v2 `process` / `test_counts` requirements, strict
  version-aware parsing, programmatic validation, and pre-merge integration.
- Added regressions for v1 exact serialized bytes, invalid versions, forbidden
  v1 fields, missing/null v2 requirements, incomplete/zero counts, process results,
  aggregate-adequacy bypass and the authoritative pre-merge path.
- Updated the checked-in sensor-profile schema and its compatibility cases.
- Implementer ran targeted rustfmt and `git diff --check`, both exit 0. Parent
  inspected the contract, gate and regression diffs. No independent-provider
  review or runtime validation is claimed.
- Tests: `not_run` — the user's test-execution prohibition remains active.
- Compilation and schema regeneration: not run during this implementation slice.
- The schema was synchronized statically; equivalence with generated output is
  unverified until the existing schema checks may be executed.
- Static inspection is not a substitute for runtime verification.

## Continued implementation — 12B / 12C

- Added the Rust `project-verification` workspace crate with profile validation,
  read-only `prepare`, bounded `run`, and library-level collection normalization.
- Added `SensorResultBinding` to v2 profiles. Core compares expected configuration
  digest, parser version and argv against submitted passed evidence. V1 bytes and
  rejection of unsupported fields are preserved.
- Added Vitest 3.2.4 JSON, Playwright 1.55.0 JSON and VSTest 17.14.1 flat TRX
  parsers. Original report bytes are bound by digest; individual test identities,
  failure/skip details, global diagnostics and counts are retained.
- Added React/TypeScript, .NET and fullstack 1.0.0 profile assets. Installed package
  versions and selected lockfile entries are checked before execution. Fullstack
  requires both application components plus shared contract and E2E suites.
- Input/source digest mismatches, existing reports, symlink escapes, old report
  timestamps, inconsistent transport, failed process/version checks and missing
  artifacts cannot become successful normalized sensors. Source completeness and
  actual Git/CI identity remain the trusted integration's responsibility.
- Wrote parser, profile, normalization and path regressions. No JS/C#/Python
  execution code or native app/test project was added; report fixtures are data.
- Added `roxmltree = 0.20.0`. Lockfile resolution ran offline, without compilation;
  an incidental getrandom update was removed using a precise offline resolution.
  Final lockfile changes are the new workspace package and XML dependency only.
- A bounded parser agent completed the parser and fixtures. Its follow-up for
  additional tests could not run because the agent service reported exhausted
  credits. Parent completed those tests and static integration review locally;
  no independent whole-change review is claimed.

## Verification and remaining work

Tests, compilation, schema generation, CLI execution and native consuming-project
runs are `not_run`. Existing CI and release gates are retained. Targeted rustfmt
and whitespace inspection are the only source-level checks performed; they cannot
establish type correctness, schema-generation equivalence or runtime success.

When execution is permitted, run the new crate's relevant suites, affected
contract/core/pre-merge regressions and schema checks, then final-head workspace
CI. Confirm real producer reports and the native command/configuration matrix in
Task 14 before distributing the assets. The pinned versions are candidates;
successful source inspection is not a supported-runtime certification.

Task 13 bootstrap and Task 14 external acceptance are not implemented here. No
remote push, merge, policy adoption or authoritative observation was performed.

Final static checks: targeted rustfmt and tracked `git diff --check` exited 0.
During final inspection, untracked `verification/mutants.out/` appeared in the
shared checkout. This session did not launch that mutation run, did not modify
those outputs, and does not use them as Task 12 verification evidence.
