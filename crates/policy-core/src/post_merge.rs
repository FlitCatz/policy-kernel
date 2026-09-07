use crate::{canonical_sha256, pre_merge_policy_digest};
use policy_contracts::*;
use serde_json::json;
use std::collections::BTreeSet;

/// Evaluate explicitly supplied required-check and protected-merge observations.
pub fn evaluate_post_merge(input: &PostMergeEvaluationInput) -> PostMergeDecision {
    let mut gates = Gates::default();
    validate_policy_and_versions(input, &mut gates);
    validate_authorization_chain(input, &mut gates);
    validate_merge_policy(input, &mut gates);

    gates
        .reasons
        .sort_by(|left, right| (&left.code, &left.message).cmp(&(&right.code, &right.message)));
    gates.reasons.dedup();

    let pre_merge = input.pre_merge.as_decision();
    let mut decision = PostMergeDecision {
        schema_version: SCHEMA_VERSION,
        status: gates.status,
        binding: DecisionBinding {
            repository: pre_merge.binding.repository.clone(),
            pull_request_number: pre_merge.binding.pull_request_number,
            head_sha: pre_merge.binding.head_sha.clone(),
            policy_digest: input.policy.policy_digest.clone(),
            evidence_digest: pre_merge.binding.evidence_digest.clone(),
            evaluated_at: input.evaluation_time.clone(),
        },
        merge_commit_sha: Some(input.merge.merge_commit_sha.clone()),
        reasons: gates.reasons,
        // The field is excluded from its own preimage.
        decision_digest: pre_merge.decision_digest.clone(),
    };
    decision.decision_digest = post_merge_decision_digest(input, &decision);
    decision
}

fn validate_policy_and_versions(input: &PostMergeEvaluationInput, gates: &mut Gates) {
    let mut versions = vec![
        input.schema_version,
        input.policy.schema_version,
        input.pre_merge.as_decision().schema_version,
        input.merge.schema_version,
    ];
    if let Some(check) = &input.check {
        versions.push(check.schema_version);
    }
    if versions.iter().any(|version| *version != SCHEMA_VERSION) {
        gates.unknown(
            "PK-POST-SCHEMA-UNSUPPORTED",
            "all runtime schema versions must be supported",
        );
    }

    if pre_merge_policy_digest(&input.policy).ok().as_ref() != Some(&input.policy.policy_digest) {
        gates.unknown(
            "PK-POST-POLICY-DIGEST",
            "policy content does not match its canonical digest",
        );
    }

    let policy = &input.policy;
    let actor_count = policy
        .authorized_actors
        .iter()
        .collect::<BTreeSet<_>>()
        .len();
    let branch_count = policy
        .protected_target_branches
        .iter()
        .collect::<BTreeSet<_>>()
        .len();
    let methods_are_unique = policy
        .allowed_merge_methods
        .iter()
        .enumerate()
        .all(|(index, method)| !policy.allowed_merge_methods[index + 1..].contains(method));
    if policy.required_check_name.trim().is_empty()
        || policy.max_evidence_age_seconds == 0
        || policy
            .authorized_actors
            .iter()
            .any(|item| item.trim().is_empty())
        || policy
            .protected_target_branches
            .iter()
            .any(|item| item.trim().is_empty())
        || actor_count != policy.authorized_actors.len()
        || branch_count != policy.protected_target_branches.len()
        || !methods_are_unique
    {
        gates.unknown(
            "PK-POST-POLICY-INVALID",
            "post-merge policy requires unambiguous check, freshness, branch, actor and method declarations",
        );
    }
}

fn validate_authorization_chain(input: &PostMergeEvaluationInput, gates: &mut Gates) {
    let pre_merge = input.pre_merge.as_decision();
    let merge = &input.merge;

    if pre_merge.binding.repository != merge.repository {
        gates.unknown(
            "PK-POST-REPOSITORY-MISMATCH",
            "merge repository must match the pre-merge authorization",
        );
    }
    if pre_merge.binding.pull_request_number != merge.pull_request_number {
        gates.unknown(
            "PK-POST-PR-MISMATCH",
            "merge pull request must match the pre-merge authorization",
        );
    }
    if pre_merge.binding.head_sha != merge.head_sha {
        gates.unknown(
            "PK-POST-HEAD-MISMATCH",
            "merge headRefOid must match the evaluated pull request head",
        );
    }
    if pre_merge.binding.policy_digest != input.policy.policy_digest {
        gates.unknown(
            "PK-POST-POLICY-MISMATCH",
            "pre-merge authorization must match the current policy digest",
        );
    }

    if merge.merged_at > input.evaluation_time {
        gates.unknown(
            "PK-POST-MERGE-TIME-INVALID",
            "merge must precede evaluation and remain inside the freshness window",
        );
    } else if !is_fresh(
        &merge.merged_at,
        &input.evaluation_time,
        input.policy.max_evidence_age_seconds,
    ) {
        gates.unknown(
            "PK-POST-MERGE-STALE",
            "merge must precede evaluation and remain inside the freshness window",
        );
    }
    if !is_fresh(
        &pre_merge.binding.evaluated_at,
        &merge.merged_at,
        input.policy.max_evidence_age_seconds,
    ) {
        gates.unknown(
            "PK-POST-PRE-MERGE-STALE",
            "pre-merge authorization must precede the merge inside the freshness window",
        );
    }

    let Some(check) = &input.check else {
        gates.unknown(
            "PK-POST-CHECK-MISSING",
            "a trusted required-check observation is required",
        );
        return;
    };
    if check.repository != pre_merge.binding.repository || check.repository != merge.repository {
        gates.unknown(
            "PK-POST-REPOSITORY-MISMATCH",
            "required check repository must match the authorized merge",
        );
    }
    if check.pull_request_number != pre_merge.binding.pull_request_number
        || check.pull_request_number != merge.pull_request_number
    {
        gates.unknown(
            "PK-POST-PR-MISMATCH",
            "required check pull request must match the authorized merge",
        );
    }
    if check.head_sha != pre_merge.binding.head_sha || check.head_sha != merge.head_sha {
        gates.unknown(
            "PK-POST-HEAD-MISMATCH",
            "required check head must match the exact evaluated pull request head",
        );
    }
    if check.policy_digest != input.policy.policy_digest
        || check.policy_digest != pre_merge.binding.policy_digest
    {
        gates.unknown(
            "PK-POST-POLICY-MISMATCH",
            "required check must publish the exact evaluated policy digest",
        );
    }
    if check.decision_digest != pre_merge.decision_digest {
        gates.unknown(
            "PK-POST-DECISION-MISMATCH",
            "required check must publish the exact pre-merge decision digest",
        );
    }
    if check.check_name != input.policy.required_check_name {
        gates.unknown(
            "PK-POST-CHECK-NAME-MISMATCH",
            "required check name must match the project policy",
        );
    }
    if check.conclusion != CheckConclusion::Success {
        gates.unknown(
            "PK-POST-CHECK-NOT-SUCCESS",
            "required check must conclude successfully",
        );
    }
    if check.started_at > check.completed_at || pre_merge.binding.evaluated_at > check.completed_at
    {
        gates.unknown(
            "PK-POST-CHECK-TIME-INVALID",
            "required check completion must follow its start and the pre-merge evaluation",
        );
    }
    if check.completed_at >= merge.merged_at {
        gates.unknown(
            "PK-POST-CHECK-AFTER-MERGE",
            "required check must complete before the merge",
        );
    }
    if !is_fresh(
        &check.started_at,
        &merge.merged_at,
        input.policy.max_evidence_age_seconds,
    ) || !is_fresh(
        &check.completed_at,
        &merge.merged_at,
        input.policy.max_evidence_age_seconds,
    ) {
        gates.unknown(
            "PK-POST-CHECK-STALE",
            "required check must complete inside the merge-time freshness window",
        );
    }
}

fn validate_merge_policy(input: &PostMergeEvaluationInput, gates: &mut Gates) {
    let merge = &input.merge;
    match &merge.target_branch {
        Some(branch)
            if merge.protected && input.policy.protected_target_branches.contains(branch) => {}
        Some(_) if !input.policy.protected_target_branches.is_empty() => gates.block(
            "PK-POST-TARGET-UNPROTECTED",
            "merge must target a policy-declared protected branch",
        ),
        _ => gates.unknown(
            "PK-POST-TARGET-UNPROTECTED",
            "a policy-declared protected target branch observation is required",
        ),
    }

    match merge.actor_type {
        Some(MergeActorType::Human)
            if !merge.merged_by.trim().is_empty()
                && input.policy.authorized_actors.contains(&merge.merged_by) => {}
        Some(_) => gates.block(
            "PK-POST-MERGE-ACTOR-UNAUTHORIZED",
            "merge must be performed by a policy-authorized human actor",
        ),
        None => gates.unknown(
            "PK-POST-MERGE-ACTOR-UNAUTHORIZED",
            "a human merge actor observation is required",
        ),
    }

    if input.policy.allowed_merge_methods.is_empty() {
        gates.unknown(
            "PK-POST-MERGE-METHOD-UNDECLARED",
            "policy must declare an allowed merge method",
        );
    } else if !input.policy.allowed_merge_methods.contains(&merge.method) {
        gates.block(
            "PK-POST-MERGE-METHOD-UNDECLARED",
            "observed merge method is not declared by policy",
        );
    }

    match merge.via_merge_queue {
        Some(false) => {}
        Some(true) => gates.block(
            "PK-POST-MERGE-QUEUE-UNSUPPORTED",
            "merge-queue observations require a separately declared contract",
        ),
        None => gates.unknown(
            "PK-POST-MERGE-QUEUE-UNKNOWN",
            "merge-queue use must be explicitly observed",
        ),
    }
}

fn is_fresh(observed: &UtcTimestamp, reference: &UtcTimestamp, max_age_seconds: u64) -> bool {
    reference
        .as_datetime()
        .signed_duration_since(*observed.as_datetime())
        .to_std()
        .is_ok_and(|age| age <= std::time::Duration::from_secs(max_age_seconds))
}

fn post_merge_decision_digest(
    input: &PostMergeEvaluationInput,
    decision: &PostMergeDecision,
) -> Sha256Digest {
    let mut decision_value =
        serde_json::to_value(decision).expect("typed post-merge decision serializes as JSON");
    decision_value
        .as_object_mut()
        .expect("decision is an object")
        .remove("decision_digest");
    canonical_sha256(&json!({
        "domain": "policy-kernel/post-merge-decision/v1",
        "input": input,
        "decision": decision_value,
    }))
    .expect("typed post-merge input has a canonical JSON representation")
}

struct Gates {
    status: PostMergeStatus,
    reasons: Vec<Reason>,
}

impl Default for Gates {
    fn default() -> Self {
        Self {
            status: PostMergeStatus::Shipped,
            reasons: Vec::new(),
        }
    }
}

impl Gates {
    fn insert(&mut self, status: PostMergeStatus, code: &str, message: &str) {
        if status == PostMergeStatus::Blocked
            || (status == PostMergeStatus::Inconclusive && self.status == PostMergeStatus::Shipped)
        {
            self.status = status;
        }
        self.reasons.push(Reason {
            code: code.into(),
            message: message.into(),
        });
    }

    fn unknown(&mut self, code: &str, message: &str) {
        self.insert(PostMergeStatus::Inconclusive, code, message);
    }

    fn block(&mut self, code: &str, message: &str) {
        self.insert(PostMergeStatus::Blocked, code, message);
    }
}
