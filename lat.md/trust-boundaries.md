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
