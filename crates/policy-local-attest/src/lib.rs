#![forbid(unsafe_code)]

#[cfg(unix)]
mod gate;
#[cfg(unix)]
pub use gate::{GateAttestation, GateStatement, SigningKey, DEFAULT_SSH_KEYGEN};
