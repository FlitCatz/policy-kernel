use policy_contracts::{
    Reason, Risk, SensorCondition, SensorEvidence, SensorId, SensorProfile, SensorStatus,
    TestAdequacyEvidence, TestAdequacyPolicy,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorEvaluationStatus {
    Passed,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorEvaluation {
    pub status: SensorEvaluationStatus,
    pub reasons: Vec<Reason>,
}

impl SensorEvaluation {
    pub fn has_reason(&self, code: &str) -> bool {
        self.reasons.iter().any(|reason| reason.code == code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SensorResolutionError {
    #[error("changed path contains a parent-directory segment: {0}")]
    ParentTraversal(String),
}

pub fn resolve_required_sensors(
    profile: &SensorProfile,
    changed_paths: &[&str],
    risk: Risk,
) -> Result<Vec<SensorId>, SensorResolutionError> {
    let paths = changed_paths
        .iter()
        .map(|path| normalize_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut required = Vec::new();
    let mut seen = BTreeSet::new();

    for sensor in &profile.sensors {
        if (sensor.required
            || sensor
                .required_when
                .iter()
                .any(|condition| condition_matches(condition, &paths, risk)))
            && seen.insert(sensor.id.clone())
        {
            required.push(sensor.id.clone());
        }
    }

    Ok(required)
}

pub fn evaluate_sensor_evidence(
    profile: &SensorProfile,
    required_sensor_ids: &[SensorId],
    evidence: &[SensorEvidence],
    adequacy: Option<&TestAdequacyEvidence>,
) -> SensorEvaluation {
    let definitions = profile
        .sensors
        .iter()
        .map(|sensor| (&sensor.id, sensor))
        .collect::<BTreeMap<_, _>>();
    let mut by_id: BTreeMap<&SensorId, Vec<&SensorEvidence>> = BTreeMap::new();
    for item in evidence {
        by_id.entry(&item.sensor_id).or_default().push(item);
    }

    let mut reasons = Vec::new();
    for sensor_id in required_sensor_ids {
        let Some(items) = by_id.get(sensor_id) else {
            insert_reason(
                &mut reasons,
                "PK-SENSOR-MISSING",
                "required sensor evidence is missing",
            );
            continue;
        };
        if items.len() != 1 {
            insert_reason(
                &mut reasons,
                "PK-SENSOR-DUPLICATE",
                "required sensor evidence must occur exactly once",
            );
            continue;
        }

        let item = items[0];
        match item.status {
            SensorStatus::Passed => {}
            SensorStatus::Failed => insert_reason(
                &mut reasons,
                "PK-SENSOR-FAILED",
                "a required sensor reported an executed failure",
            ),
            SensorStatus::TimedOut => insert_reason(
                &mut reasons,
                "PK-SENSOR-TIMEOUT",
                "a required sensor timed out",
            ),
            SensorStatus::Skipped => insert_reason(
                &mut reasons,
                "PK-SENSOR-SKIPPED",
                "a required sensor was skipped",
            ),
            SensorStatus::Unparsable => insert_reason(
                &mut reasons,
                "PK-SENSOR-UNPARSABLE",
                "a required sensor result could not be parsed",
            ),
            SensorStatus::NotApplicable => {
                let condition_matches_definition = definitions
                    .get(sensor_id)
                    .zip(item.not_applicable_condition_id.as_deref())
                    .is_some_and(|(definition, condition_id)| {
                        definition
                            .required_when
                            .iter()
                            .any(|condition| condition.id == condition_id)
                    });
                let reason_recorded = item
                    .not_applicable_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty());
                if !condition_matches_definition || !reason_recorded {
                    insert_reason(
                        &mut reasons,
                        "PK-SENSOR-NOT-APPLICABLE-INVALID",
                        "not-applicable evidence needs a declared condition and reason",
                    );
                }
            }
        }
    }

    evaluate_adequacy(&profile.adequacy, adequacy, &mut reasons);

    let status = if reasons.iter().any(|reason| {
        matches!(
            reason.code.as_str(),
            "PK-SENSOR-FAILED"
                | "PK-ADEQ-NO-TESTS"
                | "PK-ADEQ-COUNT-MISMATCH"
                | "PK-ADEQ-CRITERION-UNMAPPED"
                | "PK-ADEQ-BEHAVIOR-UNMAPPED"
                | "PK-ADEQ-NEGATIVE-CASE-MISSING"
                | "PK-ADEQ-UNEXECUTED-MAPPING"
        )
    }) {
        SensorEvaluationStatus::Blocked
    } else if reasons.is_empty() {
        SensorEvaluationStatus::Passed
    } else {
        SensorEvaluationStatus::Inconclusive
    };

    SensorEvaluation { status, reasons }
}

fn normalize_path(path: &str) -> Result<String, SensorResolutionError> {
    let normalized = path.replace('\\', "/");
    if normalized.split('/').any(|segment| segment == "..") {
        return Err(SensorResolutionError::ParentTraversal(path.to_owned()));
    }
    Ok(normalized
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("/"))
}

fn condition_matches(condition: &SensorCondition, paths: &[String], risk: Risk) -> bool {
    let risk_matches = condition.risks.is_empty() || condition.risks.contains(&risk);
    let path_matches = condition.path_globs.is_empty()
        || condition.path_globs.iter().any(|pattern| {
            paths
                .iter()
                .any(|path| glob_matches(pattern.as_bytes(), path.as_bytes()))
        });
    risk_matches && path_matches
}

fn glob_matches(pattern: &[u8], value: &[u8]) -> bool {
    fn matches_from(
        pattern: &[u8],
        value: &[u8],
        pattern_index: usize,
        value_index: usize,
        memo: &mut BTreeMap<(usize, usize), bool>,
    ) -> bool {
        if let Some(result) = memo.get(&(pattern_index, value_index)) {
            return *result;
        }
        let result = if pattern_index == pattern.len() {
            value_index == value.len()
        } else if pattern[pattern_index] == b'*' && pattern.get(pattern_index + 1) == Some(&b'*') {
            let next = pattern_index + 2;
            matches_from(pattern, value, next, value_index, memo)
                || (value_index < value.len()
                    && matches_from(pattern, value, pattern_index, value_index + 1, memo))
        } else if pattern[pattern_index] == b'*' {
            matches_from(pattern, value, pattern_index + 1, value_index, memo)
                || (value_index < value.len()
                    && value[value_index] != b'/'
                    && matches_from(pattern, value, pattern_index, value_index + 1, memo))
        } else if value_index < value.len()
            && (pattern[pattern_index] == b'?' && value[value_index] != b'/'
                || pattern[pattern_index] == value[value_index])
        {
            matches_from(pattern, value, pattern_index + 1, value_index + 1, memo)
        } else {
            false
        };
        memo.insert((pattern_index, value_index), result);
        result
    }

    matches_from(pattern, value, 0, 0, &mut BTreeMap::new())
}

fn evaluate_adequacy(
    policy: &TestAdequacyPolicy,
    evidence: Option<&TestAdequacyEvidence>,
    reasons: &mut Vec<Reason>,
) {
    let Some(evidence) = evidence else {
        insert_reason(
            reasons,
            "PK-ADEQ-MISSING",
            "test adequacy evidence is missing",
        );
        return;
    };
    if evidence.discovered == 0 {
        insert_reason(reasons, "PK-ADEQ-NO-TESTS", "no tests were discovered");
    }
    if evidence.discovered != evidence.executed
        || evidence.executed != evidence.passed
        || evidence.executed != evidence.executed_test_ids.len() as u64
    {
        insert_reason(
            reasons,
            "PK-ADEQ-COUNT-MISMATCH",
            "discovered, executed, passed, and recorded test counts must agree",
        );
    }

    validate_mappings(
        &policy.acceptance_criterion_ids,
        &evidence.acceptance_criteria,
        "PK-ADEQ-CRITERION-UNMAPPED",
        reasons,
    );
    validate_mappings(
        &policy.changed_behavior_ids,
        &evidence.changed_behaviors,
        "PK-ADEQ-BEHAVIOR-UNMAPPED",
        reasons,
    );

    let negatives = evidence
        .negative_case_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if policy
        .negative_case_ids
        .iter()
        .any(|id| !negatives.contains(id.as_str()))
    {
        insert_reason(
            reasons,
            "PK-ADEQ-NEGATIVE-CASE-MISSING",
            "a required negative case is missing",
        );
    }

    let executed = evidence
        .executed_test_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if evidence
        .acceptance_criteria
        .iter()
        .chain(&evidence.changed_behaviors)
        .flat_map(|mapping| &mapping.test_ids)
        .any(|test_id| !executed.contains(test_id.as_str()))
    {
        insert_reason(
            reasons,
            "PK-ADEQ-UNEXECUTED-MAPPING",
            "adequacy mappings may reference only executed tests",
        );
    }
}

fn validate_mappings(
    required_ids: &[String],
    mappings: &[policy_contracts::TestMapping],
    reason_code: &str,
    reasons: &mut Vec<Reason>,
) {
    if required_ids.iter().any(|required_id| {
        !mappings
            .iter()
            .any(|mapping| mapping.requirement_id == *required_id && !mapping.test_ids.is_empty())
    }) {
        insert_reason(reasons, reason_code, "a required test mapping is missing");
    }
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

#[cfg(test)]
mod tests {
    use super::glob_matches;

    #[test]
    fn glob_star_does_not_cross_a_directory_boundary() {
        assert!(glob_matches(b"src/*.rs", b"src/api.rs"));
        assert!(!glob_matches(b"src/*.rs", b"src/api/mod.rs"));
        assert!(glob_matches(b"src/**", b"src/api/mod.rs"));
    }
}
