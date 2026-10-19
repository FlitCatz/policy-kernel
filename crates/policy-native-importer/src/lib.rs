#![forbid(unsafe_code)]
//! Adapter-owned verification of already-executed native runs, on Unix.
//! No execution, source observation, scope decision or core evidence conversion.
#[cfg(unix)]
mod fs;
#[cfg(unix)]
mod reports;

#[cfg(unix)]
mod importer;
#[cfg(unix)]
pub use importer::*;
