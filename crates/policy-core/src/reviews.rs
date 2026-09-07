use policy_contracts::{
    CommitSha, EvaluationMode, Finding, FindingSeverity, ModificationEvidence, PolicySnapshot,
    Reason, RepositoryId, ReviewEvidence, ReviewStatus, Sha256Digest, TrustClass, UtcTimestamp,
    WaiverEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvaluationInput {
    pub mode: EvaluationMode,
    pub policy: PolicySnapshot,
    pub repository: RepositoryId,
    pub head_sha: CommitSha,
    pub evidence_digest: Sha256Digest,
    pub evaluation_time: UtcTimestamp,
    pub modifications: Vec<ModificationEvidence>,
    pub reviews: Vec<ReviewEvidence>,
    pub findings: Vec<Finding>,
    pub waivers: Vec<WaiverEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewEvaluation {
    pub status: ReviewStatus,
    pub reasons: Vec<Reason>,
}

impl ReviewEvaluation {
    pub fn has_reason(&self, code: &str) -> bool {
        self.reasons.iter().any(|reason| reason.code == code)
    }
}

pub fn evaluate_reviews(input: ReviewEvaluationInput) -> ReviewEvaluation {
    let mut reasons = Vec::new();
    if input.reviews.is_empty() {
        insert_reason(
            &mut reasons,
            "PK-REVIEW-MISSING",
            "at least one independent review is required",
        );
    }

    validate_review_provenance(&input, &mut reasons);

    let finding_ids = input
        .findings
        .iter()
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();
    if input
        .waivers
        .iter()
        .any(|waiver| !finding_ids.contains(waiver.finding_id.as_str()))
    {
        insert_reason(
            &mut reasons,
            "PK-WAIVER-INVALID",
            "every waiver must reference a finding in this evaluation",
        );
    }

    let has_approved = input
        .reviews
        .iter()
        .any(|review| review.status == ReviewStatus::Approved);
    let has_blocked = input
        .reviews
        .iter()
        .any(|review| review.status == ReviewStatus::Blocked);
    if has_approved && has_blocked {
        insert_reason(
            &mut reasons,
            "PK-REVIEW-CONFLICT",
            "independent reviewers returned contradictory reports",
        );
    }

    let review_by_execution = input
        .reviews
        .iter()
        .filter(|review| {
            input
                .reviews
                .iter()
                .filter(|other| other.execution_id == review.execution_id)
                .count()
                == 1
        })
        .map(|review| (&review.execution_id, review))
        .collect::<BTreeMap<_, _>>();
    let mut findings_by_id: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for finding in &input.findings {
        if finding.id.trim().is_empty()
            || !review_by_execution.contains_key(&finding.reviewer_execution_id)
        {
            insert_reason(
                &mut reasons,
                "PK-PROV-REVIEW-FINDING",
                "every finding must have a stable ID and reference a recorded review",
            );
        }
        findings_by_id.entry(&finding.id).or_default().push(finding);
    }

    let mut blocked = false;
    for (finding_id, findings) in findings_by_id {
        let severity = findings[0].severity;
        let conflict = findings.iter().any(|finding| finding.severity != severity);
        if conflict {
            insert_reason(
                &mut reasons,
                "PK-REVIEW-MEDIUM-CONFLICT",
                "reviewers reported conflicting severities for the same stable finding ID",
            );
        }

        let (waived, waiver_problem) = waiver_state(&input, finding_id);
        if waived {
            insert_reason(
                &mut reasons,
                "PK-WAIVER-APPLIED",
                "an authorized human waiver was applied",
            );
            continue;
        }
        if let Some((code, message)) = waiver_problem {
            insert_reason(&mut reasons, code, message);
        }

        if findings.iter().any(|finding| {
            matches!(
                finding.severity,
                FindingSeverity::Critical | FindingSeverity::High
            )
        }) {
            blocked = true;
            insert_reason(
                &mut reasons,
                "PK-REVIEW-HIGH",
                "an unwaived critical or high finding blocks approval",
            );
            continue;
        }
        if conflict {
            continue;
        }
        match severity {
            FindingSeverity::Critical | FindingSeverity::High => {
                blocked = true;
                insert_reason(
                    &mut reasons,
                    "PK-REVIEW-HIGH",
                    "an unwaived critical or high finding blocks approval",
                );
            }
            FindingSeverity::Medium => {
                let providers = findings
                    .iter()
                    .filter_map(|finding| review_by_execution.get(&finding.reviewer_execution_id))
                    .filter(|review| crate::verifier_is_independent(review, &input.modifications))
                    .map(|review| &review.provider)
                    .collect::<BTreeSet<_>>();
                if providers.len() >= 2 {
                    blocked = true;
                    insert_reason(
                        &mut reasons,
                        "PK-REVIEW-MEDIUM-CONFIRMED",
                        "two independent providers confirmed the same medium finding",
                    );
                } else {
                    insert_reason(
                        &mut reasons,
                        "PK-REVIEW-MEDIUM-UNCONFIRMED",
                        "a medium finding lacks confirmation by a second independent provider",
                    );
                }
            }
            FindingSeverity::Low | FindingSeverity::Info => {}
        }
    }

    if input
        .reviews
        .iter()
        .any(|review| review.status == ReviewStatus::Inconclusive)
    {
        insert_reason(
            &mut reasons,
            "PK-REVIEW-INCONCLUSIVE",
            "a reviewer returned an inconclusive report",
        );
    }

    for review in &input.reviews {
        if review.status != ReviewStatus::Blocked {
            continue;
        }
        let actionable: Vec<_> = input
            .findings
            .iter()
            .filter(|finding| {
                finding.reviewer_execution_id == review.execution_id
                    && matches!(
                        finding.severity,
                        FindingSeverity::Medium | FindingSeverity::High | FindingSeverity::Critical
                    )
            })
            .collect();
        if actionable.is_empty() {
            insert_reason(
                &mut reasons,
                "PK-REVIEW-BLOCKED-UNRESOLVED",
                "a blocked report must identify actionable findings",
            );
        }
    }

    let status = if blocked {
        ReviewStatus::Blocked
    } else if reasons
        .iter()
        .any(|reason| reason.code != "PK-WAIVER-APPLIED")
    {
        ReviewStatus::Inconclusive
    } else {
        ReviewStatus::Approved
    };
    ReviewEvaluation { status, reasons }
}

fn validate_review_provenance(input: &ReviewEvaluationInput, reasons: &mut Vec<Reason>) {
    if input.mode == EvaluationMode::Authoritative
        && (input
            .reviews
            .iter()
            .any(|r| r.trust == TrustClass::LocalUntrusted)
            || input
                .modifications
                .iter()
                .any(|m| m.trust == TrustClass::LocalUntrusted))
    {
        insert_reason(
            reasons,
            "PK-PROV-UNTRUSTED",
            "authoritative review requires trusted evidence",
        );
    }
    for timestamp in input
        .reviews
        .iter()
        .map(|r| &r.observed_at)
        .chain(input.modifications.iter().map(|m| &m.observed_at))
    {
        let age = input
            .evaluation_time
            .as_datetime()
            .signed_duration_since(*timestamp.as_datetime());
        if !age.to_std().is_ok_and(|age| {
            age <= std::time::Duration::from_secs(input.policy.max_evidence_age_seconds)
        }) {
            insert_reason(
                reasons,
                "PK-PROV-STALE",
                "review evidence falls outside the policy freshness window",
            );
        }
    }
    let mut providers = BTreeSet::new();
    let mut executions = BTreeSet::new();
    for modification in &input.modifications {
        if !executions.insert(modification.execution_id.as_str()) {
            insert_reason(
                reasons,
                "PK-PROV-DUPLICATE-EXECUTION",
                "execution IDs must be unique across modifications and reviews",
            );
        }
        if modification.head_sha != input.head_sha {
            insert_reason(
                reasons,
                "PK-PROV-BINDING-MISMATCH",
                "review inputs must be trusted and bound to the evaluated head",
            );
        }
    }
    for review in &input.reviews {
        if !providers.insert(review.provider.as_str()) {
            insert_reason(
                reasons,
                "PK-PROV-DUPLICATE-PROVIDER",
                "review providers must be unique",
            );
        }
        if !executions.insert(review.execution_id.as_str()) {
            insert_reason(
                reasons,
                "PK-PROV-DUPLICATE-EXECUTION",
                "execution IDs must be unique across modifications and reviews",
            );
        }
        if review.head_sha != input.head_sha {
            insert_reason(
                reasons,
                "PK-PROV-BINDING-MISMATCH",
                "review inputs must be trusted and bound to the evaluated head",
            );
        }
        if input.modifications.iter().any(|modification| {
            modification.provider == review.provider
                || modification.execution_id == review.execution_id
        }) {
            insert_reason(
                reasons,
                "PK-SELF-PROVIDER",
                "a reviewer must be independent from every code contributor",
            );
        }
    }
}

fn waiver_state(
    input: &ReviewEvaluationInput,
    finding_id: &str,
) -> (bool, Option<(&'static str, &'static str)>) {
    let waivers = input
        .waivers
        .iter()
        .filter(|waiver| waiver.finding_id == finding_id)
        .collect::<Vec<_>>();
    if waivers.is_empty() {
        return (false, None);
    }
    if waivers.len() != 1 {
        return (
            false,
            Some((
                "PK-WAIVER-INVALID",
                "a finding must have at most one waiver",
            )),
        );
    }
    let waiver = waivers[0];
    let authorized = input
        .policy
        .authorized_waiver_actors
        .iter()
        .any(|actor| actor == &waiver.actor);
    let age = input
        .evaluation_time
        .as_datetime()
        .signed_duration_since(*waiver.issued_at.as_datetime());
    let bindings_match = !waiver.actor.trim().is_empty()
        && !finding_id.trim().is_empty()
        && input
            .policy
            .authorized_waiver_actors
            .iter()
            .all(|actor| !actor.trim().is_empty())
        && age.to_std().is_ok_and(|age| {
            age <= std::time::Duration::from_secs(input.policy.max_evidence_age_seconds)
        })
        && waiver.repository == input.repository
        && waiver.head_sha == input.head_sha
        && waiver.evidence_digest == input.evidence_digest
        && !waiver.reason.trim().is_empty()
        && waiver.issued_at.as_datetime() < waiver.expires_at.as_datetime();
    if !authorized || !bindings_match {
        return (
            false,
            Some((
                "PK-WAIVER-INVALID",
                "waiver authorization, binding, reason, or issuance time is invalid",
            )),
        );
    }
    if waiver.expires_at.as_datetime() <= input.evaluation_time.as_datetime() {
        return (
            false,
            Some(("PK-WAIVER-EXPIRED", "the matching human waiver has expired")),
        );
    }
    (true, None)
}

fn insert_reason(reasons: &mut Vec<Reason>, code: &str, message: &str) {
    if reasons.iter().all(|reason| reason.code != code) {
        reasons.push(Reason {
            code: code.to_owned(),
            message: message.to_owned(),
        });
        reasons.sort_by(|left, right| left.code.cmp(&right.code));
    }
}
