mod common;
use policy_contracts::{ExecutionStatus, SensorStatus, TrustClass};
use policy_core::canonical_sha256;
use project_verification::run::{normalize_collection, prepare};
const REPORT: &[u8] =
    include_bytes!("../../policy-report-adapters/tests/fixtures/project-reports/vitest.json");

#[test]
fn normalization_preserves_original_artifact_and_never_promotes_trust() {
    let root = tempfile::tempdir().unwrap();
    let request = common::request(root.path());
    let config = prepare(&request).unwrap().configurations.remove(0);
    std::fs::create_dir(root.path().join("reports")).unwrap();
    std::fs::write(root.path().join("reports/unit.json"), REPORT).unwrap();
    let collection = common::collection(&request, &config, REPORT);
    let normalized = normalize_collection(&request, config.clone(), collection.clone()).unwrap();
    assert_eq!(normalized.evidence.status, SensorStatus::Passed);
    assert_eq!(normalized.evidence.trust, TrustClass::LocalUntrusted);
    assert_eq!(
        normalized.collection.evidence.trust,
        TrustClass::LocalUntrusted
    );
    assert_eq!(normalized.evidence.discovered, Some(1));
    assert_eq!(
        normalized.result.original_report.unwrap().digest,
        project_verification::bytes_digest(REPORT)
    );
    assert_eq!(
        normalize_collection(&request, config, collection)
            .unwrap()
            .evidence
            .result_digest,
        normalized.evidence.result_digest
    );
}

#[test]
fn report_mismatch_missing_and_old_run_are_non_success() {
    for replacement in [None, Some(b"{}".as_slice()), Some(REPORT)] {
        let root = tempfile::tempdir().unwrap();
        let request = common::request(root.path());
        let config = prepare(&request).unwrap().configurations.remove(0);
        let mut collection = common::collection(&request, &config, REPORT);
        if let Some(bytes) = replacement {
            std::fs::create_dir(root.path().join("reports")).unwrap();
            std::fs::write(root.path().join("reports/unit.json"), bytes).unwrap();
        }
        if replacement == Some(REPORT) {
            collection.record.execution.started_at =
                policy_contracts::UtcTimestamp::parse("2025-09-02T00:00:00Z").unwrap();
            collection.record.execution.completed_at =
                policy_contracts::UtcTimestamp::parse("2025-09-02T00:00:01Z").unwrap();
            collection.evidence.completed_at =
                policy_contracts::UtcTimestamp::parse("2025-09-02T00:00:02Z").unwrap();
            common::reseal(&mut collection);
        }
        assert_ne!(
            normalize_collection(&request, config, collection)
                .unwrap()
                .evidence
                .status,
            SensorStatus::Passed
        );
    }
}

#[test]
fn transport_status_cannot_hide_failed_process_or_version_probe() {
    let root = tempfile::tempdir().unwrap();
    let request = common::request(root.path());
    let config = prepare(&request).unwrap().configurations.remove(0);
    std::fs::create_dir(root.path().join("reports")).unwrap();
    std::fs::write(root.path().join("reports/unit.json"), REPORT).unwrap();
    for variant in 0..5 {
        let mut c = common::collection(&request, &config, REPORT);
        match variant {
            0 => c.record.execution.exit_code = Some(1),
            1 => {
                c.evidence.status = ExecutionStatus::TimedOut;
                c.record.execution.status = ExecutionStatus::TimedOut;
            }
            2 => c.record.version_probe.stdout = b"wrong-version".to_vec(),
            3 => c.record.artifact_errors.push("missing".into()),
            _ => c.record.execution.error = Some("spawn failure".into()),
        }
        common::reseal(&mut c);
        let normalized = normalize_collection(&request, config.clone(), c).unwrap();
        assert_ne!(normalized.evidence.status, SensorStatus::Passed);
        if variant == 1 {
            assert_eq!(normalized.evidence.status, SensorStatus::TimedOut);
        }
    }
    let mut c = common::collection(&request, &config, REPORT);
    c.record.command.head_sha = policy_contracts::CommitSha::parse("b".repeat(40)).unwrap();
    common::reseal(&mut c);
    assert!(normalize_collection(&request, config, c).is_err());
}

#[test]
fn consuming_project_simulation_rejects_previous_head_and_binds_corrected_collection() {
    let root = tempfile::tempdir().unwrap();
    let previous_request = common::request(root.path());
    let previous_config = prepare(&previous_request).unwrap().configurations.remove(0);
    std::fs::create_dir(root.path().join("reports")).unwrap();
    std::fs::write(root.path().join("reports/unit.json"), REPORT).unwrap();
    let previous_collection = common::collection(&previous_request, &previous_config, REPORT);

    std::fs::remove_file(root.path().join("reports/unit.json")).unwrap();
    let corrected_source = b"{\"revision\":2}";
    std::fs::write(root.path().join("config.json"), corrected_source).unwrap();
    let mut corrected_request = previous_request.clone();
    corrected_request.head_sha = policy_contracts::CommitSha::parse("b".repeat(40)).unwrap();
    corrected_request.source_files[0].digest = project_verification::bytes_digest(corrected_source);
    corrected_request.expected_source_digest =
        canonical_sha256(&corrected_request.source_files).unwrap();
    let corrected_config = prepare(&corrected_request)
        .unwrap()
        .configurations
        .remove(0);
    std::fs::write(root.path().join("reports/unit.json"), REPORT).unwrap();

    assert!(normalize_collection(
        &corrected_request,
        corrected_config.clone(),
        previous_collection,
    )
    .is_err());

    let corrected_collection = common::collection(&corrected_request, &corrected_config, REPORT);
    let normalized =
        normalize_collection(&corrected_request, corrected_config, corrected_collection).unwrap();
    assert_eq!(normalized.evidence.status, SensorStatus::Passed);
    assert_eq!(normalized.evidence.head_sha, corrected_request.head_sha);
    assert_eq!(
        normalized.evidence.source_digest.as_ref(),
        Some(&corrected_request.expected_source_digest)
    );
    assert_eq!(
        normalized.collection.record.command.head_sha,
        corrected_request.head_sha
    );
    assert_eq!(normalized.evidence.trust, TrustClass::LocalUntrusted);
}
