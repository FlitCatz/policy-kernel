# Task 10 validation

Task 10 adds the GitHub adapter, authoritative `policy-ci` pre/post-merge commands,
and reusable composite Action. It preserves the existing pure evaluator and
contract schemas. The Action consumption contract is in
[`action/README.md`](../../action/README.md).

## Verification

Run in the isolated `task10-github-ci` worktree with the pinned Rust toolchain:

```text
rtk cargo test --offline -p github-adapter -p policy-ci
cargo test: 25 passed (6 suites, 2.06s)

rtk cargo test --offline --workspace
cargo test: 115 passed (23 suites, 1.82s)

rtk cargo clippy --offline --workspace --all-targets -- -D warnings
cargo clippy: No issues found

rtk cargo fmt --all -- --check
exit 0

rtk git diff --check
exit 0
```

The initial three CI guard tests failed against the stub. The integration tests
exercise the real binary/evaluator with temporary Git repositories and a fake
`gh` transport. They cover authoritative ready/blocked/inconclusive publication,
exact-head and current-run bindings, altered evidence/observations, squash shipping,
forged check rejection, and a merge racing with Check Run publication. Restoring
the earlier publication-timestamp behavior made the race regression fail;
restoring post-response receipt time made it pass.

Adapter tests cover the 4 KiB canonical Check Run binding cap, external-ID and
workflow/attempt matching, exact head versus squash commit, label/work-item
normalization, route injection, numeric IDs, request/output limits, process failure,
and a child retaining stdout after the direct `gh` process exits. The Action test
executes its shell block and checks literal argument preservation and mode rejection.

## Integration decisions

- Add `actions: read` to the plan's four caller permissions because the workflow
  REST endpoint needs it for private repositories.
- Preserve the specified neutral conclusion for inconclusive decisions. GitHub
  treats neutral checks as passing, so promotion must require the failing Actions
  job alongside the dedicated Check Run. The Action documentation states this.
- Use trusted runner publication and merge receipts alongside fresh API responses.
  Check Run payloads do not prove their own workflow provenance; current REST PR
  data does not establish historical protection or an unambiguous merge method.
- Use the trusted post-response publication time as the core-facing completion
  bound. This conservatively rejects checks published during/after merge even
  though the API binding records an earlier whole-second completion timestamp.
- The caller must protect the collector, observation storage, workflow, immutable
  Action revision, binary and executable search path from evaluated PR code.
  Directory location is a consistency check, not authentication of arbitrary JSON.

No live GitHub writes, merge operations, protection changes, package publication,
or deployment were performed. Live promotion remains a later integration step.

## Final review

The final review identified a mismatch between the Actions `GITHUB_SHA` and REST
workflow-run `head_sha` for PR workflows. The receipt now binds both independently:
`workflow_trigger_head_sha` checks the Actions environment; `workflow_head_sha`
checks the API response and is retained in publication receipts. Correcting the
fixture to a PR head reproduced the failure before the fix. The corrected fixtures,
success paths and separate tamper cases pass. Scoped re-review approved the fix
with no residual findings.
