# Authenticated collection reference

`policy-controller` authenticates an assembled collection against owner-installed
trust anchors and produces the files consumed by `policy-ci pre-merge`. It is a
Rust reference implementation of the independent attestation boundary. Its local
tests generate fixture RSA keys and simulate GitHub. They demonstrate signature,
inventory and binding enforcement; they do not establish real provider identity,
live execution, deployment, publication, protected merge or promotion.

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

Every modification, execution and review needs its producer signature. The
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
rejected. `ControllerConfig.schema_version` is 1. Its fields are `expected`
(`RunBinding`), `observer_public_key` (absolute PEM path), `producer_keys`,
`openssl` (absolute executable path), and `minimum_reviewers` (at least 2).
Each producer registry entry has `key_id`, `public_key`, `provider` and `model`.
Observer and producer public keys must all be distinct, compared as DER public
keys rather than PEM formatting. A producer key may authenticate multiple
executions of its configured provider and model.

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

Inventory entries must match evidence exactly, with no duplicates. An empty
contributor ledger or missing required reviews/sensors is rejected. Context and
work item are signed together with evidence, and the sensor profile digest must
match the adopted policy. The policy's required-knowledge declarations remain
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
`trusted_runner_attested` and recomputed child/full digests. Version 2 identity
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
```

Cryptographic controller tests require OpenSSL (optionally set the absolute
`POLICY_TEST_OPENSSL`). They generate local test keys, exercise valid collection
through the real controller CLI and CI consumer with simulated GitHub, and reject
tampering, replay, policy/provider changes and incomplete inventories. Existing
live GitHub smoke remains ignored by default. Local signing does not prove who
owns a production key or that an attester honestly performed an execution.
Key registration/rotation, independently authenticated provider services,
complete ledger ingestion and isolated runner deployment must be established by
the consuming project. Post-merge authenticated storage and protected-merge
observations retain their existing separate integration contract. Live
deployment and promotion still require [the promotion evidence](../docs/promotion.md).
