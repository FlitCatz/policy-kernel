use crate::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    #[error("{0}")]
    Invalid(&'static str),
}
type Result = std::result::Result<(), ContractError>;
fn require(condition: bool, message: &'static str) -> Result {
    if condition {
        Ok(())
    } else {
        Err(ContractError::Invalid(message))
    }
}
fn unique<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result {
    let mut seen = BTreeSet::new();
    for id in ids {
        require(seen.insert(id), "duplicate ID or path")?;
    }
    Ok(())
}
fn role(digest: &CanonicalDigest, expected: DigestRole) -> Result {
    require(digest.role == expected, "canonical digest role mismatch")
}
fn subject(subject: &SubjectV1) -> Result {
    role(&subject.source_set_digest, DigestRole::SourceSet)?;
    role(
        &subject.scope_definition_digest,
        DigestRole::ScopeDefinition,
    )
}
fn exclusions(values: &[ExclusionV1]) -> Result {
    unique(values.iter().map(|v| v.id.as_str()))
}
// Lexical wire validation only. No filesystem observation or symlink guarantee.
fn relative_path(path: &str, cwd: bool) -> Result {
    require(
        (cwd && path == ".")
            || (!path.is_empty()
                && !path.contains(['\\', ':', '\0'])
                && path
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != "..")),
        "path must be a portable relative path",
    )
}

impl ExpectedScopeV1 {
    pub fn validate(&self) -> Result {
        subject(&self.subject)?;
        let rules = self.requirements.observations.as_slice();
        let requirements = self.requirements.requirements.as_slice();
        let bindings = self.bindings.observations.as_slice();
        unique(rules.iter().map(|r| r.id.as_str()))?;
        unique(requirements.iter().map(|r| r.id.as_str()))?;
        unique(bindings.iter().map(|r| r.id.as_str()))?;
        require(
            self.bindings.maximum_age_seconds > 0,
            "maximum age must be positive",
        )?;
        require(
            rules.len() == bindings.len(),
            "binding set differs from required observation set",
        )?;
        for rule in rules {
            let binding = bindings
                .iter()
                .find(|b| b.id == rule.id)
                .ok_or(ContractError::Invalid("missing observation binding"))?;
            role(&binding.command_digest, DigestRole::Command)?;
            role(&binding.configuration_digest, DigestRole::Configuration)?;
            require(
                rule.tests.is_some() == binding.parser.is_some(),
                "suite parser binding mismatch",
            )?;
            if let Some(t) = &rule.tests {
                require(
                    t.minimum_discovered > 0 && t.minimum_executed > 0 && t.minimum_passed > 0,
                    "invalid required test count bounds",
                )?;
            }
        }
        for requirement in requirements {
            require(
                requirement.minimum_executed_tests > 0,
                "mapping minimum must be positive",
            )?;
            for suite in requirement.suites.as_slice() {
                require(
                    rules.iter().any(|r| r.id == *suite && r.tests.is_some()),
                    "mapping rule references unknown suite",
                )?;
            }
        }
        exclusions(&self.requirements.exemptions)
    }
    pub fn digest(&self) -> std::result::Result<CanonicalDigest, policy_core::CanonicalError> {
        canonical_digest(DigestRole::ExpectedScope, self)
    }
}

impl NativeRunV1 {
    pub fn validate(&self) -> Result {
        subject(&self.subject.source)?;
        unique(self.observations.iter().map(|o| o.id.as_str()))?;
        unique(self.observations.iter().map(|o| o.execution_id.as_str()))?;
        unique(
            self.test_observations
                .iter()
                .map(|t| t.observation_id.as_str()),
        )?;
        unique(self.artifacts.iter().map(|a| a.id.as_str()))?;
        unique(self.artifacts.iter().map(|a| a.path.as_str()))?;
        unique(self.mappings.iter().map(|m| m.requirement_id.as_str()))?;
        exclusions(&self.exclusions)?;
        for artifact in &self.artifacts {
            relative_path(&artifact.path, false)?;
        }
        for observation in &self.observations {
            require(
                observation.subject == self.subject,
                "observation subject mismatch",
            )?;
            role(&observation.configuration_digest, DigestRole::Configuration)?;
            require(
                observation
                    .command
                    .argv
                    .first()
                    .is_some_and(|v| !v.trim().is_empty())
                    && observation.command.argv.iter().all(|v| !v.contains('\0')),
                "invalid command argv",
            )?;
            relative_path(&observation.command.cwd, true)?;
            require(
                observation.started_at <= observation.completed_at
                    && observation.completed_at <= observation.observed_at,
                "observation time order mismatch",
            )?;
        }
        for report in &self.test_observations {
            require(
                self.observations
                    .iter()
                    .any(|o| o.id == report.observation_id),
                "test report references unknown observation",
            )?;
            require(
                self.artifacts.iter().any(|a| {
                    a.id == report.raw_report.id
                        && a.digest == report.raw_report.digest
                        && a.role == ArtifactRole::RawReport
                }),
                "raw report artifact mismatch",
            )?;
            unique(report.tests.iter().map(|t| t.id.as_str()))?;
            let mut counts = TestCountsV1 {
                discovered: report.tests.len() as u64,
                executed: 0,
                passed: 0,
                failed: 0,
                skipped: 0,
                retries: 0,
            };
            for test in &report.tests {
                match test.outcome {
                    TestOutcome::Passed | TestOutcome::Failed => {
                        require(test.attempts > 0, "executed test has no attempt")?;
                        counts.executed += 1;
                        if test.outcome == TestOutcome::Passed {
                            counts.passed += 1;
                        } else {
                            counts.failed += 1;
                        }
                        counts.retries = counts
                            .retries
                            .checked_add(u64::from(test.attempts - 1))
                            .ok_or(ContractError::Invalid("retry count overflow"))?;
                    }
                    TestOutcome::Skipped | TestOutcome::NotExecuted => {
                        require(test.attempts == 0, "unexecuted test has attempts")?;
                        if test.outcome == TestOutcome::Skipped {
                            counts.skipped += 1;
                        }
                    }
                }
            }
            require(
                counts == report.counts,
                "test counts disagree with test records",
            )?;
        }
        for mapping in &self.mappings {
            for reference in mapping.tests.as_slice() {
                require(
                    self.test_observations.iter().any(|r| {
                        r.observation_id == reference.suite_id
                            && r.tests.iter().any(|t| {
                                t.id == reference.test_id
                                    && matches!(
                                        t.outcome,
                                        TestOutcome::Passed | TestOutcome::Failed
                                    )
                            })
                    }),
                    "mapping references unexecuted or unknown test",
                )?;
            }
        }
        Ok(())
    }
    pub fn digest(&self) -> std::result::Result<CanonicalDigest, policy_core::CanonicalError> {
        canonical_digest(DigestRole::NativeRun, self)
    }
}

/// Checks the caller's independent subject against a run. This is consistency,
/// not Git observation, policy evaluation or authentication of either input.
pub fn validate_subject_binding(expected: &ExpectedScopeV1, run: &NativeRunV1) -> Result {
    expected.validate()?;
    run.validate()?;
    require(
        expected.subject == run.subject.source,
        "expected and observed subjects differ",
    )
}

impl LocalScopeResultV1 {
    pub fn validate(&self) -> Result {
        subject(&self.subject.source)?;
        unique(self.requirements.iter().map(|r| r.id.as_str()))?;
        exclusions(&self.exclusions)?;
        role(&self.digests.expected_scope, DigestRole::ExpectedScope)?;
        role(&self.digests.native_run, DigestRole::NativeRun)?;
        role(&self.digests.policy, DigestRole::Policy)?;
        if let Some(digest) = &self.digests.evidence {
            role(digest, DigestRole::Evidence)?;
        }
        require(
            self.status != ScopeStatus::Satisfied
                || (!self.requirements.is_empty()
                    && self.digests.evidence.is_some()
                    && self.reasons.is_empty()
                    && self
                        .requirements
                        .iter()
                        .all(|r| r.status == ScopeStatus::Satisfied)),
            "satisfied result requires evidence and satisfied requirements",
        )?;
        require(
            self.status == ScopeStatus::Error || self.digests.evidence.is_some(),
            "result without evidence must be error",
        )?;
        require(
            self.status == ScopeStatus::Satisfied || !self.reasons.is_empty(),
            "non-success result requires reasons",
        )?;
        for requirement in &self.requirements {
            require(
                (requirement.status == ScopeStatus::Satisfied) == requirement.reasons.is_empty(),
                "requirement status and reasons disagree",
            )?;
        }
        Ok(())
    }
    pub fn digest(&self) -> std::result::Result<CanonicalDigest, policy_core::CanonicalError> {
        canonical_digest(DigestRole::LocalScopeResult, self)
    }
}
