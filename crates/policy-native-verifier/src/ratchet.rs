use policy_core::CanonicalError;
use policy_native_contracts::*;
use std::collections::BTreeMap;

fn result(mut violations: Vec<RatchetViolationV1>) -> RatchetResultV1 {
    violations.sort();
    violations.dedup();
    RatchetResultV1 {
        schema_version: VersionV1::default(),
        status: if violations.is_empty() {
            RatchetStatus::Pass
        } else {
            RatchetStatus::Fail
        },
        violations,
        rule_digest: None,
        observation_digest: None,
    }
}

fn counts(entries: &[RatchetEntryV1]) -> BTreeMap<&RatchetKeyV1, u64> {
    entries
        .iter()
        .map(|entry| (&entry.key, entry.count))
        .collect()
}

/// Compare only typed data. No I/O, language parser, process or clock is used.
/// Invalid input produces all structural violations and skips the comparison;
/// no ambiguous duplicate is allowed to select a permissive count.
pub fn evaluate_ratchet(
    rule: &RatchetRuleV1,
    observation: &RatchetObservationV1,
) -> Result<RatchetResultV1, CanonicalError> {
    let mut violations = rule.validate();
    violations.extend(observation.validate());
    if !violations.is_empty() {
        return Ok(result(violations));
    }

    let base = counts(&rule.approved_baseline);
    let head = counts(&rule.head_baseline);
    for (key, count) in &head {
        if *count > base.get(key).copied().unwrap_or(0) {
            violations.push(RatchetViolationV1 {
                input: RatchetInput::HeadBaseline,
                key: Some((*key).clone()),
                code: RatchetReasonCode::AllowanceIncreased,
            });
        }
    }
    // Group without the context hash only to label replacements. Counts are
    // always compared by the complete key, never by this diagnostic grouping.
    let contexts: std::collections::BTreeSet<_> = base
        .keys()
        .map(|key| (&key.tool, &key.rule, &key.path, &key.symbol))
        .collect();
    for entry in &observation.entries {
        let key = &entry.key;
        let approved = base.get(key).copied().unwrap_or(0);
        let allowance = approved.min(head.get(key).copied().unwrap_or(0));
        if entry.count > allowance {
            let code = if approved > 0 {
                RatchetReasonCode::DebtIncreased
            } else if contexts.contains(&(&key.tool, &key.rule, &key.path, &key.symbol)) {
                RatchetReasonCode::ContextReplacement
            } else {
                RatchetReasonCode::NewDebt
            };
            violations.push(RatchetViolationV1 {
                input: RatchetInput::Observation,
                key: Some(key.clone()),
                code,
            });
        }
    }
    let mut evaluated = result(violations);
    evaluated.rule_digest = Some(rule.digest()?);
    evaluated.observation_digest = Some(observation.digest()?);
    Ok(evaluated)
}

/// Strict JSON boundary, including duplicate object fields. A parse error is
/// represented by a stable reason rather than a dependency-specific message.
/// Each document's parse error is reported; semantic validation still runs on
/// the other document when it can be decoded.
pub fn evaluate_ratchet_json(
    rule_json: &[u8],
    observation_json: &[u8],
) -> Result<RatchetResultV1, CanonicalError> {
    let rule = serde_json::from_slice::<RatchetRuleV1>(rule_json);
    let observation = serde_json::from_slice::<RatchetObservationV1>(observation_json);
    if let (Ok(rule), Ok(observation)) = (&rule, &observation) {
        return evaluate_ratchet(rule, observation);
    }
    let mut violations = Vec::new();
    match rule {
        Ok(rule) => violations.extend(rule.validate()),
        Err(_) => violations.push(RatchetViolationV1 {
            input: RatchetInput::Rule,
            key: None,
            code: RatchetReasonCode::MalformedInput,
        }),
    }
    match observation {
        Ok(observation) => violations.extend(observation.validate()),
        Err(_) => violations.push(RatchetViolationV1 {
            input: RatchetInput::Observation,
            key: None,
            code: RatchetReasonCode::MalformedInput,
        }),
    }
    Ok(result(violations))
}
