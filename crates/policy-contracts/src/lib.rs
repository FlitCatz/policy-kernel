#![forbid(unsafe_code)]

pub mod decision;
pub mod evidence;
pub mod github;
pub mod identity;
pub mod policy;

pub use decision::*;
pub use evidence::*;
pub use github::*;
pub use identity::*;
pub use policy::*;

pub const SCHEMA_VERSION: u32 = 1;

pub(crate) fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let version = u32::deserialize(deserializer)?;
    if version == SCHEMA_VERSION {
        Ok(version)
    } else {
        Err(serde::de::Error::custom(format!(
            "unsupported schema version {version}; expected {SCHEMA_VERSION}"
        )))
    }
}
