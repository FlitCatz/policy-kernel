# Native contracts

The additive v1 contract describes already-executed native runs. The contract validates typed consistency; a separate adapter imports raw reports. Execution, core evidence conversion and a scope command are outside this boundary.

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

## Adapter importer

The Unix adapter verifies raw artifacts and subject/time consistency, recomputes reports and retains failures. Its audit record is local-untrusted and never a scope decision or core evidence conversion.

Entry [[crates/policy-native-importer/src/importer.rs#import_run]] requires
independent run/scope IDs alongside ExpectedScopeV1. Verification
[[crates/policy-native-importer/src/importer.rs#verify]] compares subject, age,
raw report timing and claimed observations. Parser outcomes come from shared
readers; [[crates/policy-native-importer/src/reports.rs#recompute]] supplements
raw identity pairs and attempts without changing those readers.

Descriptor access [[crates/policy-native-importer/src/fs.rs#directory]],
bounded inventory [[crates/policy-native-importer/src/fs.rs#inventory]],
raw reads [[crates/policy-native-importer/src/fs.rs#read]] and atomic marker
[[crates/policy-native-importer/src/fs.rs#complete]] prevent path following and
silent replacement. A completion marker denotes persistence even for rejected
imports. Originals remain untouched when unsafe or oversize artifacts cannot be copied.

Conformance [[crates/policy-native-importer/tests/import.rs#shared_corpus_imports_all_formats_and_preserves_exact_bytes]],
replay [[crates/policy-native-importer/tests/import.rs#wrong_subject_identity_stale_future_and_replayed_raw_are_rejected]],
reports [[crates/policy-native-importer/tests/import.rs#missing_hash_malformed_skip_retry_duplicate_and_summary_fail_closed]],
claims [[crates/policy-native-importer/tests/import.rs#forged_tests_identities_counts_attempts_and_parser_bindings_are_rejected]],
paths [[crates/policy-native-importer/tests/import.rs#path_attacks_extra_entries_and_size_limits_preserve_safe_artifacts]],
output [[crates/policy-native-importer/tests/import.rs#output_is_fresh_disjoint_and_never_overwritten]],
and failure data [[crates/policy-native-importer/tests/import.rs#failed_native_tests_are_retained_without_a_scope_decision]].
Nested roots [[crates/policy-native-importer/tests/import.rs#nested_artifacts_and_symlinked_roots_have_no_implicit_path_exceptions]]
and missing-report process errors [[crates/policy-native-importer/tests/import.rs#failed_process_logs_survive_even_when_no_raw_report_was_produced]]
retain diagnostics without a weaker symlink path or empty success record.
