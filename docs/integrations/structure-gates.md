# Structural verification gates

Repository engineering checks reject argument growth beyond the measured
baseline and unreviewed internal crate dependency directions. They do not change
kernel decision states or establish production promotion; see
[the promotion protocol](../promotion.md).

## Clippy limits

Every workspace package inherits the two deny-level Clippy lints from
`Cargo.toml`. Root `clippy.toml` sets `too-many-arguments-threshold = 5` and
`max-fn-params-bools = 2`. Functions at these limits pass; exceeding either limit
fails the existing strict Clippy step:

```console
cargo clippy --locked --workspace --all-targets -- -D warnings
```

`too_many_arguments` and `fn_params_excessive_bools` use Clippy's own applicability
rules (including its trait/ABI/macro exceptions). Struct bool fields are outside
this gate: the existing `Protection` snapshot has five independent bool fields.
No lint `allow` attributes were added.

## Workspace dependency directions

```console
cargo run --locked -p xtask -- check-residue
```

The command exits 0 on success and 1 on verification/input failure. It reuses
`cargo metadata --locked --offline --no-deps --format-version 1` for source
scanning and checks declared workspace-to-workspace edges against
`WORKSPACE_DEPENDENCIES` in `xtask/src/verification.rs`. Dependency paths identify
workspace members, so renamed dependencies retain their package identity.
Normal, dev, build, optional and target-specific declarations all count,
regardless of active features or host target. A new crate has no implicitly
allowed edges. Edge removal is permitted; a new or reversed edge requires an
explicit allow-list change. Failure names the edge, for example:

```text
xtask: unapproved workspace dependency: policy-contracts -> policy-core
```

The separate resolved `cargo tree --locked -p policy-core` check still rejects
`reqwest`, `octocrab`, `clap` and `tokio`, including transitive dependencies.
External dependencies of other crates are outside this structural allow-list;
the existing cargo-deny/advisory checks retain their own roles.

| Workspace crate | Allowed workspace dependencies |
| --- | --- |
| `evidence-collector` | `policy-contracts`, `policy-core` |
| `github-adapter` | `policy-contracts` |
| `policy-acceptance` | `policy-bootstrap`, `policy-contracts`, `policy-core`, `project-verification` |
| `policy-bootstrap` | `project-verification` |
| `policy-ci` | `github-adapter`, `policy-contracts`, `policy-core` |
| `policy-cli` | `policy-contracts`, `policy-core`, `policy-native-contracts`, `policy-native-verifier` |
| `policy-contracts` | none |
| `policy-core` | `policy-contracts` |
| `policy-native-contracts` | `policy-contracts`, `policy-core` |
| `policy-native-importer` | `policy-contracts`, `policy-native-contracts`, `policy-native-verifier`, `policy-report-adapters`; dev: `evidence-collector`, `policy-cli`, `policy-core`, `project-verification` |
| `policy-native-verifier` | `policy-contracts`, `policy-core`, `policy-native-contracts` |
| `policy-report-adapters` | `policy-contracts` |
| `project-verification` | `evidence-collector`, `policy-contracts`, `policy-core`, `policy-report-adapters` |
| `xtask` | `policy-contracts` |

The allow-list fixes directed crate pairs, not dependency kinds. The four dev
edges above are part of the 35 allowed pairs; changing their kind alone does not
introduce a new direction.

## Measured baseline

Measurement preceded the gate changes on clean `origin/main` commit
`e41e7e3db7492bdb4a452190b6ad3635422b515c` (2026-11-03). A temporary Rust `syn`
visitor inspected 1,160 named signatures under `crates/`, `bootstrap/`,
`acceptance/` and `xtask/`, including tests, trait declarations and method
receivers. Generated macro expansions are covered by Clippy rather than this
source inventory. Maximum argument count was 5 at these original locations:

| Original location | Function |
| --- | --- |
| `crates/evidence-collector/src/process.rs:99` | `execute` |
| `crates/policy-core/src/sensors.rs:78` | `evaluate_sensor_evidence_for_change` |
| `crates/policy-core/src/sensors.rs:99` | `evaluate_sensor_evidence_inner` |
| `crates/policy-native-importer/src/importer.rs:98` | `import_run` |
| `crates/policy-native-importer/src/importer.rs:238` | `verify` |
| `crates/project-verification/src/npm_workspace.rs:847` | `inventory` |
| `acceptance/src/execution.rs:458` | `run_case` |
| `acceptance/src/execution.rs:589` | `execute` |

Bool parameter counts were 0 for 1,139 signatures, 1 for 20 and 2 for one:
`acceptance/tests/verdict.rs:28` (`statuses`). No signature had more than 2.
Cargo's unfiltered declared workspace graph contained the 35 pairs above, of
which four were dev edges; all remaining pairs were normal dependencies.

On the unchanged baseline, strict workspace/all-target Clippy passed with both
lints denied and the proposed 5/2 configuration supplied through
`CLIPPY_CONF_DIR`. All 35 measured pairs are in the allow-list, and the completed
`xtask check-residue` passed against an untouched `git archive origin/main`
snapshot of that commit. These are local zero-finding observations for that
commit, not claims of promotion or an authoritative kernel decision.
