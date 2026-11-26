# Advisory CI gate

The gate reads policy from the PR base and evaluates scoped GitHub Actions evidence for the observed head. It writes an advisory verdict and detailed evidence.

The [CI gate guide](../docs/integrations/ci-gate.md) documents the command,
policy, upload contract, and exit codes.
[[crates/policy-cli/src/gate/mod.rs#run]] runs the advisory command.
[[crates/policy-core/src/scope.rs#resolve_required_scope]] applies the v5
rules from [[kernel#Declarative PR scope rules]] to changed files and labels.

## GitHub observations

The gate uses read-only GitHub REST requests. Its evidence binds a workflow path, PR head and run to the latest eligible attempt.

Run selection [[crates/policy-cli/src/gate/github.rs#GitHub]]
ignores skipped workflows and confirms PR association. The same observation
is required after evaluation. A moved PR head, base, label set, or file count
or an unreadable observation yields unknown. Review reads are advisory.

## Light and native checks

Light evidence records successful named jobs. Native checks additionally require raw verification and scope evaluation of the job's uploaded NativeRunV1 bundle.

[[crates/policy-cli/src/gate/native.rs#NativePolicy#evaluate]] binds the upload to
the job attempt and reuses [[native-contracts#Adapter importer]] and
[[native-contracts#Local scope verifier]]. The gate keeps each job's sensor
identities under its distinct scoped check ID. A conflicting duplicate or
missing artifact yields unknown, except that a failed or timed-out job is not
ready without its artifact.

## Fixture coverage

Recorded API response fixtures exercise the base policy read, latest attempt, head or base changes, reviews, artifact provenance, and all three verdicts.

[[crates/policy-cli/src/gate/tests.rs#base_policy_scopes_light_jobs_and_reviews_remain_advisory]]
checks additive label scope, a newer skipped run, and light evidence.
[[crates/policy-cli/src/gate/tests.rs#a_move_during_reading_overrides_an_otherwise_ready_verdict]]
checks obsolescence.
[[crates/policy-cli/src/gate/tests.rs#merge_preserves_same_named_sensors_across_jobs_and_runs]]
and [[crates/policy-cli/src/gate/tests.rs#missing_expired_and_conflicting_artifacts_fail_closed]]
check merged and unavailable native evidence.
[[crates/policy-cli/src/gate/tests.rs#failed_native_job_without_artifact_is_not_ready]]
separates failed jobs from cancelled ones without an artifact.
[[crates/policy-cli/src/gate/tests.rs#own_gate_policy_scopes_jobs_present_in_pull_request_workflows]]
keeps policy-kernel's own `.policy-kernel/gate.json` valid against its CI jobs.
