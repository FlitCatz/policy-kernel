//! Development-only generation of crate-owned schemas and conformance vectors.
use policy_native_contracts::*;
use serde_json::json;
use std::{error::Error, fs, path::Path};
#[path = "../tests/support/attestation.rs"]
mod attestation;
#[path = "../tests/support/mod.rs"]
mod support;

fn write(path: &Path, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, schema) in [
        ("expected-scope-v1", schemars::schema_for!(ExpectedScopeV1)),
        ("native-run-v1", schemars::schema_for!(NativeRunV1)),
        ("ratchet-rule-v1", schemars::schema_for!(RatchetRuleV1)),
        (
            "ratchet-observation-v1",
            schemars::schema_for!(RatchetObservationV1),
        ),
        ("ratchet-result-v1", schemars::schema_for!(RatchetResultV1)),
        (
            "local-scope-result-v1",
            schemars::schema_for!(LocalScopeResultV1),
        ),
        (
            "local-attestation-v1",
            schemars::schema_for!(LocalAttestationV1),
        ),
        (
            "signed-local-attestation-v1",
            schemars::schema_for!(SignedLocalAttestationV1),
        ),
    ] {
        write(&root.join(format!("schemas/{name}.schema.json")), &schema)?;
    }
    write(&root.join("tests/corpus/v1.json"), &support::corpus())?;
    let input = json!({"z":1,"a":2});
    let vectors = json!({"value":input,
        "command":canonical_digest(DigestRole::Command, &input)?,
        "configuration":canonical_digest(DigestRole::Configuration, &input)?});
    write(&root.join("tests/corpus/canonical-v1.json"), &vectors)?;
    write(
        &root.join("tests/corpus/attestation-v1.json"),
        &attestation::corpus(),
    )?;
    write(
        &root.join("tests/corpus/canonical-attestation-v1.json"),
        &attestation::canonical_vectors(),
    )?;
    Ok(())
}
