# Current status

## Task 16 consumer extension

[Task 16](tasks/task-16/README.md) adds opt-in project schema 3/selection schema 2
and custom bootstrap 1.2.0 for the TJERP-shaped consumer. Its exact toolchain and
composition are covered by [local verification](archive/reviews/2026-09-29-tjerp-consumer-compatibility.md).
This does not establish TJERP installation, new hosted CI evidence or production
promotion. Existing Task 15 external conditions below remain applicable.

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
Hosted mutation run `35050598715` passed the former policy on the same candidate: 461 mutants,
448 caught, 13 unviable, zero missed or timed out. Its downloaded artifact matched
the GitHub SHA-256, and local Rust validation passed separately for the core,
controller and workflow-source inventories. Those observations do not satisfy the
new requirement that every selected mutation must build and be caught: the new
validator rejects the old core/controller results for nonzero unviable counts.

The [strict mutation follow-up](archive/reviews/2026-09-21-task-15-strict-mutation.md)
records the later dirty candidate based on `470dcf6`, all thirteen compiler
diagnoses, the retained product-source parentheses, and twelve exact typed
generator replacements. The Rust runner reconciles full generation, native
selection and replacement execution, and CI invokes the same path. The complete
local `full-02` rerun caught 449 native selections and all 12 replacements: 461
executed mutants, zero unviable/missed/timeouts, five successful baselines, and
zero missing or unmapped entries. The twelve ill-typed generator exclusions are
recorded separately and receive no caught credit. Rust 1.85 and 1.95 workspace
tests each passed 349 / 3 ignored. The incomplete `full-01` record is not accepted.
The tested changes were subsequently committed as `1d48a92`, merged into main
as `c79571e`, and pushed on 2026-09-21 with owner authorization. A remote ref
query confirmed main at `c79571e0ff4d05a3e1deca4f827d54a07e70467b` immediately
after that push. The merged workspace passed 349 tests / 3 ignored. The mutation
execution record retains its original dirty-candidate binding; no new hosted
result or mutation rerun of the merge commit is claimed.
These checks do not establish a production release or a new hosted CI result.

The controller verifies owner-configured observer/producer signatures and creates
version 2 workflow-bound receipts. Its local cryptographic/CI transport tests use
fixture keys and simulated GitHub. Actual attester services, complete production
ledger ingestion, isolated deployment, publishing App credentials and independently
observed required knowledge remain the consuming owner's responsibility. Collector
output remains LocalUntrusted; local CodeGraph/LAT files do not authenticate it.

GitHub's PR #6 record, queried on 2026-09-21, reports `MERGED` at
`2026-09-16T05:45:57Z`, with merge commit `e2d9f9e`; the earlier open-Draft
observation remains in its dated record. This code integration is not evidence of
a protected merge or production promotion. On 2026-09-16 the connected GitHub App could observe the
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

The later full rerun and strict gate are now recorded in the
[strict mutation follow-up](archive/reviews/2026-09-21-task-15-strict-mutation.md):
449 native selections and 12 typed replacements were caught, with no unviable,
missed or timed-out result. The earlier hosted evidence remains bound only to
`751fa76`; neither the targeted check nor the later local suite establishes
production readiness.

## Verified evidence

- The [strict mutation follow-up](archive/reviews/2026-09-21-task-15-strict-mutation.md)
  distinguishes the former permissive result from the strengthened policy and
  records exact generation/replacement accounting and later local observations.
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
