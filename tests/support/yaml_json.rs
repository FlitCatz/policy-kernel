//! Test-only YAML loading. `yaml-rust2` parses exactly one document with the
//! YAML 1.2 core schema (duplicate mapping keys are rejected), and the result is
//! converted to `serde_json::Value` so existing serde types deserialize it with
//! `serde_json::from_value`, keeping their `deny_unknown_fields` checks.

use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

pub fn yaml_to_json(source: &str) -> Result<Value, String> {
    let mut documents = YamlLoader::load_from_str(source).map_err(|error| error.to_string())?;
    if documents.len() != 1 {
        return Err(format!(
            "expected exactly one YAML document, found {}",
            documents.len()
        ));
    }
    convert(documents.remove(0))
}

fn convert(node: Yaml) -> Result<Value, String> {
    Ok(match node {
        Yaml::Null => Value::Null,
        Yaml::Boolean(value) => Value::Bool(value),
        Yaml::Integer(value) => Value::from(value),
        real @ Yaml::Real(_) => real
            .as_f64()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| format!("YAML float {real:?} has no JSON representation"))?,
        Yaml::String(value) => Value::String(value),
        Yaml::Array(items) => {
            Value::Array(items.into_iter().map(convert).collect::<Result<_, _>>()?)
        }
        Yaml::Hash(entries) => {
            let mut object = Map::new();
            for (key, value) in entries {
                let Yaml::String(key) = key else {
                    return Err(format!("YAML mapping key {key:?} is not a string"));
                };
                object.insert(key, convert(value)?);
            }
            Value::Object(object)
        }
        Yaml::Alias(_) | Yaml::BadValue => {
            return Err("unresolved YAML alias or invalid YAML value".into())
        }
    })
}
