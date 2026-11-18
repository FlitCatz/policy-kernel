# Evaluation kernel

Maps each authorization rule to the Rust symbol that enforces it. Rules are specified in the [trust model](../docs/trust-model.md); this page only anchors them to code.

`lat check` fails if a referenced symbol is renamed or removed, so update the matching section in the same change.

## Scope boundary

The kernel owns deterministic evidence, scope and release decisions. Native types, importer and local scope verifier exist. TJERP owns its harness; adapter separation remains planned. Compatibility and required environment checks remain.

[Task 18](../docs/tasks/task-18/README.md) links the accepted ADR and stage-1
sequence. New exact native combinations and bootstrap presets stop. Typed
verification stays in the kernel; raw parsing/execution belongs to the planned
adapter product and TJERP composition to its consumer. [[native-contracts]] records
the contracts, adapter importer and kernel verifier; [[status]] summarizes the current-status authority.

Local scope [[crates/policy-native-verifier/src/lib.rs#evaluate_scope]] enforces
approved bindings and delegates sensor/adequacy decisions to core. Assembly
[[crates/policy-native-verifier/src/lib.rs#assemble]] uses local recorder identity,
local-untrusted executions/sensors and no reviews. It never calls pre-merge evaluation.
The local-runner attestation wire class does not change this
assembly. See [[trust-boundaries#Local-runner attestation]].

## Pre-merge gates

Pre-merge evaluation combines independent gates; a blocked or inconclusive gate always wins over readiness, and a reasonless non-ready gate records `PK-PRE-GATE-UNEXPLAINED`.

Entry point [[crates/policy-core/src/pre_merge.rs#evaluate_pre_merge]]; gate composition [[crates/policy-core/src/pre_merge.rs#Gates#extend]]; freshness [[crates/policy-core/src/pre_merge.rs#check_freshness]].

## Sensor evaluation

Required sensors use any-path matches; exemptions must cover every changed path. Sensor and knowledge globs accept ASCII literals, segment-local `*`, and whole-segment `**`; unsupported syntax fails closed.

Resolution [[crates/policy-core/src/sensors.rs#resolve_required_sensors]] and [[crates/policy-core/src/sensors.rs#condition_matches]]; exemptions [[crates/policy-core/src/sensors.rs#condition_covers]]; glob semantics [[crates/policy-core/src/sensors.rs#path_glob]]; evidence rules [[crates/policy-core/src/sensors.rs#evaluate_sensor_evidence_for_change]].

The [trust model](../docs/trust-model.md) specifies the exact GitHub-compatible subset
and migration from `src/*.jsx?` to explicit alternatives. Conformance:
[[crates/policy-core/tests/sensor_adversarial.rs#github_filter_subset_conformance]].
Rejection and exemptions:
[[crates/policy-core/tests/sensor_adversarial.rs#unsupported_github_filters_fail_closed_in_both_sensor_conditions]]
and [[crates/policy-core/tests/sensor_adversarial.rs#explicit_js_and_jsx_patterns_require_evidence_and_cover_only_matching_changes]].
False-ready regression:
[[crates/policy-core/tests/pre_merge.rs#optional_frontend_sensor_cannot_be_silently_skipped_by_github_question_mark]].
Required knowledge:
[[crates/policy-core/tests/knowledge.rs#github_optional_extension_is_invalid_and_explicit_alternatives_are_enforced]].

## Test adequacy

Empty policies cannot pass. Negative cases need required changed-behavior IDs and mappings to executed tests; duplicate or blank IDs cannot establish coverage.

Core [[crates/policy-core/src/sensors.rs#evaluate_adequacy]] and [[crates/policy-core/src/sensors.rs#validate_mappings]]; consumer validation [[crates/project-verification/src/profile.rs#ProjectProfile#validate]].
The [adequacy contract](../docs/integrations/adequacy.md) records preserved legacy parsing and changed readiness. Direct-call regressions are in [[crates/policy-core/tests/sensors.rs#empty_policy_is_explained_non_pass_even_with_positive_counts]] and [[crates/policy-core/tests/sensors.rs#missing_or_malformed_adequacy_ids_never_pass_direct_core_calls]].

## Report adapters

The shared report-adapter crate owns strict TRX, Vitest, compatibility-only Jest and Playwright parsing. The compatibility runner re-exports the same types; parsing alone supplies no authentication or release authority.

Entry [[crates/policy-report-adapters/src/reports.rs#parse_report]] rejects skips;
bounds and the native skip-preserving mode [[crates/policy-report-adapters/src/reports.rs#parse_report_with_skips]];
versions [[crates/policy-report-adapters/src/reports.rs#ReportFormat#parser_version]];
identities [[crates/policy-report-adapters/src/reports.rs#id]];
duplicate and outcome checks [[crates/policy-report-adapters/src/reports.rs#finish]].
The only workspace dependency is `policy-contracts`, for the existing timestamp type.
TRX attribute lookups require unqualified names, so namespaced extensions cannot
substitute for required fields or shadow their values. See
[[crates/policy-report-adapters/tests/corpus.rs#trx_attributes_must_be_unqualified]].
Fixed pre-extraction bytes are checked by
[[crates/policy-report-adapters/tests/corpus.rs#corpus_serialization_matches_pre_extraction_bytes]];
legacy API compatibility by
[[crates/project-verification/tests/report_compatibility.rs#legacy_module_reexports_identical_types_and_corpus_outputs]].
See [report formats](../docs/integrations/report-formats.md) for the unchanged contract.

## Review consensus and waivers

V3 binds an approving-review quorum and provider allowlist. Independent providers remain mandatory; waivers never supply approvals. Legacy policies require explicit v3 reapproval before readiness or shipping.

Consensus [[crates/policy-core/src/reviews.rs#evaluate_reviews]]; waiver validity [[crates/policy-core/src/reviews.rs#waiver_state]]; waived blocked reports [[crates/policy-core/src/reviews.rs#blocked_report_is_waived]].

Version boundary [[crates/policy-contracts/src/policy.rs#PolicySnapshot#validate_version]];
rules [[crates/policy-contracts/src/policy.rs#RequiredReviewPolicy]].
V1/v2 serialization and digests stay unchanged; `PK-POLICY-V3-MIGRATION-REQUIRED`
blocks legacy authorization. V3 requires positive quorum and independence enabled;
an empty allowlist permits any provider. Unwaived High/Critical findings still
block, and a fully waived blocked report no longer conflicts with an approval.
Regression [[crates/policy-core/tests/pre_merge.rs#one_review_cannot_reach_ready_after_transport_hashes_are_recomputed]];
boundaries and providers [[crates/policy-core/tests/reviews.rs#approving_quorum_boundaries_and_provider_constraints]];
legacy digests [[crates/policy-core/tests/pre_merge.rs#legacy_policy_digest_preimage_is_unchanged]];
legacy receipts [[crates/policy-core/tests/post_merge.rs#legacy_policy_cannot_ship_through_a_preexisting_receipt]].

## Provenance and head binding

Evidence children must match their manifest digests and head; authoritative evaluation also requires trusted evidence and head-bound execution and adequacy records.

Bundle integrity [[crates/policy-core/src/provenance.rs#validate_bundle]]; trust, freshness and head presence [[crates/policy-core/src/provenance.rs#validate_provenance]]; required knowledge [[crates/policy-core/src/provenance.rs#validate_required_knowledge]].

[[crates/policy-core/src/provenance.rs#is_ci_grade]] explicitly allows
CI observations and trusted-runner attestations. Its exhaustive match requires a
decision for every future trust class. Provenance, required knowledge, reviews,
modifications and agent-ready observations retain this CI-grade requirement.
Pre-merge sensor and execution acceptance additionally uses
[[crates/policy-core/src/provenance.rs#validate_pre_merge_provenance]] and
[[crates/policy-core/src/provenance.rs#validate_local_runner]].

Required knowledge delegates glob validation to
[[crates/policy-core/src/sensors.rs#path_glob]], which rejects blank and
NUL-containing patterns as well as unsupported glob syntax.

## Single-owner review policy

V4 can opt into fresh owner authorization plus an independent automated approval of the same head. Explicit actor identities and CI-grade trust are mandatory; absent opt-in preserves the default quorum.

Declaration [[crates/policy-contracts/src/policy.rs#OwnerWithIndependentAgentReviewPolicy]];
identity [[crates/policy-contracts/src/evidence.rs#ReviewIdentity]];
authorization [[crates/policy-contracts/src/evidence.rs#OwnerApprovalEvidence]].
Rule [[crates/policy-core/src/reviews.rs#owner_review_quorum]] preserves provider,
execution, finding and conflict gates. [[crates/policy-core/src/reviews.rs#validate_agent_review]]
checks explicit actor independence; [[crates/policy-core/src/reviews.rs#validate_owner_approval]]
checks owner identity, repository/head binding and CI-grade provenance.
[[crates/policy-core/src/reviews.rs#owner_review_is_fresh]] uses the stricter age limit.

Tests [[crates/policy-core/tests/owner_reviews.rs#owner_rule_fails_closed_for_each_evidence_boundary]],
[[crates/policy-core/tests/owner_reviews.rs#freshness_uses_the_stricter_window_with_exact_fractional_boundaries]],
[[crates/policy-core/tests/owner_reviews.rs#full_pre_merge_uses_the_owner_rule_and_binds_all_new_evidence]]
and [[crates/policy-contracts/tests/owner_reviews.rs#owner_review_option_is_v4_only_nonnull_and_omitted_when_absent]]
cover failures, exact time boundaries, digest binding and legacy compatibility.
The [wire contract](../docs/integrations/owner-review-policy-v4.md) records reason
codes and the unchanged controller floor; collection authentication is separate.

## Local-runner policy contract

Policy v4 optionally enables fresh local-class sensors and executions for changes exempt from CI. Other evidence stays CI-grade; local readiness carries a digest-bound marker that prevents shipping.

Version validation [[crates/policy-contracts/src/policy.rs#PolicySnapshot#validate_version]]
keeps each optional v4 extension independent; legacy serialization and digests
stay unchanged. Structural validation
[[crates/policy-contracts/src/policy.rs#LocalRunnerEvidencePolicy#validate]] requires
nonempty unique nonblank runner IDs without NUL, nonempty scope-template digests
and a positive age. The required conditions list reuses
[[crates/policy-contracts/src/policy.rs#SensorCondition]] and may be empty.
[[crates/policy-contracts/src/evidence.rs#TrustClass]] adds the wire value without
changing evidence version 1; the existing CI-grade predicate explicitly rejects it.

Schema and Rust parity tests
[[xtask/tests/local_runner_contract.rs#v4_local_runner_block_is_optional_and_legacy_versions_reject_its_presence]],
[[xtask/tests/local_runner_contract.rs#local_runner_policy_shapes_match_rust_and_json_schema]],
[[xtask/tests/local_runner_contract.rs#direct_construction_obeys_local_runner_and_review_boundaries]]
and [[xtask/tests/local_runner_contract.rs#local_runner_trust_round_trips_in_v1_evidence]]
cover version, shape and wire boundaries. Context-free CI-grade rejection is pinned by
[[crates/policy-core/tests/provenance.rs#local_runner_attestation_is_not_ci_grade]].

Core shares [[crates/policy-core/src/sensors.rs#validate_conditions]] and
[[crates/policy-core/src/sensors.rs#condition_matches]] with sensor profiles:
invalid or matching CI conditions reject local evidence. Local execution starts,
sensor observations and command-bound execution starts use the inclusive local age
window on the evaluator clock; local completions cannot be in the future. Other
evidence retains global freshness, and head/digest binding still applies.
Both freshness call sites share
[[crates/policy-core/src/provenance.rs#uses_local_freshness]] for mode and class selection.
[[crates/policy-core/tests/local_runner.rs#exhaustive_trust_kind_mode_and_policy_matrix]]
exercises all 192 trust/kind/mode/block/CI combinations.
[[crates/policy-core/tests/local_runner.rs#local_execution_age_uses_start_and_exact_subsecond_boundaries]],
[[crates/policy-core/tests/local_runner.rs#local_sensor_window_is_measured_from_execution_start_even_with_ci_execution]]
and [[crates/policy-core/tests/local_runner.rs#invalid_ci_conditions_are_rejected_even_when_other_conditions_do_not_match]]
pin freshness and fail-closed conditions.
[[crates/policy-core/tests/local_runner.rs#local_freshness_can_outlive_global_freshness_without_relaxing_ci_or_advisory]]
keeps local and global age limits independent without relaxing CI or advisory checks;
[[crates/policy-core/tests/local_runner.rs#local_execution_completion_cannot_be_in_the_future]]
rejects future completions under the local window.
[[crates/policy-core/tests/local_runner.rs#mixed_ci_and_local_evidence_preserves_the_local_marker_and_digest_binding]]
checks informational reason hashing without lowering readiness;
[[crates/policy-core/tests/local_runner.rs#local_ready_decision_can_never_ship_even_with_a_successful_bound_check]]
pins the shipping guard. Authentication remains outside core.

## Canonical digests

All policy, evidence and decision digests hash canonical JSON with sorted keys and integral numbers; signed controller bytes use the same encoding.

Encoding [[crates/policy-core/src/canonical.rs#canonical_json]] and [[crates/policy-core/src/canonical.rs#canonical_sha256]]; evidence identity [[crates/policy-core/src/pre_merge_digest.rs#pre_merge_evidence_digest]]; policy identity [[crates/policy-core/src/pre_merge_digest.rs#pre_merge_policy_digest]].

## UTC timestamps

Every wire timestamp is a `UtcTimestamp`: RFC 3339 in, UTC-normalized canonical bytes out. Jiff does the calendar arithmetic privately; no other crate uses a time library.

Grammar [[crates/policy-contracts/src/identity.rs#parse_rfc3339]]; the canonical form ends in `Z` with 0, 3, 6 or 9 fractional digits.
Elapsed time [[crates/policy-contracts/src/identity.rs#UtcTimestamp#duration_since]] is absent for a negative age, so freshness checks fail closed.
Whole-second form for published checks [[crates/policy-contracts/src/identity.rs#UtcTimestamp#truncated_to_seconds]].
Leap seconds and UTC years outside 0000-9999 are rejected because they have no canonical form.
[[crates/policy-contracts/tests/utc_timestamp.rs#accepted_inputs_serialize_to_the_pinned_canonical_bytes]]
pins the bytes the previous chrono implementation produced, and
[[crates/policy-contracts/tests/utc_timestamp.rs#malformed_and_unrepresentable_inputs_are_rejected]]
the rejections. Native start age shares
[[crates/policy-native-verifier/src/lib.rs#started_within_age]] between the verifier and importer.
See the [timestamp library note](../docs/provenance.md#timestamp-library).

## Post-merge shipping

Shipping needs an authoritative ready decision, its published check, and a protected merge of the same head; the receipt's protection claim is cross-checked with GitHub.

Evaluation [[crates/policy-core/src/post_merge.rs#evaluate_post_merge]]; receipt parsing [[crates/github-adapter/src/parse.rs#parse_merge]]; protection cross-check [[crates/github-adapter/src/parse.rs#validate_merge_protection]].

## Published pre-merge conclusions

Ready publishes success; blocked and inconclusive publish failure with distinct titles and outputs. Legacy neutral checks require republication; shipping still requires actual success.

Publication [[crates/github-adapter/src/parse.rs#publish_pre_merge_check]];
receipt authentication [[crates/github-adapter/src/parse.rs#parse_trusted_check]];
legacy rejection [[crates/github-adapter/src/parse.rs#parse_conclusion]].
Recorded transport verifies titles, outputs and exit codes in
`ready_blocked_and_inconclusive_publish_bound_checks_before_exit` in
[the CI integration tests](../crates/policy-ci/tests/ci.rs).
Parser coverage [[crates/github-adapter/tests/github.rs#trusted_check_accepts_failure_but_rejects_legacy_neutral]];
shipping coverage [[crates/policy-core/tests/post_merge.rs#unsuccessful_or_invalid_check_observations_are_inconclusive]].
The schema is unchanged; upgrade the installed binary and republish outstanding
neutral checks as described in the [Action contract](../action/README.md#results).

## Controller authentication

The controller authenticates observer/producer records and approved local scopes. Only verified local sensors/executions receive the weaker local-runner class; other records retain trusted-runner provenance.

Authentication [[crates/policy-ci/src/controller.rs#produce]]; producer payload [[crates/policy-ci/src/controller.rs#producer_payload]]; signed bytes [[crates/policy-ci/src/controller.rs#canonical]]; input bounds [[crates/policy-ci/src/controller.rs#read_json]].

The v3 policy governs quorum. Controller configuration must mirror the policy
minimum with a floor of two approving providers; mismatches fail before issuance.

Schema-2 [[crates/policy-ci/src/controller.rs#local_registry]] restricts runner keys
and rejects no-touch options. [[crates/policy-ci/src/controller.rs#local_signature_presence]]
requires the signed FIDO presence bit; [[crates/policy-ci/src/controller.rs#verify_local]]
uses bounded OpenSSH verification. [[crates/policy-ci/src/controller.rs#local_evidence]]
checks GitHub bindings, approval digests, scope recomputation and controller-time
freshness. [[crates/policy-ci/src/controller.rs#install_local]] replaces covered
records and rebuilds adequacy; no review semantics change.

### Local-runner controller tests

Synthetic software-generated FIDO signatures exercise the cryptographic path without a device. Rejections cover runner/key policy, presence, replay, template/digest changes, unsatisfied scope, freshness and coverage; legacy bytes remain stable.

The [local controller suite](../crates/policy-ci/tests/support/local_controller.rs)
covers reconstruction, signatures/presence, runner policy, key options, replay,
scope/digests, freshness and disjoint coverage. Independent cases pin each
versioned configuration field, nonzero run intervals and local placeholders
outside the CI freshness window. Parser tests pin the exact SSHSIG size limit
and registry comment/blank-line handling. The
[existing controller suite](../crates/policy-ci/tests/support/controller.rs)
also pins v1 signing bytes and byte-identical output without local evidence.
The [[crates/policy-ci/tests/support/attester_e2e.rs#issued_attestations_reach_ci_and_respect_required_paths]]
fixture uses real commands for [[native-contracts#Local attester]], controller
and CI, with mocked GitHub. It checks ordinary readiness plus policy-defined
CI-required paths using the issued attestation.

## Project verification binding

Project verification binds each sensor's configuration without the absolute checkout location, so a prepared profile binds identically on every runner.

Configuration identity [[crates/project-verification/src/run.rs#configuration_digest]]; preflight [[crates/project-verification/src/run.rs#prepare]]; normalization [[crates/project-verification/src/run.rs#normalize_collection]].

## Bootstrap installer

Bootstrap publishes managed files transactionally with a private journal; new files follow the umask and replacements keep the replaced file's permissions.

Planning [[bootstrap/src/plan.rs#plan]]; application [[bootstrap/src/transaction.rs#apply]] and [[bootstrap/src/transaction.rs#recover]]; publication [[bootstrap/src/fs.rs#Root#publish]].

Preset 1.1.1 patches Vite/Vitest while retaining the exact earlier release assets.
Receipt schema 2 and inventory reconstruction [[bootstrap/src/plan.rs#owned]]
remain release-specific. Native staging [[acceptance/src/staging.rs#stage]]
and runtime selection [[acceptance/src/staging.rs#builtin_profile]] use 1.1.1.
Installer coverage [[bootstrap/tests/installer.rs#patched_release_installs_repeats_and_uninstalls_schema_two_presets]],
[[bootstrap/tests/installer.rs#selected_release_upgrades_and_downgrades_all_frozen_releases]]
and [[bootstrap/tests/installer.rs#patched_release_rejects_old_installed_pins_and_preserves_edited_managed_files]]
checks pins, repeat installation, exact old receipt restoration and edit preservation.
Custom install coverage [[bootstrap/tests/installer.rs#user_profile_is_bound_but_never_owned_and_removal_survives_source_edits]]
also checks that uninstall instructions name the installed release.

## Npm workspace graph

P1 binds explicit npm members to one root lock and audit. Manifest inventory, lock links and dependency presence must agree; local native verification does not establish adoption or trust.

Contract [[crates/project-verification/src/npm_workspace.rs#validate_contract]];
lock declarations [[crates/project-verification/src/npm_workspace.rs#validate_files]];
filesystem inventory [[crates/project-verification/src/npm_workspace.rs#validate_layout]].
Dependency inventory [[crates/project-verification/src/npm_workspace.rs#validate_dependency_inventory]].
Bundle exclusions [[crates/project-verification/src/npm_workspace.rs#bundled_inventory]]
require registry tarball/sha512 parents and consistent explicit child entries.
[[crates/project-verification/src/npm_workspace.rs#BundledDependency]] records
`bundled, not individually audited` in the bound audit configuration, retaining
optional platform parents absent on the host. Root/member bundles stay forbidden.
Regressions [[crates/project-verification/tests/npm_workspace.rs#bundles_reject_unpinned_parents_conflicts_and_undeclared_names]];
native lock shape [[acceptance/tests/native_npm_workspace.rs#native_npm_bundled_platform_inventory]].
Reviewed root overrides [[crates/project-verification/src/npm_workspace.rs#override_rules]]
bind exact stable versions into the profile and reject member/contract targets.
Logical lock ancestry [[crates/project-verification/src/npm_workspace.rs#validate_overrides]]
checks hoisted/scoped targets and rejects dead rules and drift.
Shared target declarations reachable outside the reviewed scope fail closed.
Ancestor lookup [[crates/project-verification/src/npm_workspace.rs#resolve]] skips
node_modules itself and resolves workspace links.
The local regressions are in
[[crates/project-verification/tests/npm_workspace.rs#root_workspace_binds_members_and_uses_one_audit]].
Override regressions [[crates/project-verification/tests/npm_workspace.rs#overrides_reject_contract_targets_dead_rules_and_lock_drift]];
native audit proof [[acceptance/tests/native_npm_workspace.rs#native_npm_overrides_audit_resolved_versions]].
See [integration limits](../docs/integrations/npm-workspaces.md) and [[status]].

## Compiler-generated output

P2 binds the exact compiler-output property pair to reviewed NuGet projects and rejects inheritance, command overrides and output symlinks. Native success requires readable Mapperly source, not an empty directory.

Opt-in and argv [[crates/project-verification/src/generated_files.rs#validate_contract]];
exact XML [[crates/project-verification/src/generated_files.rs#validate_xml]];
reference overrides [[crates/project-verification/src/generated_files.rs#validate_reference_overrides]];
filesystem [[crates/project-verification/src/generated_files.rs#validate_layout]].
Negative fixtures [[crates/project-verification/tests/generated_files.rs#rejects_paths_expansions_duplicates_conditions_and_inheritance]];
native proof [[acceptance/tests/native_generated_files.rs#mapperly_files_are_generated_and_read_by_architecture_test]].
See [contract and limits](../docs/integrations/consumer-schema3.md#compiler-generated-files-p2) and [[status]].

## Mutation gate

Authorization modules, including sensor evaluation, must have every selected mutant built and caught; typed replacements stand in for uncompilable generator forms.

Scope and result validation [[xtask/src/verification.rs#mutation_scope]]; the contract is [verification/mutation-contract.json](../verification/mutation-contract.json).
The trust iterator's generated `Default` forms cannot construct `TrustClass`;
explicit CI-grade replacements test bypassing supplied trust assertions instead.
Ordinary viable iterator mutations stay in the generated selection.
Local-key, registry and reconstructed-evidence return types also have no `Default`;
their exact replacements reject valid local input. Controller source hashes and
shifted repair spans bind these checks to the current implementation.

## Split-consumer conformance

The synthetic split-consumer fixture preserves report, audit and tool-version regressions. Product catalogs, parent requirements and split-scope assertions now belong to the consumer-owned harness described in [[tjerp]].

Staging [[acceptance/tests/native_split_consumer.rs#schema_three_fixture_stages_without_preset_layout]]
keeps the explicit layout contract. Opt-in native regression
[[acceptance/tests/native_split_consumer.rs#native_split_consumer_reports_audits_and_rejection_paths]]
checks real report failures, audits and stale bindings. See the
[fixture guide](../examples/split-consumer/README.md) for execution limits.

The frontend profile and manifest select Vite 6.4.5 / Vitest 4.1.12, with
source-map-js 1.2.2 locked. Node 26.10.0/npm 10.9.3 selections remain unchanged.
The [audit record](../docs/integrations/native-fixture-audit.md#split-consumer-follow-up)
describes generation and verification separately.

## Alternate frontend conformance

Alternate native coverage uses Vitest for Node 22.19.0/npm 10.9.4, `web/` SSR, producer skip and nested dev-only audit rejection. Managed Jest execution and CI guarantees have ended; its reader remains compatible.

The replacement [[acceptance/tests/native_vitest.rs#native_vitest_selected_paths_failures_corrections_and_prior_head_rejection]]
asserts both exact runtime tokens before staging and reuses
[[acceptance/tests/common/alternate.rs#nested_graph_audit]] and report/head checks.
General profile tests use the Vitest fixture; compatibility decoding and migration
remain in [[crates/project-verification/tests/report_compatibility.rs#legacy_jest_profile_keeps_wire_contracts_and_schema_four_migration]].
The SSR producer tests use TypeScript. Compatibility checks isolate the schema-one
Jest guard and the legacy capability ceiling from profile version requirements.
Vitest JSON has no retry history: retry zero configuration is not success-after-retry rejection parity.
See [native procedure](../docs/integrations/acceptance.md#alternate-vitest-transition).

Managed Jest removal preserves [[acceptance/tests/native_project_versions.rs#native_project_versions]]
with its npm10/npm12 Vitest/Playwright, audit and trust checks. Its final native
phase executes [[acceptance/tests/native_project_versions.rs#migrated_vitest_profile]]:
schema two upgraded to four, record acceptance, no provisioning and every sensor passing.
Frozen bootstrap
bytes and receipt recovery/uninstall are unchanged. The
[support boundary](../docs/integrations/jest-transition.md) preserves generic
profile execution and old wire contracts without a schema change.
