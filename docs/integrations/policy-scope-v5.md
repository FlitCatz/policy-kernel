# Policy v5 PR scope rules

A [policy snapshot](../../schemas/policy.schema.json) with `schema_version: 5`
requires a digest-bound `scope_rules` object. The kernel resolves the required
sensors from the approved policy and current PR observations.

```json
{
  "default_required_sensor_ids": ["compile"],
  "rules": [
    {"when": "paths", "path_globs": ["src/**"], "required_sensor_ids": ["tests"]},
    {"when": "label", "label": "run-ci", "required_sensor_ids": ["tests", "lint"]}
  ]
}
```

Each path rule applies when any changed path matches any of its globs. Globs
use the [sensor glob subset](../trust-model.md#boundaries) with
literal characters, segment-local `*`, and whole-segment `**`. The default
applies when the change is empty or when any changed path matches no path rule,
so a change that mixes matched and unmatched paths still requires it. A label
rule adds its IDs when a supplied PR label matches its name using ASCII
case-insensitive comparison. The policy and context retain their original
label spelling. Labels preserve the path or default requirements, and unknown
labels have no effect on scope. The kernel
unions all applicable IDs with the policy's `required_sensor_ids` and applicable
requirements from the bound sensor profile. Overlap contributes a single ID.
In the example, an unmatched unlabeled change requires `compile` and adding
`run-ci` also requires `tests` and `lint`. An unlabeled change to `src/lib.rs`
and `README.md` requires both `tests` and `compile`.

Each requirement list is nonempty with distinct sensor IDs. Every referenced
sensor must exist in the bound profile, including references in inactive rules.
Each path rule declares one or more supported globs. A rule label must be
nonblank ASCII without NUL. The context can still carry unrelated non-ASCII PR
labels. A policy that only uses the default can declare `"rules": []`.

## Observations and evaluation

Provide the complete `changed_paths` and `labels` lists in the current
`PreMergeContext` for the PR under evaluation. The kernel and controller rely
on the caller for the accuracy and completeness of both lists. The controller
can verify the signed context and its digest. That binding authenticates the
submitted values. A syntactically valid list with omitted entries can narrow
the required scope. The planned `gate` command will read both lists directly
from GitHub.

An empty list represents a known empty result. If collection fails or is
incomplete, preserve that failure. Missing or malformed observations prevent
readiness through input rejection or `PK-PRE-SCOPE-INVALID`. The latter also
covers invalid glob syntax and unknown sensor references. Refresh scope when
the labels or changed paths change.

V5 can omit the caller's `risk`. If supplied, risk has no effect on the
requirements or verdict status.
An inherited risk-conditioned sensor or knowledge requirement can still apply
when its path condition matches. A risk-conditioned sensor exemption cannot
apply without trusted risk. Existing path-only exemptions still require
coverage of every changed path. All other pre-merge gates, including test
adequacy, provenance, and authorized agent readiness, still apply. V5 can
declare the optional `local_runner_evidence` block with the same validation and
acceptance rules as v4. It also accepts the optional
[`owner_with_independent_agent_review`](owner-review-policy-v4.md) block.
Because v5 ignores caller risk, a risk-restricted
`ci_required_when` condition applies whenever its path condition matches.

Rust callers with complete typed observations can use
`policy_core::resolve_required_scope` to get deduplicated sensor IDs in ID order.
The pre-merge evaluator also verifies policy and profile digests and evaluates
the required evidence. `policy-cli evaluate-pre FILE` runs the existing advisory
entry point with the v5 input. It exits 0 for readiness, 2 for blocked, 3 for
inconclusive and 64 for invalid input. Native `evaluate-scope` accepts an
independently approved `ExpectedScopeV1`.

Policies v1 through v4 retain their existing bytes, digests and verdicts.
They reject `scope_rules` and require a non-null `risk` in any supplied context.
Labels do not affect their verdict, and the decision digest still binds the
supplied context. `evaluate-pre` exits 64 when the legacy risk is missing or
null. The generated schema for `PreMergeEvaluationInput` uses the policy version
to enforce this requirement. A standalone `PreMergeContext` schema allows
optional risk because it has no policy version. The authoritative pre-merge
and controller entry points validate the context against their policy before
publishing a decision or issuing a receipt.
