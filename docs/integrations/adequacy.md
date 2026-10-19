# Adequacy requirements

An executed test count is not requirement coverage. The adopted policy names
requirements, and evidence maps those IDs to tests that actually executed.

## Empty policy and negative cases

All three lists empty is invalid for consumer `ProjectProfile` schemas 3/4.
Both `validate-profile` (exit 64 for invalid input) and typed deserialization
reject it; the generated project schema also rejects the all-empty shape.
There is no opt-out field or new adequacy schema. At least one list must contain
a requirement; negative IDs additionally require matching changed-behavior IDs.

Direct core evaluation reports `PK-ADEQ-POLICY-EMPTY`, regardless of whether
evidence exists or reports positive counts. Missing evidence also reports
`PK-ADEQ-MISSING`; the existing pre-merge missing-evidence guard is unchanged.
Policy defects produce an inconclusive result unless another failure blocks it.

Every declared or claimed negative ID must occur in the policy's
`changed_behavior_ids`, and its `changed_behaviors` mapping must name executed
test IDs. Merely listing `negative_case_ids` is not coverage. Required mappings
cannot be omitted. IDs must be nonblank and unique within each policy list,
negative evidence list, mapping list and mapping's test list. Reusing a test for
different requirements is permitted; repeating a test within one mapping is not.
Counts must agree with unique executed IDs and remain nonzero.

The collector's report adapters supply normalized test IDs, not bare method
names. Resolve the original reporter's selectors to these IDs, retain the raw
reports, and submit mappings for the execution being evaluated. `run` collects
evidence; its successful exit does not establish business adequacy or readiness.
Evaluate the returned bound sensor profile and evidence with the core.

## Compatibility change on 2026-10-16

Schemas 1/2 and frozen bootstrap profiles retain their bytes and parsing.
An empty legacy profile now yields `PK-ADEQ-POLICY-EMPTY` instead of silently
skipping adequacy. A previously parseable legacy profile can therefore cease to
be ready under the new evaluator. Adopt a new consumer policy and collect fresh
evidence; do not rewrite historical receipts or promote old evidence.
Migrating an empty legacy profile to schema 4 also fails validation: author the
requirements in a consumer-owned copy before requesting that migration.

The frozen `tests/fixtures/legacy/sensor-missing.json` still parses unchanged.
Its historical `PK-SENSOR-001` expectation was Inconclusive for a missing sensor;
the new evaluator additionally blocks its unmapped negative name. The legacy
regression explicitly checks this change without editing the historical matrix.
Current synthetic pre-merge fixtures include negative mappings and refreshed
bindings so their unrelated gates remain testable.

The maintained Jest and synthetic split-consumer examples declare greeting
requirements. Their request authors must map those requirements to actual
report IDs; neither example is a product business-requirement catalog. Frozen
empty templates are compatibility inputs, not runnable adequacy examples.

## Ownership and evidence limits

The slice owner maintains its requirement catalog; the captain owns business
requirements and reviews list shrinkage separately. The kernel cannot discover
a requirement removed from both an adopted policy and its external catalog.
Removing only its evidence mapping fails. Consumer-specific catalog rules belong to the consumer-owned harness; see the
[TJERP ownership pointer](tjerp-adoption.md) for its historical pilot boundary.

Local observations remain LocalUntrusted and may only be described as **local
scope satisfied** when the declared scope is actually satisfied. These checks
do not authorize a protected merge, production promotion or Trusted evidence.
