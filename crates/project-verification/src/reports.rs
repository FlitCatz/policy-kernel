//! Strict adapters for the pinned reporter formats documented in report-formats.md.
use std::collections::BTreeSet;

use policy_contracts::UtcTimestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    VitestJson,
    PlaywrightJson,
    Trx,
}

impl ReportFormat {
    pub fn parser_version(self) -> &'static str {
        match self {
            Self::VitestJson => "vitest-json-3.2.4/v1",
            Self::PlaywrightJson => "playwright-json-1.55.0/v1",
            Self::Trx => "vstest-trx-17.14.1/v1",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedReport {
    pub discovered: u64,
    pub executed: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub tests: Vec<TestResult>,
    pub messages: Vec<String>,
    pub successful: bool,
    pub started_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub outcome: TestOutcome,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid report: {0}")]
pub struct ReportError(pub String);

fn invalid(message: impl Into<String>) -> ReportError {
    ReportError(message.into())
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, ReportError> {
    v.get(key).ok_or_else(|| invalid(format!("missing {key}")))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, ReportError> {
    field(v, key)?
        .as_array()
        .ok_or_else(|| invalid(format!("invalid array {key}")))
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, ReportError> {
    field(v, key)?
        .as_str()
        .ok_or_else(|| invalid(format!("invalid string {key}")))
}
fn count(v: &Value, key: &str) -> Result<u64, ReportError> {
    field(v, key)?
        .as_u64()
        .ok_or_else(|| invalid(format!("invalid count {key}")))
}
fn timestamp(s: &str) -> Result<UtcTimestamp, ReportError> {
    UtcTimestamp::parse(s).map_err(|_| invalid("invalid timestamp"))
}
fn id(namespace: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{namespace}:{}", hex::encode(digest.finalize()))
}
fn finish(
    tests: Vec<TestResult>,
    started_at: UtcTimestamp,
    clean: bool,
) -> Result<ParsedReport, ReportError> {
    let mut ids = BTreeSet::new();
    if tests.iter().any(|t| !ids.insert(&t.id)) {
        return Err(invalid("duplicate test identity"));
    }
    let passed = tests
        .iter()
        .filter(|t| t.outcome == TestOutcome::Passed)
        .count() as u64;
    let failed = tests
        .iter()
        .filter(|t| t.outcome == TestOutcome::Failed)
        .count() as u64;
    let skipped = tests.len() as u64 - passed - failed;
    Ok(ParsedReport {
        discovered: tests.len() as u64,
        executed: passed + failed,
        passed,
        failed,
        skipped,
        successful: clean && !tests.is_empty() && failed == 0 && skipped == 0,
        tests,
        messages: vec![],
        started_at,
    })
}
fn same(actual: u64, expected: u64) -> Result<(), ReportError> {
    if actual == expected {
        Ok(())
    } else {
        Err(invalid("summary count mismatch"))
    }
}
pub fn parse_report(
    format: ReportFormat,
    namespace: &str,
    bytes: &[u8],
) -> Result<ParsedReport, ReportError> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("report exceeds 16 MiB"));
    }
    if namespace.is_empty() || namespace.len() > 256 || namespace.chars().any(char::is_control) {
        return Err(invalid("invalid namespace"));
    }
    if format == ReportFormat::Trx {
        return trx(namespace, bytes);
    }
    let v: Value = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    match format {
        ReportFormat::VitestJson => vitest(namespace, &v),
        ReportFormat::PlaywrightJson => playwright(namespace, &v),
        ReportFormat::Trx => unreachable!(),
    }
}

fn vitest(namespace: &str, v: &Value) -> Result<ParsedReport, ReportError> {
    let millis = field(v, "startTime")?
        .as_i64()
        .ok_or_else(|| invalid("invalid startTime"))?;
    let date = chrono::DateTime::from_timestamp_millis(millis)
        .ok_or_else(|| invalid("invalid startTime"))?;
    let started_at = timestamp(&date.to_rfc3339())?;
    let mut clean = field(v, "success")?
        .as_bool()
        .ok_or_else(|| invalid("invalid success"))?;
    let mut tests = vec![];
    let mut messages = vec![];
    let mut todo = 0;
    for suite in array(v, "testResults")? {
        let file = string(suite, "name")?;
        clean &= match string(suite, "status")? {
            "passed" => true,
            "failed" => false,
            _ => return Err(invalid("invalid suite status")),
        };
        let suite_message = string(suite, "message")?;
        clean &= suite_message.is_empty();
        if !suite_message.is_empty() {
            messages.push(suite_message.to_owned());
        }
        for test in array(suite, "assertionResults")? {
            let status = string(test, "status")?;
            let outcome = match status {
                "passed" => TestOutcome::Passed,
                "failed" => TestOutcome::Failed,
                "todo" => {
                    todo += 1;
                    TestOutcome::Skipped
                }
                "skipped" | "pending" | "disabled" => TestOutcome::Skipped,
                _ => return Err(invalid("invalid assertion status")),
            };
            let mut messages = match field(test, "failureMessages")? {
                Value::Null => vec![],
                Value::Array(values) => values
                    .iter()
                    .map(|x| {
                        x.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| invalid("invalid failure message"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(invalid("invalid failureMessages")),
            };
            if !suite_message.is_empty() {
                messages.push(suite_message.into());
            }
            if outcome == TestOutcome::Skipped {
                messages.push(format!("Vitest status: {status}"));
            }
            if outcome == TestOutcome::Passed && !messages.is_empty() {
                clean = false;
            }
            let title = string(test, "fullName")?;
            if file.is_empty() || title.is_empty() {
                return Err(invalid("empty test identity"));
            }
            tests.push(TestResult {
                id: id(namespace, &[file, title]),
                outcome,
                messages,
            });
        }
    }
    let report = finish(tests, started_at, clean)?;
    same(count(v, "numTotalTests")?, report.discovered)?;
    same(count(v, "numPassedTests")?, report.passed)?;
    same(count(v, "numFailedTests")?, report.failed)?;
    same(count(v, "numTodoTests")?, todo)?;
    same(count(v, "numPendingTests")?, report.skipped - todo)?;
    let total = count(v, "numTotalTestSuites")?;
    if total < array(v, "testResults")?.len() as u64 {
        return Err(invalid("suite total smaller than reported files"));
    }
    let passed = count(v, "numPassedTestSuites")?;
    let failed = count(v, "numFailedTestSuites")?;
    let pending = count(v, "numPendingTestSuites")?;
    same(
        total,
        passed
            .checked_add(failed)
            .and_then(|n| n.checked_add(pending))
            .ok_or_else(|| invalid("counter overflow"))?,
    )?;
    Ok(ParsedReport {
        successful: report.successful && failed == 0 && pending == 0,
        messages,
        ..report
    })
}

fn playwright(namespace: &str, v: &Value) -> Result<ParsedReport, ReportError> {
    let stats = field(v, "stats")?;
    let started_at = timestamp(string(stats, "startTime")?)?;
    let mut tests = vec![];
    let mut counts = [0u64; 4];
    let mut clean = array(v, "errors")?.is_empty();
    let mut suites: Vec<&Value> = array(v, "suites")?.iter().collect();
    while let Some(suite) = suites.pop() {
        if suite.get("suites").is_some() {
            suites.extend(array(suite, "suites")?);
        }
        for spec in array(suite, "specs")? {
            let spec_id = string(spec, "id")?;
            if spec_id.is_empty() {
                return Err(invalid("empty spec id"));
            }
            for test in array(spec, "tests")? {
                let status = string(test, "status")?;
                let index = match status {
                    "expected" => 0,
                    "unexpected" => 1,
                    "flaky" => 2,
                    "skipped" => 3,
                    _ => return Err(invalid("invalid test status")),
                };
                counts[index] += 1;
                let expected = string(test, "expectedStatus")?;
                if !matches!(expected, "passed" | "failed" | "skipped") {
                    return Err(invalid("invalid expected status"));
                }
                let results = array(test, "results")?;
                if results.is_empty() && status != "skipped" {
                    return Err(invalid("missing test attempt"));
                }
                let mut messages = vec![];
                for annotation in array(test, "annotations")? {
                    if let Some(description) = annotation.get("description") {
                        messages.push(
                            description
                                .as_str()
                                .ok_or_else(|| invalid("invalid annotation"))?
                                .into(),
                        );
                    }
                }
                let mut all_passed = !results.is_empty();
                let mut all_skipped = true;
                for result in results {
                    timestamp(string(result, "startTime")?)?;
                    let actual = string(result, "status")?;
                    if !matches!(
                        actual,
                        "passed" | "failed" | "timedOut" | "skipped" | "interrupted"
                    ) {
                        return Err(invalid("invalid result status"));
                    }
                    all_passed &= actual == "passed" && count(result, "retry")? == 0;
                    all_skipped &= actual == "skipped";
                    for error in array(result, "errors")? {
                        messages.push(
                            serde_json::to_string(error).map_err(|e| invalid(e.to_string()))?,
                        );
                        all_passed = false;
                    }
                    if result.get("error").is_some_and(|e| !e.is_null()) {
                        all_passed = false;
                        messages.push(result["error"].to_string());
                    }
                }
                let outcome = if status == "skipped" && all_skipped {
                    TestOutcome::Skipped
                } else if status == "expected"
                    && expected == "passed"
                    && all_passed
                    && results.len() == 1
                {
                    TestOutcome::Passed
                } else {
                    messages.push(format!(
                        "Playwright status: {status}; expected: {expected}; attempts: {}",
                        results.len()
                    ));
                    TestOutcome::Failed
                };
                clean &= outcome == TestOutcome::Passed;
                tests.push(TestResult {
                    id: id(namespace, &[spec_id, string(test, "projectId")?]),
                    outcome,
                    messages,
                });
            }
        }
    }
    for (key, actual) in ["expected", "unexpected", "flaky", "skipped"]
        .into_iter()
        .zip(counts)
    {
        same(count(stats, key)?, actual)?;
    }
    let mut report = finish(tests, started_at, clean)?;
    report.messages = array(v, "errors")?.iter().map(Value::to_string).collect();
    Ok(report)
}

const TRX_NS: &str = "http://microsoft.com/schemas/VisualStudio/TeamTest/2010";
fn child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Result<roxmltree::Node<'a, 'input>, ReportError> {
    let mut nodes = node.children().filter(|n| n.has_tag_name((TRX_NS, name)));
    let result = nodes
        .next()
        .ok_or_else(|| invalid(format!("missing {name}")))?;
    if nodes.next().is_some() {
        return Err(invalid(format!("duplicate {name}")));
    }
    Ok(result)
}
fn attr<'a, 'input>(node: roxmltree::Node<'a, 'input>, name: &str) -> Result<&'a str, ReportError> {
    node.attribute(name)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid(format!("missing attribute {name}")))
}
fn trx(namespace: &str, bytes: &[u8]) -> Result<ParsedReport, ReportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("TRX must be UTF-8"))?;
    // Default roxmltree parsing rejects DTD; explicit option prevents entity expansion.
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 500_000,
            ..Default::default()
        },
    )
    .map_err(|e| invalid(e.to_string()))?;
    let root = doc.root_element();
    if !root.has_tag_name((TRX_NS, "TestRun")) {
        return Err(invalid("invalid TRX root or namespace"));
    }
    let started_at = timestamp(attr(child(root, "Times")?, "start")?)?;
    let summary = child(root, "ResultSummary")?;
    let counters = child(summary, "Counters")?;
    let mut clean = match attr(summary, "outcome")? {
        "Completed" | "Passed" => true,
        "Failed" | "Error" | "Aborted" => false,
        _ => return Err(invalid("unsupported run outcome")),
    };
    for info in summary
        .descendants()
        .filter(|n| n.has_tag_name((TRX_NS, "RunInfo")))
    {
        clean &= attr(info, "outcome")? == "Passed";
    }
    let mut tests = vec![];
    for result in child(root, "Results")?
        .children()
        .filter(|n| n.is_element())
    {
        if !result.has_tag_name((TRX_NS, "UnitTestResult"))
            || result
                .descendants()
                .any(|n| n.has_tag_name((TRX_NS, "InnerResults")))
        {
            return Err(invalid("only flat UnitTestResult supported"));
        }
        let outcome = match attr(result, "outcome")? {
            "Passed" => TestOutcome::Passed,
            "Failed" => TestOutcome::Failed,
            "NotExecuted" => TestOutcome::Skipped,
            _ => return Err(invalid("unsupported TRX test outcome")),
        };
        let messages = result
            .descendants()
            .filter(|n| {
                n.is_element()
                    && matches!(
                        n.tag_name().name(),
                        "Message" | "StackTrace" | "StdOut" | "StdErr"
                    )
            })
            .filter_map(|n| n.text())
            .map(str::to_owned)
            .collect();
        tests.push(TestResult {
            id: id(
                namespace,
                &[attr(result, "testId")?, attr(result, "testName")?],
            ),
            outcome,
            messages,
        });
    }
    let mut report = finish(tests, started_at, clean)?;
    report.messages = summary
        .descendants()
        .filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "Text" | "Message" | "StdOut" | "StdErr"
                )
        })
        .filter_map(|n| n.text())
        .map(str::to_owned)
        .collect();
    for (key, actual) in [
        ("total", report.discovered),
        ("executed", report.executed),
        ("passed", report.passed),
        ("failed", report.failed),
    ] {
        same(
            attr(counters, key)?
                .parse()
                .map_err(|_| invalid("invalid TRX counter"))?,
            actual,
        )?;
    }
    // VSTest 17.14.1 writes all remaining counters as zero, including notExecuted.
    for key in [
        "error",
        "timeout",
        "aborted",
        "inconclusive",
        "passedButRunAborted",
        "notRunnable",
        "notExecuted",
        "disconnected",
        "warning",
        "completed",
        "inProgress",
        "pending",
    ] {
        same(
            attr(counters, key)?
                .parse()
                .map_err(|_| invalid("invalid TRX counter"))?,
            0,
        )?;
    }
    Ok(report)
}
