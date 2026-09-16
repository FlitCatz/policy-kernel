# Current status

This page is the canonical summary of the repository's current documentation
state. It links to dated records for evidence and does not replace their
contemporaneous statements.

## Implemented scope

Tasks 1–14 provide implemented components in this workspace: typed policy contracts
and core evaluation; advisory and GitHub Actions boundaries; bounded collection; release
verification; project profiles; transactional bootstrap; and native consumer
acceptance. The [Task index](tasks/README.md) describes each scope and its
supporting records.

## Task 15 local implementation and external conditions

[Task 15](tasks/task-15/README.md) now implements shared tool/version/path selection,
schema 2 user profiles and safe 1.1.0 bootstrap receipts, Jest report support,
complete selected dependency graph auditing, policy-declared required knowledge,
and a cryptographically authenticated controller reference. Frozen 1.0.0–1.0.2
assets and legacy policy/receipt semantics remain available. Current fullstack
1.1.0 has 17 required sensors including separate .NET app, Unit and Integration audits.

Local native React, .NET, fullstack and alternate React/Jest runs passed on macOS
ARM64. Rust 1.85.0 and 1.95.0 each passed 340 workspace tests; three ignored tests
are counted separately (two native network cases were executed explicitly, one
live GitHub smoke remains ignored). Native empty/missing report and nested npm
omission regressions passed. Final mutation checks covered 461 mutants: 448 caught,
13 unviable, zero missed or timed out. Local 15.10 checks and the 15.12 record are complete;
see the [follow-up execution record](archive/reviews/2026-09-15-task-15-follow-up-verification.md).
These local observations were collected before committing the follow-up.
Published candidate `751fa760bb0fb85e8c7b739f1ef8f9dc782a1da0` subsequently passed
the hosted Rust 1.85/1.95 Ubuntu/macOS matrix and the Release verification job,
each with 340 passed / 3 ignored tests. Four hosted native jobs also passed:
React, .NET, fullstack and alternate Node/npm/Jest. The
[hosted execution record](archive/reviews/2026-09-16-task-15-hosted-verification.md)
binds those observations to the candidate, run, workflow, App and artifact hashes.
Hosted mutation run `35050598715` also passed on the same candidate: 461 mutants,
448 caught, 13 unviable, zero missed or timed out. Its downloaded artifact matched
the GitHub SHA-256, and local Rust validation passed separately for the core,
controller and workflow-source inventories. These checks do not establish a
production release.

The controller verifies owner-configured observer/producer signatures and creates
version 2 workflow-bound receipts. Its local cryptographic/CI transport tests use
fixture keys and simulated GitHub. Actual attester services, complete production
ledger ingestion, isolated deployment, publishing App credentials and independently
observed required knowledge remain the consuming owner's responsibility. Collector
output remains LocalUntrusted; local CodeGraph/LAT files do not authenticate it.

The owner authorized committing and pushing the feature branch; draft PR #6 is
open without a merge. On 2026-09-16 the connected GitHub App could observe the
candidate's successful runs, but classic protection read returned HTTP 403
`Resource not accessible by integration`. The separate repository/parent rulesets
read returned HTTP 403 requiring GitHub Pro or public visibility. After user CLI
authentication was restored, both classic protection and repository/parent rulesets
returned the GitHub Pro/public-visibility requirement. A protection baseline remains
unavailable. No billing, visibility, protection or promotion settings were changed.

The three defects from the final codebase review were remediated in `5841d8b`,
including the fullstack shared audit in bootstrap 1.0.2. Task 15 preserves those
regressions; its first batch corrects current entry points without rewriting the
historical root profile assets or earlier execution records.

## Mutation follow-up after candidate 751fa76

Commit `470dcf6` parenthesizes the input-length cast in `controller::read_file`.
A local targeted cargo-mutants 27.1.0 run passed the policy-ci baseline and
compiled and caught the `>` to `<` mutation that previously failed to compile.
Its artifacts are in the feature worktree's
`target/task15-read-file-parentheses/mutants.out/` directory.

The full mutation suite has not been rerun on this updated source. The earlier
461-mutant hosted result remains evidence only for `751fa76`. The current gate
still permits unviable results; stricter rejection and reviewable handling of the
remaining invalid generated mutations are pending follow-up work. The targeted
result does not establish a new full-suite count or production readiness.

## Verified evidence

- The [Task 15 hosted record](archive/reviews/2026-09-16-task-15-hosted-verification.md)
  records exact-candidate Release and native runs, artifact integrity, PR merge/head
  distinctions, recovered CLI authentication and remaining operational conditions.
- The [Task 15 follow-up record](archive/reviews/2026-09-15-task-15-follow-up-verification.md)
  records the new selection/audit/knowledge/controller behavior, native results,
  source and executable bindings, and current external constraints.
- The [Task 15 foundation verification record](archive/reviews/2026-09-12-task-15-foundation-verification.md)
  records the first implementation batch, framework-reference classification,
  two-compiler local checks, and the boundaries of that evidence.

- The [final codebase verification record](archive/reviews/2026-09-10-final-codebase-verification.md)
  records local automated checks and CI observations for its reviewed candidate,
  while also recording defects and unfinished work.
- The [Task 14 execution record](archive/reviews/2026-09-09-task-14-consuming-acceptance.md)
  records local native acceptance evidence for React/TypeScript, .NET, and
  full-stack fixtures.
- The [Task 14 remediation record](archive/reviews/2026-09-10-task14-remediation.md)
  records follow-up changes and validation context.

These records are evidence for their stated revisions and environments. They do
not authenticate a later candidate automatically.

## Not demonstrated in production

The repository does not claim a completed production promotion. The following
remain governed by the release protocol: an owner-approved candidate, authenticated
exact-head required-check observations, protected-merge evidence, the required
shadow window, independent-provider reviews, and a live rollback observation.

## Canonical references

- [Task index](tasks/README.md)
- [Task 15 consolidated follow-up](tasks/task-15/README.md)
- [Trust model](trust-model.md)
- [Promotion and rollback protocol](promotion.md)
- [Project verification integration](integrations/project-verification.md)
- [Bootstrap integration](integrations/bootstrap.md)
- [Consumer acceptance integration](integrations/acceptance.md)
