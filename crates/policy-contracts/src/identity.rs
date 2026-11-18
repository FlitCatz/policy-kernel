use jiff::{civil::DateTime, SignedDuration};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    str::FromStr,
    time::{Duration, SystemTime},
};

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
        pub struct $name(
            #[schemars(regex(pattern = r"[^\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]"))]
            String,
        );

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
pub struct PullRequestNumber(#[schemars(schema_with = "pull_request_number_schema")] u64);

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

// A UTC instant with nanosecond precision from `0000-01-01T00:00:00Z` through
// `9999-12-31T23:59:59.999999999Z`, so its canonical form always parses back.
// The inner value is a civil date-time on the UTC time scale; no time library
// type appears in the public API. Not a doc comment: schemars would copy one
// into every generated schema as a description.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(schema_with = "utc_timestamp_schema")]
pub struct UtcTimestamp(DateTime);

const UNIX_EPOCH: UtcTimestamp = UtcTimestamp(DateTime::constant(1970, 1, 1, 0, 0, 0, 0));

fn utc_timestamp_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "string", "format": "date-time"})
}

fn pull_request_number_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "minimum": 1,
        "maximum": u64::MAX
    })
}

impl UtcTimestamp {
    /// Parses an RFC 3339 `date-time` and normalizes it to UTC.
    ///
    /// The accepted grammar is deliberately narrower than ISO 8601: a four-digit
    /// year, `T`, `t` or a space before the time, `Z`, `z` or a numeric offset of
    /// at most 23:59, and any number of fractional digits (truncated to
    /// nanoseconds). A leap second (`:60`) and an instant whose UTC year falls
    /// outside 0000-9999 are rejected because neither has a canonical form here.
    pub fn parse(value: &str) -> Result<Self, IdentityError> {
        parse_rfc3339(value).ok_or(IdentityError::InvalidTimestamp)
    }

    /// The instant `millis` milliseconds after (or before) the Unix epoch.
    pub fn from_unix_millis(millis: i64) -> Option<Self> {
        let distance = Duration::from_millis(millis.unsigned_abs());
        if millis < 0 {
            UNIX_EPOCH.checked_sub(distance)
        } else {
            UNIX_EPOCH.checked_add(distance)
        }
    }

    /// Time elapsed from `earlier` to `self`, or `None` when `earlier` is later.
    pub fn duration_since(&self, earlier: &Self) -> Option<Duration> {
        Duration::try_from(self.0.duration_since(earlier.0)).ok()
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        Self::in_range(self.0.checked_add(duration).ok()?)
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        Self::in_range(self.0.checked_sub(duration).ok()?)
    }

    /// Drops the fractional second without rounding.
    pub fn truncated_to_seconds(&self) -> Self {
        let whole = self.0.with().subsec_nanosecond(0).build();
        Self(whole.expect("a zero fractional second is always valid"))
    }

    // The civil type also covers years before 0000, which the canonical
    // four-digit form cannot express.
    fn in_range(value: DateTime) -> Option<Self> {
        (value.year() >= 0).then_some(Self(value))
    }
}

fn number<T: TryFrom<u32>>(bytes: &[u8], start: usize, length: usize) -> Option<T> {
    let digits = bytes.get(start..start + length)?;
    let value = digits.iter().try_fold(0u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + u32::from(byte - b'0'))
    })?;
    T::try_from(value).ok()
}

fn parse_rfc3339(value: &str) -> Option<UtcTimestamp> {
    let bytes = value.as_bytes();
    let separated = bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && matches!(bytes.get(10), Some(b'T' | b't' | b' '))
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':');
    if !separated {
        return None;
    }
    let mut rest = bytes.get(19..)?;
    let mut nanosecond = 0;
    if let Some(fraction) = rest.strip_prefix(b".") {
        let length = fraction.iter().take_while(|b| b.is_ascii_digit()).count();
        // Digits beyond nanosecond precision are dropped, not rounded.
        let kept = length.clamp(1, 9);
        nanosecond = number::<i32>(fraction, 0, kept)? * 10i32.pow(9 - kept as u32);
        rest = &fraction[length..];
    }
    let offset_seconds = match rest {
        [b'Z' | b'z'] => 0,
        _ => {
            let (negative, offset) = match rest.strip_prefix(b"+") {
                Some(offset) => (false, offset),
                // U+2212 MINUS SIGN is accepted like the ASCII hyphen.
                None => (
                    true,
                    rest.strip_prefix(b"-")
                        .or_else(|| rest.strip_prefix("\u{2212}".as_bytes()))?,
                ),
            };
            if offset.len() != 5 || offset[2] != b':' {
                return None;
            }
            let hours = number::<i64>(offset, 0, 2).filter(|hours| *hours <= 23)?;
            let minutes = number::<i64>(offset, 3, 2).filter(|minutes| *minutes <= 59)?;
            let seconds = hours * 3600 + minutes * 60;
            if negative {
                -seconds
            } else {
                seconds
            }
        }
    };
    let local = DateTime::new(
        number(bytes, 0, 4)?,
        number(bytes, 5, 2)?,
        number(bytes, 8, 2)?,
        number(bytes, 11, 2)?,
        number(bytes, 14, 2)?,
        number(bytes, 17, 2)?,
        nanosecond,
    )
    .ok()?;
    let utc = local
        .checked_sub(SignedDuration::from_secs(offset_seconds))
        .ok()?;
    UtcTimestamp::in_range(utc)
}

impl TryFrom<SystemTime> for UtcTimestamp {
    type Error = IdentityError;
    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(after) => UNIX_EPOCH.checked_add(after),
            Err(before) => UNIX_EPOCH.checked_sub(before.duration()),
        }
        .ok_or(IdentityError::InvalidTimestamp)
    }
}

/// The canonical form: RFC 3339 with a `Z` suffix and 0, 3, 6 or 9 fractional
/// digits, whichever is the shortest that loses nothing.
impl fmt::Display for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time = self.0;
        write!(
            formatter,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second()
        )?;
        match time.subsec_nanosecond() {
            0 => {}
            nanos if nanos % 1_000_000 == 0 => write!(formatter, ".{:03}", nanos / 1_000_000)?,
            nanos if nanos % 1_000 == 0 => write!(formatter, ".{:06}", nanos / 1_000)?,
            nanos => write!(formatter, ".{nanos:09}")?,
        }
        formatter.write_str("Z")
    }
}

impl fmt::Debug for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("UtcTimestamp")
            .field(&format_args!("{self}"))
            .finish()
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
        serializer.collect_str(self)
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
