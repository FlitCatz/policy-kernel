# ADR: Signed local-runner attestation for ordinary PR readiness

Date: 2026-11-12. Decision: accepted design; implementation planned and not yet
available. This record partly supersedes one guardrail of the archived
[2026-10-19 scope-reduction ADR](../archive/specs/2026-10-19-scope-reduction-adr.md),
as described under [Supersession](#supersession). Current implementation status
belongs to [Current status](../status.md).

## Context

A consuming project runs its heavy verification on the owner's machine before a
single push, and keeps the hosted CI for a few named cases. Today the kernel cannot
take part in that flow. Local native results are typed `local_untrusted` with the
review gate `unmet`, and authoritative pre-merge evaluation rejects any
`local_untrusted` evidence with `PK-PROV-UNTRUSTED`. A person therefore compares
the recorded head and result by eye. The recorded result is an unsigned line, not
bound to a tree, a base, a runner identity or an approved scope.

The goal is to let the kernel accept evidence produced by a registered local
runner for ordinary changes, without weakening what a hosted, independently
operated run proves.

## Decisions

All six are accepted as recommended. Each is default-closed: with no explicit
owner opt-in, behavior is exactly as before.

1. **Signed attestation with a weaker class.** A local runner signs a statement
   binding repository, head, tree, base, approved scope template and the native
   run and result digests. The authenticated collection boundary verifies it and
   promotes only the covered sensor and execution evidence to a new trust class,
   `local_runner_attested`. The class is distinct from, and weaker than,
   `ci_observed` and `trusted_runner_attested`, and appears in the decision as an
   informational reason (`PK-PROV-LOCAL-ATTESTED`) that cannot be removed without
   changing the decision digest.
2. **Per-signature owner presence.** The runner's signing key is required to need
   the owner's physical touch or biometric confirmation for each signature (a
   hardware-backed security key). A key file readable by the owner's account is not
   accepted, because an agent running under that account could sign fabricated
   results. The requirement is checked, not left to procedure: the statement is an
   SSH signature verified with `ssh-keygen -Y verify` against an owner-controlled
   `allowed_signers` file. The controller accepts only the security-key types
   `sk-ssh-ed25519@openssh.com` and `sk-ecdsa-sha2-nistp256@openssh.com` and
   rejects any `allowed_signers` entry that carries the `no-touch-required`
   option. A signature made without user presence therefore fails verification,
   and a plain software key is refused.
3. **Verdict in the light hosted job.** The authoritative verdict is computed in
   the lightweight GitHub job that already runs for every PR, so head, base,
   changed paths and the clock come from GitHub observations rather than from the
   local machine. A local-only advisory verdict stays advisory.
4. **CI evidence stays required for named classes.** Changes that the consuming
   project already routes to hosted CI stay CI-required: security and dependency
   updates, static-analysis policy transitions, changes whose resolution depends on
   hosted review records, and the scheduled main-branch check. Changes to the
   checking system itself (the runner, check tooling, workflows, toolchain pins)
   are added to the CI-required set, because a change must not be approved by the
   checker it modifies. The path and risk conditions are policy data owned by the
   consuming project (`ci_required_when`).
5. **Platform divergence accepted.** A passing local run on one operating system
   does not prove the same result on the hosted Linux environment, and ignored
   build output can influence a clean-looking tree. The owner accepts this
   residual risk. The compensating control is the scheduled hosted check of the
   main branch, with a documented procedure to trace a failure to its PR and to
   re-evaluate trust in the runner.
6. **No shipping from local-class evidence.** A ready decision that relies on
   `local_runner_attested` evidence may support merge readiness only. `SHIPPED`
   and promotion require a ready decision built from CI-grade evidence.

## Threat model

Actors and goals:

| Actor | Capability | Goal of this design |
| --- | --- | --- |
| Honest but error-prone worker | Runs another head, forgets a rerun, runs part of the checks, uses a dirty tree | Prevent |
| Shortcut-taking agent | Same OS account and credentials as the owner; can edit or invent result files | Prevent: the controller accepts only security-key signatures made with the owner's per-signature presence (decision 2) |
| Malicious code inside the PR under test | Runs with the runner's permissions during local execution; can read files the account can read | Prevent key theft; manipulating its own tests is equally possible in hosted CI and is handled by review and by the CI-required set |
| Outsider with repository write access only | Can push branches, notes and comments; no access to the owner's machine | Prevent |
| Out of scope | The owner, a fully compromised owner machine, the hosting platform | Treated as the root of trust |

A signature fixes who vouched for what. It does not prove that the run was honest.
Hosted CI earns trust from an independently operated environment, which is why a
local attestation is a separate, weaker class rather than an equal.

Bindings the design relies on, each checked at the authenticated boundary against
GitHub-observed values or owner-controlled configuration:

- Head commit, tree and base commit, so a result cannot be reused for a different
  or later state of the change.
- Source-set and inventory digests recorded by an attestor that checked the tree
  was clean and unchanged across the run.
- An approved scope template digest, so weakened expectations (lower test-count
  floors, filters, different parsers or configuration) are not accepted.
- Original report hashes and reparsing, performed by the attestor that signs.
- A registered runner identity and an owner-installed key registry; contributor
  supplied keys are never accepted.
- Freshness measured on the evaluator's clock, with a local-evidence-specific
  maximum age.
- A signing domain string and the policy digest, so a signature cannot be reused
  for another purpose or after a policy change.
- Re-evaluation: the boundary recomputes the scope verdict itself and takes sensor
  and execution evidence from that recomputation, not from submitted claims.
- Independent review is untouched: local attestation can never raise the trust
  class of reviews, modifications, agent-ready observations or required knowledge.

Residual risks the design does not close:

- **Environment divergence.** See decision 5.
- **Selective submission.** Retries within one run are forced to zero, but running
  the whole suite repeatedly and submitting only a pass is not detectable without
  a ledger.
- **Ignored build output.** A clean `git status` does not exclude influence from
  ignored artifacts. A fresh worktree per attested run is recommended and is the
  consuming project's responsibility.
- **A PR manipulating its own tests.** Identical to hosted CI; addressed by review
  and the CI-required set, not by trust class.
- **Unproven key custody.** A key readable by the same account as an agent cannot
  stop that agent; decision 2 refuses such keys and signatures made without the
  owner's presence. Hardware security-key signing has not yet been exercised on
  the owner's machine. If the check in decision 2 proves unworkable there, the
  decision returns to the owner rather than being relaxed to a software key.

## Default-closed rules

1. A policy with no local-evidence block behaves exactly as today. A
   `local_runner_attested` value in such a bundle is rejected. Existing policy
   versions keep their serialization and digests.
2. `local_untrusted` is never usable in authoritative evaluation, under any
   configuration.
3. Reviews, modifications, agent-ready observations and required knowledge are
   never accepted at the local class. Required knowledge continues to need
   `ci_observed` or `trusted_runner_attested`.
4. A change matching a CI-required condition rejects local-class evidence.
5. Unregistered or unlisted runners, unapproved scope templates, a different
   repository, head, tree or base, an expired or future timestamp, and a
   re-evaluation that is not satisfied are all rejected.
6. `policy-cli` stays advisory-only. No local command can issue an authoritative
   verdict; authoritative evaluation still requires the hosted Actions
   environment.
7. `LocalScopeResultV1` keeps `local_untrusted` and an unmet review gate. The
   attestation is a separate, additional document.
8. A ready decision that depends on local-class evidence cannot reach `SHIPPED`.
9. Keys and expected values come only from owner-controlled configuration.
10. A signature is rejected unless its key is a hardware-backed security-key type
    (`sk-ssh-ed25519@openssh.com` or `sk-ecdsa-sha2-nistp256@openssh.com`), its
    `allowed_signers` entry has no `no-touch-required` option, and
    `ssh-keygen -Y verify` confirms the owner's presence for that signature.

## Supersession

The 2026-10-19 ADR states that local results remain `LocalUntrusted` with review
unmet, that scope satisfaction does not imply `ReadyToMerge`, and that no synthetic
provider, model or review may substitute for the authoritative observer and
controller path. That archived text is not edited.

This ADR supersedes that guardrail **only for the attestation path**, once it is
implemented: sensor and execution evidence covered by a verified signed
attestation may be promoted to `local_runner_attested` and, under the rules above,
support merge readiness. Everything else in the guardrail still holds: local
results and the native facade stay `local_untrusted`; reviews are never
substituted; hashes alone do not authenticate a producer. Until the implementation
lands, the archived guardrail describes the actual behavior.

## Consequences and sequencing

The change touches shared contracts and the authorization modules, so it lands as
a sequence of reviewed pull requests merged one at a time:

1. This record and the trust-model revision (documentation only).
2. A behavior-preserving change that turns every "reject `local_untrusted`" check
   into an allow-list of CI-grade classes. This must precede adding a class: with
   deny-list checks, a new class value would silently pass every one of them.
3. Contracts: the new class and a new policy version with an optional
   local-evidence block (allowed runners, approved scope templates, maximum age,
   CI-required conditions).
4. Core acceptance rules, new reason codes and the shipping restriction.
5. A sibling native-contract document for the signed statement and an
   approved-scope-template digest.
6. Controller support for registered runners, signature verification and
   re-evaluation. Verification must enforce decision 2: only security-key types,
   no `no-touch-required` entries, and a signature without user presence fails.
7. An adapter-side attestor, end-to-end fixture tests and final documentation.

Consuming-project work (runner output in the native run format, approved scope
templates, key registration, policy approval, delivery of the attestation without
changing the head, and the hosted verdict job) is separate from these kernel
changes. Real acceptance also depends on the independent review policy, which is a
separate decision.

Fixture tests with temporary keys will not demonstrate real key custody, honest
execution on a real runner, or a live deployment, and the documentation must say
so.
