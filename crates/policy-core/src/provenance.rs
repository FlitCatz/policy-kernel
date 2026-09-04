use crate::canonical_sha256;
use policy_contracts::{
    EvaluationMode, EvidenceBundle, ModificationEvidence, Reason, ReviewEvidence, TrustClass,
    UtcTimestamp,
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
        });
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

    if timestamps(bundle).any(|timestamp| {
        let age = evaluation_time
            .as_datetime()
            .signed_duration_since(timestamp.as_datetime());
        age.num_seconds() < 0 || age.num_seconds() as u64 > max_evidence_age_seconds
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
