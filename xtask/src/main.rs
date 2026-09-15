#![forbid(unsafe_code)]

use policy_contracts::{
    EvidenceBundle, PolicySnapshot, PostMergeDecision, PreMergeDecision, SensorProfile,
};
use schemars::JsonSchema;
use serde_json::{Map, Value};
use std::{env, fs, path::Path};
mod verification;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command] if command == "schema" => generate_schemas().map_err(|e| e.to_string()),
        [command, path] if command == "check-mutation-results" => verification::mutation(Path::new(path)),
        [command, path, flag, scope] if command == "check-mutation-results" && flag == "--scope" => verification::mutation_scope(Path::new(path), scope),
        [command, path] if command == "check-residue" => verification::residue(Path::new(path)),
        [command] if command == "check-residue" => verification::residue(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").as_path()),
        [command, docs, schemas] if command == "check-public-api" => verification::public_api(Path::new(docs), Path::new(schemas)),
        [command, rest @ ..] if command == "rollback-drill" => verification::rollback(rest),
        _ => Err("usage: xtask schema | check-mutation-results OUTCOMES [--scope core|controller|workflow-source] | check-residue [REPO] | check-public-api DOCS SCHEMAS | rollback-drill --baseline FILE --promoted FILE --restored FILE --evidence-before FILE --evidence-after FILE".into()),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            std::process::ExitCode::from(if args.first().is_some_and(|s| s == "rollback-drill") {
                64
            } else {
                1
            })
        }
    }
}

fn generate_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas");
    fs::create_dir_all(&output_dir)?;
    write_schema::<PolicySnapshot>(&output_dir, "policy.schema.json")?;
    write_schema::<EvidenceBundle>(&output_dir, "evidence-bundle.schema.json")?;
    write_schema::<SensorProfile>(&output_dir, "sensor-profile.schema.json")?;
    write_schema::<PreMergeDecision>(&output_dir, "pre-merge-decision.schema.json")?;
    write_schema::<PostMergeDecision>(&output_dir, "post-merge-decision.schema.json")?;
    Ok(())
}

fn write_schema<T: JsonSchema>(output_dir: &Path, name: &str) -> Result<(), std::io::Error> {
    let mut value = serde_json::to_value(schemars::schema_for!(T))
        .expect("generated JSON Schema must serialize");
    add_standard_integer_bounds(&mut value);
    sort_json_keys(&mut value);
    let mut bytes = serde_json::to_vec_pretty(&value).expect("JSON Schema value must serialize");
    bytes.push(b'\n');
    fs::write(output_dir.join(name), bytes)
}

fn add_standard_integer_bounds(value: &mut Value) {
    match value {
        Value::Object(object) => {
            match object.get("format").and_then(Value::as_str) {
                Some("uint64") => {
                    object.entry("minimum").or_insert_with(|| 0_u64.into());
                    object.entry("maximum").or_insert_with(|| u64::MAX.into());
                }
                Some("int32") => {
                    object.entry("minimum").or_insert_with(|| i32::MIN.into());
                    object.entry("maximum").or_insert_with(|| i32::MAX.into());
                }
                _ => {}
            }
            object.values_mut().for_each(add_standard_integer_bounds);
        }
        Value::Array(items) => items.iter_mut().for_each(add_standard_integer_bounds),
        _ => {}
    }
}

fn sort_json_keys(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<_> = std::mem::take(object).into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, mut child) in entries {
                sort_json_keys(&mut child);
                sorted.insert(key, child);
            }
            *object = sorted;
        }
        Value::Array(items) => items.iter_mut().for_each(sort_json_keys),
        _ => {}
    }
}
