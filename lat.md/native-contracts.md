# Native contracts

The additive v1 contract describes already-executed native runs. It validates typed consistency only; no importer, execution, core evidence conversion or scope command exists here.

The [contract reference](../docs/integrations/native-contracts-v1.md) defines the
boundary and [Task 18](../docs/tasks/task-18/README.md) tracks later work.
[[status]] remains a summary of the current-status authority.

## Strict records

Expected scope, native run and local result have independent version-1 wire contracts. Required sets reject emptiness and duplicates; top-level decoding invokes structural validation.

Validation [[crates/policy-native-contracts/src/validation.rs#ExpectedScopeV1#validate]],
[[crates/policy-native-contracts/src/validation.rs#NativeRunV1#validate]], and
[[crates/policy-native-contracts/src/validation.rs#LocalScopeResultV1#validate]].
Subject comparison [[crates/policy-native-contracts/src/validation.rs#validate_subject_binding]]
checks consistency, not authenticity. Results remain local-untrusted with unmet review.

## Digest domains

Raw byte hashes cannot substitute for canonical typed hashes. Canonical preimages include the native schema discriminator and role, using existing core integer and sorted-key rules.

Encoding [[crates/policy-native-contracts/src/digest.rs#canonical_digest]] delegates
to [[crates/policy-core/src/canonical.rs#canonical_sha256]]. Existing contracts and
approved legacy digests remain unchanged. Required-set array order is preserved.

## Conformance

The corpus covers valid records, unknown versions and fields, empty or duplicate sets, ID collisions, subject mismatches and hash confusion. Schema-valid relational failures must still fail typed validation.

Corpus [[crates/policy-native-contracts/tests/conformance.rs#serialized_corpus_rejects_invalid_documents]];
schemas [[crates/policy-native-contracts/tests/conformance.rs#committed_schemas_and_corpus_conform]];
hash vectors [[crates/policy-native-contracts/tests/conformance.rs#canonical_vectors_pin_preimage_and_byte_digest_boundary]];
failure preservation [[crates/policy-native-contracts/tests/conformance.rs#failed_and_incomplete_records_preserve_diagnostics_without_success]].
The Rust development example regenerates only crate-owned assets; workspace tests
check their committed forms without changing the five root schemas.
