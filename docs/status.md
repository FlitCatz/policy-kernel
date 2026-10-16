# Current status

## P2 bounded MSBuild generated output

The [P2 contract](integrations/consumer-schema3.md#compiler-generated-files-p2)
adds explicit reviewed NuGet graph opt-in for the exact `true` / `obj/generated`
property pair. XML inheritance, actual command/environment and real output-path
checks reject unsupported overrides. Rust negative fixtures preserve audit and
lock protections. The offline native fixture requires actual Mapperly output
read by an ArchitectureTests-style test, including an empty-output failure case.
This is local advisory fixture coverage, not TJERP adoption, authoritative CI,
Trusted evidence or production promotion. Frozen bootstrap assets are unchanged.
The [2026-10-16 execution record](archive/reviews/2026-10-16-msbuild-generated-output-local.md)
records **local scope satisfied** using the installed RC1 SDK and cached packages,
with a successful generated-source read and the required empty-output rejection.

## P3 explicit adequacy requirements

The [adequacy contract](integrations/adequacy.md) makes empty core policies an
explained non-pass and rejects all-empty consumer policies in schemas 3/4.
Negative names require changed-behavior policy IDs and executed-test mappings.
Legacy parsing and frozen bootstrap bytes remain unchanged; readiness now
requires adopting real requirements and collecting matching evidence.
This change is local advisory work, not TJERP adoption or promotion to Trusted.
No production promotion or authoritative CI decision is established by local tests.

## Native fixture dependency refresh

The Jest fixture uses 30.5.2 and overrides `js-yaml` beneath
`@istanbuljs/load-nyc-config` to 4.x, removing the vulnerable `argparse` 1 /
`sprintf-js` chain. React/TypeScript retains parser 8.42.0 and every frozen
bootstrap 1.1.0 pin; its parser-scoped `typescript-estree` 8.48.0 override
removes the vulnerable `fast-glob` / `micromatch` / `braces` chain.
Compatible lockfile updates fix `brace-expansion` and `source-map-js`, including
the project-owned frontend fixture. Bootstrap assets are unchanged.
These fixture changes do not establish hosted CI success or production promotion.

## P1 npm workspace shared lock

The optional [P1 workspace extension](integrations/npm-workspaces.md) adds
explicit npm members under one root graph in profile schema 4 / selection 3.
Root declarations, actual manifests, lock members and installed workspace links
must agree; each graph has one audit. Standalone npm and NuGet lock rules remain.

The [2026-10-16 local execution](archive/reviews/2026-10-16-npm-workspace-local.md)
records native conformance with the captain's revised Node 26.12.0 pin and
npm 11.20.0: TS6/TS5 resolution, root/member-only vulnerability detection and
clean recovery. Approved registry installations, cache and logs stayed inside
temporary fixtures. Local evidence is labelled **local scope satisfied** and
remains LocalUntrusted. This is PK fixture coverage, not TJERP adoption,
an authoritative CI decision, protected-branch merge or production promotion.

## Task 17 project-owned frontend tool versions

[Task 17](tasks/task-17/README.md) adds profile schema 4 / selection schema 3
for frontend npm consumers, optional provisioning, explicit adapter contract IDs,
schema-4 migration and custom bootstrap 1.3.0. The schema 2/3 .NET/fullstack
paths and their version bounds remain separate. Local Rust regressions exercise
reader/schema boundaries, legacy preservation, report contract selection,
runtime/provisioning agreement, migration and receipt ownership. The ignored
native matrix passed locally for the dirty candidate recorded in the
[Task 17 local execution](archive/reviews/2026-09-29-project-owned-tool-versions-local.md):
Node 26.10.0, npm 10.9.3/12.1.0, Vitest 4.1.11, Playwright 1.63.0 and Jest
30.2.0. It includes actual report, retry, advisory and service-error negative
cases. A [2026-09-30 follow-up](archive/reviews/2026-09-30-project-owned-tool-versions-follow-up.md)
records a repeat matrix with retained raw reports and a successful supplied
candidate fixture run. [Hosted PR #7 observations](archive/reviews/2026-09-30-project-owned-tool-versions-hosted.md)
record successful Task 17 native acceptance and release verification for exact
feature head `f75796a6a2bf26ca781df949aac7be03788e3567`. No authoritative
required-check decision, protected-branch merge or production promotion is
recorded here.

## 2026-09-30 codebase review remediation

The [remediation record](archive/reviews/2026-09-30-codebase-review-remediation.md)
covers a local change set on top of `91ee415`. It closes a not-applicable
exemption that matched any changed path, a review conflict that an extra approval
could trigger, and a pre-merge gate that ignored reasonless non-ready results.
Controller signatures now use explicit canonical JSON, and `policy-ci` JSON
inputs use the controller's bounded reader. The mutation contract was reconciled
with the edited sources. It records local checks only. Hosted results, a new
full mutation run and production promotion are not claimed. The Task 17 hosted
observations above remain bound to `f75796a`.

Hosted Release verification and Task 4 sensors later succeeded for the merge
commit `0510f41`. The subsequent [remaining area review](archive/reviews/2026-09-30-remaining-area-review.md)
makes bootstrap-installed files follow the umask and adds `sensors.rs` to the
strict mutation gate (557 of 557 local mutants caught). It likewise records local
checks only. Hosted Release verification and Task 4 sensors then succeeded for
`c8b74b9`. The later [design decision changes](archive/reviews/2026-09-30-design-decisions.md)
bind sensor configuration independently of checkout location, bind execution and
adequacy evidence to the head, cross-check merge protection with GitHub's branch
observation, and make `*` in path globs segment-local. They also record local
checks only.

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
