//! Additive asset generation; never rewrites existing v1 schemas or corpus.
use policy_native_contracts::*;
use std::{error::Error, fs, path::Path};
#[path = "../tests/support/attestation.rs"]
mod attestation;
#[path = "../tests/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, value) in [
        (
            "schemas/local-attestation-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(LocalAttestationV1))?,
        ),
        (
            "schemas/signed-local-attestation-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(SignedLocalAttestationV1))?,
        ),
        ("tests/corpus/attestation-v1.json", attestation::corpus()),
        (
            "tests/corpus/canonical-attestation-v1.json",
            attestation::canonical_vectors(),
        ),
    ] {
        fs::write(
            root.join(path),
            format!("{}\n", serde_json::to_string_pretty(&value)?),
        )?;
    }
    Ok(())
}
