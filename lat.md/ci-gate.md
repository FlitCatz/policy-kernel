# Advisory PR gate

The gate reads policy at the tip of an open PR's base branch and evaluates scoped CI or signed local-runner evidence for the observed head. It writes an advisory verdict and detailed evidence.

The [CI gate guide](../docs/integrations/ci-gate.md) documents the command,
policy, upload contract, and exit codes.
[[crates/policy-cli/src/gate/mod.rs#run]] runs the advisory command.
[[crates/policy-core/src/scope.rs#resolve_required_scope]] applies the v5
rules from [[kernel#Declarative PR scope rules]] to changed files and labels.

## GitHub observations

The gate uses read-only GitHub REST requests. Its evidence binds a workflow path, PR head and run to the latest eligible attempt.

Run selection [[crates/policy-cli/src/gate/github.rs#GitHub]]
ignores skipped workflows and confirms PR association. A required CI workflow
without a PR association, as on fork PRs, yields unknown. A head branch with
multiple open PRs also yields unknown. Multiple PR links on any
candidate run, workflow or action changes, and closed PRs also yield unknown.
The final read must agree on PR state, head, reported base, independently
resolved base tip, labels, and file count. Review reads are advisory.

## Light and native checks

Light evidence records successful named jobs. Native checks additionally require raw verification and scope evaluation of the job's uploaded NativeRunV1 bundle.

[[crates/policy-cli/src/gate/native.rs#NativePolicy#evaluate]] binds the upload to
the run attempt and the job's time window. It reuses [[native-contracts#Adapter importer]] and
[[native-contracts#Local scope verifier]]. The gate keeps each job's sensor
identities under its distinct scoped check ID. A conflicting duplicate or
missing artifact yields unknown. Only successful jobs proceed to artifact
intake. Light and native checks share one conclusion mapping: failed or
timed-out jobs yield not ready, and other unsuccessful conclusions yield unknown.

## Fixture coverage

Recorded API response fixtures exercise the base policy read, latest attempt, head or base changes, reviews, artifact provenance, and all three verdicts.

[[crates/policy-cli/src/gate/tests.rs#base_policy_scopes_light_jobs_and_reviews_remain_advisory]]
checks additive label scope, a newer skipped run, and light evidence.
[[crates/policy-cli/src/gate/tests.rs#unsuccessful_light_job_is_unknown_not_failed]]
keeps cancelled, skipped, and neutral light jobs unknown.
[[crates/policy-cli/src/gate/tests.rs#a_move_during_reading_overrides_an_otherwise_ready_verdict]]
and [[crates/policy-cli/src/gate/tests.rs#a_base_tip_move_invalidates_an_unchanged_pr_snapshot]]
check obsolescence.
[[crates/policy-cli/src/gate/tests.rs#policy_uses_the_base_branch_tip_even_when_pr_base_sha_is_stale]]
checks policy selection when the PR reports a stale base SHA.
[[crates/policy-cli/src/gate/tests.rs#rejects_multiple_pr_links_even_on_a_skipped_candidate]]
and [[crates/policy-cli/src/gate/tests.rs#rejects_an_open_sibling_even_when_the_run_has_one_link]]
check ambiguous associations.
[[crates/policy-cli/src/gate/tests.rs#only_open_pull_requests_are_evaluated_and_closure_invalidates_evidence]]
checks PR state. [[crates/policy-cli/src/gate/tests.rs#reads_the_second_page_of_pr_files_before_accepting_evidence]]
and [[crates/policy-cli/src/gate/tests.rs#rejects_a_total_count_change_between_job_pages]]
check pagination and completeness.
[[crates/policy-cli/src/gate/tests.rs#merge_preserves_same_named_sensors_across_jobs_and_runs]]
and [[crates/policy-cli/src/gate/tests.rs#missing_expired_and_conflicting_artifacts_fail_closed]]
check per-job attribution and unavailable native evidence.
[[crates/policy-cli/src/gate/tests.rs#failed_native_job_without_artifact_is_not_ready]]
separates failed jobs from cancelled ones without an artifact.
[[crates/policy-cli/src/gate/tests.rs#unsuccessful_native_job_with_valid_artifact_is_unknown]]
keeps cancelled, skipped, and neutral jobs unknown even with valid uploads.
[[crates/policy-cli/src/gate/tests.rs#own_gate_policy_scopes_jobs_present_in_pull_request_workflows]]
keeps policy-kernel's own `.policy-kernel/gate.json` valid against its CI jobs.

## Signed local evidence

One attestation can satisfy the base policy's named native checks. The base must approve the runner and scope template and supply the exact-principal public key registry.

[[crates/policy-cli/src/gate/local.rs#LocalPolicy#evaluate]] verifies the
signature and matches the statement to the observed PR number, head, tree,
base and exact policy bytes. It checks the signed scope and run digests and
re-evaluates scope at the attested clock, subject to freshness measured by the
gate. [[crates/policy-core/src/scope.rs#requires_ci_for_local_evidence]] applies
owner-defined CI conditions conservatively in the absence of trusted risk.
CI-only labels, policy or registry edits force CI.
[[crates/policy-cli/src/gate/local.rs#uncovered]] keeps a listed check unknown
unless the signed expected scope contains its declared native requirements and
observation rules. The advisory verdict names
the classes used. The [gate guide](../docs/integrations/ci-gate.md#signed-local-runs)
describes the handoff from [[native-contracts#Local attester]].

### Local gate fixtures

Recorded signatures and GitHub responses exercise the same gate path as the CLI, including scope satisfaction, trust selection, revocation by registry removal and replay rejection.

[[crates/policy-cli/src/gate/tests.rs#accepted_hardware_signature_uses_base_signers_and_reports_local_evidence]]
asserts local readiness.
[[crates/policy-cli/src/gate/tests.rs#signed_run_leaves_a_required_local_check_it_did_not_cover_unknown]]
keeps a second required local check unknown when the signed run omits its
requirement. Missing base keys, invalid signatures
and absent presence are covered by
[[crates/policy-cli/src/gate/tests.rs#unavailable_or_unsigned_local_evidence_never_passes]].
[[crates/policy-cli/src/gate/tests.rs#expired_or_obsolete_attestations_never_pass]]
and [[crates/policy-cli/src/gate/tests.rs#missing_opt_in_and_policy_digest_changes_fail_closed]]
check freshness, changed head and policy.
[[crates/policy-cli/src/gate/tests.rs#check_system_paths_and_ci_only_labels_use_ci_instead]]
checks routing precedence.
