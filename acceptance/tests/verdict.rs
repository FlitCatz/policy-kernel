use policy_acceptance::negative_verified;
use std::collections::BTreeMap;

#[test]
fn contract_correlated_failures_are_explicit_and_unrelated_failures_still_reject() {
    let expected = ["shared:contract", "shared:e2e", "backend:integration"];
    let mut baseline: BTreeMap<String, bool> =
        expected.iter().map(|id| ((*id).into(), true)).collect();
    baseline.insert("frontend:dependencies".into(), true);
    let mut negative = baseline.clone();
    for id in expected {
        negative.insert(id.into(), false);
    }
    assert!(policy_acceptance::negative_verified_set(
        &baseline, &negative, &expected
    ));
    negative.insert("frontend:dependencies".into(), false);
    assert!(!policy_acceptance::negative_verified_set(
        &baseline, &negative, &expected
    ));
    negative.insert("frontend:dependencies".into(), true);
    negative.insert("shared:e2e".into(), true);
    assert!(!policy_acceptance::negative_verified_set(
        &baseline, &negative, &expected
    ));
}

fn statuses(unit: bool, build: bool) -> BTreeMap<String, bool> {
    BTreeMap::from([
        ("frontend:unit".into(), unit),
        ("frontend:build".into(), build),
    ])
}

#[test]
fn requires_successful_baseline_and_targeted_failure() {
    assert!(negative_verified(
        &statuses(true, true),
        &statuses(false, true),
        "frontend:unit"
    ));
}

#[test]
fn unrelated_failure_missing_result_and_broken_baseline_do_not_prove_rejection() {
    assert!(!negative_verified(
        &statuses(true, true),
        &statuses(false, false),
        "frontend:unit"
    ));
    assert!(!negative_verified(
        &statuses(true, true),
        &statuses(true, false),
        "frontend:unit"
    ));
    assert!(!negative_verified(
        &statuses(false, true),
        &statuses(false, true),
        "frontend:unit"
    ));
    assert!(!negative_verified(
        &BTreeMap::new(),
        &statuses(false, true),
        "frontend:unit"
    ));
    assert!(!negative_verified(
        &statuses(true, true),
        &BTreeMap::new(),
        "frontend:unit"
    ));
}
