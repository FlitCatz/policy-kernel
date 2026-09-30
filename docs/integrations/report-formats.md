# Project report adapters

Schema-4 frontend profiles select `vitest-json/v1`, `jest-json/v1`, or
`playwright-json/v1` independently of the producer version and bind npm graph
audits to `npm-ci-audit/v1`. These names map to the parser versions below. The
same required counters, execution times, identities, errors, retries and process
statuses are checked. Additional irrelevant metadata is accepted; a semantic
change that still emits success-shaped JSON is outside parser-only detection.
Actual producer upgrades need the frozen [native acceptance](acceptance.md)
cases before adoption.

These adapters normalize advisory local reports. Parsing does not authenticate a
runner, establish complete discovery, or replace its recorded exit status. A
successful report also requires successful command execution and profile count
requirements in the collector. Empty or skipped-only reports never succeed.

| Format | Supported producer | Parser version |
| --- | --- | --- |
| `vitest_json` | Vitest 3.2.4 / 4.1.11 JSON reporter | `vitest-json-3.2.4/v1` |
| `playwright_json` | Playwright 1.55.0 / 1.55.1 JSON reporter; 1.63.0 exercised through schema 4 | `playwright-json-1.55.0/v1` |
| `trx` | VSTest 17.14.1 TRX logger, flat unit results | `vstest-trx-17.14.1/v1` |
| `jest_json` | Jest 30.2.0 JSON output | `jest-json/v1` |

Producer versions identify exercised combinations, not claims that untrusted input proves
which executable produced it. Schema 2 separates adapter intervals from actual
observations; see the [selection contract](tool-selection.md). Earlier fixtures in
`tests/fixtures/project-reports/` are hand-authored minimal examples. `jest.json`
was captured from the native Jest fixture, with path and timestamp normalized for
deterministic parsing tests; raw native runs retain the original reports.
Parser identifiers retain their original names because the normalized field
contract is unchanged. A `parser_version` names the reporter format baseline the
parser was written against, not the producer that emitted a report; for example,
a schema-4 Playwright 1.63.0 report is still recorded as `playwright-json-1.55.0/v1`.
The [Task 17 local execution](../archive/reviews/2026-09-29-project-owned-tool-versions-local.md)
records the schema-4 producer versions that were exercised. Actual producer versions remain separately bound by the
installed profile and checked version probes. Task 14 retains native execution
reports for the 1.0.1 producers; see the acceptance execution record.
Unrelated reporter metadata is ignored. Required summary counters, run timestamps,
outcomes, and identity fields must be present and well typed. Summary counters must
agree with individual tests; duplicate normalized identities are rejected. Input
is limited to 16 MiB. Failure and skip details present in supported result fields
are retained. No attachment, XML entity, or report-provided path is opened.

IDs are the caller namespace plus a SHA-256 digest of length-prefixed identity
components. Vitest uses reported file name and full test name; Playwright uses
spec ID and project ID; TRX uses test ID and test name. Reordering does not change
an individual ID. Vitest absolute paths can change IDs across checkout locations.
TRX duplicate test ID/name pairs and Playwright repeated spec/project pairs are
unsupported, including repeats whose results cannot be distinguished reliably.

## Vitest

The [pinned reporter source](https://raw.githubusercontent.com/vitest-dev/vitest/v3.2.4/packages/vitest/src/node/reporters/json.ts)
defines the JSON summary and flattened assertions. Pending, disabled, skipped,
and todo assertions normalize to skipped; todo has its own source counter.
Failed suites, pending suites, false success, and suite messages prevent success.
Suite counts include nested suites and cannot be equated with the file array.

This reporter does not expose retry history and its `onFinished` ignores the
unhandled-error argument. The adapter cannot reconstruct either. Projects needing
first-attempt success must disable retries in their Vitest configuration. The
collector must retain nonzero runner exit status even if the JSON says success.
Explicit skip reasons are not a dedicated field here; source statuses and any
available failure messages are preserved.

## Playwright

The [pinned reporter source](https://raw.githubusercontent.com/microsoft/playwright/v1.55.0/packages/playwright/src/reporters/json.ts)
provides nested suites and per-project attempts. Stats are reconciled against
source expected/unexpected/flaky/skipped categories before normalization. An
expected failure, flaky test, retry, interruption, timeout, missing attempt, or
attempt error cannot normalize to passed. Only one passed attempt with retry zero
and expectedStatus passed qualifies. Global errors prevent report success.
Annotations and attempt errors retain available explanations. Skipped tests
normalize to skipped only when their attempts are also skipped (or absent).

## TRX

The [pinned summary implementation](https://raw.githubusercontent.com/microsoft/vstest/v17.14.1/src/Microsoft.TestPlatform.Extensions.TrxLogger/ObjectModel/TestRunSummary.cs)
and [logger implementation](https://raw.githubusercontent.com/microsoft/vstest/v17.14.1/src/Microsoft.TestPlatform.Extensions.TrxLogger/TrxLogger.cs)
define counters and run completion. The logger sets executed to passed plus
failed; its remaining counters, including notExecuted, are zero. Skips are
therefore derived from individual NotExecuted results, not that counter.

Only UTF-8 TestRun documents in the TeamTest/2010 namespace with flat
UnitTestResult entries are supported. Passed, Failed, and NotExecuted individual
outcomes are supported; other outcomes and InnerResults are rejected. Run summary
Completed or Passed can succeed; Failed, Error, Aborted and run diagnostics cannot.
DTD parsing is disabled and XML nodes are capped at 500,000. Ordered tests and
nested data-driven result structures require a future explicit adapter version.

## Verification status

Rust regression cases cover normal reports, count mismatches, missing fields,
duplicate identities, skips, failures, empty reports, Playwright retry/expected
failure/global errors, and TRX namespace/DTD rejection. The current native harness
and its additional Jest test exercise real producers; see [acceptance](acceptance.md).
These local observations do not authenticate hosted CI or authorize promotion.

## Jest

The [Jest 30 CLI contract](https://jestjs.io/docs/30.0/cli) provides `--json`
and `--outputFile`. The adapter validates counters and each assertion, suite
failures, runtime errors, interruptions, snapshots and open handles. Every executed
test must report exactly one invocation; retry history, expected failures, skips,
todo, snapshot updates/unmatched/unchecked snapshots, empty discovery and absent
or malformed reports fail. Identities use the namespace, reported file and full
test name; duplicates are rejected. The native fixture uses Node syntax checking
and React server rendering, and explicitly exercises empty, missing, skipped and
retried report cases as well as failing assertions, correction and head replay.
