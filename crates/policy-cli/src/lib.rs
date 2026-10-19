#![forbid(unsafe_code)]
//! Byte-oriented local scope command shared with adapter conformance tests.
use policy_native_contracts::{ExpectedScopeV1, LocalScopeResultV1, ScopeStatus};
use policy_native_verifier::{evaluate_scope, ImportedObservationsV1};

/// JSON-only consistency evaluation, with no filesystem or raw report observation.
/// The caller obtains the approved expectation independently of the import output.
pub fn evaluate_scope_json(
    expected: &[u8],
    observations: &[u8],
) -> Result<(LocalScopeResultV1, u8), String> {
    let expected: ExpectedScopeV1 =
        serde_json::from_slice(expected).map_err(|e| format!("invalid expected scope: {e}"))?;
    let observations: ImportedObservationsV1 = serde_json::from_slice(observations)
        .map_err(|e| format!("invalid imported observations: {e}"))?;
    let result = evaluate_scope(&expected, &observations)
        .map_err(|e| e.to_string())?
        .result;
    let exit = match result.status {
        ScopeStatus::Satisfied => 0,
        ScopeStatus::NonPass => 2,
        ScopeStatus::Error => 64,
    };
    Ok((result, exit))
}
