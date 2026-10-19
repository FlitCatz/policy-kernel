use crate::{CanonicalDigest, RawByteDigest, RequiredSet, VersionV1};
use policy_contracts::{CommitSha, ExecutionId, RepositoryId, SensorId, UtcTimestamp, WorkItemId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Opaque nonblank IDs reuse the existing contract's validated string representation.
pub type RecordId = WorkItemId;

// Top-level deserialization runs the same structural checks available to Rust callers.
macro_rules! document {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Wire { $($field: $ty),* }
                let Wire { $($field),* } = Wire::deserialize(d)?;
                let value = Self { $($field),* };
                value.validate().map_err(serde::de::Error::custom)?;
                Ok(value)
            }
        }
    };
}

document!(ExpectedScopeV1 {
    schema_version: VersionV1,
    subject: SubjectV1,
    requirements: RequirementsV1,
    bindings: BindingsV1,
});
document!(NativeRunV1 {
    schema_version: VersionV1,
    subject: RunSubjectV1,
    observations: Vec<ObservationV1>,
    test_observations: Vec<TestObservationV1>,
    artifacts: Vec<ArtifactV1>,
    mappings: Vec<MappingV1>,
    exclusions: Vec<ExclusionV1>,
});
document!(LocalScopeResultV1 {
    schema_version: VersionV1,
    status: ScopeStatus,
    subject: RunSubjectV1,
    requirements: Vec<RequirementResultV1>,
    reasons: Vec<ReasonV1>,
    digests: ResultDigestsV1,
    evaluated_at: UtcTimestamp,
    verifier_version: RecordId,
    adapter_version: RecordId,
    trust: LocalTrust,
    review: ReviewGate,
    exclusions: Vec<ExclusionV1>,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubjectV1 {
    pub repository: RepositoryId,
    pub head: CommitSha,
    pub source_set_digest: CanonicalDigest,
    pub source_inventory_digest: RawByteDigest,
    pub scope_definition_digest: CanonicalDigest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunSubjectV1 {
    pub source: SubjectV1,
    pub run_id: ExecutionId,
    pub scope_id: RecordId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementKind {
    AcceptanceCriterion,
    ChangedBehavior,
    NegativeCase,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementV1 {
    pub id: RecordId,
    pub kind: RequirementKind,
    pub minimum_executed_tests: u64,
    pub suites: RequiredSet<SensorId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementsV1 {
    pub approved_file_digest: RawByteDigest,
    pub observations: RequiredSet<ObservationRuleV1>,
    pub requirements: RequiredSet<RequirementV1>,
    pub exemptions: Vec<ExclusionV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationRuleV1 {
    pub id: SensorId,
    pub accepted_exit_codes: RequiredSet<i32>,
    /// None identifies a process sensor; Some identifies a test suite.
    pub tests: Option<TestRuleV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestRuleV1 {
    pub minimum_discovered: u64,
    pub minimum_executed: u64,
    pub minimum_passed: u64,
    pub maximum_failed: u64,
    pub maximum_skipped: u64,
    pub maximum_retries: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingsV1 {
    pub observations: RequiredSet<ObservationBindingV1>,
    pub evaluated_at: UtcTimestamp,
    pub maximum_age_seconds: u64,
    pub mode: LocalMode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalMode {
    LocalAdvisory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationBindingV1 {
    pub id: SensorId,
    pub command_digest: CanonicalDigest,
    pub configuration_digest: CanonicalDigest,
    pub parser: Option<ParserBindingV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParserBindingV1 {
    pub id: RecordId,
    pub version: RecordId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandV1 {
    #[schemars(length(min = 1))]
    pub argv: Vec<String>,
    pub cwd: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationV1 {
    pub id: SensorId,
    pub execution_id: ExecutionId,
    pub subject: RunSubjectV1,
    pub command: CommandV1,
    pub configuration_digest: CanonicalDigest,
    pub started_at: UtcTimestamp,
    pub completed_at: UtcTimestamp,
    pub observed_at: UtcTimestamp,
    pub completion: CompletionV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompletionV1 {
    Exited { code: i32 },
    TimedOut { detail: RecordId },
    Interrupted { detail: RecordId },
    Error { detail: RecordId },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestObservationV1 {
    pub observation_id: SensorId,
    pub parser: ParserBindingV1,
    pub raw_report: ArtifactRefV1,
    pub counts: TestCountsV1,
    pub tests: Vec<TestV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestCountsV1 {
    pub discovered: u64,
    pub executed: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub retries: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestV1 {
    pub id: RecordId,
    /// Original producer identity, including absolute paths if emitted by the producer.
    pub raw_identity: RecordId,
    pub outcome: TestOutcome,
    pub attempts: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
    NotExecuted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactV1 {
    pub id: RecordId,
    pub path: String,
    pub role: ArtifactRole,
    pub digest: RawByteDigest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    RawReport,
    CommandLog,
    SourceInventory,
    Configuration,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRefV1 {
    pub id: RecordId,
    pub digest: RawByteDigest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestRefV1 {
    pub suite_id: SensorId,
    pub test_id: RecordId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MappingV1 {
    pub requirement_id: RecordId,
    pub tests: RequiredSet<TestRefV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExclusionV1 {
    pub id: RecordId,
    pub scope: RecordId,
    pub reason: RecordId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeStatus {
    Satisfied,
    NonPass,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalTrust {
    LocalUntrusted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewGate {
    Unmet,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementResultV1 {
    pub id: RecordId,
    pub status: ScopeStatus,
    pub reasons: Vec<ReasonV1>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReasonV1 {
    pub code: ReasonCode,
    pub location: LocationV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReasonCode {
    MissingObservation,
    SubjectMismatch,
    StaleObservation,
    BindingMismatch,
    ArtifactMismatch,
    InvalidReport,
    UnexecutedMapping,
    ScopeUnsatisfied,
    IncompleteRun,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocationV1 {
    pub suite_id: Option<SensorId>,
    pub requirement_id: Option<RecordId>,
    pub artifact_id: Option<RecordId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResultDigestsV1 {
    pub expected_scope: CanonicalDigest,
    pub native_run: CanonicalDigest,
    pub policy: CanonicalDigest,
    /// Absent when no valid normalized evidence exists.
    pub evidence: Option<CanonicalDigest>,
}
