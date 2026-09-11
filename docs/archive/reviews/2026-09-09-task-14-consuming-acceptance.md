# Task 14 execution record

Implemented 2026-09-09; resumed and reviewed 2026-09-10 after the previous
workspace exhausted its credits. Work remains on
`feat/task-14-consuming-acceptance`, based on
`fb5b29783a670e29c5b41f5f4192dca65daa55f3`.

**Status: 1.0.1 security update and all three local native acceptance profiles
passed; remote CI and authoritative promotion remain pending.** The user approved
dependency remediation and remote publication/CI on 2026-09-10. The original
1.0.0 observations below are historical and are superseded for local acceptance
by the following 1.0.1 runs. No policy authority is inferred from that approval.

## Security update follow-up (2026-09-10)

- Retained the exact 1.0.0 asset inventory and introduced 1.0.1 with Vite 7.3.6,
  Vitest 4.1.11 and Playwright 1.55.1. React and shared fullstack lockfiles report
  zero npm vulnerabilities after the update. Vitest 3.x is affected by
  [GHSA-82fw-gwwq-j7x9](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9);
  the shared Playwright graph also required the 1.55.1 security patch.
- npm 10.9.3 failed lock generation with an Arborist `edgesOut` exception even
  in clean directories. npm 11.6.2 generated the locks in isolated temporary
  directories; npm 10.9.3 then installed and ran them successfully. Its temporary
  generator engine warning is recorded, not hidden or used to change runtime pins.
- Bootstrap regressions cover old captured receipts, upgrade/downgrade,
  idempotence, uninstall and local-edit conflict preservation. A separate read-only
  code review found no substantive defects; its old-receipt coverage recommendation
  was implemented. This is engineering review, not independent-provider approval.
- All runs used Node 22.18.0/npm 10.9.3 and/or .NET SDK 9.0.304, with the same
  environment as the commands below and `--provision`.

| Output directory | Result |
| --- | --- |
| `/private/tmp/task14-security-react` | Exit 0; baseline passed; type error, component failure and empty report each `verified_rejection_and_correction` |
| `/private/tmp/task14-security-dotnet` | Exit 0; baseline passed; API authorization `verified_rejection_and_correction` |
| `/private/tmp/task14-security-fullstack` | Exit 0; baseline passed; contract mismatch `verified_rejection_and_correction`, all three correlated attributions true |

Every negative case rejected old-head replay. Raw reports, requests and normalized
collections remain in those temporary directories. Harness sources were dirty;
these are local advisory records, not final-head CI proof. CI is configured to
check out the actual PR head and upload retained observations.

Follow-up verification: workspace tests **277 passed, 1 ignored** across 53
suites; formatting and strict workspace Clippy passed; schema generation produced
no diff; residue scan passed. The 1.0.0 asset tree has no diff. The older Rust
verification table below records the pre-update run. Actual CI results will be
recorded separately as final-head remote evidence.

## Implementation and review

- Native React, .NET and fullstack examples use real resolved npm/NuGet locks.
  TS/TSX/C# are limited to example app/tests under the user's 2026-09-09 exception.
- The Rust runner installs frozen bootstrap 1.0.0 profiles into isolated consumers,
  provisions dependencies, runs native checks, retains raw reports, and records
  baseline/negative/corrected Git heads and source inventories.
- Failure attribution requires the expected failed test and diagnostic, a valid
  parsed report and matching tool version. Report names are extracted from
  digest-checked native reports; normalized test IDs are hashes.
- Contract mismatch requires all three correlated failures: shared contract,
  backend payload assertion and browser heading assertion. Unrelated failures
  remain disqualifying. The read-only reviewer’s important finding about this
  interaction was fixed and the follow-up found no new important issues.
- Completed nonzero collector commands now retain a version probe within the
  original deadline. Spawn errors, signal exits and truncated executions remain
  ineligible; negative execution evidence is still failed.
- Harness HEAD, dirty source inventory/digest and executable digest are recorded
  separately. They do not prove a reproducible build or confer CI authority.
- Predecessor fixes enforce unknown-field rejection for `Impact::Unknown` and
  preserve real TRX parsing/formatting compatibility. Core simulation tests cover
  failed/empty/local-untrusted required sensors without claiming native CI trust.

## Rust verification

| Command | Observed result |
| --- | --- |
| `rtk cargo test --workspace --locked` | 274 passed, 1 ignored; 53 suites |
| `rtk cargo test --locked -p policy-acceptance` | 20 passed after final attribution telemetry |
| `rtk cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `rtk cargo fmt --all --check` | Passed |
| `rtk cargo run --locked -p xtask -- schema` and `rtk git diff --exit-code -- schemas` | Passed, no schema diff |
| `rtk cargo run --locked -p xtask -- check-residue` | Passed |

The ignored test is the live GitHub check requiring an explicitly enabled
same-repository Task 10 PR workflow and `checks:write`; it is not counted as a
pass. These checks are not a fresh complete
release-gate run: remote CI, mutation acceptance, and independent-provider
authoritative reviews remain outside this local record.

## Native observations

All runs used Node **22.18.0**, npm **10.9.3** and/or .NET SDK **9.0.304**,
as applicable. Network-enabled execution was necessary for dependency restore
and audits; local IPC/loopback access was necessary for the .NET host and browser.

| Profile/scenario | Observation | Acceptance verdict |
| --- | --- | --- |
| .NET baseline | All 5 sensors passed | `passed` |
| .NET API authorization | Only integration failed with expected OK / actual Unauthorized; corrected run passed all 5 sensors; old HEAD rejected | `verified_rejection_and_correction` |
| React baseline | Build, lint, Node, typecheck and unit passed; dependency audit failed | Failed baseline |
| React type error | Injected `src/type-error.ts` produced TS2322; corrected typecheck passed; old HEAD rejected | `inconclusive` because baseline/correction audits failed |
| React component failure | Target component test failed with unexpected greeting payload; corrected unit test passed; old HEAD rejected | `inconclusive` because baseline/correction audits failed |
| React empty report | Producer exited 0 with discovered/executed 0; unit sensor rejected it; correction restored 1 test; old HEAD rejected | `inconclusive`, targeted attribution true |
| Fullstack baseline/correction | 13 of 14 sensors passed, including actual API contract and Chromium E2E; frontend audit failed | Failed baseline/correction |
| Fullstack contract mismatch | Target API payload, shared decoder and browser heading assertions all failed; all three native attributions true; correction recovered them; old HEAD rejected | `inconclusive` because audits failed |

The npm audit reported **3 vulnerabilities (1 moderate, 1 high, 1 critical)** in
the frozen React dependency graph, including Vite and Vitest/@vitest/mocker.
This is a real rejection, not the earlier sandbox DNS error. Audit checks and
release pins were preserved. Passing React/fullstack acceptance requires a
separately reviewed update of the frozen profile/template dependencies and pins,
followed by a new native and CI run; this record does not declare those gates green.

## Commands and retained evidence

The invoked executable was built with
`rtk cargo build --locked -p policy-acceptance`. Native invocations used:

```console
rtk proxy env PATH=/private/tmp/policy-kernel-node-22.18.0/node_modules/.bin:/private/tmp/task14-dotnet-9.0.304:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin DOTNET_ROOT=/private/tmp/task14-dotnet-9.0.304 NUGET_PACKAGES=/private/tmp/policy-task14-nuget NPM_CONFIG_CACHE=/private/tmp/policy-task14-npm-cache target/debug/policy-acceptance run --profile dotnet --output /private/tmp/task14-verified-dotnet --provision
```

The same environment was used with the profile/scenario/output arguments below.
Raw `result.json`, reports, requests and HEAD-replay records remain under each
case directory. These are machine-local temporary artifacts, not durable CI
uploads; the added workflow uploads the same artifact classes when run remotely.

| Run arguments / output directory | Retained coverage |
| --- | --- |
| `--profile dotnet --output /private/tmp/task14-verified-dotnet` | Completed baseline and API rejection/correction; final summary `passed` |
| `--profile react-typescript --output /private/tmp/task14-verified-react-typescript` | Completed baseline, type-error and component-failure cases; parent interrupted during empty-report, so use individual case records, not its unfinished summary as completion proof |
| `--profile fullstack --output /private/tmp/task14-verified-fullstack` | Completed baseline; parent interrupted during contract case |
| `--profile react-typescript --scenario empty-report --output /private/tmp/task14-completed-empty-report` | Completed replacement run; exit 2, final summary `incomplete` |
| `--profile fullstack --scenario contract-mismatch --output /private/tmp/task14-completed-contract-mismatch` | Completed replacement run; exit 2, final summary `incomplete` |

Every invocation above included `--provision`. The replacement runs include
per-case `failure_attribution`; the earlier records predate that additive field.
All runs report dirty harness sources and `ci_authoritative: unverified`.

Earlier exploratory `/private/tmp/task14-resumed-*` and `/private/tmp/task14-final-*`
records predate the final collector/attribution fixes. They explain debugging
history but do not supersede the completed records above. In particular, an
exploratory Playwright installation used a custom browser cache absent from the
frozen sensor environment allowlist. Installing at Playwright's default location
resolved that environment mismatch without weakening the installed profile.

## Outstanding acceptance

- Run the pinned three-profile matrix in actual consuming-project CI with the
  1.0.1 release. Adding workflow YAML does not satisfy this.
- Bind authenticated observations and independent approvals to the real evaluated
  consumer head and adopted policy through the existing CI adapter.
- Retain existing Rust release/native gates. Local example commits and simulated
  authority regressions are not promotion approval.
