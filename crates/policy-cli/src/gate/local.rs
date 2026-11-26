//! Qualify a single signed run for the base policy's locally eligible checks.
use super::{
    github::{GitHub, Pull},
    native::safe_path,
    Check, Evidence, Result, Source,
};
use policy_contracts::{CommitSha, PolicySnapshot, SensorId};
use policy_core::requires_ci_for_local_evidence;
use policy_local_attest::{GateAttestation, GateStatement};
use policy_native_contracts::{
    canonical_local_digest, ExpectedScopeV1, LocalDigestRole, RequirementsV1, ScopeStatus,
};
use policy_native_verifier::{evaluate_scope, ImportedObservationsV1};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::Path,
    time::Duration,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LocalPolicy {
    allowed_signers: String,
    pub checks: BTreeSet<SensorId>,
    ci_required_labels: Vec<String>,
}

pub(super) struct LocalRequest<'a> {
    pub pull: &'a Pull,
    pub base_tip: &'a CommitSha,
    pub gate: &'a Evidence<'a>,
    pub directory: Option<&'a Path>,
}

impl LocalPolicy {
    pub fn validate(&self, checks: &BTreeMap<&SensorId, &Check>) -> Result<()> {
        let mut labels = BTreeSet::new();
        if !safe_path(&self.allowed_signers)
            || self.checks.is_empty()
            || self.checks.iter().any(|id| {
                checks
                    .get(id)
                    .is_none_or(|check| !matches!(check.source, Source::Native(_)))
            })
            || self.ci_required_labels.iter().any(|label| {
                label.trim().is_empty()
                    || label.chars().any(char::is_control)
                    || !labels.insert(label.to_ascii_lowercase())
            })
        {
            return Err("invalid local gate policy: configure a registry, native checks and distinct CI-only labels".into());
        }
        Ok(())
    }

    pub fn needs_ci(
        &self,
        snapshot: &PolicySnapshot,
        paths: &[String],
        request: &LocalRequest<'_>,
        labels: &[String],
    ) -> Result<bool> {
        let policy = snapshot
            .local_runner_evidence
            .as_ref()
            .ok_or("local gate requires policy v5 local runner opt-in at the base commit")?;
        Ok(requires_ci_for_local_evidence(policy, paths)?
            || paths
                .iter()
                .any(|path| path == request.gate.policy_path || path == &self.allowed_signers)
            || labels.iter().any(|observed| {
                self.ci_required_labels
                    .iter()
                    .any(|label| observed.eq_ignore_ascii_case(label))
            }))
    }

    pub fn evaluate(
        &self,
        api: &GitHub<'_>,
        snapshot: &PolicySnapshot,
        request: &LocalRequest<'_>,
    ) -> Result<(GateStatement, RequirementsV1)> {
        let policy = snapshot
            .local_runner_evidence
            .as_ref()
            .ok_or("local gate requires policy v5 local runner opt-in at the base commit")?;
        let directory = request
            .directory
            .ok_or("supply the signed local run with --local-attestation")?;
        let signed: GateAttestation = read_document(directory, "gate-attestation.json")?;
        let statement = &signed.statement;
        let run = &statement.run;
        let pull = request.pull;
        if statement.pr != pull.number
            || run.subject.repository != pull.base.repo.full_name
            || run.subject.head != pull.head.sha
            || run.base != *request.base_tip
            || Some(&statement.policy_sha256) != request.gate.policy_sha256.as_ref()
            || run.head_tree != api.head_tree(&pull.head.sha)?
        {
            return Err("local attestation does not match the current repository, PR, head, tree, base or policy".into());
        }
        if !policy
            .allowed_runner_ids
            .iter()
            .any(|id| id == run.runner.runner_id.as_str())
        {
            return Err("local runner is not allowed by the base policy".into());
        }
        let registry = api
            .base_file(request.base_tip, &self.allowed_signers)
            .map_err(|error| {
                format!(
                    "could not read allowed signers at base {}: {error}",
                    request.base_tip
                )
            })?;
        signed.verify(std::str::from_utf8(&registry)?, request.gate.evaluated_at)?;

        let expected: ExpectedScopeV1 = read_document(directory, "expected.json")?;
        let observations: ImportedObservationsV1 = read_document(directory, "observations.json")?;
        let template = expected.template_digest()?;
        if !policy
            .approved_scope_template_digests
            .contains(&template.sha256)
        {
            return Err("local scope template is not approved by the base policy".into());
        }
        if run.subject != expected.subject
            || run.run_id != observations.run.subject.run_id
            || run.scope_id != observations.run.subject.scope_id
            || run.expected_scope_digest != expected.digest()?
            || run.scope_template_digest != template
            || run.native_run_digest != observations.run.digest()?
            || run.observations_digest
                != canonical_local_digest(LocalDigestRole::ImportedObservations, &observations)?
        {
            return Err(
                "local attestation does not match the submitted native run and scope".into(),
            );
        }
        let maximum_age = Duration::from_secs(
            policy
                .maximum_age_seconds
                .min(snapshot.max_evidence_age_seconds)
                .min(expected.bindings.maximum_age_seconds),
        );
        if [
            &run.started_at,
            &run.completed_at,
            &run.attested_at,
            &expected.bindings.evaluated_at,
        ]
        .into_iter()
        .any(|time| {
            request
                .gate
                .evaluated_at
                .duration_since(time)
                .is_none_or(|age| age > maximum_age)
        }) {
            return Err("local attestation is stale or future dated".into());
        }
        if expected.bindings.evaluated_at < run.completed_at
            || expected.bindings.evaluated_at > run.attested_at
            || observations.run.observations.iter().any(|observation| {
                observation.started_at < run.started_at
                    || observation.completed_at > run.completed_at
                    || observation.observed_at > run.completed_at
            })
        {
            return Err("local observations fall outside the signed run interval".into());
        }
        let result = evaluate_scope(&expected, &observations)?.result;
        if result.status != ScopeStatus::Satisfied || run.result_digest != result.digest()? {
            return Err("local attestation does not match a satisfied native scope".into());
        }
        Ok((signed.statement, expected.requirements))
    }
}

/// Name the first native requirement or observation rule of `check` that the signed scope lacks.
pub(super) fn uncovered(scope: &RequirementsV1, check: &Check) -> Option<String> {
    let Source::Native(native) = &check.source else {
        return Some("local check has no native requirements".into());
    };
    let declared = &native.requirements;
    if let Some(requirement) = declared
        .requirements
        .as_slice()
        .iter()
        .find(|requirement| !scope.requirements.as_slice().contains(requirement))
    {
        return Some(format!(
            "signed local scope does not cover requirement {}",
            requirement.id()
        ));
    }
    declared
        .observations
        .as_slice()
        .iter()
        .find(|rule| !scope.observations.as_slice().contains(rule))
        .map(|rule| format!("signed local scope does not cover observation {}", rule.id))
}

fn read_document<T: DeserializeOwned>(directory: &Path, name: &str) -> Result<T> {
    const LIMIT: u64 = 8 * 1024 * 1024;
    let path = directory.join(name);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("could not read local {name}: {error}"))?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err(format!("local {name} is not a supported JSON file").into());
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(format!("local {name} exceeds the input limit").into());
    }
    Ok(serde_json::from_slice(&bytes).map_err(|error| format!("invalid local {name}: {error}"))?)
}
