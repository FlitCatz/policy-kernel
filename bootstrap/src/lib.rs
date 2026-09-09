//! Local configuration installation. This crate makes no policy decisions.
#![cfg(unix)]

mod fs;
mod plan;
mod templates;
mod transaction;

pub use plan::{inspect, plan, Action, Change, Installation, Operation, Plan, Request};
pub use transaction::{apply, recover};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn digest(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(bytes))
}

fn json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}
