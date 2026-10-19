//! Development-only generation of crate-owned schemas and conformance vectors.
use policy_native_contracts::*;
use serde_json::json;
use std::{error::Error, fs, path::Path};
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
        (
            "local-scope-result-v1",
            schemars::schema_for!(LocalScopeResultV1),
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
    Ok(())
}
