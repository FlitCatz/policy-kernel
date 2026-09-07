use crate::{
    canonical_sha256, evaluate_reviews, evaluate_sensor_evidence_for_change,
    pre_merge_digest::{decision_digest, pre_merge_evidence_digest, pre_merge_policy_digest},
    validate_provenance, ReviewEvaluationInput, SensorEvaluationStatus,
};
use policy_contracts::*;
use std::collections::BTreeSet;

/// Evaluate complete, explicitly supplied observations without reading ambient state.
pub fn evaluate_pre_merge(input: &PreMergeEvaluationInput) -> PreMergeDecision {
    let mut gates = Gates::default();
    validate_policy_and_bindings(input, &mut gates);
    validate_observations(input, &mut gates);

    let provenance = validate_provenance(
        input.mode,
        &input.evidence,
        &input.evaluation_time,
        input.policy.max_evidence_age_seconds,
    );
    gates.extend(PreMergeStatus::Inconclusive, provenance.reasons().to_vec());
    validate_execution(input, &mut gates);
    validate_sensors(input, &mut gates);

    // All fields in these typed contracts serialize to integral JSON data.
    let evidence_digest = pre_merge_evidence_digest(&input.evidence)
        .expect("typed evidence has a canonical JSON representation");
    let reviews = evaluate_reviews(ReviewEvaluationInput {
        mode: input.mode,
        policy: input.policy.clone(),
        repository: input.pull_request.repository.clone(),
        head_sha: input.pull_request.head_sha.clone(),
        evidence_digest: evidence_digest.clone(),
        evaluation_time: input.evaluation_time.clone(),
        modifications: input.evidence.modifications.clone(),
        reviews: input.evidence.reviews.clone(),
        findings: input.evidence.findings.clone(),
        waivers: input.evidence.waivers.clone(),
    });
    gates.extend(
        match reviews.status {
            ReviewStatus::Approved => PreMergeStatus::ReadyToMerge,
            ReviewStatus::Blocked => PreMergeStatus::Blocked,
            ReviewStatus::Inconclusive => PreMergeStatus::Inconclusive,
        },
        reviews.reasons,
    );

    gates
        .reasons
        .sort_by(|left, right| (&left.code, &left.message).cmp(&(&right.code, &right.message)));
    gates.reasons.dedup();
    let mut decision = PreMergeDecision {
        schema_version: SCHEMA_VERSION,
        mode: input.mode,
        advisory: input.mode == EvaluationMode::Advisory,
        status: gates.status,
        binding: DecisionBinding {
            repository: input.pull_request.repository.clone(),
            pull_request_number: input.pull_request.pull_request_number,
            head_sha: input.pull_request.head_sha.clone(),
            policy_digest: input.policy.policy_digest.clone(),
            evidence_digest: evidence_digest.clone(),
            evaluated_at: input.evaluation_time.clone(),
        },
        reasons: gates.reasons,
        // Excluded from the hash preimage; initialized with a valid typed digest.
        decision_digest: evidence_digest,
    };
    decision.decision_digest = decision_digest(input, &decision)
        .expect("typed pre-merge input has a canonical JSON representation");
    decision
}

fn validate_policy_and_bindings(input: &PreMergeEvaluationInput, gates: &mut Gates) {
    let policy = &input.policy;
    let manifest = &input.evidence.manifest;
    let pr = &input.pull_request;
    let mut versions = vec![
        input.schema_version,
        policy.schema_version,
        input.work_item.schema_version,
        pr.schema_version,
        input.evidence.schema_version,
        manifest.schema_version,
    ];
    if let Some(context) = &input.context {
        versions.extend([
            context.schema_version,
            context.sensor_profile.schema_version,
        ]);
        if let Some(ready) = &context.agent_ready {
            versions.push(ready.schema_version);
        }
    } else {
        gates.unknown(
            "PK-PRE-CONTEXT-MISSING",
            "pre-merge evaluation context is required",
        );
    }
    if versions.iter().any(|version| *version != SCHEMA_VERSION) {
        gates.unknown(
            "PK-PRE-SCHEMA-UNSUPPORTED",
            "all runtime schema versions must be supported",
        );
    }
    if pre_merge_policy_digest(policy).ok().as_ref() != Some(&policy.policy_digest) {
        gates.unknown(
            "PK-PRE-POLICY-DIGEST",
            "policy content does not match its canonical digest",
        );
    }
    if policy.required_check_name.trim().is_empty()
        || policy.max_evidence_age_seconds == 0
        || policy
            .authorized_actors
            .iter()
            .chain(&policy.authorized_waiver_actors)
            .any(|actor| actor.trim().is_empty())
        || policy
            .required_sensor_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != policy.required_sensor_ids.len()
    {
        gates.unknown(
            "PK-PRE-POLICY-INVALID",
            "policy requires a check, freshness window and unambiguous identities",
        );
    }
    if manifest.repository != pr.repository
        || input.work_item.repository != pr.repository
        || manifest.work_item_id != input.work_item.work_item_id
        || manifest.base_sha != pr.base_sha
        || manifest.head_sha != pr.head_sha
        || manifest.policy_digest != policy.policy_digest
    {
        gates.unknown(
            "PK-PRE-BINDING-MISMATCH",
            "manifest must match the current repository, work item, base, head and policy",
        );
    }
    if let Some(context) = &input.context {
        if manifest.workflow_run_id != context.workflow_run_id {
            gates.unknown(
                "PK-PRE-WORKFLOW-MISMATCH",
                "manifest must belong to the current workflow run",
            );
        }
        if canonical_sha256(&context.sensor_profile).ok().as_ref()
            != policy.sensor_profile_digest.as_ref()
            || policy.sensor_profile_digest.is_none()
        {
            gates.unknown(
                "PK-PRE-PROFILE-DIGEST",
                "sensor profile must match the policy-bound content digest",
            );
        }
    }
}

fn validate_observations(input: &PreMergeEvaluationInput, gates: &mut Gates) {
    let pr = &input.pull_request;
    if !input.work_item.authorized {
        gates.block(
            "PK-PRE-WORK-UNAUTHORIZED",
            "the current work item is not authorized",
        );
    }
    if pr.state != PullRequestState::Open {
        gates.block(
            "PK-PRE-PR-NOT-OPEN",
            "pre-merge authorization requires an open pull request",
        );
    }
    if pr.is_draft {
        gates.block(
            "PK-PRE-PR-DRAFT",
            "a draft pull request cannot be ready to merge",
        );
    }
    for timestamp in [&input.work_item.observed_at, &pr.observed_at] {
        check_freshness(input, timestamp, gates);
    }
    let Some(context) = &input.context else {
        return;
    };
    let Some(ready) = &context.agent_ready else {
        gates.unknown(
            "PK-PRE-AGENT-READY-MISSING",
            "an authorized agent-ready observation is required",
        );
        return;
    };
    if ready.repository != pr.repository
        || ready.work_item_id != input.work_item.work_item_id
        || ready.pull_request_number != pr.pull_request_number
        || ready.head_sha != pr.head_sha
    {
        gates.unknown(
            "PK-PRE-AGENT-READY-BINDING",
            "agent-ready must bind this work item and exact PR head",
        );
    }
    if ready.label != "agent-ready"
        || ready.actor.trim().is_empty()
        || !input.policy.authorized_actors.contains(&ready.actor)
    {
        gates.unknown(
            "PK-PRE-AGENT-READY-UNAUTHORIZED",
            "agent-ready must be applied by a policy-authorized actor",
        );
    }
    if input.mode == EvaluationMode::Authoritative && ready.trust == TrustClass::LocalUntrusted {
        gates.unknown(
            "PK-PROV-UNTRUSTED",
            "authoritative evaluation requires a trusted agent-ready observation",
        );
    }
    check_freshness(input, &ready.observed_at, gates);
}

fn validate_execution(input: &PreMergeEvaluationInput, gates: &mut Gates) {
    let bundle = &input.evidence;
    check_freshness(input, &bundle.manifest.created_at, gates);
    if bundle.modifications.is_empty() {
        gates.unknown(
            "PK-PRE-MODIFICATION-MISSING",
            "contributor evidence is required to establish independent review",
        );
    }
    for timestamp in bundle
        .modifications
        .iter()
        .map(|item| &item.observed_at)
        .chain(bundle.reviews.iter().map(|item| &item.observed_at))
        .chain(bundle.sensors.iter().map(|item| &item.observed_at))
        .chain(bundle.waivers.iter().map(|item| &item.issued_at))
    {
        check_freshness(input, timestamp, gates);
    }
    if bundle.executions.is_empty() {
        gates.unknown("PK-EXEC-MISSING", "execution evidence is required");
    }
    for execution in &bundle.executions {
        check_freshness(input, &execution.started_at, gates);
        check_freshness(input, &execution.completed_at, gates);
        if execution.started_at > execution.completed_at {
            gates.unknown(
                "PK-EXEC-TIME-INVALID",
                "execution completion cannot precede its start",
            );
        }
        match execution.status {
            ExecutionStatus::Passed => {}
            ExecutionStatus::Failed => gates.block("PK-EXEC-FAILED", "a recorded execution failed"),
            ExecutionStatus::TimedOut => {
                gates.unknown("PK-EXEC-TIMEOUT", "a recorded execution timed out")
            }
            ExecutionStatus::Truncated => gates.unknown(
                "PK-EXEC-TRUNCATED",
                "a recorded execution exceeded its output limit",
            ),
            ExecutionStatus::Skipped => {
                gates.unknown("PK-EXEC-SKIPPED", "a recorded execution was skipped")
            }
            ExecutionStatus::Unparsable => gates.unknown(
                "PK-EXEC-UNPARSABLE",
                "a recorded execution result could not be parsed",
            ),
        }
    }
    if bundle.sensors.iter().any(|sensor| {
        sensor
            .source_digest
            .as_ref()
            .is_some_and(|digest| digest != &bundle.manifest.source_set_digest)
    }) {
        gates.unknown(
            "PK-PRE-SOURCE-MISMATCH",
            "sensor source digest must match the evaluated source set",
        );
    }
}

fn validate_sensors(input: &PreMergeEvaluationInput, gates: &mut Gates) {
    if input.evidence.test_adequacy.is_none() {
        gates.unknown(
            "PK-ADEQ-MISSING",
            "pre-merge authorization requires test adequacy evidence",
        );
    }
    let Some(context) = &input.context else {
        return;
    };
    // Union explicit policy requirements with profile-resolved requirements while
    // keeping the real change context for not-applicable validation.
    let mut profile = context.sensor_profile.clone();
    for id in &input.policy.required_sensor_ids {
        if let Some(sensor) = profile.sensors.iter_mut().find(|sensor| &sensor.id == id) {
            sensor.required = true;
        } else {
            gates.unknown(
                "PK-SENSOR-UNKNOWN",
                "policy requires a sensor absent from its bound profile",
            );
        }
    }
    let paths: Vec<_> = context.changed_paths.iter().map(String::as_str).collect();
    match evaluate_sensor_evidence_for_change(
        &profile,
        &paths,
        context.risk,
        &input.evidence.sensors,
        input.evidence.test_adequacy.as_ref(),
    ) {
        Ok(result) => gates.extend(
            match result.status {
                SensorEvaluationStatus::Passed => PreMergeStatus::ReadyToMerge,
                SensorEvaluationStatus::Blocked => PreMergeStatus::Blocked,
                SensorEvaluationStatus::Inconclusive => PreMergeStatus::Inconclusive,
            },
            result.reasons,
        ),
        Err(error) => gates.unknown("PK-SENSOR-PROFILE-INVALID", &error.to_string()),
    }
}

fn check_freshness(input: &PreMergeEvaluationInput, timestamp: &UtcTimestamp, gates: &mut Gates) {
    let age = input
        .evaluation_time
        .as_datetime()
        .signed_duration_since(*timestamp.as_datetime());
    // Duration conversion rejects negative ages and preserves fractional seconds
    // at both boundaries without overflowing large policy windows.
    if !age.to_std().is_ok_and(|age| {
        age <= std::time::Duration::from_secs(input.policy.max_evidence_age_seconds)
    }) {
        gates.unknown(
            "PK-PROV-STALE",
            "evidence or observation falls outside the policy freshness window",
        );
    }
}

struct Gates {
    status: PreMergeStatus,
    reasons: Vec<Reason>,
}

impl Default for Gates {
    fn default() -> Self {
        Self {
            status: PreMergeStatus::ReadyToMerge,
            reasons: Vec::new(),
        }
    }
}

impl Gates {
    fn extend(&mut self, status: PreMergeStatus, reasons: Vec<Reason>) {
        if status == PreMergeStatus::Blocked
            || (status == PreMergeStatus::Inconclusive
                && self.status == PreMergeStatus::ReadyToMerge
                && !reasons.is_empty())
        {
            self.status = status;
        }
        self.reasons.extend(reasons);
    }

    fn unknown(&mut self, code: &str, message: &str) {
        self.extend(
            PreMergeStatus::Inconclusive,
            vec![Reason {
                code: code.into(),
                message: message.into(),
            }],
        );
    }

    fn block(&mut self, code: &str, message: &str) {
        self.extend(
            PreMergeStatus::Blocked,
            vec![Reason {
                code: code.into(),
                message: message.into(),
            }],
        );
    }
}
