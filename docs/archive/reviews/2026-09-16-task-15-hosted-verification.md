# Task 15 hosted candidate verification

Observed 2026-09-16 Asia/Seoul. GitHub timestamps below are UTC.
This record follows the [local verification record](2026-09-15-task-15-follow-up-verification.md)
without changing its pre-publication observations.

## Candidate and repository

- Repository: `FlitCatz/policy-kernel`, private, repository ID `1356895759`.
- Branch: `feat/task-15-compatibility`.
- Published head: `751fa760bb0fb85e8c7b739f1ef8f9dc782a1da0`.
- Base main: `abd6cb0a1fcd1440cc96a05bfe4ce00280742511`.
- [Draft PR #6](https://github.com/FlitCatz/policy-kernel/pull/6) was opened to run
  the existing PR-triggered native verification; no merge was performed.
- PR synthetic merge: `738a2a1ff7869b0dbae89fa1c31b86b40db8a14a`, distinct from
  the evaluated branch head. Native workflow checkout explicitly selects the PR
  head; artifact names using `github.sha` may contain the synthetic merge SHA.

The existing worktree and the original checkout's separate changes were preserved.
No product code or frozen bootstrap asset changed during this observation.
Current documentation was updated separately; its uncommitted edits are not part
of the frozen `751fa76` candidate. The Task index, Rust compatibility and Task
10/14 pages now link these observations without changing dated earlier records.

## Release verification

[Run 34966889607](https://github.com/FlitCatz/policy-kernel/actions/runs/34966889607)
completed successfully for the published head on push, attempt 1. Workflow ID is
`352947477`, path `.github/workflows/ci.yml`, check suite `94686871912`, actor
`FlitCatz`. The run was created at `2026-09-15T12:05:45Z` and last updated at
`2026-09-15T12:19:11Z`. API check-run observations bind all five results to the
same head and App `github-actions`, ID `15368`.

| Job | Job/check ID | Actual compiler target | Tests | Result |
| --- | --- | --- | --- | --- |
| verify | 104373363737 | Rust 1.95.0, x86_64-unknown-linux-gnu | 340 passed / 3 ignored | success |
| Rust 1.85.0 / ubuntu-latest | 104373364076 | x86_64-unknown-linux-gnu | 340 passed / 3 ignored | success |
| Rust 1.95.0 / ubuntu-latest | 104373364074 | x86_64-unknown-linux-gnu | 340 passed / 3 ignored | success |
| Rust 1.85.0 / macos-latest | 104373363971 | aarch64-apple-darwin | 340 passed / 3 ignored | success |
| Rust 1.95.0 / macos-latest | 104373364041 | aarch64-apple-darwin | 340 passed / 3 ignored | success |

The images were Ubuntu 24.04 and macOS 26 ARM64. Rust 1.85.0 reported rustc commit
`4d91de4e4`, Cargo `d73d2caf9`; Rust 1.95.0 reported rustc `59807616e`, Cargo
`f2d3ce0bd`. Each compatibility artifact includes the complete versions and head.
The verify job additionally passed format, locked check, strict Clippy, schema
determinism, deny, audit, runtime/public API residue and a fresh-target build.
Ignored network/live tests are not counted as passing workspace tests.

Five downloaded ZIP archives matched their GitHub artifact SHA-256 values:

| Artifact ID | Scope | SHA-256 |
| --- | --- | --- |
| 10396500387 | Release verification | `2125647085444d650574a20f26a7d6c4f8e49d1d6d8c1a1189dec388aaba99ee` |
| 10396305139 | Rust 1.85 Ubuntu | `4f0372528f7e64e673139c98a56cc493493256d14fff6885a083825223f32814` |
| 10395930755 | Rust 1.85 macOS | `c23bb4f3a8a27913c7596a4e7c052fd21c23d58d6102cdf961aaa63b04ed94c8` |
| 10395805735 | Rust 1.95 Ubuntu | `872b6811513d4cb006fb67adab6f36e7fb16b9388d3bad63bb77e3a4cda706cb` |
| 10394913837 | Rust 1.95 macOS | `707814a825308aa808c7f43ece09f9be9671c1db0a64c8539f946be0c7ef7918` |

Raw API observations, decoded complete job logs, original ZIPs and extracted logs
are retained locally under `target/task15-hosted/2026-09-16/751fa76/`. GitHub
artifact retention is 30 days. These authenticated GitHub execution observations
do not establish owner policy adoption or an authoritative kernel decision.

## Additional hosted executions

[Native run 35021204028](https://github.com/FlitCatz/policy-kernel/actions/runs/35021204028)
completed successfully for all four jobs, attempt 1. All check runs bind to
candidate `751fa76` and GitHub Actions App `15368`.

| Job | Job/check ID | Verified native scope | Artifact ID |
| --- | --- | --- | --- |
| React/TypeScript | 104557024409 | Baseline, type error, component failure, empty/missing report; corrected heads and stale-head rejection | 10418026487 |
| .NET | 104557024195 | Baseline, API authorization failure, empty/missing TRX; corrected heads and stale-head rejection | 10417564061 |
| Fullstack | 104557024364 | Baseline, contract mismatch, empty/missing E2E report; corrected heads and stale-head rejection | 10418017125 |
| Alternate React SSR/Jest | 104557024481 | Ten native cases including skip, retry, empty/missing report and nested dependency audit; stale-head rejection | 10417975534 |

These are 13 three-preset scenarios and 10 Jest cases, not 23 workspace tests.
The native summaries report `ci_authoritative: unverified`; actual GitHub execution
does not change the underlying sensor evidence's `local_untrusted` trust class.
The separate network NuGet bypass reproduction remains a local observation in the
September 15 record; this hosted workflow does not execute that ignored test.

All four summaries report the exact harness head `751fa76`, `dirty: false`, and
source digest `0b618f4732178efbeb14d86ed0988662afa0e3553615ff6ca18a4bd67388b869`.
A local Rust inspector compared each complete 363-file inventory and every file
digest against that Git commit, then recomputed the aggregate source digest.
The three preset jobs report executable digest
`124382a2c08339006c1557e0d4fe4b93b7eea10fd92ec5a3f2bc4ad3b2a8e8aa`;
the Jest test binary reports
`fc1cdf049379bacdb208069ffa683e4e8a38de64505554dfc9d5cc4d2f6927cd`.
These executable observations are not a reproducible-build proof.

The three presets observed Node `22.18.0`, npm `10.9.3` and .NET SDK `9.0.304`
where applicable. Jest used Node `22.19.0`, npm `10.9.4`, Jest `30.2.0` and the
`web` component path. Requested ranges, provision choices, actual probes, commands,
input/configuration/profile digests and each generated consumer head remain in the
downloaded request/result pairs. Consumer heads are distinct from the harness head.
A Rust inspector also verified 43 request/result pairs: profile and configuration
digests, command digests, source/head equality and unchanged `local_untrusted`
classification. All 89 retained original report digests matched their collection
records, including the rejected native report cases. The inspector's output and
source are retained with the local hosted artifacts.

The four native ZIP SHA-256 values matched the GitHub API:

| Artifact ID | SHA-256 |
| --- | --- |
| 10418026487 | `9b203a6bac5cddc83f32d1dccc76d5d6df18bab0714c0756ed806cb5ceed098a` |
| 10417564061 | `cb86561f682e5ceca268f06f3391bef0023bef2f9e14771e0e39e4176d3f1eda` |
| 10418017125 | `c87d93fd892c7b4bf1d4aff5017baa32c39142dfbf057d282a23b04b40688dc1` |
| 10417975534 | `11847aa08c7028f838283121656a29982f28ad0ba2abb53b8099c989e07c6e2f` |

[PR Release run 35021203828](https://github.com/FlitCatz/policy-kernel/actions/runs/35021203828)
also passed all five jobs, each with 340 passed / 3 ignored workspace tests.
Those job logs explicitly check out synthetic merge `738a2a1`, while API check
metadata identifies head `751fa76`. Their success is an additional PR observation;
the push run above provides the direct-head Release evidence. Complete decoded
logs and artifact metadata are retained locally for both runs.
[Task 4 sensors run 35021203922](https://github.com/FlitCatz/policy-kernel/actions/runs/35021203922)
also completed successfully.

Task 10 live API smoke run `35021203852` was skipped. Its existing workflow and
Rust test are deliberately restricted to branch `task10-github-ci`, so opening
this Task 15 PR does not run or satisfy that live publication test.

`promotion.yml` is a manual verification workflow and performs no promotion or
protection change. Local CLI authentication initially
failed (`gh auth status`: active FlitCatz token invalid); read-only evidence was
retrieved through the connected GitHub app instead. That connector has no initial
workflow-dispatch tool. A user-approved device login was started to restore the
CLI needed to dispatch the existing workflow on the exact branch candidate.
Two device-login attempts subsequently ended with `context deadline exceeded`;
the CLI still reported an invalid token at `2026-09-15T21:53Z`. Read-only GitHub
App access continued to work and showed no mutation run for this candidate.
A further device login was opened. A chat instruction to continue does not
complete the separate GitHub device authorization.

Once authenticated, verify that the remote branch still resolves to the published
head before invoking:

```console
rtk proxy gh workflow run promotion.yml --repo FlitCatz/policy-kernel --ref feat/task-15-compatibility
```

Alternatively, an authenticated owner can open the
[mutation workflow](https://github.com/FlitCatz/policy-kernel/actions/workflows/promotion.yml),
choose **Run workflow** and branch `feat/task-15-compatibility`. The connected
GitHub App can observe and download the resulting run even when CLI login is
unavailable. The selected run's head, attempt, workflow, App and outcomes must
still be checked; dispatch itself is not a passing result.

User CLI authentication was subsequently restored for `FlitCatz`. After verifying
the remote branch still resolved to `751fa76`, the command above started
[mutation run 35050598715](https://github.com/FlitCatz/policy-kernel/actions/runs/35050598715)
at `2026-09-16T03:07:27Z`. It is attempt 1, `workflow_dispatch`, workflow ID
`352947478`, path `.github/workflows/promotion.yml`, actor `FlitCatz`, job
`104649910238`. The run is in progress; no mutation outcome is claimed yet.

## Subsequent mutation completion observation

The earlier in-progress observation above is preserved. Run `35050598715`
subsequently completed successfully; job `104649910238` finished at
`2026-09-16T03:55:22Z`. The final API check identifies the same full candidate
SHA and GitHub Actions App `15368`. The artifact's `versions.txt` independently
records that SHA, Rust `1.95.0` (`59807616e`) and cargo-mutants `27.1.0`.

Artifact `10430105941`, named
`mutation-evidence-751fa760bb0fb85e8c7b739f1ef8f9dc782a1da0`, was downloaded.
Its ZIP SHA-256 matched the GitHub artifact metadata:
`98c272182a3c999b259926e1bdf9b3a20740c5c20477ec1b780c15b01e9e5b9e`.
The existing Rust `xtask check-mutation-results` validator was rerun locally on
each extracted `outcomes.json` and sibling `mutants.json`, with these results:

| Scope | Total | Caught | Unviable | Missed | Timed out |
| --- | --- | --- | --- | --- | --- |
| core (five authorization modules) | 310 | 301 | 9 | 0 | 0 |
| controller | 143 | 139 | 4 | 0 | 0 |
| workflow-source | 8 | 8 | 0 | 0 | 0 |
| Total | 461 | 448 | 13 | 0 | 0 |

All three validators exited 0 after checking the successful baseline, complete
inventory, scope and outcome counters. Unviable mutants are not counted as caught.
These are hosted results for the published candidate, separately validated from
the earlier local mutation observations.

The final run/check/artifact API responses, complete job log, original ZIP and
extracted evidence are retained under the same local evidence directory described
above. This completes the outstanding hosted mutation verification, not operational
promotion or a protected merge. Documentation changes after `751fa76` are outside
that candidate's execution evidence.

Final documentation handoff checks on the existing worktree passed:
`cargo test --workspace --locked --offline` (340 passed, 3 ignored), 260 relative
Markdown targets and 17 LAT wiki targets (none missing), and `git diff --check`
including a separate whitespace check of this new record. The retained evidence
directory also has a Rust-generated `sha256-manifest.json`; it records local
retention integrity only and does not add producer authentication. The authenticated
protection/ruleset responses and empty PR review response were copied into that
directory from the prior session's original checkout outputs, which were preserved.

## Current external conditions

The connected GitHub API returned repository admin/push metadata, but classic
protection read returned HTTP 403 `Resource not accessible by integration`.
The repository/parent rulesets read separately returned HTTP 403 requiring GitHub
Pro or public repository visibility. The failed local CLI returned 401/404 and
does not establish the repository's protection state. After CLI authentication
was restored, read-only requests for both main classic protection and all
repository/parent rulesets returned HTTP 403 with the GitHub Pro/public-visibility
requirement. Their complete CLI responses are retained separately from the earlier
App responses. No protection or ruleset baseline was obtained; account, billing
and visibility were not changed.

The authenticated PR #6 reviews endpoint returned an empty array during this
observation. No independent provider review was submitted or inferred from a
same-provider engineering review. No review requests or messages were sent.

Production observer/producer identities and keys, a complete authenticated ledger,
adopted policy/profile, isolated controller/publication deployment, publishing App
identity, and two independent review providers remain owner-supplied requirements.
The [promotion protocol](../../promotion.md) also requires approval, at least
14 days / 30 distinct heads of shadow evidence, protected merge/post-merge and
live rollback observations. A successful GitHub Actions build or fixture signature
does not satisfy those requirements. Task 15 remains incomplete.

| Remaining condition | Current evidence | Required external input or observation |
| --- | --- | --- |
| Owner-adopted candidate/policy/profile | Candidate SHA and native fixture profiles are recorded | Adoption of the actual operational policy and profile, with their canonical digests; fixture selection is not adoption |
| Authenticated production collection | Rust reference and cryptographic fixture tests implemented | Independent observer/producer identities and public-key registry, complete execution ledger, authenticated storage and isolated controller/publication deployment |
| Independent semantic reviews | PR #6 reviews API returned `[]` | Two provider reviews independent of each other and every code modifier, bound to the chosen candidate and adopted policy |
| Native protection baseline | Authenticated protection and rulesets reads returned feature-restriction HTTP 403 | Owner decision about available repository/account protection capability, then an authenticated baseline; no account or visibility change is authorized by this record |
| Operational promotion evidence | No completed shadow window, required kernel check, protected merge or live rollback is claimed | Owner-approved criteria, at least 14 consecutive days / 30 distinct heads and required sample classes, authenticated publication and protected merge, post-merge decision and live rollback observations |
