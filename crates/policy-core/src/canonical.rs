use policy_contracts::Sha256Digest;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum CanonicalError {
    #[error("value cannot be represented as JSON: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("floating-point numbers are not canonical policy data")]
    FloatingPoint,
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<Sha256Digest, CanonicalError> {
    let value = normalize(serde_json::to_value(value)?)?;
    let bytes = serde_json::to_vec(&value)?;
    let digest = hex::encode(Sha256::digest(bytes));
    Ok(Sha256Digest::parse(digest).expect("SHA-256 always produces a valid digest"))
}

fn normalize(value: Value) -> Result<Value, CanonicalError> {
    match value {
        Value::Number(number) if !number.is_i64() && !number.is_u64() => {
            Err(CanonicalError::FloatingPoint)
        }
        Value::Array(values) => values
            .into_iter()
            .map(normalize)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => {
            let sorted = values
                .into_iter()
                .map(|(key, value)| normalize(value).map(|value| (key, value)))
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            Ok(Value::Object(sorted.into_iter().collect()))
        }
        value => Ok(value),
    }
}
