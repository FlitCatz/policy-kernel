#![forbid(unsafe_code)]

mod canonical;
mod provenance;
mod reviews;
mod sensors;

pub use canonical::{canonical_sha256, CanonicalError};
pub use provenance::{
    validate_bundle, validate_provenance, verifier_is_independent, ValidationResult,
};
pub use reviews::{evaluate_reviews, ReviewEvaluation, ReviewEvaluationInput};
pub use sensors::{
    evaluate_sensor_evidence, evaluate_sensor_evidence_for_change, resolve_required_sensors,
    SensorEvaluation, SensorEvaluationStatus, SensorResolutionError,
};
