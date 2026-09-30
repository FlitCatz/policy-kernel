#![forbid(unsafe_code)]

pub mod adapter_contracts;
pub mod consumer;
mod paths;
pub mod profile;
pub mod reports;
pub mod run;
pub mod selection;
pub mod tool_versions;

pub use profile::{parse_profile, ProjectProfile};
pub use run::{run, RunRequest, RunResult};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid project verification input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Digest(#[from] policy_core::CanonicalError),
    #[error(transparent)]
    Collection(#[from] evidence_collector::CollectionError),
}

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

pub fn bytes_digest(bytes: &[u8]) -> policy_contracts::Sha256Digest {
    use sha2::{Digest, Sha256};
    policy_contracts::Sha256Digest::parse(hex::encode(Sha256::digest(bytes)))
        .expect("SHA-256 is a valid digest")
}
