# TJERP-shaped consumer verification — 2026-09-29

Scope: local synthetic consumer acceptance and Rust regression tests. This record
does not establish TJERP installation, hosted CI, protected-branch integration or
production promotion.

## Implemented changes

Project schema 3/selection schema 2 provides exact producer contracts, explicit
suite/graph bindings, bounded framework inheritance and custom bootstrap 1.2.0.
The old preset assets are unchanged. The design and plan are linked from
[Task 16](../../tasks/task-16/README.md).

## Development and review

Producer/schema rejection tests were observed failing before implementation and
then passing. Combined test graphs initially failed the legacy Integration graph
requirement. Bootstrap schema-3 installation failed against the schema-2 boundary
before the new release was implemented. Existing package suites were rerun after
each integration.

A fresh Astra reviewer independently reproduced four inherited-MSBuild gaps:
case-insensitive framework overrides, ancestor targets outside the component,
custom import-hook properties and references in inherited props. The fixes add
case-insensitive comparisons, an explicit scalar-property allowlist, ancestor
checks and rejection of props items. Regression tests observed the framework and
ancestor bypasses before the fixes, then passed. The RC/TRX test SDK declaration
and lock binding was also added with an observed failing regression test.

The reviewer stopped because its workspace reported exhausted credits before
issuing a final assessment. This is a partial independent review, not a clean
reviewer verdict. The implementer checked the fixes and remaining contract paths.

## Native observations

The synthetic fixture uses Node 26.10.0, npm 10.9.3, .NET SDK
11.0.100-rc.1.26425.128, Microsoft.NET.Test.Sdk 17.14.1, Vitest 4.1.11 and
Playwright 1.63.0 on macOS. React is 19.0.0 and Vite 6.4.3. Node is the available
local fixture choice, not a claim about the TJERP repository's runtime selection.

The initial unlocked npm 10.9.3 resolution crashed inside Arborist while resolving
optional peers. npm 12.1.0 generated the fixture lock; the actual native harness
used npm 10.9.3 ci/audit with dev, optional and peer dependencies included. No
legacy-peer override or omitted graph was used. A sandboxed NuGet restore could
not reach advisory data; the subsequent network-enabled strict audit passed.

Run `native-02` passed in 65.99 seconds on base 35b9ff2 with dirty changes recorded
by the harness. Ten sensors passed, including all three backend graph audits,
three separately filtered TRX suites, frontend runtime/audit, Vitest and browser
E2E. Each of five test suites executed one passing test. The run saved 26 negative
checks: empty/truncated/missing/stale-head checks for five reports, actual failing
Vitest/TRX/Playwright executions, actual prior-head rejection, stale-profile and
changed-source-props rejection. A corrected consumer commit passed all sensors.

These report-byte tampering checks are transport/normalizer checks; the three
failing producer cases are actual native process executions. Neither is represented
as a full mutation run or a hosted observation. Each result remains LocalUntrusted.

## Final native candidate

`native-03` passed in 69.70 seconds on clean commit
`59d82edbc407576dd5814ceff26a34c6143b102e`. It repeats the ten passing sensors,
five actual suites, 26 negative checks and corrected-head success after the
MSBuild and VSTest contract fixes. Raw artifacts were written to
`/private/tmp/policy-tjerp-native-03` and retained alongside the development
worktree at `.worktrees/tjerp-consumer-evidence/native-03` in the owning repository.

| Artifact/binding | SHA-256 |
| --- | --- |
| Harness source inventory | `d55aa6797e16fc026665393dc32ca3ea94134b554b6ffb5d176ea16661f62318` |
| Harness executable | `7463dcef5d84986aefaf20ef654f170287dc53bbe6e2921bd7b7bb3d83191309` |
| Project profile | `cc047802d9632062318adea17be83244badd328f353be6a07611506fd5eb59f6` |
| Baseline result file | `5e71380dc85c8785c285a3f38efe89757699b7c17362b250147471bf7c7d94eb` |
| Corrected result file | `198e12e301a1e9f8ceb1c62934eb2d02bdd5659699ec2c65c3c2c987cac9fd54` |
| Negative checks file | `0ec15c1c1e0bb5186a59e0601566491b33b472739c6d90bdcff22ae30d02f2f8` |

## Regression follow-up

The full workspace run on the native candidate exposed an existing intermittent
failure in `forged_json_local_artifacts_and_producer_signature_tampering_are_rejected`:
replacing a randomly generated signature's first byte with `00` can leave it
unchanged. The test now changes a hex digit to a different value and asserts the
signature changed before checking rejection. The targeted cryptographic test
passed. This follow-up changes test logic only; native producer implementation
and fixture bytes remain those of the clean candidate above.

The subsequent complete `cargo test --locked --offline --workspace` run passed
362 tests with zero failures and four ignored tests. The new native test is one
of the ignored defaults and was explicitly exercised separately as recorded above;
the other opt-in native/live checks were not rerun for this change.
`cargo clippy --locked --offline --workspace --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, `xtask check-residue`, `git diff --check`, 155 local
Markdown link target checks and `lat check` passed. The existing lat init-version
warning remains; no hook installation was performed. Frozen template directories
1.0.0, 1.0.1, 1.0.2 and 1.1.0 have no diff against the baseline.

No new full mutation campaign or hosted verification is claimed. Workspace and
Clippy logs are retained in `.worktrees/tjerp-consumer-evidence/`.
