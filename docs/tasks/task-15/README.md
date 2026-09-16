# Task 15 — Compatibility, runtime independence, and verification completion

## Current status

Local implementation and verification for 15.01–15.10, and the 15.12 documentation
record, are complete in the existing `feat/task-15-compatibility` worktree.
Candidate `751fa76` also passed hosted Release, Rust compatibility and all four
native consumer jobs. Hosted mutation run `35050598715` also passed on the
same candidate (448 caught, 13 unviable, zero missed or timed out).
Task 15 as a whole remains incomplete: 15.11 requires external
services, approvals and live operational evidence.

The [follow-up verification record](../../archive/reviews/2026-09-15-task-15-follow-up-verification.md)
identifies the uncommitted candidate, actual commands, source/executable bindings,
native results and remaining constraints. The
[foundation record](../../archive/reviews/2026-09-12-task-15-foundation-verification.md)
retains its earlier 15.01–15.03 facts. The
[hosted record](../../archive/reviews/2026-09-16-task-15-hosted-verification.md)
records the published candidate and subsequent remote observations.
The dated 2026-09-11 plan and earlier reviews
remain historical; this page tracks subsequent implementation.

## Work packages

| ID | Deliverable | Current state |
| --- | --- | --- |
| 15.01 | Current profile entry points | Updated together in both READMEs; 1.1.0 is explicit, historical profiles retained |
| 15.02 | Framework independence | Isolated built-binary regressions retained for four documented product paths |
| 15.03 | Rust compatibility and locked builds | Rust 1.85.0 / 1.95.0 local macOS ARM64 and hosted Ubuntu/macOS verification on candidate 751fa76 |
| 15.04 | Common tool/runtime/package/path selection | Implemented across profile, bootstrap, runner, acceptance, CI inputs and evidence |
| 15.05 | User profiles and receipt ownership | Schema 2 / release 1.1.0; user source remains unowned; old receipts/assets and recovery retained |
| 15.06 | Supported adapters and alternate consumption | npm/NuGet roles explicit; Jest 30.2.0 React SSR consumer with alternate Node/npm and web path |
| 15.07 | Every selected dependency graph | Separate app/Unit/Integration/shared audits; renamed sensor checks; conditional test-only NuGet bypass reproduced and rejected |
| 15.08 | Explicit required knowledge | Policy schema 2 enforcement, policy digest, provenance and old-reader/schema regressions |
| 15.09 | Authenticated production reference | Observer/producer signatures, complete inventory and exact run/workflow/App bindings; local cryptographic CI tests |
| 15.10 | Compatibility and regression verification | Local checks complete; hosted Release/matrix, four native jobs and mutation run 35050598715 passed; hosted mutations 448 caught / 13 unviable, zero missed or timed out |
| 15.11 | Live candidate and promotion evidence | Exact-candidate GitHub Actions observations recorded; adopted policy, independent reviews, deployment and operational evidence still required |
| 15.12 | Documentation and execution record | Separate local and hosted records preserve candidate/source/artifact bindings and remaining conditions |

## External conditions

Local verification used worktree changes based on
`bb7dab9cc329f4b7445fa4acbfc37ed44106d7ce`. The owner subsequently authorized
committing and pushing `feat/task-15-compatibility`, now published at
`751fa760bb0fb85e8c7b739f1ef8f9dc782a1da0`. Draft PR #6 started native verification;
the archived local record preserves its pre-publication observations. No new
worktree, merge or production promotion was performed.

On 2026-09-16 Release run `34966889607` and native run `35021204028` passed on
the published candidate. PR Release run `35021203828` also passed, with its
synthetic merge checkout recorded separately. All check metadata identifies
GitHub Actions App `15368`. Restored user CLI authentication allowed mutation run
`35050598715` to start on the same head. Classic protection and repository/parent
rulesets both returned HTTP 403 requiring GitHub Pro or public visibility through
that authentication. The earlier connected App's classic-protection response was
an integration-permission failure. No protection baseline has been obtained.

The consuming owner still needs to supply/adopt the exact candidate, policy and
profile, independent observer/producer identities and keys, complete execution
ledger, isolated controller/publication environment, publishing App/workflow
identity, and two reviewers independent of each other and all modifiers.
Changing account/visibility/protection settings and operational promotion still
require separate authorization. The promotion protocol
also requires its owner approvals, 14 days / 30 distinct heads of shadow
observations, protected merge, post-merge decision and live rollback evidence.
Local fixture signatures or a directory outside a checkout do not satisfy these.

## References

- [Approved consolidated plan](../../archive/plans/2026-09-11-task-15-compatibility-and-verification.md)
- [Common selection and support contract](../../integrations/tool-selection.md)
- [Bootstrap and user-profile ownership](../../integrations/bootstrap.md)
- [Report adapters](../../integrations/report-formats.md)
- [Native acceptance](../../integrations/acceptance.md)
- [Authenticated controller](../../../action/controller.md)
- [Final codebase review](../../archive/reviews/2026-09-10-final-codebase-verification.md)
- [Dependency optionality review](../../archive/reviews/2026-09-10-dependency-optionality-review.md)
- [Current status](../../status.md)
- [Trust model](../../trust-model.md)
- [Promotion protocol](../../promotion.md)
