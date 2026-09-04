#![forbid(unsafe_code)]

mod canonical;
mod provenance;

pub use canonical::{canonical_sha256, CanonicalError};
pub use provenance::{
    validate_bundle, validate_provenance, verifier_is_independent, ValidationResult,
};
