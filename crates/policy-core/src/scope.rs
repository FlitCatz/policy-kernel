use crate::sensors::{
    evaluate_sensor_evidence_inner, normalize_path, path_glob, required_sensors, validate_profile,
};
use crate::{SensorEvaluation, SensorResolutionError};
use policy_contracts::{PolicySnapshot, PreMergeContext, ScopeRule, SensorId, SensorProfile};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeResolutionError {
    #[error("{0}")]
    InvalidPolicy(&'static str),
    #[error("scope resolution requires a complete PR label list")]
    MissingLabels,
    #[error("PR label list contains a blank label or NUL")]
    InvalidLabel,
    #[error("changed path is not a readable repository-relative path: {0:?}")]
    InvalidPath(String),
    #[error("policy requires a sensor absent from its bound profile: {0}")]
    UnknownSensor(SensorId),
    #[error(transparent)]
    Sensor(#[from] SensorResolutionError),
}

/// Return the v5 required sensors in ID order, using complete PR observations.
/// An empty label or path list must describe a known empty set.
pub fn resolve_required_scope(
    policy: &PolicySnapshot,
    profile: &SensorProfile,
    changed_paths: &[String],
    labels: &[String],
) -> Result<Vec<SensorId>, ScopeResolutionError> {
    let paths = scope_paths(changed_paths)?;
    resolve_scope(policy, profile, &paths, labels)
}

pub(crate) fn evaluate_scoped_sensors(
    input: &policy_contracts::PreMergeEvaluationInput,
    context: &PreMergeContext,
) -> Result<SensorEvaluation, ScopeResolutionError> {
    let labels = context
        .labels
        .as_deref()
        .ok_or(ScopeResolutionError::MissingLabels)?;
    let paths = scope_paths(&context.changed_paths)?;
    let profile = &context.sensor_profile;
    let required = resolve_scope(&input.policy, profile, &paths, labels)?;
    Ok(evaluate_sensor_evidence_inner(
        profile,
        &required,
        &input.evidence.sensors,
        input.evidence.test_adequacy.as_ref(),
        Some((&paths, None)),
    ))
}

fn scope_paths(paths: &[String]) -> Result<Vec<String>, ScopeResolutionError> {
    paths
        .iter()
        .map(|path| {
            let normalized = normalize_path(path)?;
            if normalized.trim().is_empty() || path.contains('\0') || path.starts_with(['/', '\\'])
            {
                return Err(ScopeResolutionError::InvalidPath(path.clone()));
            }
            Ok(normalized)
        })
        .collect()
}

fn resolve_scope(
    policy: &PolicySnapshot,
    profile: &SensorProfile,
    paths: &[String],
    labels: &[String],
) -> Result<Vec<SensorId>, ScopeResolutionError> {
    policy
        .validate_version()
        .map_err(ScopeResolutionError::InvalidPolicy)?;
    let scope = policy
        .scope_rules
        .as_ref()
        .ok_or(ScopeResolutionError::InvalidPolicy(
            "declarative scope resolution requires policy v5",
        ))?;
    validate_profile(profile)?;
    if labels
        .iter()
        .any(|label| label.trim().is_empty() || label.contains('\0'))
    {
        return Err(ScopeResolutionError::InvalidLabel);
    }

    // Validate every reference and glob, even in rules inactive for this change.
    let defined: BTreeSet<_> = profile.sensors.iter().map(|sensor| &sensor.id).collect();
    for id in policy
        .required_sensor_ids
        .iter()
        .chain(&scope.default_required_sensor_ids)
        .chain(scope.rules.iter().flat_map(ScopeRule::required_sensor_ids))
    {
        if !defined.contains(id) {
            return Err(ScopeResolutionError::UnknownSensor(id.clone()));
        }
    }

    let mut required: BTreeSet<_> = required_sensors(profile, paths, None).collect();
    required.extend(policy.required_sensor_ids.iter().cloned());
    let mut path_matched = false;
    for rule in &scope.rules {
        let matches = match rule {
            ScopeRule::Label { label, .. } => labels.contains(label),
            ScopeRule::Paths { path_globs, .. } => {
                let matchers = path_globs
                    .iter()
                    .map(|glob| path_glob(glob))
                    .collect::<Result<Vec<_>, _>>()?;
                let matches = matchers
                    .iter()
                    .any(|glob| paths.iter().any(|path| glob.is_match(path)));
                path_matched |= matches;
                matches
            }
        };
        if matches {
            required.extend(rule.required_sensor_ids().iter().cloned());
        }
    }
    if !path_matched {
        required.extend(scope.default_required_sensor_ids.iter().cloned());
    }
    Ok(required.into_iter().collect())
}
