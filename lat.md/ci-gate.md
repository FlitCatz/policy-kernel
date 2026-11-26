# Advisory CI gate

The gate reads policy at the tip of an open PR's base branch and evaluates scoped GitHub Actions evidence for the observed head. It writes an advisory verdict and detailed evidence.

The [CI gate guide](../docs/integrations/ci-gate.md) documents the command,
policy, upload contract, and exit codes.
[[crates/policy-cli/src/gate/mod.rs#run]] runs the advisory command.
[[crates/policy-core/src/scope.rs#resolve_required_scope]] applies the v5
rules from [[kernel#Declarative PR scope rules]] to changed files and labels.

## GitHub observations

The gate uses read-only GitHub REST requests. Its evidence binds a workflow path, PR head and run to the latest eligible attempt.

Run selection [[crates/policy-cli/src/gate/github.rs#GitHub]]
ignores skipped workflows and confirms PR association. A fork PR or a head
branch with multiple open PRs yields unknown. Multiple PR links on any
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
