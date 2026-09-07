use chrono::{DateTime, SecondsFormat, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("{kind} must not be empty")]
    Empty { kind: &'static str },
    #[error("repository must have owner/name form")]
    InvalidRepository,
    #[error("{kind} must contain exactly {expected} lowercase hexadecimal characters")]
    InvalidHex { kind: &'static str, expected: usize },
    #[error("pull request number must be positive")]
    InvalidPullRequestNumber,
    #[error("timestamp must be RFC 3339 with a UTC offset")]
    InvalidTimestamp,
}

macro_rules! string_id {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(#[schemars(regex(pattern = r"\S"))] String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdentityError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(IdentityError::Empty { kind: $kind });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(WorkItemId, "work item ID");
string_id!(ProviderId, "provider ID");
string_id!(ModelId, "model ID");
string_id!(ExecutionId, "execution ID");
string_id!(WorkflowRunId, "workflow run ID");
string_id!(SensorId, "sensor ID");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct RepositoryId(#[schemars(regex(pattern = r"^[^/]+/[^/]+$"))] String);

impl RepositoryId {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityError> {
        let value = value.into();
        let mut parts = value.split('/');
        if parts.next().is_some_and(|part| !part.is_empty())
            && parts.next().is_some_and(|part| !part.is_empty())
            && parts.next().is_none()
        {
            Ok(Self(value))
        } else {
            Err(IdentityError::InvalidRepository)
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RepositoryId {
    type Error = IdentityError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for RepositoryId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for RepositoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

macro_rules! hex_id {
    ($name:ident, $kind:literal, $length:expr, $pattern:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(#[schemars(regex(pattern = $pattern))] String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdentityError> {
                let value = value.into();
                if value.len() != $length
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(IdentityError::InvalidHex {
                        kind: $kind,
                        expected: $length,
                    });
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

hex_id!(CommitSha, "commit SHA", 40, r"^[0-9a-f]{40}$");
hex_id!(Sha256Digest, "SHA-256 digest", 64, r"^[0-9a-f]{64}$");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct PullRequestNumber(#[schemars(range(min = 1))] u64);

impl PullRequestNumber {
    pub fn new(value: u64) -> Result<Self, IdentityError> {
        (value > 0)
            .then_some(Self(value))
            .ok_or(IdentityError::InvalidPullRequestNumber)
    }
    pub fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PullRequestNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(schema_with = "utc_timestamp_schema")]
pub struct UtcTimestamp(DateTime<Utc>);

fn utc_timestamp_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "string", "format": "date-time"})
}

impl UtcTimestamp {
    pub fn parse(value: &str) -> Result<Self, IdentityError> {
        DateTime::parse_from_rfc3339(value)
            .map(|timestamp| Self(timestamp.with_timezone(&Utc)))
            .map_err(|_| IdentityError::InvalidTimestamp)
    }
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl FromStr for UtcTimestamp {
    type Err = IdentityError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for UtcTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339_opts(SecondsFormat::AutoSi, true))
    }
}

impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
