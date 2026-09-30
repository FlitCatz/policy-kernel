//! Project-owned version acceptance, independent of adapter compatibility.
use crate::{
    invalid,
    selection::{version_matches, VersionNormalization},
    Error,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VersionAcceptance {
    Record {},
    Exact { value: String },
    NumericRange { requirement: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectVersionRule {
    pub normalization: VersionNormalization,
    pub acceptance: VersionAcceptance,
}

pub fn version_token(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 128
        || !value.as_bytes()[0].is_ascii_digit()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".+_-".contains(&byte))
    {
        return Err(invalid("invalid observed version token"));
    }
    Ok(())
}

impl VersionAcceptance {
    pub fn validate(&self) -> Result<(), Error> {
        match self {
            Self::Record {} => Ok(()),
            Self::Exact { value } => version_token(value),
            Self::NumericRange { requirement } => {
                version_matches(requirement, "0.0.0")?;
                Ok(())
            }
        }
    }

    pub fn accepts(&self, actual: &str) -> Result<bool, Error> {
        version_token(actual)?;
        match self {
            Self::Record {} => Ok(true),
            Self::Exact { value } => Ok(actual == value),
            Self::NumericRange { requirement } => version_matches(requirement, actual),
        }
    }
}

impl ProjectVersionRule {
    pub fn validate(&self) -> Result<(), Error> {
        self.acceptance.validate()
    }

    pub fn observe(&self, stdout: &[u8]) -> Result<String, Error> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| invalid("version output is not UTF-8"))?
            .trim();
        let token = match self.normalization {
            VersionNormalization::Plain => text,
            VersionNormalization::Node => text
                .strip_prefix('v')
                .ok_or_else(|| invalid("Node version requires one leading v"))?,
        };
        if !self.acceptance.accepts(token)? {
            return Err(invalid(format!(
                "version {token} is outside project acceptance"
            )));
        }
        Ok(token.into())
    }
}
