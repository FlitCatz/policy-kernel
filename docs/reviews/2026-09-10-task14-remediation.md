# Task 14 follow-up remediation

The owner authorized remediation, publication and CI execution. This record
supersedes the 1.0.1 candidate for the three defects discovered in subsequent
codebase review; it does not confer operational policy authority.

## Changes and regression evidence

- Fullstack previously audited only its frontend npm graph. New bootstrap 1.0.2
  adds required `shared:dependencies`, with the shared package/lock/runtime/config
  inputs, npm 10.9.3 version probe and `npm audit --audit-level=low`. All 15
  sensors must pass. A staged-profile regression first passed despite a shared-only
  failure; it now blocks that failure and rejects missing shared audit evidence.
  Existing 1.0.0/1.0.1 assets remain byte-identical, with cross-release receipt,
  downgrade and uninstall tests. Dependency pins are unchanged from 1.0.1.
- The public SensorResultBinding schema now matches Rust validation for nonblank
  parser/executable names, nonempty argv and NUL rejection. Differential tests
  cover Unicode whitespace and valid empty trailing arguments. The regression
  failed before the schema fix and passes after deterministic regeneration.
- GitHub CLI execution now owns a process group, retains its unreaped leader
  until cleanup and uses bounded nonblocking I/O. Descendants are killed on
  timeout, success, process failure, overflow and blocked input; detached pipe
  threads are eliminated. The original descendant-survival repro failed before
  the change. This implementation supports Linux/macOS and rejects unsupported
  platforms before spawning.
- A push-triggered release run at 235fe2c failed because the unknown-command CLI
  test raced writing unused stdin against child exit (`BrokenPipe`). The test now
  checks the same command rejection without writing unused input. The separately
  passing PR-triggered run did not prove the push run passed.

Separate read-only engineering review found no substantive defects in these
fixes. Workspace tests: 281 passed, 1 live GitHub test ignored. This review is
from the same provider as code contributors and is not a policy-independent
approval. Native and remote validation results are retained in PR #4, bound to
the final commit; old mutation evidence must not be relabeled for a new commit.

Local 1.0.2 runs completed with exit 0 under
`/private/tmp/task14-remediation-{react-typescript,dotnet,fullstack}`. All three
baselines and five negative/corrected scenarios passed, including old-head replay
rejection. Fullstack's new shared audit passed alongside its frontend audit.
Strict workspace Clippy, formatting and residue checks passed. Schema generation
is deterministic under the schema regression suite. These runs were performed
before the follow-up commit and are local advisory evidence.

## Actual integration constraints

Authenticated reads of both `branches/main/protection` and applicable `rulesets`
returned HTTP 403 on 2026-09-10: GitHub requires Pro or a public repository for
these private-repository features. The repository stays private; no protection
configuration or account billing was changed.

The Action consumes observations issued by an external trusted controller;
no deployed controller, production policy snapshot or independent-provider
approval packet is supplied here. Hosted fixture execution remains advisory.
The existing promotion protocol also requires real shadow samples (14 days,
30 distinct heads), protected merge and rollback observations. Owner permission
to execute work is already granted; these are missing capabilities/observations,
not a request for that permission again.
