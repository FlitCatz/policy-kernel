use policy_native_contracts::*;
use policy_native_verifier::{evaluate_ratchet, evaluate_ratchet_json};
use serde_json::{json, Value};

fn entry(context: char, count: u64) -> RatchetEntryV1 {
    serde_json::from_value(json!({
        "key": {"tool":"analyzer", "rule":"R1", "path":"src/lib.rs",
            "symbol":"compute", "context_hash":context.to_string().repeat(64)},
        "count": count
    }))
    .unwrap()
}

fn rule(base: Vec<RatchetEntryV1>, head: Vec<RatchetEntryV1>) -> RatchetRuleV1 {
    RatchetRuleV1 {
        schema_version: VersionV1::default(),
        approved_baseline: base,
        head_baseline: head,
    }
}

fn observed(entries: Vec<RatchetEntryV1>) -> RatchetObservationV1 {
    RatchetObservationV1 {
        schema_version: VersionV1::default(),
        entries,
    }
}

fn codes(result: &RatchetResultV1) -> Vec<RatchetReasonCode> {
    result.violations.iter().map(|v| v.code).collect()
}

// @lat: [[native-contracts#Native contracts#Debt ratchet conformance]]
#[test]
fn ratchet_decrease_equal_increase_new_and_removed_entries() {
    for (count, status, reasons) in [
        (1, RatchetStatus::Pass, vec![]),
        (2, RatchetStatus::Pass, vec![]),
        (
            3,
            RatchetStatus::Fail,
            vec![RatchetReasonCode::DebtIncreased],
        ),
    ] {
        let result = evaluate_ratchet(
            &rule(vec![entry('a', 2)], vec![entry('a', 2)]),
            &observed(vec![entry('a', count)]),
        )
        .unwrap();
        assert_eq!(result.status, status);
        assert_eq!(codes(&result), reasons);
    }
    let new = evaluate_ratchet(&rule(vec![], vec![]), &observed(vec![entry('a', 1)])).unwrap();
    assert_eq!(codes(&new), vec![RatchetReasonCode::NewDebt]);
    let removed = evaluate_ratchet(&rule(vec![entry('a', 2)], vec![]), &observed(vec![])).unwrap();
    assert_eq!(removed.status, RatchetStatus::Pass);
    let empty = evaluate_ratchet(&rule(vec![], vec![]), &observed(vec![])).unwrap();
    assert_eq!(empty.status, RatchetStatus::Pass);
}

#[test]
fn head_allowance_cannot_grow_even_when_unused_and_lower_allowance_is_enforced() {
    let growth = evaluate_ratchet(
        &rule(vec![entry('a', 1)], vec![entry('a', 2), entry('b', 1)]),
        &observed(vec![]),
    )
    .unwrap();
    assert_eq!(
        codes(&growth),
        vec![
            RatchetReasonCode::AllowanceIncreased,
            RatchetReasonCode::AllowanceIncreased
        ]
    );
    let lower = evaluate_ratchet(
        &rule(vec![entry('a', 3)], vec![entry('a', 1)]),
        &observed(vec![entry('a', 2)]),
    )
    .unwrap();
    assert_eq!(codes(&lower), vec![RatchetReasonCode::DebtIncreased]);
    let removed_allowance = evaluate_ratchet(
        &rule(vec![entry('a', 3)], vec![]),
        &observed(vec![entry('a', 1)]),
    )
    .unwrap();
    assert_eq!(
        codes(&removed_allowance),
        vec![RatchetReasonCode::DebtIncreased]
    );
    let grown = evaluate_ratchet(
        &rule(vec![entry('a', 1)], vec![entry('a', 3)]),
        &observed(vec![entry('a', 2)]),
    )
    .unwrap();
    assert_eq!(
        codes(&grown),
        vec![
            RatchetReasonCode::AllowanceIncreased,
            RatchetReasonCode::DebtIncreased
        ]
    );
}

#[test]
fn context_replacement_cannot_spend_another_contexts_allowance() {
    let result = evaluate_ratchet(
        &rule(vec![entry('a', 2)], vec![entry('b', 2)]),
        &observed(vec![entry('b', 2)]),
    )
    .unwrap();
    assert_eq!(
        codes(&result),
        vec![
            RatchetReasonCode::AllowanceIncreased,
            RatchetReasonCode::ContextReplacement
        ]
    );
    let observations_only = evaluate_ratchet(
        &rule(vec![entry('a', 2)], vec![entry('a', 2)]),
        &observed(vec![entry('b', 2)]),
    )
    .unwrap();
    assert_eq!(
        codes(&observations_only),
        vec![RatchetReasonCode::ContextReplacement]
    );
}

#[test]
fn duplicate_keys_in_every_input_fail_without_picking_an_allowance() {
    let result = evaluate_ratchet(
        &rule(
            vec![entry('a', 1), entry('a', 2)],
            vec![entry('b', 1), entry('b', 1)],
        ),
        &observed(vec![entry('c', 1), entry('c', 2)]),
    )
    .unwrap();
    assert_eq!(result.status, RatchetStatus::Fail);
    assert_eq!(codes(&result), vec![RatchetReasonCode::DuplicateKey; 3]);
    assert_eq!(
        result
            .violations
            .iter()
            .map(|v| v.input)
            .collect::<Vec<_>>(),
        vec![
            RatchetInput::ApprovedBaseline,
            RatchetInput::HeadBaseline,
            RatchetInput::Observation
        ]
    );
    assert!(result.rule_digest.is_none());
    assert!(result.observation_digest.is_none());
    let decoded = evaluate_ratchet_json(
        &serde_json::to_vec(&rule(
            vec![entry('a', 1), entry('a', 2)],
            vec![entry('b', 1), entry('b', 1)],
        ))
        .unwrap(),
        &serde_json::to_vec(&observed(vec![entry('c', 1), entry('c', 2)])).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded, result);
}

#[test]
fn malformed_json_negative_counts_unknown_fields_and_versions_fail_closed() {
    let valid_rule = serde_json::to_vec(&rule(vec![], vec![])).unwrap();
    let valid_observation = serde_json::to_vec(&observed(vec![])).unwrap();
    let both = evaluate_ratchet_json(b"{", b"[]").unwrap();
    assert_eq!(both.status, RatchetStatus::Fail);
    assert_eq!(codes(&both), vec![RatchetReasonCode::MalformedInput; 2]);
    let base_observation = serde_json::to_value(observed(vec![entry('a', 1)])).unwrap();
    for (pointer, value) in [
        ("/entries/0/count", json!(-1)),
        ("/entries/0/count", json!(1.5)),
        ("/entries/0/count", json!("1")),
        ("/entries/0/key/context_hash", json!("bad")),
        ("/entries/0/key/symbol", json!(" ")),
        ("/schema_version", json!(2)),
    ] {
        let mut invalid = base_observation.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let result =
            evaluate_ratchet_json(&valid_rule, &serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert_eq!(
            codes(&result),
            vec![RatchetReasonCode::MalformedInput],
            "{pointer}"
        );
    }
    let unknown = br#"{"schema_version":1,"entries":[],"approved":true}"#;
    assert_eq!(
        codes(&evaluate_ratchet_json(&valid_rule, unknown).unwrap()),
        vec![RatchetReasonCode::MalformedInput]
    );
    let duplicate_field = br#"{"schema_version":1,"schema_version":1,"entries":[]}"#;
    assert_eq!(
        codes(&evaluate_ratchet_json(&valid_rule, duplicate_field).unwrap()),
        vec![RatchetReasonCode::MalformedInput]
    );
    assert_eq!(
        codes(&evaluate_ratchet_json(b"null", &valid_observation).unwrap()),
        vec![RatchetReasonCode::MalformedInput]
    );
    let invalid_entries = serde_json::to_vec(&observed(vec![entry('a', 0)])).unwrap();
    assert_eq!(
        codes(&evaluate_ratchet_json(b"null", &invalid_entries).unwrap()),
        vec![
            RatchetReasonCode::MalformedInput,
            RatchetReasonCode::InvalidCount
        ]
    );
}

#[test]
fn invalid_entries_are_all_reported_for_direct_rust_callers() {
    let mut invalid = entry('a', 0);
    invalid.key.path = "../source.rs".into();
    let result = evaluate_ratchet(
        &rule(vec![invalid], vec![entry('b', 0)]),
        &observed(vec![entry('c', 0)]),
    )
    .unwrap();
    assert_eq!(result.status, RatchetStatus::Fail);
    assert_eq!(
        codes(&result),
        vec![
            RatchetReasonCode::InvalidCount,
            RatchetReasonCode::InvalidPath,
            RatchetReasonCode::InvalidCount,
            RatchetReasonCode::InvalidCount
        ]
    );
    for path in [
        "/root.rs",
        "C:/source.rs",
        "a\\b",
        "a//b",
        "a/./b",
        "",
        "a\0b",
    ] {
        let mut invalid = entry('a', 1);
        invalid.key.path = path.into();
        let result = evaluate_ratchet(&rule(vec![invalid], vec![]), &observed(vec![])).unwrap();
        assert_eq!(
            codes(&result),
            vec![RatchetReasonCode::InvalidPath],
            "{path:?}"
        );
    }
}

#[test]
fn tuple_identity_prevents_cross_tool_rule_path_symbol_and_context_allowances() {
    for field in ["tool", "rule", "path", "symbol", "context_hash"] {
        let mut changed = serde_json::to_value(entry('a', 1)).unwrap();
        changed["key"][field] = if field == "context_hash" {
            json!("b".repeat(64))
        } else {
            json!("other")
        };
        let changed = serde_json::from_value(changed).unwrap();
        let result = evaluate_ratchet(
            &rule(vec![entry('a', 1)], vec![entry('a', 1)]),
            &observed(vec![changed]),
        )
        .unwrap();
        assert_eq!(result.status, RatchetStatus::Fail, "{field}");
    }
}

#[test]
fn maximum_counts_do_not_overflow_and_removed_entries_cannot_offset_growth() {
    let baseline = vec![entry('a', u64::MAX), entry('b', u64::MAX)];
    let equal = evaluate_ratchet(
        &rule(baseline.clone(), baseline.clone()),
        &observed(baseline),
    )
    .unwrap();
    assert_eq!(equal.status, RatchetStatus::Pass);
    let mixed = evaluate_ratchet(
        &rule(vec![entry('a', 100), entry('b', 1)], vec![entry('b', 1)]),
        &observed(vec![entry('b', 2)]),
    )
    .unwrap();
    assert_eq!(codes(&mixed), vec![RatchetReasonCode::DebtIncreased]);
}

#[test]
fn ordering_determinism_and_domain_separated_digests_preserve_wire_array_order() {
    let first = rule(vec![entry('a', 1)], vec![entry('b', 2), entry('c', 2)]);
    let observation = observed(vec![entry('c', 3), entry('b', 3)]);
    let a = evaluate_ratchet(&first, &observation).unwrap();
    assert_eq!(a.violations.len(), 4);
    let mut second = first.clone();
    second.head_baseline.reverse();
    let mut reversed = observation.clone();
    reversed.entries.reverse();
    let b = evaluate_ratchet(&second, &reversed).unwrap();
    assert_eq!(a.violations, b.violations);
    assert_eq!(a.status, b.status);
    assert_ne!(a.rule_digest, b.rule_digest);
    assert_ne!(a.observation_digest, b.observation_digest);
    let encoded: Value = serde_json::to_value(&first).unwrap();
    let expected = policy_core::canonical_sha256(
        &json!({"schema":"policy_native_v1", "role":"ratchet_rule", "value":encoded}),
    )
    .unwrap();
    assert_eq!(a.rule_digest.as_ref().unwrap().sha256, expected);
    assert_eq!(
        a.rule_digest.as_ref().unwrap().role,
        RatchetDigestRole::RatchetRule
    );
    assert_eq!(
        a.observation_digest.as_ref().unwrap().role,
        RatchetDigestRole::RatchetObservation
    );
    let roundtrip: RatchetResultV1 =
        serde_json::from_slice(&serde_json::to_vec(&a).unwrap()).unwrap();
    assert_eq!(a, roundtrip);
}
