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

Final candidate verification and artifact identities are recorded below after the
last code changes are tested.
