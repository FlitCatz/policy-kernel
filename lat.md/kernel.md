# Evaluation kernel

Maps each authorization rule to the Rust symbol that enforces it. Rules are specified in the [trust model](../docs/trust-model.md); this page only anchors them to code.

`lat check` fails if a referenced symbol is renamed or removed, so update the matching section in the same change.

## Pre-merge gates

Pre-merge evaluation combines independent gates; a blocked or inconclusive gate always wins over readiness, and a reasonless non-ready gate records `PK-PRE-GATE-UNEXPLAINED`.

Entry point [[crates/policy-core/src/pre_merge.rs#evaluate_pre_merge]]; gate composition [[crates/policy-core/src/pre_merge.rs#Gates#extend]]; freshness [[crates/policy-core/src/pre_merge.rs#check_freshness]].

## Sensor evaluation

Required sensors come from any-path `required_when` matches, while a not-applicable exemption must cover every changed path. Globs keep `*` within one segment.

Resolution [[crates/policy-core/src/sensors.rs#resolve_required_sensors]] and [[crates/policy-core/src/sensors.rs#condition_matches]]; exemptions [[crates/policy-core/src/sensors.rs#condition_covers]]; glob semantics [[crates/policy-core/src/sensors.rs#path_glob]]; evidence rules [[crates/policy-core/src/sensors.rs#evaluate_sensor_evidence_for_change]].

## Test adequacy

Empty policies cannot pass. Negative cases need required changed-behavior IDs and mappings to executed tests; duplicate or blank IDs cannot establish coverage.

Core [[crates/policy-core/src/sensors.rs#evaluate_adequacy]] and [[crates/policy-core/src/sensors.rs#validate_mappings]]; consumer validation [[crates/project-verification/src/profile.rs#ProjectProfile#validate]].
The [adequacy contract](../docs/integrations/adequacy.md) records preserved legacy parsing and changed readiness. Direct-call regressions are in [[crates/policy-core/tests/sensors.rs#empty_policy_is_explained_non_pass_even_with_positive_counts]] and [[crates/policy-core/tests/sensors.rs#missing_or_malformed_adequacy_ids_never_pass_direct_core_calls]].

## Review consensus and waivers

Reviews need independent providers; unwaived High or Critical findings block, and a blocked report whose actionable findings are all validly waived does not conflict with an approval.

Consensus [[crates/policy-core/src/reviews.rs#evaluate_reviews]]; waiver validity [[crates/policy-core/src/reviews.rs#waiver_state]]; waived blocked reports [[crates/policy-core/src/reviews.rs#blocked_report_is_waived]].

## Provenance and head binding

Evidence children must match their manifest digests and head; authoritative evaluation also requires trusted evidence and head-bound execution and adequacy records.

Bundle integrity [[crates/policy-core/src/provenance.rs#validate_bundle]]; trust, freshness and head presence [[crates/policy-core/src/provenance.rs#validate_provenance]]; required knowledge [[crates/policy-core/src/provenance.rs#validate_required_knowledge]].

## Canonical digests

All policy, evidence and decision digests hash canonical JSON with sorted keys and integral numbers; signed controller bytes use the same encoding.

Encoding [[crates/policy-core/src/canonical.rs#canonical_json]] and [[crates/policy-core/src/canonical.rs#canonical_sha256]]; evidence identity [[crates/policy-core/src/pre_merge_digest.rs#pre_merge_evidence_digest]]; policy identity [[crates/policy-core/src/pre_merge_digest.rs#pre_merge_policy_digest]].

## Post-merge shipping

Shipping needs an authoritative ready decision, its published check, and a protected merge of the same head; the receipt's protection claim is cross-checked with GitHub.

Evaluation [[crates/policy-core/src/post_merge.rs#evaluate_post_merge]]; receipt parsing [[crates/github-adapter/src/parse.rs#parse_merge]]; protection cross-check [[crates/github-adapter/src/parse.rs#validate_merge_protection]].

## Controller authentication

The controller verifies observer and producer signatures with owner-installed keys before promoting local evidence to trusted-runner provenance.

Authentication [[crates/policy-ci/src/controller.rs#produce]]; producer payload [[crates/policy-ci/src/controller.rs#producer_payload]]; signed bytes [[crates/policy-ci/src/controller.rs#canonical]]; input bounds [[crates/policy-ci/src/controller.rs#read_json]].

## Project verification binding

Project verification binds each sensor's configuration without the absolute checkout location, so a prepared profile binds identically on every runner.

Configuration identity [[crates/project-verification/src/run.rs#configuration_digest]]; preflight [[crates/project-verification/src/run.rs#prepare]]; normalization [[crates/project-verification/src/run.rs#normalize_collection]].

## Bootstrap installer

Bootstrap publishes managed files transactionally with a private journal; new files follow the umask and replacements keep the replaced file's permissions.

Planning [[bootstrap/src/plan.rs#plan]]; application [[bootstrap/src/transaction.rs#apply]] and [[bootstrap/src/transaction.rs#recover]]; publication [[bootstrap/src/fs.rs#Root#publish]].

## Npm workspace graph

P1 binds explicit npm members to one root lock and audit. Manifest inventory, lock links and dependency presence must agree; local native verification does not establish adoption or trust.

Contract [[crates/project-verification/src/npm_workspace.rs#validate_contract]];
lock declarations [[crates/project-verification/src/npm_workspace.rs#validate_files]];
filesystem inventory [[crates/project-verification/src/npm_workspace.rs#validate_layout]].
Dependency inventory [[crates/project-verification/src/npm_workspace.rs#validate_dependency_inventory]].
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

## TJERP local composition

The P7 harness preserves LocalUntrusted, checks exact catalog lists and raw executed selectors, and requires both profiles to bind one Git archive. It delegates decisions to existing core APIs and cannot authorize promotion.

[[acceptance/src/tjerp/catalog.rs#check_catalog]] enforces pilot completeness;
[[acceptance/src/tjerp/source.rs#git_inventory]] checks local source consistency;
[[acceptance/src/tjerp/mod.rs#evaluate_pair]] binds both results and invokes the
sensor, bundle and advisory provenance checks. See [[tjerp]] for the synthetic
matrix and P8/P9 boundary.
