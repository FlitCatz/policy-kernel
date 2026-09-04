# Policy Kernel Design — Antigravity Review Record

> Date: 2026-09-04
> Artifact: `docs/superpowers/specs/2026-09-04-policy-kernel-extraction-design.md`
> Source head: `e5f6ec8584cc91fdeca3aab40687c6cdcd303b43`
> Executor: `/opt/homebrew/bin/agy` 1.1.26
> Provider/model: Antigravity / `gemini-3.8-flash-high`
> Mode: read-only design critique with the document embedded in the prompt

## Initial independent review

- Conversation ID: `b764cb3e-b87e-4984-8be6-65ffb51677f6`
- Result: `REJECTED`
- Duration: 129.180227 seconds
- Tokens: 21,176 input; 32,496 output; 30,245 thinking; 53,672 total

The initial review reported eight findings. Each was checked against the design
rather than accepted by default.

| Severity | Finding | Adjudication and design change |
|---|---|---|
| Critical | Contract state inconsistency | Valid. Split pre-merge and post-merge inputs, decisions, and evaluator functions. Only the post-merge evaluator may return `SHIPPED`. |
| Critical | Circular trust for local OMP evidence | Valid. Local OMP evidence is always advisory. Required alternate-provider review must be captured in CI or by an explicitly accepted trusted runner. Added a narrow evidence collector. |
| High | Rollback depended on removed legacy state | Valid. Rollback now restores a predeclared native GitHub protection and deterministic-CI baseline; it does not reactivate GSD Core. |
| High | PR head and merge/squash SHA were conflated | Valid with semantic correction. The merged PR's `headRefOid` must match the evaluated PR head; the resulting merge commit OID is recorded separately. Merge queues require a separate contract. |
| High | Repair-loop orchestration leaked into the kernel | Valid. Retry budgets, failure comparison, and debugger selection remain in the external workflow. The kernel evaluates only submitted evidence and independence. |
| Medium | LAT/CodeGraph provenance was underspecified | Valid. CodeGraph now records source-set, tool, configuration, database, and head digests; required use must be rebuilt in CI or trusted-attested. The database alone is never proof. |
| Medium | Exit code zero could admit a no-op sensor | Valid. Added typed sensor results and separate test-adequacy evidence with acceptance-criterion, executed-test, and negative-case mapping. |
| Medium | GitHub observation was missing from evaluator input | Valid. Explicit aggregate pre-merge and post-merge inputs now contain the applicable GitHub observations. |

## Revision re-review

- Conversation ID: `23b6eb1f-6f76-42fb-a9f2-279256590df3`
- Result: `REJECTED`
- Duration: 138.670884 seconds
- Tokens: 21,983 input; 30,397 output; 28,711 thinking; 52,380 total

The reviewer confirmed all eight initial findings resolved and reported three new
findings:

| Severity | Finding | Adjudication and design change |
|---|---|---|
| High | Independence checked only against the last modifier | Valid. Independence now excludes every provider and execution that contributed code to the evaluated change set. A repair by a second provider cannot make the original producer an eligible verifier. |
| High | Post-merge decision was not anchored to a published required check | Valid. Added `GitHubCheckObservation`; post-merge evaluation must verify that the exact decision and policy digests were published successfully for the same PR head before merge. |
| Medium | Shared core lacked advisory/authoritative evaluation mode | Valid. Added explicit `EvaluationMode`; the core applies the trust threshold, labels its output, and rejects advisory decisions as post-merge authorization. |

## Second revision re-review

- Conversation ID: `bb2ee3dd-0d20-403a-be68-a07292f88f5d`
- Result: `APPROVED`
- Duration: 66.278642 seconds
- Tokens: 22,325 input; 10,201 output; 9,820 thinking; 32,526 total
- New Critical/High/Medium findings: none

The reviewer marked all three findings from the first revision re-review as
`RESOLVED`. The design has therefore passed the requested independent Antigravity
architecture critique. This verdict validates the written design only; it is not
implementation or end-to-end operational evidence.
