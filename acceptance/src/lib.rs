use std::collections::BTreeMap;

pub mod execution;
mod failure;
pub mod harness;
mod process;
mod staging;
pub mod tjerp;
pub use staging::{stage, stage_profile};

pub fn negative_verified(
    baseline: &BTreeMap<String, bool>,
    negative: &BTreeMap<String, bool>,
    expected: &str,
) -> bool {
    negative_verified_set(baseline, negative, &[expected])
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn negative_verified_set(
    baseline: &BTreeMap<String, bool>,
    negative: &BTreeMap<String, bool>,
    expected: &[&str],
) -> bool {
    !baseline.is_empty()
        && !expected.is_empty()
        && baseline.values().all(|passed| *passed)
        && baseline.keys().eq(negative.keys())
        && expected
            .iter()
            .all(|id| baseline.get(*id) == Some(&true) && negative.get(*id) == Some(&false))
        && negative
            .iter()
            .all(|(id, passed)| expected.contains(&id.as_str()) || *passed)
}
