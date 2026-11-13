use crate::{fs, raw_digest, reports};
use policy_contracts::SensorId;
use policy_native_contracts::*;
use std::{collections::BTreeSet, io, path::Path};

/// Explicit producer association; no filename or suite-name inference occurs.
pub struct ReportSelection {
    pub observation_id: SensorId,
    pub artifact_id: RecordId,
}

/// Producer claim that all executed tests in a suite cover one requirement.
pub struct MappingSelection {
    pub requirement_id: RecordId,
    pub suite_id: SensorId,
}

/// Fill an empty producer envelope using the same raw-report projection as import.
/// This is claim preparation, not verification, approval or scope evaluation.
/// One raw report maps to one observation; multi-project TRX needs distinct suites.
pub fn populate_claims(
    run_directory: &Path,
    expected: &ExpectedScopeV1,
    run: &NativeRunV1,
    selections: &[ReportSelection],
    mappings: &[MappingSelection],
) -> io::Result<NativeRunV1> {
    expected
        .validate()
        .map_err(|e| fs::invalid(e.to_string()))?;
    run.validate().map_err(|e| fs::invalid(e.to_string()))?;
    if selections.is_empty() || !run.test_observations.is_empty() || !run.mappings.is_empty() {
        return Err(fs::invalid("claim preparation needs reports and an envelope with empty test_observations and mappings"));
    }
    let root = fs::directory(run_directory)?;
    let entries = fs::inventory(&root)?;
    let mut populated = run.clone();
    let mut observations = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    let mut total = 0;
    for selection in selections {
        if !observations.insert(selection.observation_id.as_str())
            || !artifacts.insert(selection.artifact_id.as_str())
        {
            return Err(fs::invalid(
                "each report and observation must be selected exactly once",
            ));
        }
        if !run
            .observations
            .iter()
            .any(|o| o.id == selection.observation_id)
        {
            return Err(fs::invalid(
                "report selection references unknown observation",
            ));
        }
        let parser = expected
            .bindings
            .observations
            .as_slice()
            .iter()
            .find(|b| b.id == selection.observation_id)
            .and_then(|b| b.parser.as_ref())
            .ok_or_else(|| {
                fs::invalid("report selection needs an approved suite parser binding")
            })?;
        let artifact = run
            .artifacts
            .iter()
            .find(|a| a.id == selection.artifact_id && a.role == ArtifactRole::RawReport)
            .ok_or_else(|| {
                fs::invalid("report selection references unknown raw report artifact")
            })?;
        let entry = entries
            .iter()
            .find(|e| e.path == artifact.path)
            .ok_or_else(|| fs::invalid("missing raw report file"))?;
        if entry.kind != rustix::fs::FileType::RegularFile || entry.stat.2 > fs::FILE_LIMIT {
            return Err(fs::invalid(
                "raw report must be a regular file within 16 MiB",
            ));
        }
        total += entry.stat.2;
        if total > fs::TOTAL_LIMIT {
            return Err(fs::invalid("selected reports exceed 256 MiB"));
        }
        let bytes = fs::read(&root, entry)?;
        if raw_digest(&bytes) != artifact.digest {
            return Err(fs::invalid("raw report SHA-256 mismatch"));
        }
        let (_, claim) = reports::recompute(
            &selection.observation_id,
            parser,
            &ArtifactRefV1 {
                id: artifact.id.clone(),
                digest: artifact.digest.clone(),
            },
            &bytes,
        )
        .map_err(fs::invalid)?;
        populated.test_observations.push(claim);
    }
    if fs::inventory(&root)? != entries {
        return Err(fs::invalid("directory changed during claim preparation"));
    }
    populate_mappings(expected, &mut populated, mappings)?;
    populated
        .validate()
        .map_err(|e| fs::invalid(e.to_string()))?;
    Ok(populated)
}

fn populate_mappings(
    expected: &ExpectedScopeV1,
    run: &mut NativeRunV1,
    selections: &[MappingSelection],
) -> io::Result<()> {
    let mut seen = BTreeSet::new();
    for selection in selections {
        if !seen.insert((
            selection.requirement_id.as_str(),
            selection.suite_id.as_str(),
        )) {
            return Err(fs::invalid("duplicate requirement/suite mapping selection"));
        }
        let approved = expected
            .requirements
            .requirements
            .as_slice()
            .iter()
            .find(|r| r.id == selection.requirement_id)
            .is_some_and(|r| r.suites.as_slice().contains(&selection.suite_id));
        if !approved {
            return Err(fs::invalid(
                "mapping selection is outside the approved requirement suites",
            ));
        }
        let suite = run
            .test_observations
            .iter()
            .find(|t| t.observation_id == selection.suite_id)
            .ok_or_else(|| fs::invalid("mapping selection references an unselected suite"))?;
        let tests: Vec<_> = suite
            .tests
            .iter()
            .filter(|t| matches!(t.outcome, TestOutcome::Passed | TestOutcome::Failed))
            .map(|t| TestRefV1 {
                suite_id: selection.suite_id.clone(),
                test_id: t.id.clone(),
            })
            .collect();
        if tests.is_empty() {
            return Err(fs::invalid("mapping selection has no executed tests"));
        }
        if let Some(mapping) = run
            .mappings
            .iter_mut()
            .find(|m| m.requirement_id == selection.requirement_id)
        {
            let mut combined = mapping.tests.as_slice().to_vec();
            combined.extend(tests);
            mapping.tests = RequiredSet::new(combined).map_err(|e| fs::invalid(e.to_string()))?;
        } else {
            run.mappings.push(MappingV1 {
                requirement_id: selection.requirement_id.clone(),
                tests: RequiredSet::new(tests).map_err(|e| fs::invalid(e.to_string()))?,
            });
        }
    }
    Ok(())
}
