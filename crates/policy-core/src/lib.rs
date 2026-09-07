#![forbid(unsafe_code)]

mod canonical;
mod pre_merge;
mod pre_merge_digest;
mod provenance;
mod reviews;
mod sensors;

pub use canonical::{canonical_sha256, CanonicalError};
pub use pre_merge::evaluate_pre_merge;
pub use pre_merge_digest::{pre_merge_evidence_digest, pre_merge_policy_digest};
pub use provenance::{
    validate_bundle, validate_provenance, verifier_is_independent, ValidationResult,
};
pub use reviews::{evaluate_reviews, ReviewEvaluation, ReviewEvaluationInput};
pub use sensors::{
    evaluate_sensor_evidence, evaluate_sensor_evidence_for_change, resolve_required_sensors,
    SensorEvaluation, SensorEvaluationStatus, SensorResolutionError,
};
