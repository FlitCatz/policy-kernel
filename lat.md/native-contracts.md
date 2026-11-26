# Native contracts

The additive v1 contract describes already-executed native runs. The contract validates typed consistency, an adapter imports raw reports, and a kernel verifier assembles evidence for local advisory scope evaluation.

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
Suite rules [[crates/policy-native-contracts/src/types.rs#TestRuleV1]] optionally
name exact raw identities for skipped tests; any other skip fails. Omitted
lists keep their canonical digest and, like empty lists, permit no skips.
The sibling [[native-contracts#Local-runner attestations]] preserves these records
and supports issuance and opt-in verification through
[[trust-boundaries#Local-runner attestation]].

## Process-check requirements

An approved requirement can opt in to process evidence by naming one or more required process observations. Process-only scopes omit test adequacy. Mixed scopes require both process evidence and suite test coverage.

[[crates/policy-native-contracts/src/types.rs#ScopeRequirementV1]] defines the
two requirement shapes. A process requirement's `process_checks` references
bound observations without test rules or parser bindings. All required
observations must be present, complete and pass. Suite-backed requirements retain
their wire representation, and a scope with any suite still requires test
coverage. See the [contract](../docs/integrations/native-contracts-v1.md#process-check-requirements).
The [[native-contracts#Local attester]] does not support process-only scopes
yet and refuses them before starting the runner.

Contract tests
[[crates/policy-native-contracts/tests/process_requirements.rs#process_requirement_round_trips_without_a_suite]],
[[crates/policy-native-contracts/tests/process_requirements.rs#process_requirement_rejects_missing_empty_duplicate_and_suite_references]],
and [[crates/policy-native-contracts/tests/process_requirements.rs#mixed_scope_keeps_both_requirement_shapes_and_requires_unique_ids]]
cover the wire forms and fail-closed validation.

## Digest domains

Raw byte hashes cannot substitute for canonical typed hashes. Canonical preimages include the native schema discriminator and role, using existing core integer and sorted-key rules.

Encoding [[crates/policy-native-contracts/src/digest.rs#canonical_digest]] delegates
to [[crates/policy-core/src/canonical.rs#canonical_sha256]]. Existing contracts and
approved legacy digests remain unchanged. Required-set array order is preserved.
Ratchet input digests reuse the same envelope through
[[crates/policy-native-contracts/src/ratchet.rs#RatchetDigest]] with separate
ratchet roles, leaving scope `DigestRole` and schemas unchanged.

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
readers. [[crates/policy-native-importer/src/reports.rs#recompute]] adds
raw identity pairs and attempts to the parsed outcomes.
TRX identity and outcome lookups use unqualified attributes, as in the shared reader.
Empty and retried reports fail import verification. Explicit skips survive
recomputation and export so the verifier can apply the suite rule.

Descriptor access [[crates/policy-native-importer/src/fs.rs#directory]],
bounded inventory [[crates/policy-native-importer/src/fs.rs#inventory]],
raw reads [[crates/policy-native-importer/src/fs.rs#read]] and atomic marker
[[crates/policy-native-importer/src/fs.rs#complete]] prevent path following and
silent replacement. A completion marker denotes persistence even for rejected
imports. Originals remain untouched when unsafe or oversize artifacts cannot be copied.

Artifact copies are limited to 64 MiB per file and 256 MiB total, with 4096
entries and 32 nested directory levels. The shared parser caps test reports used
in claims at 16 MiB per report.

Conformance [[crates/policy-native-importer/tests/import.rs#shared_corpus_imports_all_formats_and_preserves_exact_bytes]],
replay [[crates/policy-native-importer/tests/import.rs#wrong_subject_identity_stale_future_and_replayed_raw_are_rejected]],
reports [[crates/policy-native-importer/tests/import.rs#missing_hash_malformed_mismatched_outcome_retry_duplicate_and_summary_fail_closed]],
skip export [[crates/policy-native-importer/tests/import.rs#native_import_exports_trx_skips_for_scope_evaluation]],
claims [[crates/policy-native-importer/tests/import.rs#forged_tests_identities_counts_attempts_and_parser_bindings_are_rejected]],
paths [[crates/policy-native-importer/tests/import.rs#path_attacks_and_extra_entries_preserve_safe_artifacts]],
output [[crates/policy-native-importer/tests/import.rs#output_is_fresh_disjoint_and_never_overwritten]],
and failure data [[crates/policy-native-importer/tests/import.rs#failed_native_tests_are_retained_without_a_scope_decision]].
Nested roots [[crates/policy-native-importer/tests/import.rs#nested_artifacts_and_symlinked_roots_have_no_implicit_path_exceptions]]
and missing-report process errors [[crates/policy-native-importer/tests/import.rs#failed_process_logs_survive_even_when_no_raw_report_was_produced]]
retain diagnostics without a weaker symlink path or empty success record.
[[crates/policy-native-importer/tests/import.rs#imports_large_raw_artifacts_without_changing_bytes_or_digest]]
checks preservation and scope evaluation with a generated 18,227,388-byte
artifact and a 64 MiB artifact.
[[crates/policy-native-importer/tests/import.rs#rejects_artifact_one_byte_over_64_mib]]
checks the diagnostic and source retention one byte over the limit.

## Native command distribution

The Unix importer command delegates raw verification to the adapter library. An owner-created version tag packages it with the advisory CLI, SHA-256 checksums and the source commit.

Command [[crates/policy-native-importer/src/main.rs#main]] calls
[[crates/policy-native-importer/src/importer.rs#import_run]] without a second
validator. It exits 0 for verified imports, 2 for persisted rejected imports and
64 for input/output errors; only verified imports export observations.
[[ci-gate]] uses the same importer internally for GitHub Actions artifacts.
Command conformance
[[crates/policy-native-importer/tests/import.rs#native_import_command_exports_only_verified_observations]]
and [[crates/policy-native-importer/tests/import.rs#native_import_command_requires_independent_identity_and_valid_documents]]
check export, evaluation, rejection, immutable output and independent expectations.

Packaging [[xtask/src/release.rs#assets]] requires both commands for macOS arm64
and Linux x64, hashes exact bytes and records the source commit. Its
[conformance test](../xtask/src/release.rs) rejects missing targets and existing
output and pins a known SHA-256 vector.
The [workflow](../.github/workflows/release-binaries.yml) uses Rust 1.95.0,
locked builds and SHA-pinned actions. The
[consumer contract](../docs/integrations/native-contracts-v1.md#pinned-binary-releases)
defines assets, install commands and checksum pins.
The [v0.1.5 release notes](../docs/integrations/native-contracts-v1.md#v015-release-notes)
record advisory evaluation of [[kernel#Declarative PR scope rules]] and the
Rust struct-literal changes for pre-merge context and policy snapshots. The
[v0.1.4 release notes](../docs/integrations/native-contracts-v1.md#v014-release-notes)
record process-check requirements. The
[v0.1.3 release notes](../docs/integrations/native-contracts-v1.md#v013-release-notes)
record support for xUnit skip-only TRX warnings. Windows x64 users can run the
Linux x64 binaries under WSL2. See [[status]] for publication status.

## Claim preparation

Explicit report selections populate empty producer test claims through the shared parser projection before import. Each TRX has its own observation and approved suite; explicit mappings claim coverage without inferring test adequacy.

Entry [[crates/policy-native-importer/src/claims.rs#populate_claims]] reuses bounded,
descriptor-relative artifact reads and [[crates/policy-native-importer/src/reports.rs#recompute]]
for counts, normalized/raw identities, outcomes and attempts. The CLI then invokes
[[crates/policy-native-importer/src/importer.rs#import_run]] to verify the run.
Preparation bounds selected test reports to 16 MiB each and 256 MiB total.
[[crates/policy-native-importer/src/claims.rs#populate_mappings]] maps all executed
tests, including failures, only for explicitly selected approved requirement/suite
pairs. No mappings are inferred and no v1 schema or canonical vector changes.

Conformance [[crates/policy-native-importer/tests/import.rs#claim_preparation_uses_shared_parsers_and_explicit_mappings]]
checks TRX, Vitest and Playwright against existing native fixtures;
[[crates/policy-native-importer/tests/import.rs#multiple_trx_reports_use_distinct_observations_and_approved_suites]]
checks per-project TRX namespaces and evaluation;
[[crates/policy-native-importer/tests/import.rs#claim_preparation_rejects_ambiguous_unsafe_or_unapproved_inputs_without_output]]
rejects ambiguity, unapproved selections, bad hashes and symlinks;
[[crates/policy-native-importer/tests/import.rs#prepared_claims_preserve_native_and_parser_failures_without_inventing_coverage]]
preserves failed tests, parser failures and missing mappings as non-pass scope.
The [consumer contract](../docs/integrations/native-contracts-v1.md#claim-preparation-from-raw-reports)
defines selections and the one-report-per-observation rule.

## Local scope verifier

The kernel assembles imported observations into core evidence and evaluates independently approved scope. It enforces required sets, mappings, exclusions and bindings; local trust and unmet review cannot authorize a merge.

Entry [[crates/policy-native-verifier/src/lib.rs#evaluate_scope]] evaluates bound
process observations and delegates sensor and adequacy evaluation to core when
scope includes suite-backed requirements. [[crates/policy-native-verifier/src/lib.rs#assemble]]
uses explicit local recorder identity and no reviews. The adapter exports
[[crates/policy-native-importer/src/importer.rs#ImportRecord#verified_observations]]
only after successful verification, retaining parser-level failure flags.
The verifier checks the skip count and any named allowances against the full
suite observation. Unexpected skips carry their raw identities in the result.
Core evidence counts exclude explicit skips, which cannot satisfy mappings.
[[crates/policy-cli/src/lib.rs#evaluate_scope_json]] implements the JSON-only
command with 0/2/64 exits. It cannot authenticate artifact bytes or grant trust.

Conformance [[crates/policy-native-verifier/tests/scope.rs#satisfied_scope_assembles_local_evidence_without_reviews]],
[[crates/policy-native-verifier/tests/scope.rs#missing_and_unexecuted_mappings_never_pass]],
[[crates/policy-native-verifier/tests/scope.rs#empty_policy_fake_trust_review_and_unknown_versions_are_rejected]],
[[crates/policy-native-verifier/tests/scope.rs#subject_binding_time_required_set_and_exclusions_fail_closed]],
[[crates/policy-native-verifier/tests/scope.rs#native_bounds_policy_digest_and_incomplete_completion_are_enforced]];
skip policy [[crates/policy-native-verifier/tests/scope.rs#skip_allowance_enforces_names_and_count_independently]];
xUnit skip warning import and scope evaluation [[crates/policy-native-importer/tests/import.rs#native_import_evaluates_named_trx_skips_with_xunit_warnings]];
exclusions [[crates/policy-native-verifier/tests/scope.rs#exclusions_never_waive_required_mappings_or_allow_unapproved_scope]];
mapping limits [[crates/policy-native-verifier/tests/scope.rs#mapping_minimum_and_allowed_suite_are_independent_of_passing_counts]];
command [[crates/policy-cli/tests/scope.rs#local_scope_command_exit_codes_and_input_boundaries]];
comparison [[crates/policy-native-importer/tests/import.rs#importer_verifier_command_and_project_normalization_agree_on_failed_fixture]];
parser failure [[crates/policy-native-importer/tests/import.rs#parser_level_failure_survives_even_when_all_tests_pass]];
failed imports [[crates/policy-native-importer/tests/import.rs#rejected_imports_never_export_verified_observations]].

Process checks
[[crates/policy-native-verifier/tests/scope.rs#process_only_scope_passes_without_test_adequacy]],
[[crates/policy-native-verifier/tests/scope.rs#process_only_scope_rejects_missing_or_interrupted_checks]],
[[crates/policy-native-verifier/tests/scope.rs#process_only_scope_rejects_failed_checks]],
and [[crates/policy-native-verifier/tests/scope.rs#mixed_scope_requires_both_process_and_suite_evidence]]
cover pass, missing, incomplete, failure, and mixed requirements.
[[crates/policy-native-importer/tests/import.rs#native_import_and_cli_evaluate_process_scope_without_reports]]
verifies evaluation of a process-only run with no reports or mappings.

## Debt ratchet

The standalone ratchet compares approved and proposed per-context allowances with observed head counts. Increases and context replacement fail; parsing languages and detecting suppressions stay with producers.

Contract [[crates/policy-native-contracts/src/ratchet.rs#RatchetRuleV1]] and
[[crates/policy-native-contracts/src/ratchet.rs#RatchetObservationV1]] keep head
allowances separate from actual counts to reject unused allowance growth.
[[crates/policy-native-verifier/src/ratchet.rs#evaluate_ratchet]] enforces the
minimum of base and head per complete key, collecting sorted violations.
[[crates/policy-native-verifier/src/ratchet.rs#evaluate_ratchet_json]] rejects
malformed wire inputs. The [contract reference](../docs/integrations/debt-ratchet-v1.md)
defines all reason codes and canonical domains. This comparison does not
establish trust, approval, review satisfaction or scope satisfaction; wiring
independent ratchet requirements into scope remains follow-up work.

`policy-cli evaluate-ratchet --rule FILE --observation FILE` delegates to that
JSON boundary and emits the unchanged result. Exits are 0 for pass, 2 for
comparison failures and 64 for invalid input; either file may use stdin, but
not both. Existing CLI commands retain their behavior.
CLI conformance covers deterministic pass output
[[crates/policy-cli/tests/ratchet.rs#ratchet_pass_prints_the_verifier_result_deterministically]],
every comparison reason
[[crates/policy-cli/tests/ratchet.rs#ratchet_comparison_failures_preserve_each_reason_and_digests]],
structural violations
[[crates/policy-cli/tests/ratchet.rs#ratchet_structural_failures_are_invalid_input_with_json_reasons]],
malformed inputs
[[crates/policy-cli/tests/ratchet.rs#ratchet_malformed_documents_preserve_stable_verifier_diagnostics]],
stdin boundaries
[[crates/policy-cli/tests/ratchet.rs#ratchet_accepts_either_input_from_stdin_but_rejects_two_stdin_inputs]],
file/argument errors
[[crates/policy-cli/tests/ratchet.rs#ratchet_missing_files_and_options_are_invalid_input_without_stdout]]
and help [[crates/policy-cli/tests/ratchet.rs#ratchet_is_discoverable_in_command_help]].

## Local-runner attestations

Sibling local-runner statements bind source/run identity, template and input/result digests, runner metadata and ordered times. Signature envelopes enforce structural consistency without cryptography or trust promotion.

[[crates/policy-native-contracts/src/attestation.rs#LocalAttestationV1#validate]]
checks source/digest roles and timestamp order;
[[crates/policy-native-contracts/src/attestation.rs#SignedLocalAttestationV1#validate]]
also requires matching runner IDs. Signatures cover canonical statement JSON.
[[crates/policy-native-contracts/src/attestation.rs#canonical_local_digest]]
reuses the native envelope encoder with separate local roles, preserving all
existing schemas and digests.
[[crates/policy-native-contracts/src/attestation.rs#ExpectedScopeV1#template_digest]]
covers requirements, observation bindings, maximum age and mode, excluding
subject and evaluation time. Standalone ratchet baselines remain outside scope;
future scope integration must bind them into the approval template.
The [contract reference](../docs/integrations/native-contracts-v1.md#local-runner-attestation-contracts)
defines the projection and authentication boundary.

## Attestation conformance

Synthetic vectors pin local and existing document digests, preserve original schemas, reject malformed statements/envelopes and check the template approval boundary without claiming authenticated execution.

Schemas/corpus [[crates/policy-native-contracts/tests/attestation.rs#attestation_schemas_and_corpus_enforce_wire_and_relational_contracts]];
template [[crates/policy-native-contracts/tests/attestation.rs#template_digest_excludes_subject_and_clock_but_binds_all_approval_fields]];
digests [[crates/policy-native-contracts/tests/attestation.rs#canonical_vectors_pin_new_and_existing_document_digests]];
direct callers/signing bytes [[crates/policy-native-contracts/tests/attestation.rs#direct_call_validation_and_canonical_signing_bytes_preserve_the_boundary]].

## Local attester

The Unix adapter observes a single explicit runner argv in a clean worktree, verifies run artifacts and issues a signed statement only after the owner approves the displayed result and statement hash.

[[crates/policy-local-attest/src/attest.rs#attest]] pairs Git snapshots and runner
timestamps before passing the manifest and artifacts to the importer. A snapshot
treats assume-unchanged and skip-worktree index entries as dirty because Git
status hides edits to them. Git runs without the owner's environment, so global
config and excludes cannot weaken the clean-tree check. It binds
the scope to the actual head and preserves the scope with the completion clock.
[[crates/policy-local-attest/src/attest.rs#sign_and_publish]] presents the hash of
the canonical statement bytes and publishes the envelope atomically after the
external signer succeeds. The [CLI reference](../action/controller.md#issuing-a-local-attestation)
documents arguments, outputs and exit codes.

### Attester rejection tests

Attester tests exercise Git and native artifact checks, owner rejection and signer failure, including preservation of existing output.

See the [attester tests](../crates/policy-local-attest/src/tests.rs) and
[[kernel#Controller authentication#Local-runner controller tests]].
[[crates/policy-local-attest/src/tests.rs#process_only_scope_is_refused_before_the_runner]]
checks that a valid process-only scope is refused before the runner starts.

## Debt ratchet conformance

Ratchet tests cover reductions, equality, increases, new and removed entries, replacement contexts, duplicate keys, malformed inputs, unused allowance growth, ordering and canonical digests.

Behavior [[crates/policy-native-verifier/tests/ratchet.rs#ratchet_decrease_equal_increase_new_and_removed_entries]];
allowances [[crates/policy-native-verifier/tests/ratchet.rs#head_allowance_cannot_grow_even_when_unused_and_lower_allowance_is_enforced]];
contexts [[crates/policy-native-verifier/tests/ratchet.rs#context_replacement_cannot_spend_another_contexts_allowance]];
duplicates [[crates/policy-native-verifier/tests/ratchet.rs#duplicate_keys_in_every_input_fail_without_picking_an_allowance]];
wire errors [[crates/policy-native-verifier/tests/ratchet.rs#malformed_json_negative_counts_unknown_fields_and_versions_fail_closed]];
paths [[crates/policy-native-verifier/tests/ratchet.rs#invalid_entries_are_all_reported_for_direct_rust_callers]];
identity [[crates/policy-native-verifier/tests/ratchet.rs#tuple_identity_prevents_cross_tool_rule_path_symbol_and_context_allowances]];
maximum counts [[crates/policy-native-verifier/tests/ratchet.rs#maximum_counts_do_not_overflow_and_removed_entries_cannot_offset_growth]];
ordering [[crates/policy-native-verifier/tests/ratchet.rs#ordering_determinism_and_domain_separated_digests_preserve_wire_array_order]];
schemas [[crates/policy-native-contracts/tests/ratchet.rs#ratchet_schemas_match_committed_assets_and_reject_invalid_wire_shapes]];
digest vector [[crates/policy-native-contracts/tests/ratchet.rs#ratchet_digest_domains_pin_canonical_preimages_without_changing_native_vectors]].
