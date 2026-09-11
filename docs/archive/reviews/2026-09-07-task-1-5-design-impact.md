# Task 1–5 impact of the verification/bootstrap revision

Date: 2026-09-07

## Scope and observed implementation

Compared the supplied Obsidian clipping, `policy-kernel - 하니스 전략 비교.md`,
with the extraction specification, verification/bootstrap revision, implementation
plan and available source. The clipping supports owning policy, verification
defaults and project knowledge while keeping execution harnesses replaceable.
Its model-routing examples do not override the repository's explicit decision to
keep model selection outside policy-core. React/TypeScript and .NET are the
repository revision's selected targets; the clipping does not pin their tools.

At inspection, `main` (`8d0253f`) contained Tasks 1–3 and the revised design.
Task 4 was available on `origin/feat/task-4-sensors` (`c04c282`). The local
alignment branch combines these histories. No Task 5 implementation was found in
the local/remote branch inventory or searched repository/worktree locations;
`reviews.rs`, its tests and its fixtures were absent. Task 5 findings below cover
the planned interfaces only, not verified implementation. Its location has been
requested from the user. Historical unchecked plan boxes are not completion data.

## Task impact

| Task | Assessment | Action |
| --- | --- | --- |
| 1: workspace/provenance | Compatible. Pure Rust workspace and frozen legacy obligations remain useful. | Preserve original source hashes and obligations; record new integration obligations here, without attributing them to legacy sources. |
| 2: contracts | Language-neutral IDs, digests and strict parsing remain appropriate. Profile support is not fully expressible yet. | Preserve existing wire fields in this patch; make the contract gaps below explicit prerequisites of Task 12. |
| 3: integrity/trust | Compatible. Canonical child digests include typed sensor metadata; local reports do not become authoritative through normalization. | Preserve validation and independence semantics. Task 6/10 must compare against trusted current policy/head/run inputs, not only the manifest's own claims. |
| 4: sensors/adequacy | Direct impact from combined profiles. Empty component suites and duplicate definitions could pass in specific paths. | Reject zero discovered tests in each required sensor reporting test counts; validate profiles through both public evaluation paths. Add combined-profile regression coverage. |
| 5: reviews/waivers | Planned consensus, contributor independence and human waiver bindings remain compatible. | Keep review scheduling/model selection external. Implementation review remains pending source availability. |

## Reproduced defects and changes

1. A required backend sensor reporting `passed`, exit 0 and counts `0/0/0`
   passed if aggregate adequacy was positive from another component. The typed
   result validator checked equality but not empty discovery. It now returns
   `BLOCKED` with `PK-ADEQ-NO-TESTS`, independently of aggregate adequacy.
2. The context-free `evaluate_sensor_evidence` path built a map without validating
   the profile. Duplicate sensor definitions could collapse silently even though
   `resolve_required_sensors` rejected them. Both evaluation paths now validate
   profiles. The context-free path returns `INCONCLUSIVE` with
   `PK-SENSOR-PROFILE-INVALID`; the change-aware path retains its existing typed
   resolution error.

Regression tests also exercise unique frontend/backend/shared sensor IDs, removal
of each required component/contract/E2E result, and skipped tests represented by
discovered counts exceeding executed counts. These are pure contract-level tests,
not actual React/.NET tool runs or delivered project profiles.

## Contract-fit prerequisites for project profiles

The current schema must not be advertised as a complete profile execution protocol.
Resolve these gaps in a versioned contract design before shipping Task 12:

- `SensorDefinition` cannot declare that a sensor requires test counts or a
  particular report type. All three count fields absent still means a non-test
  sensor; the core cannot distinguish a compiler from a test runner missing its
  report. Normalizers must emit non-success on missing/malformed reports, and a
  policy-bound result requirement is needed to enforce this independently.
- `SensorEvidence` has argv, configuration/source/result digests and parser version,
  but no explicit component cwd, tool version, profile version/content binding or
  original report reference list. `PolicySnapshot` has no bound sensor profile.
  Define the exact canonical content behind reusable digest fields and add any
  necessary typed fields with explicit schema-version handling. A digest without
  the corresponding checked content cannot prove the expected configuration ran.
- Aggregate adequacy has no per-suite mapping scope. Namespace test, criterion,
  behavior and negative-case IDs during composition; retain individual suite
  results so one component cannot cover another's empty suite. Preserve skipped,
  failed and original report details in the normalization protocol.
- The resolver takes known changed paths and risk; it has no unknown-impact state.
  The trusted integration must select the full applicable profile when impact is
  unresolved. An empty path list must not stand for unknown impact. Authentication,
  permissions, dependencies and shared configuration need explicit profile rules.
- Review waiver evidence concerns findings. Do not reinterpret it as permission
  to skip required sensors or dependency scans. Missing/stale advisory data must
  remain non-success in the external normalizer and later authoritative gate.

No contract version is changed by this patch. Earlier Task 4 additions already
changed serialized evidence shape through defaulted fields; defaulted parsing is
not proof of canonical-digest or old-reader compatibility. Task 8/12 compatibility
tests must cover old readers and stored bundles before freezing a distributable
contract version. Do not silently rehash historical evidence.

## Verification

Before the fix, the sensor suite reported 11 passed and 2 failed: both defects
above returned `Passed`. After the fix, the same suite reported 13 passed,
0 failed. Final checks on Rust/Cargo 1.95.0:

| Command | Result |
| --- | --- |
| `cargo test --workspace --locked` | Exit 0; 42 tests passed, no failures. |
| `cargo fmt --all --check` | Exit 0. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0; no issues. |
| `git diff --check` | Exit 0. |

Added `/target/` to the repository ignore file so local verification artifacts
cannot accidentally enter the source inventory. No Rust dependencies were added
by the alignment patch.

This review does not establish Task 5 implementation completion, live CI trust,
profile/bootstrap acceptance, independent-provider review or release readiness.
