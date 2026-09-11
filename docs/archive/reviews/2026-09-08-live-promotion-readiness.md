# Live promotion readiness after Task 10

## Verified

The reviewed implementation and live smoke are on draft
[PR #2](https://github.com/FlitCatz/policy-kernel/pull/2), initially validated at
`b8b525833c671d02398a4dcd9a36421783441545`.

- [Linux CI passed](https://github.com/FlitCatz/policy-kernel/actions/runs/34182571915).
- [Real GitHub publication/read-back passed](https://github.com/FlitCatz/policy-kernel/actions/runs/34182571957).
- The real `policy-ci` process returned `BLOCKED`/exit 1 and published
  [failure Check Run 101924591418](https://github.com/FlitCatz/policy-kernel/runs/101924591418).
  Its PR head, workflow run/attempt, policy digest and decision digest matched.
- GitHub Actions is enabled. The publishing job received checks:write,
  contents:read, pull-requests:read, issues:read and actions:read.

This verifies the non-ready publication path. It does not establish a real
READY_TO_MERGE authorization or SHIPPED state.

## Current blockers

Read-only GitHub API checks on 2026-09-08 returned:

```text
GET /repos/FlitCatz/policy-kernel/branches/main
protected: false
protection.enabled: false
required_status_checks.checks: []
required_status_checks.contexts: []

GET /repos/FlitCatz/policy-kernel/branches/main/protection
HTTP 403
Upgrade to GitHub Pro or make this repository public to enable this feature.

GET /repos/FlitCatz/policy-kernel/rulesets
HTTP 403
Upgrade to GitHub Pro or make this repository public to enable this feature.
```

Both classic branch protection and rulesets are unavailable in this private
repository under its current account capability. No protection setting was
changed. The repository must remain private unless its owner explicitly chooses
otherwise; publicity is not an automatic workaround.

There is also no production policy snapshot, authenticated evidence bundle, or
trusted observation set for the task10 head in this branch. The live smoke
intentionally has an unauthorized work item and empty execution/review evidence.
Those inputs cannot be relabeled as real evidence. Task9's collector implementation
exists on a separate development branch and has not been integrated into task10;
it does not by itself supply a trusted project collection workflow or independent
review evidence.

## Concrete next sequence

1. The owner enables private-repository branch protection capability, or supplies
   an already protection-enabled private validation repository. Recheck the two
   endpoints above before preparing any protection mutation.
2. Prepare an isolated validation target branch and PR with an approved policy.
   Present the exact target and protection configuration for review before
   applying them. Do not silently use main for this experiment.
3. A trusted collection workflow captures real current-head execution/sensor
   results, test adequacy, contributor identities, independent review, work-item
   authorization and exact-head agent-ready observation. Retain current-run
   receipts outside contributor-controlled storage.
4. Evaluate the evidence with `policy-ci pre-merge`. Publish success only if the
   actual core decision is READY_TO_MERGE. Require both the dedicated policy check
   and the Action job, because GitHub accepts neutral checks as passing.
5. Retain `pre-merge-input.json` and `publication-receipt.json` through authenticated
   artifact storage tied to the original workflow/run/attempt.
6. An authorized human performs the approved protected merge. The trusted merge
   integration records actual method, protection at merge time, human actor,
   queue use, PR head, resulting merge SHA and timestamp. Do not infer historical
   protection from a later branch query.
7. A separate trusted post-merge run fetches the original Check Run and workflow,
   verifies the receipts and calls `policy-ci post-merge`. Record SHIPPED only if
   every binding and merge-policy condition passes.

READY_TO_MERGE/SHIPPED cannot be established by synthesizing receipts, weakening
policy, reusing the smoke's fabricated placeholders, or treating an unprotected
merge as protected. Account billing changes, publication of the repository,
protection mutations, and actual merge operations require the owner's decision.
