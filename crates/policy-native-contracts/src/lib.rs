#![forbid(unsafe_code)]
//! Version 1 native-result records. Types and structural validation only.
//! No execution, raw parsing, filesystem observation, evidence conversion or authorization.
mod digest;
mod types;
mod validation;

pub use digest::*;
pub use types::*;
pub use validation::{validate_subject_binding, ContractError};

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};

/// A nonempty ordered wire set. Duplicate members are rejected, never deduplicated.
/// Array order remains significant to canonical hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RequiredSet<T>(Vec<T>);

impl<T: PartialEq> RequiredSet<T> {
    pub fn new(values: Vec<T>) -> Result<Self, ContractError> {
        if values.is_empty()
            || values
                .iter()
                .enumerate()
                .any(|(i, v)| values[..i].contains(v))
        {
            return Err(ContractError::Invalid(
                "required set is empty or duplicated",
            ));
        }
        Ok(Self(values))
    }
}
impl<T> RequiredSet<T> {
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}
impl<'de, T: Deserialize<'de> + PartialEq> Deserialize<'de> for RequiredSet<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(Vec::<T>::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl<T: JsonSchema> JsonSchema for RequiredSet<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        format!("RequiredSet_{}", T::schema_name()).into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = Vec::<T>::json_schema(generator);
        schema.insert("minItems".into(), 1.into());
        schema.insert("uniqueItems".into(), true.into());
        schema
    }
}

/// An unforgeable version value for each independently versioned v1 document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VersionV1(#[schemars(range(min = 1, max = 1))] u8);
impl Default for VersionV1 {
    fn default() -> Self {
        Self(1)
    }
}
impl<'de> Deserialize<'de> for VersionV1 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match u8::deserialize(d)? {
            1 => Ok(Self(1)),
            _ => Err(serde::de::Error::custom(
                "unsupported native schema version; expected 1",
            )),
        }
    }
}
