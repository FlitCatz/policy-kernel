# Trust model

The kernel decides from explicit typed observations. It cannot authenticate the
origin of caller-created JSON. A valid digest proves content consistency, not the
identity, execution, or permission of the producer.

## Boundaries

Current pre-merge publication uses `success` only for `READY_TO_MERGE` and
`failure` for both `BLOCKED` and `INCONCLUSIVE`. Titles, step outputs and exit
codes preserve the decision distinction. A required dedicated check therefore
blocks both non-ready results. Keep the invoking Actions job required as defence
in depth. Upgrade the runner binary and republish outstanding legacy neutral
checks before relying on this mapping.

Schema-1 check bindings and publication receipts retain their shape. Trusted-check
parsing authenticates the receipt and digest, not a decision status supplied by
the check title; it rejects legacy `neutral` conclusions and accepts `failure`
observations for either non-ready decision. No neutral-to-failure reinterpretation
is performed. Post-merge still requires an authoritative ready decision and an
actual `success` observation for `SHIPPED`.

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
object do not establish trusted provenance. (A planned, owner-enabled exception for
signed local-runner attestations is described under
[Planned local-runner attestation](#planned-local-runner-attestation); it is not
implemented.)

Policy schema v3 requires a digest-bound `required_reviews` object:
`minimum_reviewers` is a positive integer (1–4294967295),
`require_provider_independence` must be `true`, and `allowed_providers` is an
explicit list (empty permits any provider). The existing independence invariant
cannot be disabled: review providers must be distinct and independent of every
contributor, and execution IDs must be unique. Every supplied review must use an
allowed provider. Only distinct executions with `approved` status count toward
the quorum; waived findings do not turn a blocked report into an approval.
Below quorum is inconclusive with `PK-REVIEW-QUORUM`; an excluded provider produces
`PK-REVIEW-PROVIDER-NOT-ALLOWED`. Other provenance and blocking checks still apply.

V1/v2 policy documents retain their serialization and policy digests, but cannot
authorize readiness or shipping: `PK-POLICY-V3-MIGRATION-REQUIRED` requires owner
reapproval as v3. No implicit quorum is inserted under an old digest. Migration
adds `required_reviews`, changes `schema_version` to 3, and recomputes the policy
digest; evidence, current-run bindings, and approvals must bind the new digest.
V1/v2 reject the new field (including null); v3 rejects its absence, null, zero
quorum, or disabled independence. Direct Rust construction is checked too.
This policy version is separate from project-profile schema versions; existing
consumer project profiles are not automatically rewritten or reapproved.

The reference controller retains its floor of two approving providers. Its
`minimum_reviewers` configuration must equal
`max(2, policy.required_reviews.minimum_reviewers)`; the field is a consistency
assertion, not an independent policy knob. Policy-core enforces the declared
quorum for every caller, including local advisory evaluation and other adapters.

All identities bind the exact repository, PR number, PR head, policy and evidence
content, workflow run and explicit evaluation time. Unknown versions/fields,
missing evidence, stale timestamps, malformed digests, process failure, timeout,
truncation and authentication errors cannot become success. Clock readings and
GitHub access stay outside the core.

Sensor `required_when` conditions apply when any changed path matches. A
`not_applicable_when` exemption is valid only when its risks match and every
changed path matches one of its `path_globs`; an empty known change cannot be
exempted by a path condition. A mixed documentation and source change therefore
cannot use a documentation-only exemption. Sensor and knowledge path globs accept
a conservative subset of the [GitHub Actions filter syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#filter-pattern-cheat-sheet).
Patterns match the whole normalized repository-relative path, case-sensitively.
Literal characters are ASCII letters, digits, `.`, `_`, and `-`, with `/` separating
nonempty segments (segments `.` and `..` are invalid). `*` matches zero or more
characters within a segment; `**` is accepted only as an entire segment and crosses
directories. Thus `src/*.rs` excludes `src/nested/lib.rs`, while `src/**/*.rs`
includes both `src/lib.rs` and `src/nested/lib.rs`. `**/README.md` also matches the
root `README.md`. Changed paths retain the existing normalization of backslashes,
repeated separators and `.` segments; parent traversal is rejected.

All other pattern characters and forms are rejected, including `?`, `+`, brackets,
braces, backslash escaping, negation (`!`), whitespace, non-ASCII literals, and
embedded/repeated globstars such as `**.js`, `a**b`, or `***`. Pattern lists are
unordered positive alternatives; GitHub's ordered exclusion/re-inclusion syntax
is unsupported. Invalid patterns yield `PK-SENSOR-PROFILE-INVALID` or
`PK-KNOWLEDGE-POLICY-INVALID`, even when their conditions would not apply.
An empty `path_globs` list still means any path; an empty pattern is invalid.

In particular, GitHub's `src/*.jsx?` means `.js` or `.jsx`, whereas globset's `?`
means one arbitrary character. This pattern now fails closed: use the two entries
`src/*.js` and `src/*.jsx`. Replace `**.js` with `**/*.js`. This subset keeps the
existing matcher small without silently accepting incompatible syntax. Existing
shipped profiles and fixtures use supported patterns and need no migration.

A blocked review whose Medium, High
and Critical findings are all validly waived does not conflict with an approving
review; any unwaived, expired or low-only blocked report still does. A gate that
withholds readiness without a reason fails closed with `PK-PRE-GATE-UNEXPLAINED`.
Authoritative execution and test adequacy evidence must name the manifest head.

## Planned local-runner attestation

**Planned, not yet implemented.** The
[2026-11-12 decision record](decisions/2026-11-12-local-runner-attestation-adr.md)
accepts a design for letting a single-owner repository use signed local-runner
evidence for ordinary pull-request readiness. Today every statement above still
holds: local evidence is `local_untrusted` and cannot authorize readiness.

The planned design adds a trust class, `local_runner_attested`, that is distinct
from and weaker than `ci_observed` and `trusted_runner_attested`:

- A registered local runner signs a statement binding repository, head, tree, base,
  an approved scope template and the run and result digests. The signing key is
  required to need the owner's touch or biometric confirmation for each signature:
  the controller verifies the SSH signature with `ssh-keygen -Y verify` against an
  owner-controlled `allowed_signers` file, accepts only hardware-backed
  security-key types, and then parses the SSHSIG signature blob to require the
  user-presence flag (`0x01`), which `ssh-keygen -Y verify` does not enforce.
- The authenticated collection boundary verifies the signature against
  owner-installed keys, checks the bindings against GitHub observations,
  recomputes the scope verdict itself and promotes only the covered sensor and
  execution evidence to the new class. Core evaluation performs no cryptography.
- The authoritative verdict is computed in the lightweight GitHub job that already
  runs for each pull request. A local command can never issue it.
- A decision that relies on the class carries the informational reason
  `PK-PROV-LOCAL-ATTESTED`; such a decision cannot reach `SHIPPED` or promotion.

Default-closed rules, all of which keep today's behavior unless the owner opts in
through a new policy version:

- Without a local-evidence block in the approved policy, the class is rejected.
- `local_untrusted` never counts in authoritative evaluation.
- Reviews, modifications, agent-ready observations and required knowledge are
  never accepted at the local class.
- Changes matching the policy's CI-required conditions need CI-grade evidence.
  Beyond security and dependency updates, static-analysis policy transitions,
  changes resolved only by hosted review records, and the scheduled main-branch
  check, this includes changes to the checking system itself (runner, check
  tooling, workflows, toolchain pins).
- Unregistered runners, unapproved scope templates, mismatched repository, head,
  tree or base, stale or future timestamps and an unsatisfied re-evaluation are
  rejected.
- Signatures from key types other than `sk-ssh-ed25519@openssh.com` and
  `sk-ecdsa-sha2-nistp256@openssh.com`, or whose security-key flags lack the
  user-presence bit, are rejected.

A signature fixes who vouched for what; it does not prove the run was honest.
Residual risks accepted by the decision are environment divergence between the
local and hosted platforms, ignored build output, selective submission of passing
runs and a PR manipulating its own tests. The scheduled hosted check of the main
branch is the compensating control. An open residual risk remains: a touch proves
presence, not which statement was approved, so an agent under the owner's account
can get a fabricated statement signed during a touch the owner expected; a
mitigation such as showing the owner the statement digest before signing is an
owner decision still open before the attestor lands. Hardware security-key
signing has not yet been exercised on the owner's machine; if the presence check
proves unworkable, the decision returns to the owner. See the decision record for
the threat model.

## Pre-merge and post-merge

Local evaluation always sets advisory mode before computing the decision digest.
This remains true under the planned local-runner attestation: only the hosted
authoritative path can use the new class.
`inspect` validates typed decisions and bundle integrity, but cannot recompute or
authenticate a decision without its original evaluation input and trusted source.

`ReadyToMerge` is not `Shipped`. Shipping requires an authoritative ready decision,
the trusted required check that published its exact decision/policy binding, and
a protected merge of the same PR head. Check completion must precede the merge.
The receipt's protection claim must match GitHub's current branch observation.
The merge commit is distinct from the PR head, so squash is allowed. Direct,
unprotected and unsupported merge-queue observations cannot authorize shipping.
Under the planned local-runner attestation, a ready decision that relies on
`local_runner_attested` evidence also cannot authorize shipping.
Raw API authentication and check provenance must be established by Task 10;
core-only unit tests do not prove that boundary operates against live GitHub.

## Verification evidence

The obligation matrix and frozen legacy expectation fixtures preserve specified
behavior; they do not execute the historical application or prove shadow parity
against a deployed service. Property tests vary real inputs and exercise real
validators. Mutation testing covers the five authorization modules specified in
Task 11 plus sensor evaluation (`sensors.rs`), added on 2026-09-30. A completed nonempty run must contain zero missed mutations and zero
timeouts; unviable mutants remain separately counted, never counted as killed.
Tool failure, missing files, incomplete outcomes and skipped coverage cannot pass.

Knowledge claims and indexes provide context, not approvals. Policy schemas v2/v3 can
declare `required_knowledge` entries with `kind` (`lat_claim` or `code_graph_index`),
an exact `source` identity, and `required_when` conditions. An empty condition list
means always required. Otherwise any condition applies when both its `risks` and
`path_globs` match the current change; an empty dimension matches everything.
Globs use sensor path normalization and segment semantics, including rejection of
parent traversal.
Duplicate kind/source declarations and malformed conditions fail closed.

Each applicable requirement needs exactly one matching observation. Missing
knowledge, wrong kind, wrong source, head mismatch, source-set mismatch, insufficient
trust, duplicate observations, and invalid tool-version metadata have separate
`PK-KNOWLEDGE-*` reasons. Required knowledge must be `ci_observed` or
`trusted_runner_attested`, including in advisory evaluation. An adopted CodeGraph
requirement therefore needs CI regeneration or a trusted attestation binding its
exact tool version, index configuration digest, content digest, source-set digest,
and head. A local `.codegraph/` directory or LAT navigation document does not
satisfy that obligation. The controller must authenticate those observations;
caller-created trust labels remain insufficient. Knowledge cannot replace tests
or independent reviews.

The declaration and policy schema version enter the policy digest, and all
knowledge metadata enters the evidence and decision digests. Evidence bound to a
previous policy is rejected after adopting or changing a requirement. The new
reader preserves v1 policy serialization and digests, and accepts v2 policies
with no declaration without requiring knowledge. Legacy policies still require
the v3 review-policy migration above before readiness. A v1 policy cannot carry
`required_knowledge`, even an empty array or null; v2 rejects null. Old readers
reject v2 and unknown declarations. Evidence and decision wire versions remain
v1 because their existing fields already carry the required bindings. Existing
checks on supplied optional knowledge remain in force.
Project profiles and bootstrap are separate Tasks 12–14. Do not interpret the current SensorProfile
schema as proof that report normalization or a consuming project actually ran.

See [promotion and rollback](promotion.md) for the required evidence and explicit
separation between fixture drills and live operations.
