//! Identity/attempt projection supplements the shared parser's normalized output.
//! Outcomes, counts, normalized IDs and report validity come only from that parser.
use policy_native_contracts::*;
use policy_report_adapters::{parse_report, ParsedReport, ReportFormat};
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;

fn format(binding: &ParserBindingV1) -> Result<ReportFormat> {
    let format = match binding.id.as_str() {
        "vitest_json" => ReportFormat::VitestJson,
        "jest_json" => ReportFormat::JestJson,
        "playwright_json" => ReportFormat::PlaywrightJson,
        "trx" => ReportFormat::Trx,
        _ => return Err("unsupported parser ID".into()),
    };
    if binding.version.as_str() != format.parser_version() {
        return Err("unsupported parser version".into());
    }
    Ok(format)
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("missing array {key}"))
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string {key}"))
}
fn identity(a: &str, b: &str) -> RecordId {
    RecordId::parse(serde_json::to_string(&[a, b]).expect("string pair serializes"))
        .expect("nonempty identity")
}

pub(crate) fn recompute(
    claim: &TestObservationV1,
    bytes: &[u8],
) -> Result<(ParsedReport, TestObservationV1)> {
    let format = format(&claim.parser)?;
    let parsed =
        parse_report(format, claim.observation_id.as_str(), bytes).map_err(|e| e.to_string())?;
    let mut identities = Vec::new();
    if format == ReportFormat::Trx {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let doc = roxmltree::Document::parse_with_options(
            text,
            roxmltree::ParsingOptions {
                allow_dtd: false,
                nodes_limit: 500_000,
            },
        )
        .map_err(|e| e.to_string())?;
        let ns = "http://microsoft.com/schemas/VisualStudio/TeamTest/2010";
        let results = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name((ns, "Results")))
            .ok_or("missing Results")?;
        for test in results.children().filter(|n| n.is_element()) {
            identities.push((
                identity(
                    test.attribute("testId").ok_or("missing testId")?,
                    test.attribute("testName").ok_or("missing testName")?,
                ),
                u32::from(test.attribute("outcome") != Some("NotExecuted")),
            ));
        }
    } else {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if format == ReportFormat::PlaywrightJson {
            let mut suites: Vec<_> = array(&value, "suites")?.iter().collect();
            while let Some(suite) = suites.pop() {
                if suite.get("suites").is_some() {
                    suites.extend(array(suite, "suites")?);
                }
                for spec in array(suite, "specs")? {
                    for test in array(spec, "tests")? {
                        let results = array(test, "results")?;
                        // A nonzero retry index is incomplete even with a single result.
                        if results.iter().any(|r| r["retry"].as_u64() != Some(0)) {
                            return Err("retry index must be zero".into());
                        }
                        let attempts = if string(test, "status")? == "skipped" {
                            0
                        } else {
                            u32::try_from(results.len()).map_err(|_| "too many attempts")?
                        };
                        identities.push((
                            identity(string(spec, "id")?, string(test, "projectId")?),
                            attempts,
                        ));
                    }
                }
            }
        } else {
            for suite in array(&value, "testResults")? {
                for test in array(suite, "assertionResults")? {
                    // These extensions cannot silently hide retries in Vitest's
                    // assertion-only reporter. Jest is also checked by its parser.
                    for key in ["retryCount", "retries"] {
                        if test.get(key).is_some_and(|v| v.as_u64() != Some(0)) {
                            return Err("retry extension is nonzero or unsupported".into());
                        }
                    }
                    if test
                        .get("invocations")
                        .is_some_and(|v| v.as_u64() != Some(1))
                    {
                        return Err("multiple/invalid invocations".into());
                    }
                    let attempts =
                        u32::from(matches!(string(test, "status")?, "passed" | "failed"));
                    identities.push((
                        identity(string(suite, "name")?, string(test, "fullName")?),
                        attempts,
                    ));
                }
            }
        }
    }
    if identities.len() != parsed.tests.len() {
        return Err("identity projection count differs from parser".into());
    }
    let tests: Vec<_> = parsed
        .tests
        .iter()
        .zip(identities)
        .map(|(test, (raw_identity, attempts))| TestV1 {
            id: RecordId::parse(test.id.clone()).expect("parser supplies nonempty ID"),
            raw_identity,
            attempts,
            outcome: match test.outcome {
                policy_report_adapters::TestOutcome::Passed => TestOutcome::Passed,
                policy_report_adapters::TestOutcome::Failed => TestOutcome::Failed,
                policy_report_adapters::TestOutcome::Skipped => TestOutcome::Skipped,
            },
        })
        .collect();
    let recomputed = TestObservationV1 {
        observation_id: claim.observation_id.clone(),
        parser: claim.parser.clone(),
        raw_report: claim.raw_report.clone(),
        counts: TestCountsV1 {
            discovered: parsed.discovered,
            executed: parsed.executed,
            passed: parsed.passed,
            failed: parsed.failed,
            skipped: parsed.skipped,
            retries: tests
                .iter()
                .map(|t| u64::from(t.attempts.saturating_sub(1)))
                .sum(),
        },
        tests,
    };
    Ok((parsed, recomputed))
}
