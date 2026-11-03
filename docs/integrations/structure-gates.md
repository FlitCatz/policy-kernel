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
cargo fetch --locked
cargo run --locked -p xtask -- check-residue
```

The command exits 0 on success and 1 on verification/input failure. It reuses
`cargo metadata --locked --offline --format-version 1` for source
scanning and checks declared workspace-to-workspace edges against
`WORKSPACE_DEPENDENCIES` in `xtask/src/verification.rs`. Dependency paths identify
workspace members, so renamed dependencies retain their package identity. Every
path dependency must point to a workspace member: excluded crates inside the
repository and absolute paths outside it are rejected, even if their package
names match an allowed target. This prevents non-member shims from bypassing
the direction allow-list.

The full resolved graph also rejects every local package (`source: null`) whose
package ID is not in `workspace_members`, naming the package and its directory.
This catches registry dependencies redirected by root manifest patches,
absolute-path patches and `.cargo/config.toml` patches, including transitive
local packages. Declared path checks remain necessary for inactive optional
dependencies. A path already reported from a declaration is not repeated as a
resolved-package diagnostic.

Fetch the locked dependencies before workspace tests or residue verification:
full offline metadata needs dependency sources for all platforms, even those
not built on the host. CI performs the same `cargo fetch --locked` preparation.

Normal, dev, build, optional and target-specific declarations all count,
regardless of active features or host target. A new crate has no implicitly
allowed edges. Edge removal is permitted; a new or reversed edge requires an
explicit allow-list change. All dependency violations are reported together,
sorted lexicographically and deduplicated across declaration kinds, before the
command exits 1. One invocation can report these sorted diagnostics; only the
first line carries the CLI's `xtask:` prefix:

```text
xtask: path dependency outside workspace members: github-adapter -> shim (/repo/shims/shim)
resolved path dependency outside workspace members: patched-shim (/outside/patched-shim)
unapproved workspace dependency: policy-contracts -> policy-core
```

The separate resolved `cargo tree --locked -p policy-core` check still rejects
`reqwest`, `octocrab`, `clap` and `tokio`, including transitive dependencies.
Registry and Git dependencies of other crates are outside this structural
allow-list; the existing cargo-deny/advisory checks retain their own roles.

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

## Path membership baseline

Before the follow-up change, clean `origin/main` commit
`2d685f63e30c4aa57f7b5c8372f02758894e5841` (2026-11-03) contained 14 workspace
members and 35 declared path dependencies. All 35 pointed to workspace members;
none required an exception. The unchanged `xtask check-residue` passed locally.

Regression tests execute the CLI with real offline Cargo metadata and lockfiles.
They reject an excluded `github-adapter -> shim -> policy-core` bridge across
normal, renamed/optional, dev, build and inactive-target declarations, reject an
absolute path outside the repository even with an allowed package name, and
report multiple invalid edges and paths in one stable, deduplicated diagnostic.
The current graph and existing direction/external-dependency rejections remain
covered. These are local engineering observations, not promotion evidence.

Before the resolved-graph fix, PR head
`e71f81459a9ccab65f05f1b1457b9a3d12b9d6ae` (2026-11-03) resolved 168 packages.
Its 14 local-source packages were exactly the 14 workspace members: zero local
non-members. Regression tests now also reject root manifest, absolute-path and
Cargo configuration patches, and distinguish local packages by ID from registry
and Git sources.
