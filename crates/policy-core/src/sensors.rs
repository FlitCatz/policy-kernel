use globset::{GlobBuilder, GlobMatcher};
use policy_contracts::{
    Reason, Risk, SensorCondition, SensorEvidence, SensorId, SensorProfile,
    SensorResultRequirement, SensorStatus, TestAdequacyEvidence, TestAdequacyPolicy,
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
    #[error("{0}")]
    InvalidVersion(&'static str),
    #[error("changed path contains a parent-directory segment: {0}")]
    ParentTraversal(String),
    #[error("sensor profile contains duplicate sensor ID: {0}")]
    DuplicateSensor(String),
    #[error("sensor profile contains an invalid or duplicate condition ID: {0}")]
    InvalidCondition(String),
    #[error("sensor condition contains an invalid glob: {0}")]
    InvalidGlob(String),
}

pub fn resolve_required_sensors(
    profile: &SensorProfile,
    changed_paths: &[&str],
    risk: Risk,
) -> Result<Vec<SensorId>, SensorResolutionError> {
    validate_profile(profile)?;
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
    evaluate_sensor_evidence_inner(profile, required_sensor_ids, evidence, adequacy, None)
}

pub fn evaluate_sensor_evidence_for_change(
    profile: &SensorProfile,
    changed_paths: &[&str],
    risk: Risk,
    evidence: &[SensorEvidence],
    adequacy: Option<&TestAdequacyEvidence>,
) -> Result<SensorEvaluation, SensorResolutionError> {
    let required = resolve_required_sensors(profile, changed_paths, risk)?;
    let paths = changed_paths
        .iter()
        .map(|path| normalize_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(evaluate_sensor_evidence_inner(
        profile,
        &required,
        evidence,
        adequacy,
        Some((&paths, risk)),
    ))
}

fn evaluate_sensor_evidence_inner(
    profile: &SensorProfile,
    required_sensor_ids: &[SensorId],
    evidence: &[SensorEvidence],
    adequacy: Option<&TestAdequacyEvidence>,
    context: Option<(&[String], Risk)>,
) -> SensorEvaluation {
    if let Err(error) = validate_profile(profile) {
        return SensorEvaluation {
            status: SensorEvaluationStatus::Inconclusive,
            reasons: vec![Reason {
                code: "PK-SENSOR-PROFILE-INVALID".to_owned(),
                message: error.to_string(),
            }],
        };
    }
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
        let definition = definitions.get(sensor_id).copied();
        if definition.is_none() {
            insert_reason(
                &mut reasons,
                "PK-SENSOR-UNKNOWN",
                "required sensor is absent from the sensor profile",
            );
            continue;
        }
        match item.status {
            SensorStatus::Passed => {
                if definition
                    .unwrap()
                    .result_binding
                    .as_ref()
                    .is_some_and(|binding| {
                        item.configuration_digest.as_ref() != Some(&binding.configuration_digest)
                            || item.parser_version.as_deref()
                                != Some(binding.parser_version.as_str())
                            || item.resolved_command != binding.resolved_command
                    })
                {
                    insert_reason(
                        &mut reasons,
                        "PK-SENSOR-RESULT-BINDING",
                        "sensor result does not match its policy-bound configuration, parser, and command",
                    );
                }
                // Partial counts are rejected by `validate_typed_result`; here the
                // profile additionally forbids the all-absent form.
                if definition.unwrap().result_requirement
                    == Some(SensorResultRequirement::TestCounts)
                    && item.discovered.is_none()
                {
                    insert_reason(
                        &mut reasons,
                        "PK-SENSOR-RESULT-INCOMPLETE",
                        "sensor profile requires discovered, executed, and passed test counts",
                    );
                }
                validate_typed_result(item, &mut reasons);
                if let Some(exit_code) = item.exit_code {
                    if !definition.unwrap().accepted_exit_codes.contains(&exit_code) {
                        insert_reason(
                            &mut reasons,
                            "PK-SENSOR-EXIT-CODE",
                            "sensor exit code is not accepted by the profile",
                        );
                    }
                }
            }
            SensorStatus::Failed => {
                validate_typed_result(item, &mut reasons);
                insert_reason(
                    &mut reasons,
                    "PK-SENSOR-FAILED",
                    "a required sensor reported an executed failure",
                );
            }
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
                let condition_matches_context = context
                    .zip(item.not_applicable_condition_id.as_deref())
                    .is_some_and(|((paths, risk), condition_id)| {
                        definition
                            .unwrap()
                            .not_applicable_when
                            .iter()
                            .any(|condition| {
                                condition.id == condition_id
                                    && condition_covers(condition, paths, risk)
                            })
                    });
                let reason_recorded = item
                    .not_applicable_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty());
                if !condition_matches_context || !reason_recorded {
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
                | "PK-SENSOR-EXIT-CODE"
                | "PK-SENSOR-RESULT-COUNT-MISMATCH"
                | "PK-ADEQ-NO-TESTS"
                | "PK-ADEQ-COUNT-MISMATCH"
                | "PK-ADEQ-CRITERION-UNMAPPED"
                | "PK-ADEQ-BEHAVIOR-UNMAPPED"
                | "PK-ADEQ-NEGATIVE-CASE-MISSING"
                | "PK-ADEQ-UNEXECUTED-MAPPING"
                | "PK-ADEQ-DUPLICATE-TEST"
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

fn validate_profile(profile: &SensorProfile) -> Result<(), SensorResolutionError> {
    profile
        .validate_version()
        .map_err(SensorResolutionError::InvalidVersion)?;
    let mut sensors = BTreeSet::new();
    for sensor in &profile.sensors {
        if !sensors.insert(sensor.id.as_str()) {
            return Err(SensorResolutionError::DuplicateSensor(
                sensor.id.to_string(),
            ));
        }
        let mut conditions = BTreeSet::new();
        for condition in sensor
            .required_when
            .iter()
            .chain(&sensor.not_applicable_when)
        {
            if condition.id.trim().is_empty() || !conditions.insert(condition.id.as_str()) {
                return Err(SensorResolutionError::InvalidCondition(
                    condition.id.clone(),
                ));
            }
            for pattern in &condition.path_globs {
                path_glob(pattern)
                    .map_err(|_| SensorResolutionError::InvalidGlob(pattern.clone()))?;
            }
        }
    }
    Ok(())
}

pub(crate) fn normalize_path(path: &str) -> Result<String, SensorResolutionError> {
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
            path_glob(pattern)
                .ok()
                .is_some_and(|matcher| paths.iter().any(|path| matcher.is_match(path)))
        });
    risk_matches && path_matches
}

/// An exemption must cover the entire known change. Matching one changed path,
/// as `required_when` does, would let a docs-only condition exempt a code change.
fn condition_covers(condition: &SensorCondition, paths: &[String], risk: Risk) -> bool {
    let risk_matches = condition.risks.is_empty() || condition.risks.contains(&risk);
    let matchers = condition
        .path_globs
        .iter()
        .map(|pattern| path_glob(pattern))
        .collect::<Result<Vec<_>, _>>();
    let path_matches = condition.path_globs.is_empty()
        || matchers.is_ok_and(|matchers| {
            !paths.is_empty()
                && paths
                    .iter()
                    .all(|path| matchers.iter().any(|matcher| matcher.is_match(path)))
        });
    risk_matches && path_matches
}

fn evaluate_adequacy(
    policy: &TestAdequacyPolicy,
    evidence: Option<&TestAdequacyEvidence>,
    reasons: &mut Vec<Reason>,
) {
    let required = !policy.acceptance_criterion_ids.is_empty()
        || !policy.changed_behavior_ids.is_empty()
        || !policy.negative_case_ids.is_empty();
    if !required {
        insert_reason(
            reasons,
            "PK-ADEQ-POLICY-EMPTY",
            "test adequacy policy has no requirements",
        );
    }
    for ids in [
        &policy.acceptance_criterion_ids,
        &policy.changed_behavior_ids,
        &policy.negative_case_ids,
    ] {
        if !unique_nonempty_ids(ids) {
            insert_reason(
                reasons,
                "PK-ADEQ-POLICY-INVALID",
                "adequacy requirement IDs must be unique and nonempty",
            );
        }
    }
    if policy
        .negative_case_ids
        .iter()
        .any(|id| !policy.changed_behavior_ids.contains(id))
    {
        insert_reason(
            reasons,
            "PK-ADEQ-POLICY-INVALID",
            "negative cases must also be required changed behaviors",
        );
    }
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
    if !unique_nonempty_ids(&evidence.negative_case_ids) {
        insert_reason(
            reasons,
            "PK-ADEQ-NEGATIVE-CASE-MISSING",
            "negative case IDs must be unique and nonempty",
        );
    }
    // A name is not coverage: every claimed negative needs a required behavior
    // and a mapping to tests whose execution is checked below.
    if evidence
        .negative_case_ids
        .iter()
        .any(|id| !policy.changed_behavior_ids.contains(id))
    {
        insert_reason(
            reasons,
            "PK-ADEQ-BEHAVIOR-UNMAPPED",
            "negative cases must be required changed behaviors",
        );
    }
    validate_mappings(
        &evidence.negative_case_ids,
        &evidence.changed_behaviors,
        "PK-ADEQ-BEHAVIOR-UNMAPPED",
        reasons,
    );
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
    if executed.len() != evidence.executed_test_ids.len()
        || evidence
            .executed_test_ids
            .iter()
            .any(|test_id| test_id.trim().is_empty())
    {
        insert_reason(
            reasons,
            "PK-ADEQ-DUPLICATE-TEST",
            "executed test IDs must be unique and nonempty",
        );
    }
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

fn validate_typed_result(evidence: &SensorEvidence, reasons: &mut Vec<Reason>) {
    if evidence.resolved_command.is_empty()
        || evidence.source_digest.is_none()
        || evidence.configuration_digest.is_none()
        || evidence
            .parser_version
            .as_deref()
            .is_none_or(|version| version.trim().is_empty())
        || evidence.exit_code.is_none()
    {
        insert_reason(
            reasons,
            "PK-SENSOR-RESULT-INCOMPLETE",
            "executed sensor evidence is missing typed result provenance",
        );
    }
    match (evidence.discovered, evidence.executed, evidence.passed) {
        (None, None, None) => {}
        (Some(discovered), Some(executed), Some(passed)) => {
            if discovered == 0 {
                insert_reason(
                    reasons,
                    "PK-ADEQ-NO-TESTS",
                    "a required sensor discovered no tests",
                );
            }
            if discovered != executed || executed != passed {
                insert_reason(
                    reasons,
                    "PK-SENSOR-RESULT-COUNT-MISMATCH",
                    "sensor result counts must agree",
                );
            }
        }
        _ => {
            insert_reason(
                reasons,
                "PK-SENSOR-RESULT-INCOMPLETE",
                "sensor result counts must be either all present or all absent",
            );
        }
    }
}

fn validate_mappings(
    required_ids: &[String],
    mappings: &[policy_contracts::TestMapping],
    reason_code: &str,
    reasons: &mut Vec<Reason>,
) {
    let mut seen = BTreeSet::new();
    if mappings.iter().any(|mapping| {
        mapping.requirement_id.trim().is_empty()
            || !seen.insert(&mapping.requirement_id)
            || mapping.test_ids.is_empty()
            || !unique_nonempty_ids(&mapping.test_ids)
    }) {
        insert_reason(
            reasons,
            reason_code,
            "mapping IDs and test IDs must be unique and nonempty",
        );
    }
    if required_ids.iter().any(|required_id| {
        !mappings
            .iter()
            .any(|mapping| mapping.requirement_id == *required_id && !mapping.test_ids.is_empty())
    }) {
        insert_reason(reasons, reason_code, "a required test mapping is missing");
    }
}

fn unique_nonempty_ids(ids: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    ids.iter()
        .all(|id| !id.trim().is_empty() && seen.insert(id))
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

/// GitHub Actions subset: ASCII alphanumerics, `.`, `_`, `-`, `/`, segment-local
/// `*`, and whole-segment `**`. Reject other syntax before compiling with globset.
pub(crate) fn path_glob(pattern: &str) -> Result<GlobMatcher, SensorResolutionError> {
    let invalid = || SensorResolutionError::InvalidGlob(pattern.to_owned());
    if !pattern
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"._-/*".contains(&byte))
        || pattern.split('/').any(|segment| {
            matches!(segment, "" | "." | "..") || (segment.contains("**") && segment != "**")
        })
    {
        return Err(invalid());
    }
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|_| invalid())
}
