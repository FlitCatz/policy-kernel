# Single-owner review policy in v4

Policy v4 optionally accepts owner authorization plus an independent automated
review for a single-owner repository. This is a core contract, not authentication
of GitHub records or an update to the reference controller's review floor.

## Declaration and compatibility

Add this independent optional field to a policy v4 snapshot:

```json
"owner_with_independent_agent_review": {
  "owner_actor": "repository-owner",
  "maximum_age_seconds": 604800
}
```

The owner identity must be nonblank without NUL, and the age must be positive.
`required_reviews` remains mandatory. Versions 1–3 reject the field, including
null; v4 rejects null too. When absent, v4 retains its existing quorum evaluation.
Legacy policy/evidence serialization and digests are unchanged when the new fields
are absent. Adoption changes the policy digest and requires owner reapproval and
evidence binding to that digest; no existing policy must migrate.
Older readers reject unknown fields; upgrade the parsing/evaluation binaries
before adopting this option or its evidence metadata.

## Evidence and evaluation

Every review under this option identifies the same authenticated PR author through
its optional `identity` object. Identities are canonical, case-sensitive actor IDs
supplied by the trusted observer, not display names or model prose. An integration
must authenticate the current repository's author and all modifier/reviewer actors;
the kernel cannot discover omitted contributors or authenticate caller-created JSON.

If the author differs from `owner_actor`, the normal independent-review quorum
applies. If the author equals the owner, one approving automated review plus
exactly one separate owner authorization replaces the numerical quorum. Provider
allowlists, distinct providers/executions, provider/execution independence from all
contributors, negative findings, conflicts, and waiver rules still apply.

Each modification supplies an `actor`. Every review supplies an `identity` with
`actor`, `kind` (`automated` or `human`), and `pull_request_author`. The owner path
requires every reviewer to be explicitly automated, with an actor different from
the PR author and every modifier. A provider or model name never implies actor
independence. Missing modifier evidence or actor identity fails closed.

Attach exactly one `owner_approval` to one of the review records. It has `actor`,
`kind` (`approval` or `comment`), `repository`, `head_sha`, `record_digest`,
`observed_at`, and `trust`. A comment must be an explicit authorizing statement
recognized by the trusted observer; an arbitrary owner comment is not approval.
The actor must match the policy owner and PR author. Its digest identifies the
separate authorization record and must differ from every automated report digest.
Additional authorization records are ambiguous and fail closed.

Both authorization and reviews bind the evaluated head. Their timestamps must be
at or before evaluation time and no older than the smaller of the owner option's
age and `max_evidence_age_seconds`. The boundary is inclusive, with fractional
seconds preserved. For a seven-day window use `604800` and a global age at least
that large; the option never extends the existing global limit.

Owner authorization, automated reviews, and modifier identities require
`ci_observed` or `trusted_runner_attested`, including in advisory mode under this
option. Neither `local_untrusted` nor `local_runner_attested` satisfies it, even
when `local_runner_evidence` is enabled. The new optional evidence fields remain
inside schema-1 evidence, are covered by the review/modification child hashes and
semantic evidence/decision digests, and are omitted when absent.

## Rejection reasons

Failures remain inconclusive unless an existing finding rule blocks the change.
Existing review/provenance reasons may accompany these distinct reasons.

| Reason | Missing or invalid evidence |
| --- | --- |
| `PK-REVIEW-AUTHOR-MISSING` | Missing, blank, malformed, or inconsistent observed PR author |
| `PK-REVIEW-OWNER-MISSING` | No owner authorization |
| `PK-REVIEW-OWNER-AMBIGUOUS` | Multiple owner authorization records |
| `PK-REVIEW-OWNER-IDENTITY` | Authorization actor differs from the pinned owner |
| `PK-REVIEW-OWNER-BINDING` | Authorization repository or head differs |
| `PK-REVIEW-OWNER-RECORD-NOT-SEPARATE` | Authorization reuses an automated report digest |
| `PK-REVIEW-OWNER-STALE` | Authorization is stale or future-dated |
| `PK-REVIEW-OWNER-UNTRUSTED` | Authorization lacks CI-grade trust |
| `PK-REVIEW-MODIFIER-IDENTITY-MISSING` | Missing modifier evidence or actor identity |
| `PK-REVIEW-MODIFIER-UNTRUSTED` | Modifier identities lack CI-grade trust |
| `PK-REVIEW-AGENT-MISSING` | No automated review or no approving review |
| `PK-REVIEW-AGENT-NOT-AUTOMATED` | Reviewer kind is human |
| `PK-REVIEW-AGENT-IDENTITY-MISSING` | Blank or malformed reviewer actor |
| `PK-REVIEW-AGENT-SELF-AUTHOR` | Reviewer actor is the PR author |
| `PK-REVIEW-AGENT-SELF-MODIFIER` | Reviewer actor is a modifier |
| `PK-REVIEW-AGENT-NOT-INDEPENDENT` | Reviewer shares a modifier's provider or execution |
| `PK-REVIEW-AGENT-BINDING` | Automated review names a different head |
| `PK-REVIEW-AGENT-STALE` | Automated review is stale or future-dated |
| `PK-REVIEW-AGENT-UNTRUSTED` | Automated review lacks CI-grade trust |

## Integration limits

The [policy schema](../../schemas/policy.schema.json) and
[evidence schema](../../schemas/evidence-bundle.schema.json) describe the wire
shapes. [Trust boundaries](../trust-model.md) still apply. The reference controller
continues to require its existing approving-provider floor; an adapter must
authenticate these new observations before relying on the core option. Unit and
fixture tests do not demonstrate a deployed authoritative decision, consumer
adoption, protected merge, or production promotion.
