use policy_contracts::*;

#[test]
fn unknown_schema_version_is_rejected() {
    let error = serde_json::from_str::<PreMergeEvaluationInput>(include_str!(
        "../../../tests/fixtures/contracts/unknown-version.json"
    ))
    .unwrap_err();
    assert!(error.to_string().contains("unsupported schema version"));
}

#[test]
fn short_and_non_hex_commit_shas_are_rejected() {
    assert!(CommitSha::parse("deadbeef").is_err());
    assert!(CommitSha::parse(&"z".repeat(40)).is_err());
    assert!(
        serde_json::from_str::<PreMergeEvaluationInput>(include_str!(
            "../../../tests/fixtures/contracts/invalid-sha.json"
        ))
        .is_err()
    );
}

#[test]
fn advisory_decision_cannot_be_post_merge_authorization() {
    let decision: PreMergeDecision = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}

#[test]
fn advisory_decision_cannot_deserialize_as_post_merge_authorization() {
    assert!(
        serde_json::from_str::<AuthoritativePreMergeDecision>(include_str!(
            "../../../tests/fixtures/contracts/advisory-decision.json"
        ))
        .is_err()
    );
}

#[test]
fn authoritative_ready_decision_can_be_post_merge_authorization() {
    let mut decision: PreMergeDecision = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    decision.mode = EvaluationMode::Authoritative;
    decision.advisory = false;
    decision.status = PreMergeStatus::ReadyToMerge;
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_ok());
}

#[test]
fn unknown_wire_fields_are_rejected() {
    let json = include_str!("../../../tests/fixtures/contracts/advisory-decision.json")
        .replace("\"advisory\":true", "\"advisory\":true,\"unexpected\":1");
    assert!(serde_json::from_str::<PreMergeDecision>(&json).is_err());
}

#[test]
fn valid_pre_merge_fixture_round_trips_byte_for_byte() {
    let json = include_str!("../../../tests/fixtures/contracts/valid-pre-merge.json").trim();
    let input: PreMergeEvaluationInput = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&input).unwrap(), json);
}

#[test]
fn pull_request_numbers_must_be_positive() {
    assert!(PullRequestNumber::new(0).is_err());
    assert_eq!(PullRequestNumber::new(42).unwrap().get(), 42);
}
