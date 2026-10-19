#![forbid(unsafe_code)]
//! Kernel-owned local scope assembly. No filesystem, report parser or authorization.
use policy_contracts::*;
use policy_core::{canonical_sha256, evaluate_sensor_evidence, SensorEvaluationStatus};
use policy_native_contracts::*;
use serde::{Deserialize, Serialize};

/// Adapter output, not an attestation. Decoding this JSON checks typed consistency
/// only; it cannot establish that an importer actually read the original bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedObservationsV1 {
    pub schema_version: VersionV1,
    pub expected_scope_digest: CanonicalDigest,
    pub expected_run_id: ExecutionId,
    pub expected_scope_id: RecordId,
    pub adapter_version: RecordId,
    pub trust: LocalTrust,
    pub review: ReviewGate,
    /// Parser-level failures (for example suite diagnostics) not captured by counts.
    /// Required even when empty; supplied by the adapter, not an authorization claim.
    pub unsuccessful_reports: Vec<SensorId>,
    pub run: NativeRunV1,
}

/// Diagnostic assembly is separate from the wire result and pre-merge decisions.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeEvaluation {
    pub result: LocalScopeResultV1,
    pub profile: SensorProfile,
    pub evidence: Option<EvidenceBundle>,
    pub core_reasons: Vec<Reason>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Canonical(#[from] policy_core::CanonicalError),
}

fn reason(
    code: ReasonCode,
    suite: Option<&SensorId>,
    requirement: Option<&RecordId>,
    artifact: Option<&RecordId>,
) -> ReasonV1 {
    ReasonV1 {
        code,
        location: LocationV1 {
            suite_id: suite.cloned(),
            requirement_id: requirement.cloned(),
            artifact_id: artifact.cloned(),
        },
    }
}

fn profile(expected: &ExpectedScopeV1) -> SensorProfile {
    let mut adequacy = TestAdequacyPolicy {
        acceptance_criterion_ids: vec![],
        changed_behavior_ids: vec![],
        negative_case_ids: vec![],
    };
    for requirement in expected.requirements.requirements.as_slice() {
        match requirement.kind {
            RequirementKind::AcceptanceCriterion => adequacy
                .acceptance_criterion_ids
                .push(requirement.id.to_string()),
            RequirementKind::ChangedBehavior => adequacy
                .changed_behavior_ids
                .push(requirement.id.to_string()),
            RequirementKind::NegativeCase => {
                // Core negative cases are a subset of changed behaviors. Native IDs
                // are globally unique, so this preserves the independent obligation.
                adequacy
                    .changed_behavior_ids
                    .push(requirement.id.to_string());
                adequacy.negative_case_ids.push(requirement.id.to_string());
            }
        }
    }
    SensorProfile {
        schema_version: 2,
        adequacy,
        sensors: expected
            .requirements
            .observations
            .as_slice()
            .iter()
            .map(|r| SensorDefinition {
                id: r.id.clone(),
                required: true,
                required_when: vec![],
                not_applicable_when: vec![],
                accepted_exit_codes: r.accepted_exit_codes.as_slice().to_vec(),
                result_requirement: Some(if r.tests.is_some() {
                    SensorResultRequirement::TestCounts
                } else {
                    SensorResultRequirement::Process
                }),
                // Native approval carries a command digest, not argv. The façade checks
                // that digest (including cwd), config and parser independently below.
                result_binding: None,
            })
            .collect(),
    }
}

/// Enforces the independently approved expectation, then delegates sensor and
/// adequacy evaluation to policy-core. Never calls a pre-merge evaluator.
pub fn evaluate_scope(
    expected: &ExpectedScopeV1,
    imported: &ImportedObservationsV1,
) -> Result<ScopeEvaluation, ScopeError> {
    expected.validate()?;
    imported.run.validate()?;
    let mut seen = std::collections::BTreeSet::new();
    for id in &imported.unsuccessful_reports {
        if !seen.insert(id)
            || !imported
                .run
                .test_observations
                .iter()
                .any(|r| r.observation_id == *id)
        {
            return Err(ContractError::Invalid("invalid unsuccessful report set").into());
        }
    }
    let run = &imported.run;
    let profile = profile(expected);
    let expected_digest = expected.digest()?;
    // Include native bounds, exemptions, approval digest, bindings and time policy;
    // hashing the core profile alone would omit native-only policy obligations.
    let policy_digest = canonical_digest(
        DigestRole::Policy,
        &(&expected.requirements, &expected.bindings, &profile),
    )?;
    let mut result = LocalScopeResultV1 {
        schema_version: VersionV1::default(),
        status: ScopeStatus::Error,
        subject: run.subject.clone(),
        requirements: vec![],
        reasons: vec![],
        digests: ResultDigestsV1 {
            expected_scope: expected_digest.clone(),
            native_run: run.digest()?,
            policy: policy_digest,
            evidence: None,
        },
        evaluated_at: expected.bindings.evaluated_at.clone(),
        verifier_version: RecordId::parse(concat!(
            "policy-native-verifier/",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("static version"),
        adapter_version: imported.adapter_version.clone(),
        trust: LocalTrust::LocalUntrusted,
        review: ReviewGate::Unmet,
        exclusions: run.exclusions.clone(),
    };
    if expected_digest != imported.expected_scope_digest {
        result
            .reasons
            .push(reason(ReasonCode::BindingMismatch, None, None, None));
    }
    if expected.subject != run.subject.source
        || imported.expected_run_id != run.subject.run_id
        || imported.expected_scope_id != run.subject.scope_id
    {
        result
            .reasons
            .push(reason(ReasonCode::SubjectMismatch, None, None, None));
    }
    let rules = expected.requirements.observations.as_slice();
    let requirements = expected.requirements.requirements.as_slice();
    for rule in rules {
        let Some(observation) = run.observations.iter().find(|o| o.id == rule.id) else {
            result.reasons.push(reason(
                ReasonCode::MissingObservation,
                Some(&rule.id),
                None,
                None,
            ));
            continue;
        };
        let binding = expected
            .bindings
            .observations
            .as_slice()
            .iter()
            .find(|b| b.id == rule.id)
            .expect("validated binding set");
        let report = run
            .test_observations
            .iter()
            .find(|t| t.observation_id == rule.id);
        if canonical_digest(DigestRole::Command, &observation.command)? != binding.command_digest
            || observation.configuration_digest != binding.configuration_digest
            || report.map(|t| &t.parser) != binding.parser.as_ref()
        {
            result.reasons.push(reason(
                ReasonCode::BindingMismatch,
                Some(&rule.id),
                None,
                report.map(|t| &t.raw_report.id),
            ));
        }
        if rule.tests.is_some() && report.is_none() {
            result.reasons.push(reason(
                ReasonCode::IncompleteRun,
                Some(&rule.id),
                None,
                None,
            ));
        }
        let elapsed =
            *expected.bindings.evaluated_at.as_datetime() - *observation.started_at.as_datetime();
        if observation.observed_at > expected.bindings.evaluated_at
            || elapsed.num_seconds() < 0
            || elapsed.num_seconds() as u64 > expected.bindings.maximum_age_seconds
            || (elapsed.num_seconds() as u64 == expected.bindings.maximum_age_seconds
                && elapsed.subsec_nanos() > 0)
        {
            result.reasons.push(reason(
                ReasonCode::StaleObservation,
                Some(&rule.id),
                None,
                None,
            ));
        }
        if !matches!(observation.completion, CompletionV1::Exited { .. }) {
            result.reasons.push(reason(
                ReasonCode::IncompleteRun,
                Some(&rule.id),
                None,
                None,
            ));
        }
        if report.is_some_and(|r| {
            r.counts.discovered == 0
                || r.counts.skipped != 0
                || r.counts.retries != 0
                || r.counts.discovered != r.counts.executed
        }) {
            result.reasons.push(reason(
                ReasonCode::InvalidReport,
                Some(&rule.id),
                None,
                report.map(|t| &t.raw_report.id),
            ));
        }
    }
    for observation in &run.observations {
        if !rules.iter().any(|r| r.id == observation.id) {
            result.reasons.push(reason(
                ReasonCode::BindingMismatch,
                Some(&observation.id),
                None,
                None,
            ));
        }
    }
    for mapping in &run.mappings {
        if !requirements.iter().any(|r| r.id == mapping.requirement_id) {
            result.reasons.push(reason(
                ReasonCode::BindingMismatch,
                None,
                Some(&mapping.requirement_id),
                None,
            ));
        }
    }
    // Exclusions describe omitted scope; they never waive a required sensor or
    // mapping. Compare complete entries, including their approved explanations.
    if run.exclusions.len() != expected.requirements.exemptions.len()
        || run
            .exclusions
            .iter()
            .any(|e| !expected.requirements.exemptions.contains(e))
    {
        result
            .reasons
            .push(reason(ReasonCode::BindingMismatch, None, None, None));
    }
    if !result.reasons.is_empty() {
        result.requirements = requirements
            .iter()
            .map(|r| RequirementResultV1 {
                id: r.id.clone(),
                status: ScopeStatus::Error,
                reasons: vec![reason(ReasonCode::IncompleteRun, None, Some(&r.id), None)],
            })
            .collect();
        result.validate()?;
        return Ok(ScopeEvaluation {
            result,
            profile,
            evidence: None,
            core_reasons: vec![],
        });
    }

    let evidence = assemble(
        expected,
        run,
        &result.digests.policy,
        &imported.unsuccessful_reports,
    )?;
    let required = rules.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    let core = evaluate_sensor_evidence(
        &profile,
        &required,
        &evidence.sensors,
        evidence.test_adequacy.as_ref(),
    );
    // Evaluate each required sensor through the same core path for precise native
    // locations. Aggregate adequacy failures are mapped below per requirement.
    for rule in rules {
        let single = evaluate_sensor_evidence(
            &profile,
            std::slice::from_ref(&rule.id),
            &evidence.sensors,
            evidence.test_adequacy.as_ref(),
        );
        let report = run
            .test_observations
            .iter()
            .find(|t| t.observation_id == rule.id);
        let bounds_fail = rule.tests.as_ref().zip(report).is_some_and(|(p, r)| {
            r.counts.discovered < p.minimum_discovered
                || r.counts.executed < p.minimum_executed
                || r.counts.passed < p.minimum_passed
                || r.counts.failed > p.maximum_failed
                || r.counts.skipped > p.maximum_skipped
                || r.counts.retries > p.maximum_retries
        });
        if bounds_fail
            || single
                .reasons
                .iter()
                .any(|r| r.code.starts_with("PK-SENSOR-") || r.code == "PK-ADEQ-NO-TESTS")
        {
            result.reasons.push(reason(
                ReasonCode::ScopeUnsatisfied,
                Some(&rule.id),
                None,
                report.map(|r| &r.raw_report.id),
            ));
        }
    }
    for requirement in requirements {
        let mapping = run
            .mappings
            .iter()
            .find(|m| m.requirement_id == requirement.id);
        let mut reasons = vec![];
        if mapping.is_none_or(|m| {
            (m.tests.as_slice().len() as u64) < requirement.minimum_executed_tests
                || m.tests
                    .as_slice()
                    .iter()
                    .any(|t| !requirement.suites.as_slice().contains(&t.suite_id))
        }) {
            reasons.push(reason(
                ReasonCode::UnexecutedMapping,
                None,
                Some(&requirement.id),
                None,
            ));
        }
        if let Some(mapping) = mapping {
            for reference in mapping.tests.as_slice() {
                let report = run
                    .test_observations
                    .iter()
                    .find(|r| r.observation_id == reference.suite_id)
                    .expect("validated mapping");
                let test = report
                    .tests
                    .iter()
                    .find(|t| t.id == reference.test_id)
                    .expect("validated mapping");
                if test.outcome != TestOutcome::Passed
                    || result
                        .reasons
                        .iter()
                        .any(|r| r.location.suite_id.as_ref() == Some(&reference.suite_id))
                {
                    reasons.push(reason(
                        ReasonCode::ScopeUnsatisfied,
                        Some(&reference.suite_id),
                        Some(&requirement.id),
                        Some(&report.raw_report.id),
                    ));
                }
            }
        }
        result.reasons.extend(reasons.clone());
        result.requirements.push(RequirementResultV1 {
            id: requirement.id.clone(),
            status: if reasons.is_empty() {
                ScopeStatus::Satisfied
            } else {
                ScopeStatus::NonPass
            },
            reasons,
        });
    }
    if core.status != SensorEvaluationStatus::Passed && result.reasons.is_empty() {
        result
            .reasons
            .push(reason(ReasonCode::ScopeUnsatisfied, None, None, None));
    }
    result.status = if result.reasons.is_empty() {
        ScopeStatus::Satisfied
    } else {
        ScopeStatus::NonPass
    };
    result.digests.evidence = Some(canonical_digest(DigestRole::Evidence, &evidence)?);
    result.validate()?;
    Ok(ScopeEvaluation {
        result,
        profile,
        evidence: Some(evidence),
        core_reasons: core.reasons,
    })
}

// JSON tuple encoding is injective even when suite/test IDs contain separators.
fn test_key(suite: &SensorId, test: &RecordId) -> String {
    serde_json::to_string(&(suite, test)).expect("string tuple")
}

fn assemble(
    expected: &ExpectedScopeV1,
    run: &NativeRunV1,
    policy: &CanonicalDigest,
    unsuccessful_reports: &[SensorId],
) -> Result<EvidenceBundle, ScopeError> {
    let mut adequacy = TestAdequacyEvidence {
        acceptance_criteria: vec![],
        changed_behaviors: vec![],
        negative_case_ids: vec![],
        executed_test_ids: vec![],
        discovered: 0,
        executed: 0,
        passed: 0,
        head_sha: Some(expected.subject.head.clone()),
    };
    for report in &run.test_observations {
        // Counts are structurally checked against allocated test vectors first.
        adequacy.discovered += report.counts.discovered;
        adequacy.executed += report.counts.executed;
        adequacy.passed += report.counts.passed;
        adequacy.executed_test_ids.extend(
            report
                .tests
                .iter()
                .filter(|t| matches!(t.outcome, TestOutcome::Passed | TestOutcome::Failed))
                .map(|t| test_key(&report.observation_id, &t.id)),
        );
    }
    for requirement in expected.requirements.requirements.as_slice() {
        if let Some(mapping) = run
            .mappings
            .iter()
            .find(|m| m.requirement_id == requirement.id)
        {
            let mapping = TestMapping {
                requirement_id: requirement.id.to_string(),
                test_ids: mapping
                    .tests
                    .as_slice()
                    .iter()
                    .map(|t| test_key(&t.suite_id, &t.test_id))
                    .collect(),
            };
            match requirement.kind {
                RequirementKind::AcceptanceCriterion => adequacy.acceptance_criteria.push(mapping),
                RequirementKind::ChangedBehavior => adequacy.changed_behaviors.push(mapping),
                RequirementKind::NegativeCase => {
                    adequacy.changed_behaviors.push(mapping);
                    adequacy.negative_case_ids.push(requirement.id.to_string());
                }
            }
        }
    }
    let mut sensors = vec![];
    let mut executions = vec![];
    for observation in &run.observations {
        let report = run
            .test_observations
            .iter()
            .find(|r| r.observation_id == observation.id);
        let rule = expected
            .requirements
            .observations
            .as_slice()
            .iter()
            .find(|r| r.id == observation.id)
            .expect("checked required set");
        let CompletionV1::Exited { code } = observation.completion else {
            unreachable!("checked completion")
        };
        let passed = rule.accepted_exit_codes.as_slice().contains(&code)
            && !unsuccessful_reports.contains(&observation.id)
            && report.is_none_or(|r| r.counts.failed == 0);
        let command_digest = canonical_digest(DigestRole::Command, &observation.command)?.sha256;
        let output_digest =
            canonical_digest(DigestRole::Evidence, &(observation, report, &run.artifacts))?.sha256;
        executions.push(ExecutionEvidence {
            execution_id: observation.execution_id.clone(),
            // Required legacy slots explicitly identify a recorder, never an AI
            // provider/model or authenticated workflow identity.
            provider: ProviderId::parse("local-recorder").expect("static identity"),
            model: ModelId::parse("not-a-model").expect("static identity"),
            status: if passed {
                ExecutionStatus::Passed
            } else {
                ExecutionStatus::Failed
            },
            command_digest: command_digest.clone(),
            input_digest: expected.digest()?.sha256,
            output_digest: output_digest.clone(),
            started_at: observation.started_at.clone(),
            completed_at: observation.completed_at.clone(),
            trust: TrustClass::LocalUntrusted,
            head_sha: Some(observation.subject.source.head.clone()),
        });
        sensors.push(SensorEvidence {
            sensor_id: observation.id.clone(),
            status: if passed {
                SensorStatus::Passed
            } else {
                SensorStatus::Failed
            },
            head_sha: observation.subject.source.head.clone(),
            command_digest,
            result_digest: output_digest,
            observed_at: observation.observed_at.clone(),
            trust: TrustClass::LocalUntrusted,
            resolved_command: observation.command.argv.clone(),
            source_digest: Some(observation.subject.source.source_set_digest.sha256.clone()),
            configuration_digest: Some(observation.configuration_digest.sha256.clone()),
            parser_version: Some(
                report
                    .map(|r| {
                        serde_json::to_string(&(&r.parser.id, &r.parser.version))
                            .expect("string tuple")
                    })
                    .unwrap_or_else(|| "native-process-v1".into()),
            ),
            exit_code: Some(code),
            discovered: report.map(|r| r.counts.discovered),
            executed: report.map(|r| r.counts.executed),
            passed: report.map(|r| r.counts.passed),
            not_applicable_condition_id: None,
            not_applicable_reason: None,
        });
    }
    let mut bundle = EvidenceBundle {
        schema_version: 1,
        manifest: EvidenceManifest {
            schema_version: 1,
            repository: expected.subject.repository.clone(),
            work_item_id: run.subject.scope_id.clone(),
            // A local scope has no PR base or CI workflow. These legacy slots
            // explicitly denote a same-head snapshot and local run label.
            base_sha: expected.subject.head.clone(),
            head_sha: expected.subject.head.clone(),
            policy_digest: policy.sha256.clone(),
            source_set_digest: expected.subject.source_set_digest.sha256.clone(),
            workflow_run_id: WorkflowRunId::parse(format!("local:{}", run.subject.run_id))
                .expect("nonempty local label"),
            created_at: expected.bindings.evaluated_at.clone(),
            child_digests: vec![],
        },
        modifications: vec![],
        executions,
        reviews: vec![],
        findings: vec![],
        waivers: vec![],
        sensors,
        test_adequacy: Some(adequacy),
        knowledge: vec![],
    };
    macro_rules! child {
        ($field:ident) => {
            bundle.manifest.child_digests.push(ArtifactDigest {
                path: stringify!($field).into(),
                digest: canonical_sha256(&bundle.$field)?,
            });
        };
    }
    child!(executions);
    child!(findings);
    child!(knowledge);
    child!(modifications);
    child!(reviews);
    child!(sensors);
    child!(test_adequacy);
    child!(waivers);
    Ok(bundle)
}
