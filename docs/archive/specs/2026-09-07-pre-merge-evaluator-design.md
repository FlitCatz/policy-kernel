# Task 6: pre-merge evaluation

Approved scope: implement the existing Task 6 plan, including the missing sensor,
agent-ready and workflow inputs identified during inspection.

## Contract

Keep schema 1 legacy wire values readable without rewriting stored bundles.
Add an optional `sensor_profile_digest` to `PolicySnapshot` and an optional
versioned `PreMergeContext` to `PreMergeEvaluationInput`. Neither absent field is
serialized. Absence is compatible for readers but insufficient for readiness.
Existing strict readers reject these extensions; this is not backward reader
compatibility or a frozen distribution protocol (Task 8/12 remains responsible).

Context supplies a sensor profile, known changed paths, risk, current workflow run
ID and an optional `AgentReadyObservation`. The observation binds schema,
repository, work item, PR number, exact head, actor, label, time and trust.
The actor must occur in policy.authorized_actors and the label must be agent-ready.
All observations and context come from the trusted integration boundary, never
from ambient Git, a process, environment variables, or a system clock.

The policy digest is canonical SHA-256 of the serialized snapshot with only its
own policy_digest removed. The profile digest binds the full sensor profile.
The current PR supplies repository/PR/base/head; the current work item and workflow
must match the manifest. This checks consistency, not authenticity of caller data.

## Gates

Check all gates without early success: runtime schema/policy/profile validity;
open, non-draft PR and authorized work item; fresh, trusted and authorized
agent-ready; original manifest child integrity and provenance; at least one
modification and execution; execution status and timestamps; required sensors and
adequacy; independent reviews and human waivers. All runtime schema fields are
checked even for directly constructed Rust structs.

Resolve conditional profile requirements and union them with policy-required
sensor IDs. Keep the change context when checking not-applicable claims. An empty
changed-path list means a known empty change, never unknown impact. Missing
adequacy is inconclusive even when a profile has no mapping requirements.
Executed failures block; missing, stale or unverifiable evidence is inconclusive.
Aggregate precedence is Blocked > Inconclusive > ReadyToMerge. Advisory mode is
recorded and cannot convert into authoritative post-merge authorization.

## Digests and determinism

Validate raw child digests before any normalization. Do not mutate or re-sign the
input bundle. For semantic evidence identity only, sort the top-level evidence
record arrays by canonical JSON bytes and omit the transport child_digests list.
Keep nested ordered data such as command argv unchanged. The waiver target digest
also omits waivers, avoiding a self-reference cycle; repository, policy, head and
all non-waiver evidence remain bound. The decision binding uses this target.

The decision digest hashes a domain-tagged envelope containing the decision
without its own digest and a normalized complete input projection. That input
includes waiver content, policy, evaluation context and external observations.
Sort reasons by (code, message), deduplicate identical reasons, and sort manifest
transport entries out of the semantic projection. Tampered original transport
digests still prevent readiness through the integrity gate. Reordered records
must have freshly correct raw child digests to remain valid.

## Verification

Use static full input fixtures for ready, missing-agent-ready, head-mismatch and
advisory. Exercise every gate through evaluate_pre_merge, including authorized
waivers, failed/missing execution and sensors, binding replay, unknown runtime
schema, profile substitution, empty suites and future fractional timestamps.
Repeat ready evaluation 100 times and permute reviews, sensors and findings with
valid transport digests. Verify full workspace tests, formatting and strict Clippy.
