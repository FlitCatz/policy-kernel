# Consuming-project acceptance

Task 14 exercises the installed 1.0.2 profiles against actual example
applications. `policy-acceptance` is Rust; the user-approved language exception
is limited to native app and test sources under `examples/`. The existing Rust
release workflow and consumer native checks remain required. These examples do
not adopt policy, approve a change, or replace a consumer's business tests.

## Run

Use Node **22.18.0**, npm **10.9.3**, and .NET SDK **9.0.304** as appropriate for
the selected profile. The runner records exact runtime probes before executing
checks; it selects .NET using the bundled `global.json`, even on hosts with
additional SDKs. A missing or different runtime produces `unverified`, no native
cases, and exit 2. Restore uses checked-in npm/NuGet lockfiles. It never fabricates
lock entries or silently updates the profile pins to make a check green.

The 1.0.1 release pins Vite 7.3.6, Vitest 4.1.11 and Playwright 1.55.1.
Release 1.0.2 retains those pins and adds the required `shared:dependencies`
audit for fullstack's separate root npm graph. Both frontend and shared graphs
must pass; fullstack now has 15 required sensors.
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
| .NET | Valid fixture credential is rejected by the API | `backend:integration` |
| Fullstack | Backend response no longer matches the frontend decoder | `shared:contract`, `backend:integration`, `shared:e2e` |

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

The additional `task-14-consumer-acceptance.yml` matrix runs all three profiles
on Ubuntu 24.04 with the exact runtime pins and uploads observations on failure.
It has read-only repository permissions and leaves existing workflows intact.
The setup actions are commit-pinned to upstream releases
[setup-node 4.4.0](https://github.com/actions/setup-node/releases/tag/v4.4.0) and
[setup-dotnet 4.3.1](https://github.com/actions/setup-dotnet/releases/tag/v4.3.1).
Adding the workflow is not evidence that it ran.

See the [Task 14 execution record](../archive/reviews/2026-09-09-task-14-consuming-acceptance.md)
for actual commands, results, and open native/CI acceptance requirements.
