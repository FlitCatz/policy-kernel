# Authenticated collection reference

`policy-controller` authenticates an assembled collection against owner-installed
trust anchors and produces the files consumed by `policy-ci pre-merge`. It is a
Rust reference implementation of the independent attestation boundary. Its local
tests generate fixture RSA keys and simulate GitHub. They demonstrate signature,
inventory and binding enforcement; they do not establish real provider identity,
live execution, deployment, publication, protected merge or promotion.

The
[local-runner attestation decision](../docs/decisions/2026-11-12-local-runner-attestation-adr.md)
adds a second statement type to this boundary. Configuration schema 2 verifies
owner-registered local runners, binds their statements to signed GitHub
observations, recomputes scope and issues only the covered sensors and executions
as `local_runner_attested`. Core separately requires policy v4 or v5 opt-in, freshness
and a change exempt from CI, and prevents such readiness from shipping. The
[local attester](#issuing-a-local-attestation) prepares the three local inputs
and requests owner approval before invoking a signer.

## Ownership and isolation

The repository owner operates the controller on a separate service or isolated
runner identity. Contributor code runs in a disposable worker without the
controller's filesystem, signing credentials, publishing token or permissions.
The controller runs no PR commands. An owner-installed binary, Action revision,
OpenSSL executable, public key registry and adopted policy are immutable to
contributors. Merely using a directory outside the checkout is insufficient:
another process with the controller's OS identity can generally replace it.

Separate attesting services retain private keys; the controller receives only
public keys. The observer key belongs to a service which independently reads the
complete modification/execution ledger, changed-path inventory, adopted policy,
profile, work authorization, label authorization, sensor reports and required
knowledge measurements. It must account for every code modifier and cannot infer
completeness from the absence of extra submitted files. Each producer key belongs
to an execution or review service whose authenticated provider/model identity is
configured by the owner. A producer signs only evidence from its own observed
execution, after verifying its record and the complete assembled bundle. It must
not sign arbitrary caller JSON. A signing service that blindly signs uploads
would defeat this design.

Every modification and review, and each execution outside a verified local scope,
needs its producer signature. Covered local executions instead require the
registered local runner's signature. The
observer additionally authenticates the whole collection and its explicit
complete inventory. The observer attests sensor/knowledge measurements and any
waiver authorization; it must independently authenticate those sources. Review
findings and waivers are included in the signed bundle digest. This reference
requires at least two review providers independent of all modifiers. The core
continues to enforce required sensors, required knowledge, freshness, findings,
authorization and result adequacy; authenticated failures do not become passes.

The owner-controlled scheduler supplies the current request configuration from
authenticated run/PR metadata and adopted settings. It pins repository, PR, head,
run, attempt, trigger SHA, workflow API head, workflow ID/path, exact workflow ref,
workflow definition SHA, publishing App ID/slug and policy digest. Never derive
these expectations or trusted public keys from the submitted signed envelope.
`policy-ci` independently checks Actions environment, actual checkout, current
GitHub workflow and PR, and the publishing response before reporting readiness.

## Wire and signing contract

The strict Rust wire types are in
[`controller.rs`](../crates/policy-ci/src/controller.rs). Unknown fields are
rejected. `ControllerConfig.schema_version` accepts 1 and 2. The common fields are `expected`
(`RunBinding`), `observer_public_key` (absolute PEM path), `producer_keys`,
`openssl` (absolute executable path), and `minimum_reviewers`, which must equal
`max(2, policy.required_reviews.minimum_reviewers)`. The v3 policy is the source
of the quorum; this configuration field asserts agreement while retaining the
reference controller's floor of two approving providers. A larger or smaller
independent configuration value is rejected. Only `approved` reviews count;
waived blocked reports do not count as approvals.

Policy v3 requires `required_reviews` with a positive `minimum_reviewers`,
`require_provider_independence: true`, and `allowed_providers` (empty means any
provider). Core evaluates that digest-bound quorum and provider list, while the
controller authenticates identities and promotes provenance. V1/v2 policy
documents and digests remain valid but cannot issue a controller receipt or
authorize readiness/shipping: `PK-POLICY-V3-MIGRATION-REQUIRED` requests explicit
owner reapproval as v3. Recompute the policy digest and bind new evidence and
current-run observations after migration; do not reuse approval of an old digest.
Each producer registry entry has `key_id`, `public_key`, `provider` and `model`.
Observer and producer public keys must all be distinct, compared as DER public
keys rather than PEM formatting. A producer key may authenticate multiple
executions of its configured provider and model.

Schema 2 additionally requires `ssh_keygen`, an owner-installed absolute path,
and accepts `local_runners`: entries with `runner_id` and an absolute `public_key`
file path. Runner IDs use ASCII letters, digits, `-`, `_`, `.` and `@`. IDs and
decoded keys must be unique. Each file contains one OpenSSH public key or one
`allowed_signers` entry with that exact principal; comments and blank lines are
allowed. Wildcard/multiple principals and certificate-authority registrations are
rejected. Allowed key types are exactly `sk-ssh-ed25519@openssh.com` and
`sk-ecdsa-sha2-nistp256@openssh.com`. A `no-touch-required` option is rejected
with a distinct error from an unsupported key type. OpenSSH enforces other
allowed-signers options, including `namespaces`, `valid-after` and `valid-before`.
Schema 1 accepts no local runner configuration or local observations and retains
its existing bytes and behavior.

`SignedObservation` contains `observation`, `observer_signature`, and `producers`.
The observation fields are `domain` (exactly
`policy-kernel/controller-observation/v1`), `binding`, `policy`, `context`,
`work_item`, `evidence` and `inventory`. Evidence and agent-ready trust fields
must be `local_untrusted`. Existing child digests must validate before
authentication can produce new evidence. The required inventory fields are:

- `complete`: true only after the observer independently completed its ledger.
- `contributors`, `executions`, `reviews`: complete execution-ID arrays.
- `sensors`: complete sensor-ID array.
- `knowledge`: complete strings `LatClaim:<source>` or `CodeGraphIndex:<source>`.

The optional `local_attestations` array is omitted when empty. Each entry has
`attestation` (`SignedLocalAttestationV1`), `expected_scope` (`ExpectedScopeV1`)
and `observations` (`ImportedObservationsV1`, including its native run). No
submitted verdict or evidence projection is accepted in place of recomputation.
When local entries are present, `github_head_tree` must name the head commit's
tree independently read by the observer from GitHub. It is omitted when absent.
Both fields are covered by the observer signature. Existing observations without
them retain exactly the same signing bytes and producer payloads.

Inventory entries must match evidence exactly, with no duplicates. An empty
contributor ledger or missing required reviews/sensors is rejected. Context and
work item are signed together with evidence, and the sensor profile digest must
match the adopted policy. For policy v5, the caller supplies `changed_paths`
and `labels`. The signature binds the supplied lists, whose accuracy and
completeness depend on that caller. Execution and test adequacy records must carry the
evaluated `head_sha`; the collector fills it, and authoritative evaluation
rejects records without it. The policy's required-knowledge declarations remain
bound to its versioned digest and are checked by the evaluation core.

The observer signs `policy_ci::controller::canonical(observation)`: typed JSON
with recursively sorted object keys, compact encoding and no trailing newline.
These bytes come from `policy_core::canonical_json`, which sorts keys explicitly
instead of relying on serde_json feature selection and rejects floating-point
numbers; `canonical_sha256` hashes the same bytes.
Each producer signs the bytes returned by `producer_payload(observation,
execution_id)`, a canonical object with `domain` equal to
`policy-kernel/controller-producer/v1`, full `binding`, `evidence_digest` equal
to `canonical_sha256(full_original_typed_bundle)`, and `execution_id`.
Its envelope entry contains `key_id`, `execution_id` and `signature`.
Signatures are hexadecimal detached RSA PKCS#1 v1.5 / SHA-256 signatures.
Use owner-generated RSA keys of at least 2048 bits. RSA authentication uses an
explicit OpenSSL executable, fixed argv, cleared environment, bounded files,
private scratch storage and a ten-second per-operation timeout. It does not
invoke a shell or search PATH. The controller contains no signing CLI.

## Local-runner verification

For each local input the controller performs these checks before creating output:

1. Validate the statement/envelope and verify its SSHSIG against the registered
   key, fixed `policy-kernel/local-runner-attestation/v1` namespace and key validity
   at the controller's clock. The signature's embedded key must equal the
   registered key. Parse the signed security-key flags and require user presence
   (`0x01`): `ssh-keygen -Y verify` alone does not enforce this bit.
2. Require policy v4 or v5 opt-in and membership in `allowed_runner_ids`.
3. Match statement, scope and run subjects, repository/head to the GitHub binding,
   tree to `github_head_tree`, base to the observer's PR manifest, and source-set,
   run and scope identities to the signed inputs.
4. Recompute the scope-template digest and require it in
   `approved_scope_template_digests`.
5. Recompute expected-scope, native-run and imported-observation digests, rerun
   `evaluate_scope`, require `Satisfied` and compare the recomputed result digest.
   The verifier checks command/config/parser bindings, required sets, mappings,
   counts, skip/retry limits and exclusions. A signed claim of success is insufficient.
6. Check start, completion, attestation and scope evaluation times against the
   controller's clock, using the smaller of the approved scope and local-policy
   windows with inclusive, subsecond precision. All observations must fall inside
   the attested run interval; scope evaluation follows completion and precedes
   attestation. Local freshness does not extend the global CI evidence window.
7. Require complete, disjoint coverage of execution/sensor IDs in the observer's
   inventory. Reject duplicate runs, overlapping records, uncovered records and
   attempts to cover modifier/reviewer IDs. Replace those sensor/execution slots
   with the verifier's reconstructed records and set only them to
   `local_runner_attested`. Remaining records retain the existing authenticated
   producer/observer path and become `trusted_runner_attested`.

With local inputs the single aggregate adequacy record is rebuilt solely from
their verified scope results; submitted adequacy is discarded. Multiple disjoint
scopes combine counts and mappings, rejecting overlapping requirement IDs or
count overflow. CI sensors/executions may coexist, but their submitted adequacy
mappings do not augment this local aggregate. Approve scope templates covering
all adequacy obligations needed by the PR's adopted profile.

SSHSIG verification uses fixed argv, a cleared environment, bounded input,
private scratch storage and the existing ten-second crypto timeout. Public keys
and the SSH verifier receive the same checkout-isolation checks as RSA anchors.
The controller does not parse raw test reports, execute contributor commands,
sign statements, or change review evaluation.

## Issuing a local attestation

Build the owner-operated Unix adapter from an approved revision. Pass the clean
worktree root, the intended base ref, an independently prepared expected scope,
and the expected run and scope IDs. The runner writes a fresh native manifest
outside its raw-artifact directory. Keep runner and attester outputs outside the
checkout. The output directory must be new, its parent must exist, and its path
and the raw-artifact path must be symlink-free. Attester option paths resolve
from the invocation directory. The runner starts in the checkout.

```console
cargo run --locked -p policy-local-attest -- \
  --repo /work/checkout --base origin/main --expected /work/input/expected.json \
  --run /work/run/run.json --artifacts /work/run/reports --output /work/attested \
  --run-id run-42 --scope-id full-check --runner-id owner-runner \
  --signer /usr/bin/ssh-keygen --signer-arg=-Y --signer-arg=sign \
  --signer-arg=-f --signer-arg=/owner/keys/runner \
  --signer-arg=-n --signer-arg=policy-kernel/local-runner-attestation/v1 \
  -- /owner/bin/local-check /work/run/run.json /work/run/reports
```

The attester starts one argv in the checkout with stdin closed. It checks that the
worktree is clean before and after the command, that head and tree are unchanged,
and that all observation times fall inside the measured run. Index entries marked
assume-unchanged or skip-worktree count as dirty, so sparse checkouts are rejected.
Git runs with an empty environment: global and user Git config, including global
excludes, is ignored, and `git` is found on the default exec path. Ignore
generated files such as `.DS_Store` in the repository `.gitignore`, or attest
from a fresh worktree. Process-only scopes, which declare no test suite, are
not supported yet and are refused before the runner starts. It imports the raw
artifacts and sets the scope evaluation clock to runner completion. When the
result is `Satisfied`, the attester shows the result summary and SHA-256 of the
canonical statement signing bytes on the owner's terminal. The statement and its
timestamp are fixed before confirmation. Keep stdin and stderr
attached to the terminal and type `yes` at the prompt to authorize signing.
The signer reads the exact statement from stdin and emits an
armored SSHSIG to stdout. Use an owner-controlled signer and the registered
hardware security key. Key material is handled by the signer.

Exit 0 means `attestation.json` was published. Exit 3 means the confirmation was
declined or the terminal was unavailable. Exit 2 covers invalid input, runner or
scope failure, signer failure, and output errors. Import audit files may remain
after a failure. An existing output directory or envelope is preserved.

For a successful run, the observer uses `attestation.json`, `expected.json` and
`observations.json` from the output directory as the local input tuple. The
observer independently supplies GitHub's head tree, the current PR binding, and
its complete inventory. Transport and deployment are owned by the consuming
project.

## Invocation and output

Build and install both Rust binaries from the approved revision:

```console
cargo build --locked -p policy-ci --bins
/opt/policy-kernel/bin/policy-controller --config /controller/current-request.json --checkout /work/pr --signed-observation /incoming/signed-observation.json --output /controller/run-9001-attempt-2
```

`--config`, its public keys and the OpenSSL executable must be owner controlled.
The CLI rejects configuration/keys/executables and output inside `--checkout`.
The output directory must not already exist. The controller authenticates all
statements before creating files and writes `identity.json` last. Output also
contains `policy.json`, `evidence.json`, `context.json`, `work-item.json` and the
original `authentication.json` envelope for auditing. It leaves incoming
collector evidence unchanged. Authenticated reconstructed evidence uses
`trusted_runner_attested`, plus `local_runner_attested` for verified local
sensors/executions, with recomputed child/full digests. Version 2 identity
receipts additionally bind exact workflow ref and workflow definition SHA.
Exit 0 means receipt production succeeded; invalid configuration, malformed or
unauthenticated input, incomplete inventories and output errors return 2.
Receipt production itself is not an authorization decision.

Use the [workflow example](controller-workflow.example.yml) only after replacing
the placeholders and deploying the isolated services described above. It is
documentation, not an installed workflow. The owner must provide current request
and signed collection files through authenticated storage before invocation.
The final publication runner requires authenticated `gh`, `git`, and the
permissions in the [Action README](README.md). Its PATH must also be owner
controlled because GitHub/git transports are resolved there. No contributor
steps may run in the publication process's security context.

## Local verification and remaining operations

```console
cargo test --locked --offline -p policy-ci
cargo test --locked --offline -p policy-local-attest
```

Cryptographic controller tests require OpenSSL (optionally set the absolute
`POLICY_TEST_OPENSSL`) and `/usr/bin/ssh-keygen`. They generate local test keys,
including software-only synthetic FIDO-format signatures with and without user
presence. The attester fixture uses a pseudo-terminal from `script`, verifies
the displayed hash against the actual signer input, and passes the resulting
envelope through both CLI boundaries with simulated GitHub. The fixtures cover
ordinary readiness, CI-required changed paths, tampering, replay, policy/provider
changes and incomplete inventories. Existing live GitHub smoke remains ignored
by default. Fixture evidence cannot establish production key custody, honest
execution on an owner's machine or a live GitHub decision.
Key registration/rotation, independently authenticated provider services,
complete ledger ingestion and isolated runner deployment must be established by
the consuming project. Post-merge authenticated storage and protected-merge
observations retain their existing separate integration contract. Live
deployment and promotion still require [the promotion evidence](../docs/promotion.md).
