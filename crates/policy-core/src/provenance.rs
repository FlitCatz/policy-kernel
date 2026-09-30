use crate::canonical_sha256;
use policy_contracts::{
    EvaluationMode, EvidenceBundle, ModificationEvidence, PreMergeEvaluationInput, Reason,
    ReviewEvidence, TrustClass, UtcTimestamp,
};
use serde::Serialize;
use std::collections::BTreeSet;

const REQUIRED_CHILD_PATHS: [&str; 8] = [
    "executions",
    "findings",
    "knowledge",
    "modifications",
    "reviews",
    "sensors",
    "test_adequacy",
    "waivers",
];

/// Presence is a policy obligation, separate from validating supplied evidence.
/// Trust classes here are assertions authenticated by the controller boundary.
pub(crate) fn validate_required_knowledge(input: &PreMergeEvaluationInput) -> ValidationResult {
    let mut result = ValidationResult::default();
    if input.policy.required_knowledge.is_empty() {
        return result;
    }
    let Some(context) = &input.context else {
        result.insert(
            "PK-KNOWLEDGE-CONTEXT-INVALID",
            "knowledge requirements need the current change context",
        );
        return result;
    };
    let paths = context
        .changed_paths
        .iter()
        .map(|path| crate::sensors::normalize_path(path))
        .collect::<Result<Vec<_>, _>>();
    let Ok(paths) = paths else {
        result.insert(
            "PK-KNOWLEDGE-CONTEXT-INVALID",
            "knowledge change paths contain parent traversal",
        );
        return result;
    };
    for (index, requirement) in input.policy.required_knowledge.iter().enumerate() {
        let invalid = requirement.source.trim().is_empty()
            || requirement.source.contains('\0')
            || input.policy.required_knowledge[..index]
                .iter()
                .any(|other| other.kind == requirement.kind && other.source == requirement.source)
            || requirement
                .required_when
                .iter()
                .flat_map(|condition| &condition.path_globs)
                .any(|pattern| {
                    pattern.trim().is_empty()
                        || pattern.contains('\0')
                        || crate::sensors::path_glob(pattern).is_err()
                });
        if invalid {
            result.insert("PK-KNOWLEDGE-POLICY-INVALID", "knowledge requirements need unique kind/source identities and valid nonempty globs");
            continue;
        }
        let applies = requirement.required_when.is_empty()
            || requirement.required_when.iter().any(|condition| {
                (condition.risks.is_empty() || condition.risks.contains(&context.risk))
                    && (condition.path_globs.is_empty()
                        || condition.path_globs.iter().any(|pattern| {
                            let matcher =
                                crate::sensors::path_glob(pattern).expect("validated glob");
                            paths.iter().any(|path| matcher.is_match(path))
                        }))
            });
        if !applies {
            continue;
        }
        let matches: Vec<_> = input
            .evidence
            .knowledge
            .iter()
            .filter(|item| item.kind == requirement.kind && item.source == requirement.source)
            .collect();
        let description = format!(
            "required {:?} knowledge from {:?}",
            requirement.kind, requirement.source
        );
        if matches.is_empty() {
            if input.evidence.knowledge.is_empty() {
                result.insert("PK-KNOWLEDGE-MISSING", &format!("{description} is missing"));
            } else {
                if !input
                    .evidence
                    .knowledge
                    .iter()
                    .any(|item| item.kind == requirement.kind)
                {
                    result.insert(
                        "PK-KNOWLEDGE-KIND-MISMATCH",
                        &format!("{description} has no matching kind"),
                    );
                } else {
                    result.insert(
                        "PK-KNOWLEDGE-SOURCE-MISMATCH",
                        &format!("{description} has no matching source for its kind"),
                    );
                }
            }
            continue;
        }
        if matches.len() != 1 {
            result.insert(
                "PK-KNOWLEDGE-DUPLICATE",
                &format!("{description} must have exactly one observation"),
            );
        }
        for evidence in matches {
            if evidence.head_sha != input.evidence.manifest.head_sha {
                result.insert(
                    "PK-KNOWLEDGE-HEAD-MISMATCH",
                    &format!("{description} does not bind the evaluated head"),
                );
            }
            if evidence.source_set_digest != input.evidence.manifest.source_set_digest {
                result.insert(
                    "PK-KNOWLEDGE-SOURCE-SET-MISMATCH",
                    &format!("{description} does not bind the evaluated source set"),
                );
            }
            if evidence.trust == TrustClass::LocalUntrusted {
                result.insert(
                    "PK-KNOWLEDGE-UNTRUSTED",
                    &format!("{description} requires CI observation or trusted-runner attestation"),
                );
            }
            if evidence.version.trim().is_empty() || evidence.version.contains('\0') {
                result.insert(
                    "PK-KNOWLEDGE-METADATA-INVALID",
                    &format!("{description} must record a nonblank tool version without NUL"),
                );
            }
        }
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationResult {
    reasons: Vec<Reason>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.reasons.is_empty()
    }

    pub fn reasons(&self) -> &[Reason] {
        &self.reasons
    }

    pub fn reason_codes(&self) -> Vec<&str> {
        self.reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect()
    }

    fn insert(&mut self, code: &str, message: &str) {
        if self.reasons.iter().all(|reason| reason.code != code) {
            self.reasons.push(Reason {
                code: code.to_owned(),
                message: message.to_owned(),
            });
            self.reasons
                .sort_by(|left, right| left.code.cmp(&right.code));
        }
    }
}

pub fn validate_bundle(bundle: &EvidenceBundle) -> ValidationResult {
    let mut result = ValidationResult::default();
    let mut paths = BTreeSet::new();

    for artifact in &bundle.manifest.child_digests {
        let actual = child_digest(bundle, &artifact.path);
        if !paths.insert(artifact.path.as_str()) || actual.as_ref() != Some(&artifact.digest) {
            result.insert(
                "PK-PROV-DIGEST-MISMATCH",
                "evidence child digest does not match its manifest binding",
            );
        }
    }
    if paths != REQUIRED_CHILD_PATHS.into_iter().collect() {
        result.insert(
            "PK-PROV-DIGEST-MISMATCH",
            "evidence manifest must bind every required child exactly once",
        );
    }

    let head_matches = bundle
        .modifications
        .iter()
        .all(|evidence| evidence.head_sha == bundle.manifest.head_sha)
        && bundle
            .reviews
            .iter()
            .all(|evidence| evidence.head_sha == bundle.manifest.head_sha)
        && bundle
            .sensors
            .iter()
            .all(|evidence| evidence.head_sha == bundle.manifest.head_sha)
        && bundle.knowledge.iter().all(|evidence| {
            evidence.head_sha == bundle.manifest.head_sha
                && evidence.source_set_digest == bundle.manifest.source_set_digest
        })
        && bundle.waivers.iter().all(|evidence| {
            evidence.repository == bundle.manifest.repository
                && evidence.head_sha == bundle.manifest.head_sha
        })
        && bundle
            .executions
            .iter()
            .filter_map(|evidence| evidence.head_sha.as_ref())
            .chain(
                bundle
                    .test_adequacy
                    .iter()
                    .filter_map(|a| a.head_sha.as_ref()),
            )
            .all(|head| head == &bundle.manifest.head_sha);
    if !head_matches {
        result.insert(
            "PK-PROV-BINDING-MISMATCH",
            "evidence is not bound to the manifest head and source set",
        );
    }

    result
}

fn child_digest(bundle: &EvidenceBundle, path: &str) -> Option<policy_contracts::Sha256Digest> {
    match path {
        "modifications" => digest(&bundle.modifications),
        "executions" => digest(&bundle.executions),
        "reviews" => digest(&bundle.reviews),
        "findings" => digest(&bundle.findings),
        "waivers" => digest(&bundle.waivers),
        "sensors" => digest(&bundle.sensors),
        "test_adequacy" => digest(&bundle.test_adequacy),
        "knowledge" => digest(&bundle.knowledge),
        _ => None,
    }
}

fn digest<T: Serialize>(value: &T) -> Option<policy_contracts::Sha256Digest> {
    canonical_sha256(value).ok()
}

pub fn validate_provenance(
    mode: EvaluationMode,
    bundle: &EvidenceBundle,
    evaluation_time: &UtcTimestamp,
    max_evidence_age_seconds: u64,
) -> ValidationResult {
    let mut result = validate_bundle(bundle);

    if mode == EvaluationMode::Authoritative && contains_untrusted_evidence(bundle) {
        result.insert(
            "PK-PROV-UNTRUSTED",
            "authoritative evaluation requires CI-observed or trusted-runner evidence",
        );
    }
    if mode == EvaluationMode::Authoritative
        && (bundle.executions.iter().any(|e| e.head_sha.is_none())
            || bundle
                .test_adequacy
                .as_ref()
                .is_some_and(|a| a.head_sha.is_none()))
    {
        result.insert(
            "PK-PROV-BINDING-MISSING",
            "authoritative execution and adequacy evidence must name the evaluated head",
        );
    }

    if timestamps(bundle).any(|timestamp| {
        let age = evaluation_time
            .as_datetime()
            .signed_duration_since(timestamp.as_datetime());
        !age.to_std()
            .is_ok_and(|age| age <= std::time::Duration::from_secs(max_evidence_age_seconds))
    }) {
        result.insert(
            "PK-PROV-STALE",
            "evidence falls outside the policy freshness window",
        );
    }

    let mut executions = BTreeSet::new();
    for execution_id in bundle
        .modifications
        .iter()
        .map(|value| value.execution_id.as_str())
        .chain(
            bundle
                .executions
                .iter()
                .map(|value| value.execution_id.as_str()),
        )
        .chain(
            bundle
                .reviews
                .iter()
                .map(|value| value.execution_id.as_str()),
        )
    {
        if !executions.insert(execution_id) {
            result.insert(
                "PK-PROV-DUPLICATE-EXECUTION",
                "execution IDs must be unique across all evidence",
            );
        }
    }

    let mut providers = BTreeSet::new();
    for review in &bundle.reviews {
        if !providers.insert(review.provider.as_str()) {
            result.insert(
                "PK-PROV-DUPLICATE-PROVIDER",
                "reviewer providers must be unique",
            );
        }
        if !verifier_is_independent(review, &bundle.modifications) {
            result.insert(
                "PK-SELF-PROVIDER",
                "a verifier must be independent from every code contributor",
            );
        }
    }

    result
}

pub fn verifier_is_independent(
    verifier: &ReviewEvidence,
    modifiers: &[ModificationEvidence],
) -> bool {
    modifiers.iter().all(|modifier| {
        modifier.provider != verifier.provider && modifier.execution_id != verifier.execution_id
    })
}

fn contains_untrusted_evidence(bundle: &EvidenceBundle) -> bool {
    bundle
        .modifications
        .iter()
        .any(|value| value.trust == TrustClass::LocalUntrusted)
        || bundle
            .executions
            .iter()
            .any(|value| value.trust == TrustClass::LocalUntrusted)
        || bundle
            .reviews
            .iter()
            .any(|value| value.trust == TrustClass::LocalUntrusted)
        || bundle
            .sensors
            .iter()
            .any(|value| value.trust == TrustClass::LocalUntrusted)
        || bundle
            .knowledge
            .iter()
            .any(|value| value.trust == TrustClass::LocalUntrusted)
}

fn timestamps(bundle: &EvidenceBundle) -> impl Iterator<Item = &UtcTimestamp> {
    std::iter::once(&bundle.manifest.created_at)
        .chain(bundle.modifications.iter().map(|value| &value.observed_at))
        .chain(
            bundle
                .executions
                .iter()
                .flat_map(|value| [&value.started_at, &value.completed_at]),
        )
        .chain(bundle.reviews.iter().map(|value| &value.observed_at))
        .chain(bundle.sensors.iter().map(|value| &value.observed_at))
        .chain(bundle.waivers.iter().map(|value| &value.issued_at))
}
