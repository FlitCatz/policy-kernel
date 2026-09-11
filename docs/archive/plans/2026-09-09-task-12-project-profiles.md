# Task 12 — project profiles and report normalization

Status: 12A–12C implemented, runtime verification `not_run`.
Base: `0c20c73`; branch: `feat/task-12-project-profiles`.

This executes Task 12 of [the implementation plan](2026-09-04-policy-kernel-implementation.md)
and [the profile design](../specs/2026-09-07-project-verification-bootstrap-design.md).
Tasks 1–11 are integrated; historical unchecked boxes are not instructions to
repeat them. GSD installation or onboarding is not a prerequisite.

## Execution constraints

- Implement runtime code and tests in Rust. Keep existing authorization gates.
- The user's test-execution prohibition remains active. Record tests as `not_run`;
  implementation and static inspection do not establish verification or release.
- Reuse the existing design. Resolve specific contract gaps before normalizers
  depend on them; do not silently change historical canonical evidence.
- The first slice covered result requirements. The user subsequently requested
  continued work; 12B and 12C are now implemented but not runtime-verified.

## 12A — policy-bound required results

Introduce SensorProfile version 2 independently of the existing envelope version.
Each v2 sensor explicitly requires either a process result or test counts. Preserve
version 1 parsing and serialization for historical profiles; v1 must reject v2
requirements rather than silently ignore them. Unknown versions and missing v2
requirements must fail, including profiles constructed directly in Rust.

The core must reject omitted counts for required test-count sensors even if the
sensor claims passed and another suite supplies positive aggregate adequacy.
Existing partial, empty, skipped, failed and timeout checks remain in force.
The existing policy's sensor-profile digest binds the requirement. Upgrading a
profile therefore requires a new policy digest and fresh evidence, not rehashing
historical evidence. Update the pre-merge version check to recognize the separate
profile version without relaxing other envelope checks.

Write regressions for v1 compatibility, invalid version/requirement combinations,
missing/partial/zero counts, a legitimate process sensor, and pre-merge binding.
Update the checked-in JSON Schema and schema compatibility cases. Do not run tests
or schema-regeneration executables during the current prohibition.

## 12B — execution bindings and report normalization

Define the versioned external project-profile and normalized-report contracts before
implementing parsers. Bind component cwd, explicit argv, toolchain and lockfile
content, parser version, source/head, original artifact bytes and profile content.
Specify which checked contents each digest represents; a caller-supplied digest
alone does not prove the expected command ran.

Implement a separate Rust project-verification crate for Vitest JSON, Playwright
JSON and an explicitly pinned .NET runner report format. Confirm formats against
official versioned sources. Preserve individual test IDs, failures, skips and
artifact digests. Missing, malformed, stale and mismatched reports remain
non-success. Normalization creates local advisory evidence only.

## 12C — profiles and collection integration

Provide React/TypeScript, .NET and fullstack profiles with unique component-scoped
sensor/test IDs. Fullstack includes both component suites and explicit contract/E2E
checks. Represent unknown impact explicitly and select all applicable checks.
Validate project profiles before invoking the existing bounded collector; constrain
cwd and artifact paths to the selected component and reject unsafe bindings.

Document local/CI use, compatibility pins and incomplete external acceptance.
No profile asset is release-ready until its actual instances pass schema checks
and its consuming-project validation is observed in Task 14.

## Completion record

Record each implemented slice, static checks, unexecuted tests and remaining work
in [the Task 12 execution record](../reviews/2026-09-09-task-12-project-profiles.md).
No repeated workspace tests, paid review or remote side effects are implied.
