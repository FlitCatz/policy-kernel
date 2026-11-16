# Trust boundaries

The kernel's [trust model](../docs/trust-model.md) distinguishes content
consistency from authenticated origin. The [promotion protocol](../docs/promotion.md)
defines the evidence required before an owner authorizes production promotion.

Local CLI results, evidence collection, project verification, and consumer
acceptance are advisory or local evidence until authenticated CI binds them to the
actual repository, head, policy, and protected merge.

The [GitHub Action contract](../action/README.md) defines the authoritative CI
boundary. The [controller reference](../action/controller.md) authenticates independent
attestations with owner-installed keys; local signing tests do not establish a
production provider or deployment. See [[status]] for what has and has not been demonstrated, and
[[tasks]] for the implementation map.

## Single-owner authorization

The optional v4 owner rule requires a separate owner authorization and an independent automated approval, with explicit actors and CI-grade trust. Local-runner evidence cannot supply review authorization.

See [[kernel#Single-owner review policy]] and the
[wire contract](../docs/integrations/owner-review-policy-v4.md). The trusted observer
must authenticate the PR author, all modifiers, reviewer identities, and the
authorizing content of owner comments. Typed fields and digests do not authenticate
these observations. The reference controller's existing review floor is unchanged.

## Planned local-runner attestation

Core accepts `local_runner_attested` for fresh sensors and executions under explicit v4 opt-in and no CI requirement. It never permits shipping; authenticated collection and issuance remain planned.

The [decision record](../docs/decisions/2026-11-12-local-runner-attestation-adr.md)
holds the threat model and the default-closed rules; the
[trust model](../docs/trust-model.md#planned-local-runner-attestation) summarizes
them. The authenticated collection boundary would be the only place that raises
the class. Reviews, modifications, agent-ready and required knowledge never accept
it, and changes to the checking system stay CI-required. Current local collection
remains untrusted; core acceptance does not authenticate a submitted label.

Signatures must come from a hardware security key with the user-presence flag set.
That flag proves a touch, not which statement the owner approved, so an agent under
the owner's account could get a fabricated statement signed during an expected
touch. This residual risk is open; its mitigation is an owner decision.
