use policy_contracts::{
    EvidenceBundle, PostMergeEvaluationInput, PostMergeStatus, PreMergeEvaluationInput,
    PreMergeStatus, ReviewStatus, SensorEvidence, SensorId, SensorProfile, TestAdequacyEvidence,
};
use policy_core::{
    evaluate_post_merge, evaluate_pre_merge, evaluate_reviews, evaluate_sensor_evidence,
    validate_bundle, ReviewEvaluationInput, SensorEvaluationStatus,
};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    schema_version: u32,
    source_repository: String,
    source_commit: String,
    obligations: Vec<Obligation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Obligation {
    id: String,
    source: Vec<FrozenSource>,
    expected: ExpectedStatus,
    reason_code: String,
    evaluator: Evaluator,
    fixture: String,
    changed_by_design: bool,
    approving_spec_path: Option<String>,
    approving_spec_section: Option<String>,
    preserved_rule: String,
    rejected_coupling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenSource {
    path: String,
    blob_sha: String,
    sha256: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ExpectedStatus {
    Blocked,
    Inconclusive,
    ReadyToMerge,
    Shipped,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Evaluator {
    Evidence,
    Reviews,
    Sensors,
    PreMerge,
    PostMerge,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SensorFixture {
    profile: SensorProfile,
    required_sensor_ids: Vec<SensorId>,
    evidence: Vec<SensorEvidence>,
    adequacy: Option<TestAdequacyEvidence>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Vec<u8> {
    let path = workspace_root().join(relative);
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read referenced fixture {}: {error}",
            path.display()
        )
    })
}

fn load_matrix() -> Matrix {
    serde_yaml::from_slice(&read("docs/obligation-matrix.yaml"))
        .expect("obligation matrix must satisfy the versioned expectation schema")
}

fn observe(obligation: &Obligation) -> (ExpectedStatus, Vec<String>) {
    let bytes = read(&obligation.fixture);
    match obligation.evaluator {
        Evaluator::Evidence => {
            let input: EvidenceBundle = serde_json::from_slice(&bytes)
                .expect("evidence fixture must satisfy EvidenceBundle");
            let result = validate_bundle(&input);
            let status = if result.is_valid() {
                ExpectedStatus::ReadyToMerge
            } else {
                ExpectedStatus::Inconclusive
            };
            (
                status,
                result
                    .reason_codes()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
        }
        Evaluator::Reviews => {
            let input: ReviewEvaluationInput = serde_json::from_slice(&bytes)
                .expect("review fixture must satisfy ReviewEvaluationInput");
            let result = evaluate_reviews(input);
            let status = match result.status {
                ReviewStatus::Approved => ExpectedStatus::ReadyToMerge,
                ReviewStatus::Blocked => ExpectedStatus::Blocked,
                ReviewStatus::Inconclusive => ExpectedStatus::Inconclusive,
            };
            (
                status,
                result
                    .reasons
                    .into_iter()
                    .map(|reason| reason.code)
                    .collect(),
            )
        }
        Evaluator::Sensors => {
            let input: SensorFixture = serde_json::from_slice(&bytes)
                .expect("sensor fixture must satisfy the typed sensor harness input");
            let result = evaluate_sensor_evidence(
                &input.profile,
                &input.required_sensor_ids,
                &input.evidence,
                input.adequacy.as_ref(),
            );
            let status = match result.status {
                SensorEvaluationStatus::Passed => ExpectedStatus::ReadyToMerge,
                SensorEvaluationStatus::Blocked => ExpectedStatus::Blocked,
                SensorEvaluationStatus::Inconclusive => ExpectedStatus::Inconclusive,
            };
            (
                status,
                result
                    .reasons
                    .into_iter()
                    .map(|reason| reason.code)
                    .collect(),
            )
        }
        Evaluator::PreMerge => {
            let input: PreMergeEvaluationInput = serde_json::from_slice(&bytes)
                .expect("pre-merge fixture must satisfy PreMergeEvaluationInput");
            let result = evaluate_pre_merge(&input);
            let status = match result.status {
                PreMergeStatus::Blocked => ExpectedStatus::Blocked,
                PreMergeStatus::Inconclusive => ExpectedStatus::Inconclusive,
                PreMergeStatus::ReadyToMerge => ExpectedStatus::ReadyToMerge,
            };
            (
                status,
                result
                    .reasons
                    .into_iter()
                    .map(|reason| reason.code)
                    .collect(),
            )
        }
        Evaluator::PostMerge => {
            let input: PostMergeEvaluationInput = serde_json::from_slice(&bytes)
                .expect("post-merge fixture must satisfy PostMergeEvaluationInput");
            let result = evaluate_post_merge(&input);
            let status = match result.status {
                PostMergeStatus::Blocked => ExpectedStatus::Blocked,
                PostMergeStatus::Inconclusive => ExpectedStatus::Inconclusive,
                PostMergeStatus::Shipped => ExpectedStatus::Shipped,
            };
            (
                status,
                result
                    .reasons
                    .into_iter()
                    .map(|reason| reason.code)
                    .collect(),
            )
        }
    }
}

#[test]
fn every_frozen_obligation_executes_its_referenced_typed_fixture() {
    let matrix = load_matrix();
    assert_eq!(matrix.schema_version, 1);
    assert_eq!(matrix.source_repository, "FlitCatz/agent-dev-kit");
    assert_eq!(matrix.source_commit.len(), 40);
    assert_eq!(
        matrix.obligations.len(),
        8,
        "matrix rows require an explicit test update"
    );

    let mut ids = BTreeSet::new();
    for obligation in &matrix.obligations {
        assert!(
            ids.insert(&obligation.id),
            "duplicate obligation {}",
            obligation.id
        );
        assert!(!obligation.preserved_rule.trim().is_empty());
        assert!(!obligation.rejected_coupling.trim().is_empty());
        assert!(!obligation.source.is_empty());
        for source in &obligation.source {
            assert!(
                !source.path.trim().is_empty(),
                "{} has an empty source path",
                obligation.id
            );
            assert_eq!(
                source.blob_sha.len(),
                40,
                "{} has an unfrozen blob SHA",
                obligation.id
            );
            assert_eq!(
                source.sha256.len(),
                64,
                "{} has an invalid source digest",
                obligation.id
            );
        }
        match (
            obligation.changed_by_design,
            &obligation.approving_spec_path,
            &obligation.approving_spec_section,
        ) {
            (true, Some(path), Some(section))
                if !path.trim().is_empty() && !section.trim().is_empty() =>
            {
                let spec = String::from_utf8(read(path)).expect("approving spec must be UTF-8");
                assert!(
                    spec.contains(section),
                    "{} approving section {section:?} is absent from {path}",
                    obligation.id
                );
            }
            (false, None, None) => {}
            _ => panic!(
                "{} must pair changed_by_design with an approving spec source and section",
                obligation.id
            ),
        }

        let (status, reasons) = observe(obligation);
        assert_eq!(
            status, obligation.expected,
            "{} status; reasons={reasons:?}",
            obligation.id
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason == &obligation.reason_code),
            "{} missing reason {}; actual={reasons:?}",
            obligation.id,
            obligation.reason_code
        );
    }
}
