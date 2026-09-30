# Consuming-project acceptance

## Project-owned frontend conformance

The ignored `native_project_versions` Rust test exercises the same kernel build
with frontend schema 4, two actual npm executables, Vitest/Playwright and Jest
fixtures. It requires Node 26.10.0,
npm 12.1.0, npm 10.9.3 (supplied through `POLICY_PROJECT_VERSIONS_NPM10` or
installed by the harness), a Playwright browser and npm advisory network access.
Set `POLICY_PROJECT_VERSIONS_OUTPUT` to a fresh directory whose parent exists:

```console
POLICY_PROJECT_VERSIONS_OUTPUT=/private/tmp/project-versions-01 POLICY_PROJECT_VERSIONS_NPM10=/absolute/npm10 cargo test --locked -p policy-acceptance --test native_project_versions -- --ignored --nocapture
```

The test retains head-bound requests, normalized results, install logs, version
observations, raw report bytes per phase, and a harness binding in that directory. It exercises actual unit
failure, browser retry, vulnerable advisory and controlled audit-service failure,
plus empty/missing reports, stale head and omitted dependency-class flags. For a
consumer version-change PR, point the same frozen harness at a clean frontend
fixture directory containing its schema 4 `project-profile.json`, package lock,
source and tests:

```console
POLICY_PROJECT_VERSIONS_OUTPUT=/private/tmp/project-candidate-01 POLICY_PROJECT_VERSIONS_CANDIDATE=/absolute/consumer/frontend cargo test --locked -p policy-acceptance --test native_project_versions -- --ignored --nocapture
```

Candidate mode uses the installed Node and npm, runs `npm ci`, verifies all
selected sensors and rejects an empty report. Its summary records candidate
profile/source digests and actual tool versions. The fixed matrix above retains
the broader negative cases. Keep the exact profile, lockfile and reports. These
LocalUntrusted observations do not replace
authenticated hosted checks, independent review or production promotion.

The current harness exercises installed 1.1.0 profiles against actual example
applications. `policy-acceptance` is Rust; the user-approved language exception
is limited to native app and test sources under `examples/`. The existing Rust
release workflow and consumer native checks remain required. These examples do
not adopt policy, approve a change, or replace a consumer's business tests.

## Run

The built-in [selection contract](tool-selection.md) provisions Node **22.18.0**,
npm **10.9.3**, and .NET SDK **9.0.304** as appropriate for the selected profile. The runner records exact runtime probes before executing
checks; it selects .NET using the bundled `global.json`, even on hosts with
additional SDKs. A missing, incompatible or conflicting runtime produces `unverified`, no native
cases, and exit 2. Restore uses checked-in npm/NuGet lockfiles. It never fabricates
lock entries or silently updates the profile pins to make a check green.

The 1.0.1 release pins Vite 7.3.6, Vitest 4.1.11 and Playwright 1.55.1.
Release 1.0.2 retains those pins and adds the required `shared:dependencies`
audit for fullstack's separate root npm graph. Both frontend and shared graphs
must pass; frozen fullstack 1.0.2 has 15 required sensors. Current 1.1.0 adds
separate Unit/Integration NuGet audits and has 17.
The previous 1.0.0 template bytes remain available for existing receipts.
The npm lockfiles were regenerated in clean temporary directories using npm
11.6.2 because npm 10.9.3 crashes while resolving Vitest 4 peer dependencies.
Locked installs and native acceptance continue to use npm 10.9.3; no peer
dependency checks or audit gates are disabled.

From the repository root, select a **new** output directory whose parent exists:

```console
rtk cargo run --locked -p policy-acceptance -- run --profile react-typescript --output /private/tmp/task14-react --provision
rtk cargo run --locked -p policy-acceptance -- run --profile dotnet --output /private/tmp/task14-dotnet --provision
rtk cargo run --locked -p policy-acceptance -- run --profile fullstack --output /private/tmp/task14-fullstack --provision
```

`--provision` permits locked native dependency restore and Chromium download in
the staged consumers; it does not install Node or .NET. Without it, absent
dependencies remain a preflight failure. `--scenario type-error` narrows the run
to one applicable scenario, including its baseline and correction. Exit 0 means
the selected **local native acceptance** cases passed; 2 means unavailable,
failed or inconclusive acceptance; 64 means invalid CLI/setup input.

To inspect an installed consumer without running native tools:

```console
rtk cargo run --locked -p policy-acceptance -- stage --profile fullstack --output /private/tmp/task14-inspect
```

Staging composes `examples/react-typescript` and `examples/dotnet` into fullstack
`frontend/` and `backend/`, adds `examples/fullstack` shared tests, and calls the
real bootstrap plan/apply library. Managed configs are installed from the frozen
release inventory. An existing destination, an output inside the fixture tree,
or a symlink/special file in fixture content is refused. Partial failed staging
is preserved for inspection; reruns require another new destination.

## Scenarios

Every negative case has its own staged Git repository and dependency restore.
Within it the runner commits a baseline, an isolated defect, and a correction.
The corrected source bytes match the baseline, but the corrected Git HEAD is
new. Profiles retain all required checks, including package audits.

| Profile | Case | Expected rejecting sensor |
| --- | --- | --- |
| All three | Baseline | Every selected sensor must pass |
| React | Type error in a new source file | `frontend:typecheck` |
| React | Component behavior contradicts its test | `frontend:unit` |
| React | Zero-test native report with successful producer exit | `frontend:unit` |
| React | Successful Vitest process without its requested report | `frontend:unit` |
| .NET | Valid fixture credential is rejected by the API | `backend:integration` |
| .NET | Empty TRX or missing requested TRX with successful producer exit | `backend:unit` |
| Fullstack | Backend response no longer matches the frontend decoder | `shared:contract`, `backend:integration`, `shared:e2e` |
| Fullstack | Empty Playwright JSON or missing requested JSON with successful producer exit | `shared:e2e` |

The API additionally tests that absent/invalid credentials are rejected. The
fullstack contract calls the real API and validates its response through the
frontend decoder; Playwright loads the actual Vite app in Chromium. The fixture
token is public test data. Rust starts and stops the fullstack servers on loopback
ports 5186/5187 and refuses occupied ports.

A negative case is verified only when the full baseline passed, the expected
sensor failed with its specific native diagnostic, the correction passed on a new head, and replay of old native
collections under the corrected request was rejected. Missing results or an
unrelated failed check cannot prove the intended failure. A baseline blocked by
dependency audit, provisioning, a producer incompatibility, or missing tools is
inconclusive. Resolve vulnerable dependencies through a reviewed versioned
release and rerun acceptance; never suppress the audit gate.

The contract mutation also breaks the API payload assertion and browser heading
assertion. All three named failures must be observed with the expected messages;
any additional failed sensor rejects the verdict. Names are read from the original
digest-checked reports because normalized test IDs are hashes. Setup/connection
errors, process timeouts and unrelated assertion messages cannot prove a scenario.

CLI scenario names are `empty-report` / `missing-report` for React,
`empty-trx-report` / `missing-trx-report` for .NET, and `empty-e2e-report` /
`missing-e2e-report` for fullstack. Empty cases use the native producer's test
selection; missing cases keep the real command and request an output it does not
emit. Unexpected producer output is retained separately for inspection and never
substituted for the missing declared artifact. Corrections run on new heads and
reject earlier collections.
Ordinary nonzero executions still collect the pinned tool version within the
collector's original deadline; incomplete executions do not gain a version probe.

## Evidence and authority

`summary.json` records runtime observations, the harness repository HEAD, local
advisory status and a separate `ci_authoritative` status. Each case retains:

- `case.json`, per-phase `observation.json`, and dependency/server logs.
- `failure_attribution` in each case: whether each expected native failure was
  specifically identified. This alone is not acceptance when baseline/correction
  or another required sensor failed.
- `request.json` with sorted Git-tracked source digests, profile digest and the
  exact synthetic consumer commit SHA. Dirty/untracked source invalidates a run.
- `result.json` with original collection records, normalized sensors and bound
  profile inputs; raw native reports are copied to the phase's `reports/` before
  their known generated paths are removed for the next execution.
- `previous-head-rejection.json` documenting attempted replay of baseline
  collections against the corrected request.

The consumer heads are local fixture commits, **not** the harness PR head. The
harness head in the summary links the implementation revision separately. A
local run from a dirty harness is development evidence, not final-head CI proof.
The `harness` object records dirty status, sorted tracked/untracked nonignored
file digests, their combined digest and the running executable digest. These are
separate observations; they do not prove that the executable was built from the
recorded source tree or authenticate the run.

All project-verification observations remain `LocalUntrusted`. Running this
program inside GitHub Actions does not authenticate arbitrary evidence or make
it authoritative. The separate simulation-labeled core regressions exercise
authoritative rejection of failed, empty and local-untrusted sensors; they are
not authenticated GitHub results or independent-provider approval.

For promotion, the trusted consuming-project CI must check out its actual
evaluated head, rerun required checks, bind the complete source inventory and
adopted policy, and pass runner-controlled observations through the Task 10 CI
adapter along with all required independent approvals. Until those observations
exist, `ci_authoritative` remains **unverified** even if every native case passes.

## CI and verification status

The `task-14-consumer-acceptance.yml` matrix derives runtime provisioning values
from each selected profile and runs three presets plus the additional React/Jest
consumer on Ubuntu 24.04, retaining observations on failure.
It has read-only repository permissions and leaves existing workflows intact.
The setup actions are commit-pinned to upstream releases
[setup-node 4.4.0](https://github.com/actions/setup-node/releases/tag/v4.4.0) and
[setup-dotnet 4.3.1](https://github.com/actions/setup-dotnet/releases/tag/v4.3.1).
Adding the workflow is not evidence that it ran.

The additional native Jest test selects Node 22.19.0, npm 10.9.4 and Jest 30.2.0
with a user-owned profile and `web/` component. With those tools available, set
`POLICY_JEST_OUTPUT` to a fresh absolute directory and run
`cargo test --locked -p policy-acceptance --test native_jest -- --ignored --nocapture`.
It exercises failure/correction, skip, retry, empty/missing reports and old-head
rejection. The separate network reproduction uses `POLICY_NUGET_OUTPUT` and
`--test native_nuget_audit -- --ignored --nocapture`; it deliberately introduces a
vulnerable dependency only in a staged test fixture. Neither ignored test counts
as passed in the ordinary offline workspace run.

For read-only staging of a complete custom fixture, use
`policy-acceptance stage --examples examples/react-jest --profile-file project-profile.json --output /private/tmp/task15-custom`.
The preset `run` command remains scoped to the three built-in scenario suites.

See the [Task 14 execution record](../archive/reviews/2026-09-09-task-14-consuming-acceptance.md)
for actual commands, results, and open native/CI acceptance requirements.
