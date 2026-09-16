# Task 15 — Compatibility, runtime independence, and verification completion

## Purpose

Consolidate the remaining review findings into executable work: remove mandatory
workflow-framework coupling if present, support project-selected compatible tools
and versions, complete dependency and knowledge evidence requirements, and connect
collection to authenticated CI evaluation.

## Current status

Planned; implementation has not started. The consolidated plan was written against
`abd6cb0a1fcd1440cc96a05bfe4ce00280742511` on 2026-09-11.

## Work packages

| ID | Deliverable |
| --- | --- |
| 15.01 | Correct current profile entry points and remaining status documentation |
| 15.02 | Prove operation without workflow frameworks; remove runtime coupling if found |
| 15.03 | Separate Rust build compatibility from consumer runtime requirements |
| 15.04 | Unify tool, runtime, package, and project-path selection |
| 15.05 | Separate profile contracts from releases; preserve bootstrap ownership |
| 15.06 | Add replaceable tool and report adapters with explicit compatibility |
| 15.07 | Audit every selected dependency graph, including .NET test projects |
| 15.08 | Enforce explicitly required knowledge/index evidence |
| 15.09 | Implement and verify the authenticated collection-to-CI integration |
| 15.10 | Run compatibility, regression, and native acceptance verification |
| 15.11 | Collect candidate-specific live promotion evidence |
| 15.12 | Publish final status, support boundaries, and evidence links |

## Evidence and limits

The three defects in the first review were remediated in `5841d8b`; their regression
coverage must be preserved. Framework names in historical records or exclusion
tests do not establish a runtime dependency. The .NET test-only audit concern
still requires a reproduction before it is classified as a confirmed bypass.

Local implementation and validation can finish before production prerequisites
are available. Track those states separately; Task 15 as a whole remains incomplete
while required live evidence is missing. Writing this plan does not perform or
authorize a production promotion.

## Related implementation and references

- [Consolidated work plan (한국어)](../../archive/plans/2026-09-11-task-15-compatibility-and-verification.md)
- [Final codebase verification](../../archive/reviews/2026-09-10-final-codebase-verification.md)
- [Dependency optionality review](../../archive/reviews/2026-09-10-dependency-optionality-review.md)
- [Task 14 remediation](../../archive/reviews/2026-09-10-task14-remediation.md)
- [Current status](../../status.md)
- [Trust model](../../trust-model.md)
- [Promotion protocol](../../promotion.md)
