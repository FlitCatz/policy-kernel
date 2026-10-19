# Native contracts v1

**v1 types only; no importer or scope command yet.**
`policy-native-contracts` is the additive Rust contract crate for already-executed
native runs. It neither executes commands nor reads files, Git, reports or the
network. It does not construct core evidence or evaluate policy. Existing core
contracts, approved digests, five root schemas and TJERP execution are unchanged.

## Documents and validation

Each document has its own `schema_version: 1`, independent of core policy v3,
ProjectProfile, selection and parser versions. Unknown versions and fields are
rejected, including nested fields. Rust callers can construct records and call
`validate()`; top-level Serde deserialization calls the same validation.

| Type | Contents |
| --- | --- |
| `ExpectedScopeV1` | Independently approved repository/head/source subject; raw requirements-file digest; required observation IDs and exit/count rules; AC, changed-behavior and negative-case requirements with suite mapping rules; exemptions; command/configuration/parser bindings; evaluation time, positive maximum age and local-advisory mode |
| `NativeRunV1` | Subject plus run/scope IDs; observations with execution ID, argv, relative cwd, times, completion and configuration digest; parser bindings, raw report references, counts and test identity/outcomes/attempts; artifacts, mappings and exclusions |
| `LocalScopeResultV1` | Satisfied/non-pass/error status, subject, per-requirement results, stable reason codes and suite/requirement/artifact locations, input/policy/optional evidence digests, evaluation time, verifier/adapter versions, local-untrusted trust and unmet review gate |

Required observations and requirements are nonempty. Requirements carry an
explicit kind; a category may be absent when outside the approved consumer scope,
but the overall approved requirement set cannot be empty. Required suites,
accepted exit codes and mapping test references are also nonempty and unique.
Requirement, observation, execution, artifact and exclusion IDs must be unique
within their respective namespaces. Repeating a record ID with different contents
is rejected. Normalized test IDs are unique within a suite; references include
both suite and test ID. Requirement kinds share one ID namespace.

Bindings must cover exactly the required observations. Only suite rules have
parser bindings. Requirement suite references must resolve to declared suites.
Test counts must match their records: passed/failed tests have positive attempts;
skipped/not-executed tests have zero attempts; retries sum attempts minus one for
executed tests. Mappings must reference executed tests, which may have failed.
A failed test remains data for a later policy evaluator, never an implicit pass.

Observation subjects must equal the run subject. `validate_subject_binding`
also compares the run with the independent expected subject. This checks typed
consistency only; matching strings do not authenticate a source checkout.
Observation timestamps must be ordered. Relative artifact/cwd paths reject
absolute paths, parent/dot components, empty components, backslashes, colons and
NUL; cwd alone may be `.`. This is lexical validation, not filesystem containment
or symlink checking. Raw test identity remains opaque and may retain absolute
producer paths. Repeated argv tokens and empty non-program arguments are valid.

Incomplete runs can preserve artifacts without test observations or mappings.
An error result may omit evidence and requirement results, but must retain a
reason. Satisfied results require evidence, nonempty satisfied requirements and
no failure reasons. Non-pass results require evidence and reasons. Only
`local_untrusted` and review `unmet` deserialize; no authorization state exists.
Reason codes describe failures and locations; their presence does not authenticate
a supplied result or recompute its decision.

Cross-document completeness, freshness, approved mapping/exclusion enforcement,
command/configuration/parser comparison, raw-byte verification, report reparsing
and scope decisions remain later importer/verifier work. JSON Schema covers wire
shape; Rust validation additionally covers ID collisions, references, count/time
consistency and digest roles. Consumers must use both layers as appropriate, not
substitute schema validation for these relational checks.

## Digest boundary

`RawByteDigest` is `{kind: "raw_byte_sha256", sha256}` over exact external file
bytes. This crate records it; it does not read or hash files. `CanonicalDigest`
is `{kind: "canonical_sha256", schema: "policy_native_v1", role, sha256}`.
The canonical preimage is exactly `{schema, role, value}`, using the existing
`policy-core` canonical JSON helper: sorted object keys and integers only.
The schema discriminator and role are inside the hashed envelope, not just labels.
Raw and canonical objects cannot deserialize into each other's fields. Source-set,
scope-definition, command, configuration and result digest roles are checked.

`canonical_digest(role, value)` and each document's `digest()` use that envelope.
Array order is preserved, including required-set order; sets are never silently
sorted or deduplicated. Hash a validated typed record, not an arbitrary JSON
object, to obtain its document digest. Canonical serialization normalizes existing
UTC timestamp types. Absolute cwd paths are forbidden; original report identities
may retain them in run data. Callers must keep checkout location out of approved
configuration values; this crate cannot inspect opaque configuration digests. Legacy approved
hashes are never silently replaced by these new digests.

## Schemas and corpus

The crate owns [three schemas](../../crates/policy-native-contracts/schemas/),
[document vectors](../../crates/policy-native-contracts/tests/corpus/v1.json) and
[canonical vectors](../../crates/policy-native-contracts/tests/corpus/canonical-v1.json).
Document vectors contain valid records and invalid pointer replacements with
expected JSON Schema outcomes; relational failures may be schema-valid. For the
`pair` case, replace the run pointer and copy its subject to all observations,
then check it against the unchanged expected scope. Example artifact/policy
hashes are placeholders, not real execution or authenticated evidence.

```console
cargo test --locked -p policy-native-contracts
cargo run --locked -p policy-native-contracts --example generate_assets
```

The Rust development example regenerates only this crate's schemas and corpus.
Tests compare committed schemas byte-for-byte, validate corpus schema outcomes,
reject typed-invalid vectors and pin canonical hash values. The workspace CI
runs these tests, including on Rust 1.85.0 and 1.95.0. It does not run a native
consumer or establish promotion. See [Task 18](../tasks/task-18/README.md) and
[Current status](../status.md).
