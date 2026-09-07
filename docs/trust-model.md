# Trust model

The kernel decides from explicit typed observations. It cannot authenticate the
origin of caller-created JSON. A valid digest proves content consistency, not the
identity, execution, or permission of the producer.

## Boundaries

| Component | Responsibility | Authority |
| --- | --- | --- |
| policy-contracts | Strict versioned wire parsing and schemas | No observation authentication |
| policy-core | Deterministic binding, provenance, review, sensor and merge decisions | Trust depends on authenticated inputs |
| policy-cli | Force advisory pre-merge evaluation; inspect supplied data | Cannot issue a required check or authorize shipping |
| evidence-collector | Bounded explicit-argv execution and immutable artifact digests | Local execution alone remains untrusted |
| github-adapter / policy-ci | Authenticate current GitHub workflow and API observations; publish a bound check | Required Task 10 integration boundary |
| Consuming workflow and human owner | Preserve contributor identities, execute independent reviews, protect merge policy | Promotion requires separate owner authorization |

Every code-modifying execution belongs in modification evidence. Reviewers must be
independent of every contributor by both provider and execution identity. Calling
multiple agents from the same model provider is not provider independence. Model
prose naming a provider, a local signature, and a structurally valid CI-shaped JSON
object do not establish trusted provenance.

All identities bind the exact repository, PR number, PR head, policy and evidence
content, workflow run and explicit evaluation time. Unknown versions/fields,
missing evidence, stale timestamps, malformed digests, process failure, timeout,
truncation and authentication errors cannot become success. Clock readings and
GitHub access stay outside the core.

## Pre-merge and post-merge

Local evaluation always sets advisory mode before computing the decision digest.
`inspect` validates typed decisions and bundle integrity, but cannot recompute or
authenticate a decision without its original evaluation input and trusted source.

`ReadyToMerge` is not `Shipped`. Shipping requires an authoritative ready decision,
the trusted required check that published its exact decision/policy binding, and
a protected merge of the same PR head. Check completion must precede the merge.
The merge commit is distinct from the PR head, so squash is allowed. Direct,
unprotected and unsupported merge-queue observations cannot authorize shipping.
Raw API authentication and check provenance must be established by Task 10;
core-only unit tests do not prove that boundary operates against live GitHub.

## Verification evidence

The obligation matrix and frozen legacy expectation fixtures preserve specified
behavior; they do not execute the historical application or prove shadow parity
against a deployed service. Property tests vary real inputs and exercise real
validators. Mutation testing covers the five authorization modules specified in
Task 11. A completed nonempty run must contain zero missed mutations and zero
timeouts; unviable mutants remain separately counted, never counted as killed.
Tool failure, missing files, incomplete outcomes and skipped coverage cannot pass.

Knowledge claims and indexes provide context, not approvals. Present index evidence
is checked against the head and source-set binding. The current policy contract
cannot declare mandatory knowledge/index presence; a stale supplied index test
does not establish enforcement of a missing required index. That contract gap
must be resolved before claiming such a project requirement is protected.
Project profiles and bootstrap are separate Tasks 12–14. Do not interpret the current SensorProfile
schema as proof that report normalization or a consuming project actually ran.

See [promotion and rollback](promotion.md) for the required evidence and explicit
separation between fixture drills and live operations.
