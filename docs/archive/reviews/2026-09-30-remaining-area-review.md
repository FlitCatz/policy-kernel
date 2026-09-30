# Remaining area review — 2026-09-30

This record covers an uncommitted change set based on
`0510f419f9bd8d113711b6f3575f0f5822c40271`, following the
[codebase review remediation](2026-09-30-codebase-review-remediation.md). It is
local evidence on macOS ARM64 with Rust 1.95.0 and cargo-mutants 27.1.0. It is
not a hosted required check, a protected merge or a production promotion.

Before this change set, hosted Release verification and Task 4 sensors both
succeeded for `0510f41`, including the Rust 1.85/1.95 Ubuntu/macOS matrix.

## Scope

The review covered the areas the earlier review had only skimmed: the evidence
collector, project verification execution/normalization, impact selection and
consumer suites, the bootstrap installer, the advisory `policy-cli`, the
acceptance harness and the `xtask` mutation validator.

No defect was found in the collector (it spawns the pre-resolved executable,
shares one deadline between execution and version probe, kills the process
group and fails closed on truncation), in run normalization, in impact
selection, in bootstrap planning, in `policy-cli` or in acceptance verdicts.
Consumer suite failures reject the whole run, as documented in the schema-3
consumer guide.

## Defects corrected

1. **Bootstrap-installed files ignored the umask.** With umask `022`, an apply
   into a temporary React consumer produced `0600` files and `0700` directories,
   including `tsconfig.json`, `eslint.config.mjs` and `.policy/`. Other users and
   containers running under another UID could not read project configuration.
   Published files and directories now use modes `0666` and `0777` filtered by the
   umask. The transaction directory remains `0700`. Replacement during an update
   or rollback keeps the replaced file's permissions.
2. **Sensor evaluation was outside the strict mutation gate.** `sensors.rs`,
   where the not-applicable fail-open was found, had nine missed mutants among 84.
   Seven were test gaps for independently required typed-result fields,
   single adequacy obligations, empty mappings and blank versus duplicate
   condition IDs. Two came from a redundant partial-count check already enforced
   by `validate_typed_result` with the same reason; that check was reduced to the
   all-absent case. After new tests, a direct run caught all 78 viable mutants.

## Mutation scope extension

`sensors.rs` joins the core mutation scope in `xtask` (validator file list,
generator arguments and test fixtures, whose counts now derive from the list).
The contract adds four typed replacements for its non-compiling
`Default::default()` forms: `resolve_required_sensors` returns an unknown sensor
ID, and the three evaluation entry points force a passing result with no reasons.
Its source SHA-256 is recorded. The resulting contract SHA-256 is
`096487d34c7a1129a0fbadbb80e5a475e1531809e8556b058cc8608b4bdf4b4e`.

## Local verification

- `cargo fmt --all --check` and strict workspace Clippy passed.
- `cargo test --workspace --locked` passed 389 tests; 5 were ignored and not run.
- `run-mutation-suite target/sensors-scope-full` followed by
  `check-mutation-suite` passed: core 391 native + 13 replacements across six
  files, controller 142 native + 3 replacements, workflow source 8 native. All
  557 executed mutants were caught, with zero unviable, missed, timed-out or
  unmapped entries. The run recorded the dirty tree on head `0510f41`; every
  contract source hash matched afterward.

These observations apply only to this change set and environment. They do not
authenticate a later commit or establish hosted verification.
