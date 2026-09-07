# Task 7 Post-merge Implementation Notes

**Goal:** Authorize `Shipped` only from a protected merge and the exact successful
required check that published an authoritative pre-merge decision.

**Interface:** `evaluate_post_merge(&PostMergeEvaluationInput) -> PostMergeDecision`
is a pure deterministic function. It reads no files, processes, network, clocks,
Git state, or environment.

## Contract additions

- `PostMergeEvaluationInput.check` is optional so missing observation evidence is
  representable and returns `PK-POST-CHECK-MISSING`.
- Policy optionally declares `protected_target_branches` and
  `allowed_merge_methods`. Empty or legacy-absent declarations cannot authorize
  shipping and are omitted when serialized, preserving historical policy bytes.
- Merge observations optionally record `target_branch`, closed-enum `actor_type`,
  and `via_merge_queue`. Their absence fails closed. A true queue observation is
  rejected with `PK-POST-MERGE-QUEUE-UNSUPPORTED` until a queue-specific contract
  is declared.
- Post-merge decisions optionally record the distinct `merge_commit_sha`; the
  evaluator always supplies it, while omission preserves old serialized values.

The check and merge must equal the authoritative decision on repository, pull
request number, exact PR `head_sha`, policy digest, and decision digest. The check
name must equal the policy declaration and conclude successfully. The merge
commit SHA is output evidence and is not compared to the PR head, which permits
ordinary merge and squash commits.

## Time and freshness

Chronology is
`pre_merge.binding.evaluated_at <= check.completed_at < merge.merged_at <= evaluation_time`.
The check may start before pre-merge evaluation because one check run can span the
evaluation. Equality at check completion and merge is rejected because the check
must have completed before merge.

The policy maximum age is inclusive. Pre-merge evaluation, check start, and check
completion are measured against merge time, so republishing a check cannot renew
a stale pre-merge authorization. Merge time is measured against the injected
post-merge `evaluation_time`. A value exactly at maximum age is accepted; one
nanosecond beyond it is rejected.

## Digest and trust boundary

The post-merge decision digest hashes a versioned domain envelope containing the
complete typed input and the complete result excluding only the self-referential
digest field. This binds policy and observation fields, the separately recorded
merge commit SHA, status, reasons, and evaluation time.

The kernel verifies explicit values and their authorization relationships. Task
10's trusted GitHub adapter is responsible for authenticating raw required-check,
actor, branch-protection, and merge observations before constructing these
contracts. A caller-created object is not itself proof of GitHub provenance.
