# Task 15 follow-up implementation and local verification

Recorded 2026-09-15, following work and observations on 2026-09-14–15.
Status: local implementation and verification complete; hosted/operational evidence pending.
Task 15 is not complete while the external conditions below remain unmet.

## Candidate and preservation

The existing worktree is
`/Users/dev-r/Repository/playground/policy-kernel/.worktrees/task-15-compatibility`,
branch `feat/task-15-compatibility`, base HEAD
`bb7dab9cc329f4b7445fa4acbfc37ed44106d7ce`. Actual files took precedence over the
handoff's older `abd6cb0` reference. Foundation 15.01–15.03 was already present
at the actual base. This follow-up remains uncommitted. The original checkout's
separate planning/documentation changes were preserved. No new worktree, reset,
commit, push, merge, account change or promotion was performed.

The worktree had no `.codegraph/`; no index was created. RTK prefixed shell
commands. Repository implementation, CLIs, verification and tests are Rust;
the additional native React/Jest source stays under the approved examples scope.
No user global agent framework/plugin/skill installation was removed.

## Implemented behavior

- Project profile schema 2 separates release identity from the parser contract.
  The selection binds requested numeric ranges, exact provisioning choices,
  raw/normalized observations, capabilities, package/lock versions, graph roles,
  runtime files and app/test paths. A bounded version preflight prevents an
  incompatible selected command from running. Evidence still binds actual argv,
  executable, profile/configuration/input/source digests and head.
- Bootstrap 1.1.0 introduces schema 2 receipts and user-owned profile input.
  Known rendered inventories and exact file contents still define ownership,
  including during recovery and uninstall after user source edits/removal.
  Frozen bootstrap 1.0.0–1.0.2 and root 1.0.0 JSON assets were not rewritten.
- npm/NuGet installation, lock consistency and audits have explicit contracts.
  Fullstack 1.1.0 has 17 required sensors, including separate app, Unit and
  Integration NuGet audits. XML suppression checks follow the adapter rather than
  the historical `backend:dependencies` name. Renaming and graph omissions,
  target/runtime conflicts, missing audits and each individual graph failure
  have deterministic Rust regressions.
  Nested npm manifests bind their adjacent lockfile and use the same explicit
  prefix for installation/audit. Both commands force inclusion of dev, optional
  and peer dependencies; an ordinary `omit=dev` setting cannot remove those from
  the schema 2 audit. Bootstrap checks each existing manifest/lock independently
  even before its paired prerequisite is installed.
- The additional real consumer uses React server rendering, Jest 30.2.0,
  Node 22.19.0, npm 10.9.4 and `web/`. It needs neither Vite nor Vitest.
  Parser identity, accepted producer interval and actual version are separate.
  The supported combinations and exclusions are in
  [tool selection](../../integrations/tool-selection.md).
- Policy schema 2 declares required knowledge kind/source and applicability.
  Missing, wrong-kind/source, head/source-set mismatch and insufficient trust
  fail distinctly. Old policies with no declaration still permit empty knowledge;
  schema 1 rejects new declarations, and old readers reject schema 2. Policy
  digest, schema parity and pre-/post-merge regressions are preserved.
- The Rust controller verifies independent observer/producer RSA signatures,
  owner-configured identities, a complete inventory, policy/profile/head/run and
  exact workflow definition/App binding. It writes a new private receipt directory
  after authentication and never modifies original LocalUntrusted collection.
  The receipt consumer checks workflow ref/source SHA. Legacy receipts cannot
  partially adopt the new fields. Bounded input reads reject symlinks, special
  files/FIFOs and oversized JSON; crypto uses an owner-selected absolute OpenSSL
  executable, bounded files and a deadline. See the
  [reference trust/ownership contract](../../../action/controller.md).

This is an independent-attestation reference, not a deployed attester or a
general filesystem/network/MSBuild sandbox. An attester must independently
observe its claimed work and ledger; blindly signing arbitrary uploads defeats
the contract. CI publication and GitHub responses in crypto integration tests
are simulated. They do not establish production provider identity.

## Build and deterministic checks

On macOS ARM64, Cargo, rustc and rustdoc were explicitly matched. Stable tools
were selected by absolute paths under
`/Users/dev-r/.rustup/toolchains/stable-aarch64-apple-darwin/bin`:
Cargo 1.95.0, rustc/rustdoc 1.95.0 (`59807616e`).
The old temporary MSRV installation disappeared when the session was resumed;
Rust 1.85.0 was installed into an isolated
`RUSTUP_HOME=/private/tmp/task15-rustup-home`, without changing the user's default
toolchain. MSRV tools report Cargo 1.85.0 (`d73d2caf9`) and rustc/rustdoc 1.85.0
(`4d91de4e4`), with `CARGO_TARGET_DIR=target/msrv-1.85`.

Each command used the selected Cargo plus explicit matching `RUSTC`/`RUSTDOC`.
The verification command set is:

```console
rtk cargo build --workspace --locked --offline
rtk cargo check --workspace --locked --offline
rtk cargo test --workspace --locked --offline
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets --locked --offline -- -D warnings
rtk cargo run --locked --offline -p xtask -- schema
rtk cargo deny check
rtk cargo audit
rtk cargo run --locked --offline -p xtask -- check-residue
rtk cargo doc --no-deps --locked --offline -p policy-core -p policy-contracts
rtk cargo run --locked --offline -p xtask -- check-public-api target/doc schemas
rtk git diff --check
```

Rust 1.85 and 1.95 each passed 340 workspace tests. Three tests are ignored in
that count: live GitHub smoke and the two network native tests. The latter two
were executed explicitly as described below. Both workspace builds, strict
all-target Clippy, fmt/check, schema regeneration, runtime/public API residue,
deny/audit and a fresh empty-target build passed. The final empty target was
`/private/tmp/task15-empty-target-complete`. Deny reported the existing duplicate
`syn` warning, with advisories/bans/licenses/sources all passing. Audit scanned
151 dependencies with no reported vulnerability. Final check logs are retained
under `target/task15-evidence/checks/`.
Schema regeneration retained identical before/after hashes; the intentional
policy schema 2 change against base is not claimed to be a zero Git diff.

Final Cargo.lock SHA-256:
`6202dda576b00be5f6c9ba5434a07d136942c0e9ccd9ff240f3329aff3c6bcae`.
The only changes against base are policy-ci's new links to already-locked
`rustix` and test-only `tempfile`. No third-party selected version changed.

## Native observations

The actual built-in choices were Node 22.18.0, npm 10.9.3 and SDK 9.0.304.
The alternate consumer ran Node 22.19.0, npm 10.9.4 and Jest 30.2.0. Tools were
installed in scoped temporary prefixes, not selected by altering user defaults.
The Microsoft SDK archive SHA-512 was checked against its release metadata:
`afd4e2581ed5bc53de60c10c5dca571588a51d95cc90b832e5892ba7dfac78e3812b75a132b53c395721737092935cea71ed0f40aaabe9d326457043c750598d`.

| Run | Result | Original output |
| --- | --- | --- |
| React/TypeScript | Five cases: baseline, type error, component failure, empty/missing report and corrections passed; old-head replay rejected | `/private/tmp/task15-native-react-reports` |
| .NET | Four cases: baseline, API authorization failure, empty/missing TRX and corrections passed; old-head replay rejected | `/private/tmp/task15-native-dotnet-complete` |
| Fullstack | Four cases: baseline, contract/API/E2E failure, empty/missing Playwright reports and corrections passed with expected attributions; old-head replay rejected | `/private/tmp/task15-native-fullstack-reports` |
| React/Jest | Ten cases: baseline, assertion failure/correction, skip, retry, empty/missing reports, final correction and nested dev-only audit failure/correction passed; old-head replay rejected | `/private/tmp/task15-native-jest-closed` |
| NuGet test-only reproduction | Conditional legacy omission reproduced; new Unit audit rejected it | `/private/tmp/task15-nuget-repro-closed` |

These were real native tool invocations, with restoration/network enabled in
fresh fixture destinations. Built-in runs used
`policy-acceptance run --profile PROFILE --output NEW_DIRECTORY --provision`.
The alternate used `cargo test --locked --offline -p policy-acceptance --test native_jest -- --ignored --nocapture`
with `POLICY_JEST_OUTPUT`; NuGet used `--test native_nuget_audit` and
`POLICY_NUGET_OUTPUT`. All returned exit 0. The controlled npm provisioning CLI
also installed and verified selected npm 10.9.4 in a fresh local prefix.

Native summaries retain the base HEAD, dirty=true, complete sorted source-file
digests and independent executable digests. Their exact observations are listed
below; later documentation and TRX-only acceptance attribution changes do not
retroactively alter an earlier whole-tree observation.

| Run | Harness source digest | Executable digest |
| --- | --- | --- |
| React | `f7bf66db40b2e7f23606343f1f67acef926c2bf237125bb635c6accf221abf1a` | `570766a9503e3441acf4dccfadacd1e7695f57c9f715848e4c6891d354723c91` |
| .NET | `7a8fa6ddcb103d9eef979c00f23f30ac4fb1f299c791d1098d0af70de4b5f955` | `9d5cdf9cb8566eb7a7f56dff87fc44eac1a151e4643c618da079161daa3e8d85` |
| Fullstack | `90c1e6f8c0de1f944d57ff0b2c39a292508c05c81abf3eb46444939075efd238` | `f516c49fd7c9e8fe3ab254f1269c6ded324ec35a0b02e3912c6afe7b6b3fd4f1` |
| Jest | `d0ce66a4006e5622664523c00cec81ce8bfd1f5822a773c9e941207ea5be08b9` | `66ff5dda98a911bd7dc9927b3a4f3d088740dc4431587a02224abf81eff1e8d9` |
| NuGet reproduction | `90c1e6f8c0de1f944d57ff0b2c39a292508c05c81abf3eb46444939075efd238` | `b5609f2a72794c380fd12d788a90255aa4b19ff7f8d5454f8f5347619ee12a98` |

These observations do not prove reproducible builds or authenticate a run.
Native evidence/input files were additionally retained in this worktree at
`target/task15-evidence/native/` using a temporary Rust copy tool. This excludes
dependency/build caches and fixture Git metadata, while retaining summaries,
requests, results, original reports, inputs and logs. Each request binds the
actual profile and synthetic consumer head. The local fixture heads are distinct
from the kernel candidate and never represent protected production merges.

## .NET test-only classification

The reproduction adds `System.Text.Encodings.Web@4.7.1` only to Unit tests in a
staged consumer. Fixture lock preparation explicitly disabled audit only to
construct the vulnerable test input; it is not counted as a passing audit.
Ordinary prior Unit restore, a fresh-package-cache Unit restore, and the legacy
plain Unit test all failed with the critical advisory NU1904. App and Integration
graphs passed. Thus the default entire flow was not an unconditional bypass.

With pre-restored inputs and Unit-local `NuGetAudit=false` / warning suppression,
all five historical 1.0.2 sensor commands passed: its explicit app audit did not
cover the test-only dependency. Both plain and suppressed variants of the new
Unit audit failed with NU1904, while separate app and Integration audits passed.
No service-error diagnostic was observed in these cases. The native logs point to
[GHSA-ghhp-997w-qr28](https://github.com/advisories/GHSA-ghhp-997w-qr28).
The conditional omission is now confirmed for this fixture and configuration;
earlier reviews correctly retain their then-unconfirmed status.

## Review corrections and native report boundaries

Read-only engineering reviews found three concrete selection/installation issues:
nested npm commands could target the component root instead of the declared
nested graph; an absent lockfile could hide an incompatible existing .NET target;
and npm `omit=dev` could omit a vulnerable development dependency from audit.
Each was reproduced before correction. The rebuilt bootstrap CLI now returns
exit 2 for `net8.0` without a lockfile under both 1.0.2 and 1.1.0, with no writes;
the compatible `net9.0` case remains accepted. Additional receipt transitions
covered all four releases through preset → custom → preset → uninstall and
preserved the custom source bytes. No further ownership/recovery finding was
reported. These are engineering reviews, not independent production-provider
reviews satisfying 15.11.

The native Jest scenario puts `lodash@4.17.20` only in a nested project's
`devDependencies`, with `.npmrc` setting `omit=dev`. The old audit command reports
zero vulnerabilities and exits 0; the selected audit reports the vulnerable
package and fails only that graph. Removing it passes at a new head. The actual
installation also verifies the nested package is present despite omission config.
Native advisory output and the legacy omission log are retained with the run.

Native acceptance additionally executes empty/missing report scenarios for
Vitest, VSTest TRX and Playwright. Each keeps real tool execution, checks the
specific rejection, corrects at a new head and rejects earlier collections.
VSTest 17.14.1 emits zero counters without a `Results` element when no tests match.
An initial run correctly rejected it as unparsable, but acceptance could not yet
attribute that shape and reported inconclusive. The follow-up recognizes the
digest-bound zero summary for failure attribution; the production TRX parser
continues to reject it. Unexpected output in missing-report cases is retained
separately and never satisfies the requested missing artifact.

## Mutation and final evidence

Existing core scopes remain canonical, provenance, reviews, pre_merge and
post_merge, including required-knowledge authority. New scopes include the
whole controller and the versioned workflow-source predicate. No old candidate's
mutation result is substituted for these runs. Initial missed mutations led to
independently constructed signing payloads, per-field inventory/binding tests
and legacy partial-field rejection tests. A resumed incomplete run was restarted.
Disjoint file-open flags were expressed with explicit set union after bitwise
OR-to-XOR mutations produced equivalent behavior; no whole authentication scope
was excluded. FIFO regressions release a blocked reader before failing, so a
test failure does not need to wait for the mutation timeout.

The final runs use fresh `task15-mutation-{core,controller,workflow}-closed`
directories. Their selected production files did not change during or after the
runs; subsequent changes only extend native acceptance/report attribution and
documentation. `task15-authorization-source-sha256.txt` records the exact mutation
scope files and lockfile. The controller source digest is
`e51eab51ab2a3b0cc1d1ff23ab31f86000e09e84245d4e8c36efbbc4fb3d21e3`;
the CI main source containing `workflow_source_matches` is
`0d5049a27c9e3b473a5a5f9e04af2f540904a587d13b00510fc5020d72e5b57b`.

| Scope | Inventory | Caught | Unviable | Missed / timed out | Exit |
| --- | ---: | ---: | ---: | --- | ---: |
| Five existing core modules, including knowledge enforcement | 310 | 301 | 9 | 0 / 0 | 0 |
| Whole authenticated controller | 143 | 139 | 4 | 0 / 0 | 0 |
| Versioned workflow-source predicate | 8 | 8 | 0 | 0 / 0 | 0 |

All three `xtask check-mutation-results` gates passed, using the default core
scope and explicit `--scope controller` / `--scope workflow-source`. Unviable
mutants are recorded separately and are not counted as caught. Inventory scope
and outcome validation remain mandatory. Mutation commands used cargo-mutants
27.1.0, `--no-config`, `--test-workspace false`, `--jobs 2`, `--timeout 120`,
`--build-timeout 600`, and `-C --locked -C --offline`. Core selected the five named
files; controller selected `crates/policy-ci/src/controller.rs`; workflow selected
`crates/policy-ci/src/main.rs` with `--re workflow_source_matches`.

`target/task15-evidence/checks/` retains complete final mutation inventories,
outcomes, diffs, logs and gate results, alongside build/test/schema/residue/
deny/audit logs. `target/task15-evidence/native/` retains the native runs and the
initial inconclusive TRX-attribution run; earlier native records remain present.
The generated `target/task15-evidence/final-manifest/` contains sorted SHA-256
inventories for the final source files and retained evidence, plus the observed
Git head/status. These are local review artifacts, not authenticated CI receipts.
The new record and final manifests are separate to avoid a self-referential hash.

Markdown validation checked 248 relative targets and 17 LAT wiki targets with
zero missing files. `git diff --check`, untracked-source whitespace checks and
frozen asset/historical-record comparisons passed. No generated schema drift was
observed after the intended schema 2 change.

## Hosted and operational conditions

Read-only `gh api` on 2026-09-15 returned repository `FlitCatz/policy-kernel`,
private=true, default branch main and admin/push permissions. The main protection
endpoint returned HTTP 403: GitHub Pro or public visibility is required for this
feature. This is a current observation, not reuse of a historical 401/403.

One [successful Release verification run](https://github.com/FlitCatz/policy-kernel/actions/runs/34667908115)
was found for base `bb7dab9`. It does not include these uncommitted changes.
The changed Rust/native/mutation workflow configuration has not been observed on
a published final candidate. No independent production provider review was sent.

To proceed beyond local work, the consuming owner must authorize/fix the exact
candidate and policy/profile adoption, supply authenticated observer and producer
services with owner-owned keys and complete ledger ingestion, deploy isolated
controller/publication identities and pin the publishing App/workflow. Two
providers independent of each other and all modifiers must review that candidate.
Account visibility/billing/protection changes are not inferred from admin access.
The [promotion protocol](../../promotion.md) still requires owner approval,
14 days and 30 distinct heads of shadow evidence, required-check binding,
protected merge/post-merge evidence and live rollback observation.
Local signatures and tests satisfy none of those operational observations.

Current tracking: [Task 15](../../tasks/task-15/README.md) and
[repository status](../../status.md). The dated plan and earlier execution/review
records remain unchanged.
