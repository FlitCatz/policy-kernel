#![forbid(unsafe_code)]

use policy_contracts::{
    EvidenceBundle, PolicySnapshot, PostMergeDecision, PreMergeDecision, SensorProfile,
};
use schemars::JsonSchema;
use serde_json::Value;
use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [command] if command == "schema" => generate_schemas(),
        _ => Err("usage: cargo run -p xtask -- schema".into()),
    }
}

fn generate_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask belongs inside the workspace")
        .join("schemas");
    fs::create_dir_all(&schema_dir)?;

    write_schema::<PolicySnapshot>(&schema_dir, "policy.schema.json")?;
    write_schema::<EvidenceBundle>(&schema_dir, "evidence-bundle.schema.json")?;
    write_schema::<SensorProfile>(&schema_dir, "sensor-profile.schema.json")?;
    write_schema::<PreMergeDecision>(&schema_dir, "pre-merge-decision.schema.json")?;
    write_schema::<PostMergeDecision>(&schema_dir, "post-merge-decision.schema.json")?;
    Ok(())
}

fn write_schema<T: JsonSchema>(directory: &Path, file: &str) -> Result<(), std::io::Error> {
    let schema = schemars::schema_for!(T);
    let value = sort_json(serde_json::to_value(schema).expect("schema is JSON serializable"));
    let mut bytes = serde_json::to_vec_pretty(&value).expect("schema is JSON serializable");
    bytes.push(b'\n');
    fs::write(directory.join(file), bytes)
}

fn sort_json(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, sort_json(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json).collect()),
        scalar => scalar,
    }
}
