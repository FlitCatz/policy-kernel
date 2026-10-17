use super::*;
use project_verification::reports::{parse_report, TestOutcome};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawTest {
    pub suite: String,
    pub report: ArtifactDigest,
    pub raw_identity: Vec<String>,
    pub source_path: String,
    pub selector: String,
    pub normalized_id: String,
    pub outcome: TestOutcome,
}

/// Keep parser order and zip its IDs to raw names. The harness never fabricates hashes.
pub fn link_report(root: &Path, id: &str, suite: &Suite, bytes: &[u8]) -> Result<Vec<RawTest>> {
    let parsed = parse_report(suite.format, id, bytes)?;
    let mut raw: Vec<(Vec<String>, String, String)> = vec![];
    match suite.format {
        ReportFormat::VitestJson => {
            let v: Value = serde_json::from_slice(bytes)?;
            for file in v["testResults"].as_array().ok_or("TJ-RAW-VITEST")? {
                let name = str_field(file, "name")?;
                let path = source_relative(root, Path::new(name))?;
                for t in file["assertionResults"].as_array().ok_or("TJ-RAW-VITEST")? {
                    let full = str_field(t, "fullName")?;
                    raw.push((vec![name.into(), full.into()], path.clone(), full.into()));
                }
            }
        }
        ReportFormat::PlaywrightJson => {
            let v: Value = serde_json::from_slice(bytes)?;
            let report_root = Path::new(str_field(&v["config"], "rootDir")?);
            let mut stack: Vec<_> = v["suites"]
                .as_array()
                .ok_or("TJ-RAW-PLAYWRIGHT")?
                .iter()
                .collect();
            while let Some(s) = stack.pop() {
                if let Some(children) = s.get("suites") {
                    stack.extend(children.as_array().ok_or("TJ-RAW-PLAYWRIGHT")?);
                }
                for spec in s["specs"].as_array().ok_or("TJ-RAW-PLAYWRIGHT")? {
                    let path = source_relative(root, &report_root.join(str_field(spec, "file")?))?;
                    for test in spec["tests"].as_array().ok_or("TJ-RAW-PLAYWRIGHT")? {
                        raw.push((
                            vec![
                                str_field(spec, "id")?.into(),
                                str_field(test, "projectId")?.into(),
                            ],
                            path.clone(),
                            str_field(spec, "title")?.into(),
                        ));
                    }
                }
            }
        }
        ReportFormat::Trx => {
            let doc = roxmltree::Document::parse(std::str::from_utf8(bytes)?)?;
            let results = doc
                .root_element()
                .children()
                .find(|n| n.tag_name().name() == "Results")
                .ok_or("TJ-RAW-TRX")?;
            for t in results.children().filter(|n| n.is_element()) {
                let name = t.attribute("testName").ok_or("TJ-RAW-TRX")?;
                ensure(
                    suite
                        .classes
                        .iter()
                        .any(|class| name.starts_with(&format!("{class}."))),
                    "TJ-SUITE-CLASS: report contains an unclassified backend class",
                )?;
                // Test source is checked against the catalog class below, not invented from a hash.
                raw.push((
                    vec![
                        t.attribute("testId").ok_or("TJ-RAW-TRX")?.into(),
                        name.into(),
                    ],
                    String::new(),
                    name.into(),
                ));
            }
        }
        _ => return Err("TJ-RAW-FORMAT".into()),
    }
    ensure(raw.len() == parsed.tests.len(), "TJ-RAW-IDENTITY-COUNT")?;
    let report = ArtifactDigest {
        path: suite.report.clone(),
        digest: bytes_digest(bytes),
    };
    Ok(raw
        .into_iter()
        .zip(parsed.tests)
        .map(|((raw_identity, source_path, selector), t)| RawTest {
            suite: id.into(),
            report: report.clone(),
            raw_identity,
            source_path,
            selector,
            normalized_id: t.id,
            outcome: t.outcome,
        })
        .collect())
}
fn str_field<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| format!("TJ-RAW-FIELD: {k}").into())
}
fn source_relative(root: &Path, path: &Path) -> Result<String> {
    let relative_path = path
        .strip_prefix(root)
        .map_err(|_| "TJ-RAW-SOURCE: report from another checkout")?;
    let value = relative_path.to_str().ok_or("TJ-RAW-SOURCE: non UTF-8")?;
    relative(value)?;
    checked_read(root, value)?;
    Ok(value.into())
}

/// Exact selectors for JSON; flat TRX theory rows retain each original identity.
pub fn resolve(selector: &Selector, tests: &[RawTest], suites: &Suites) -> Result<Vec<String>> {
    let (_, _, suite) = suites
        .entries()
        .find(|(id, _, _)| id == &selector.suite)
        .ok_or("TJ-SELECTOR-SUITE")?;
    let theory_prefix = format!("{}(", selector.selector);
    let matching: Vec<_> = tests
        .iter()
        .filter(|t| {
            t.suite == selector.suite
                && if suite.format == ReportFormat::Trx {
                    let class = selector
                        .selector
                        .rsplit_once('.')
                        .map(|(class, _)| class)
                        .unwrap_or("");
                    let filename = class.rsplit('.').next().unwrap_or("");
                    suite.classes.iter().any(|c| c == class)
                        && selector.path.ends_with(&format!("/{filename}.cs"))
                        && (t.selector == selector.selector
                            || t.selector.starts_with(&theory_prefix))
                } else {
                    t.source_path == selector.path && t.selector == selector.selector
                }
        })
        .collect();
    ensure(
        !matching.is_empty(),
        &format!("TJ-SELECTOR-MISSING: {}", selector.selector),
    )?;
    ensure(
        matching.len() == 1
            || (suite.format == ReportFormat::Trx
                && matching
                    .iter()
                    .all(|t| t.selector.starts_with(&theory_prefix))),
        "TJ-SELECTOR-AMBIGUOUS",
    )?;
    ensure(
        matching.iter().all(|t| t.outcome == TestOutcome::Passed),
        &format!("TJ-REQUIREMENT-UNPROVEN: {}", selector.selector),
    )?;
    Ok(matching.iter().map(|t| t.normalized_id.clone()).collect())
}

pub fn mappings(
    component: &str,
    catalog: &Catalog,
    suites: &Suites,
    tests: &[RawTest],
) -> Result<AdequacyMappings> {
    let mut result = AdequacyMappings::default();
    for r in catalog
        .requirements
        .iter()
        .filter(|r| r.component == component)
    {
        let mut ids = BTreeSet::new();
        for s in &r.tests {
            ids.extend(resolve(s, tests, suites).map_err(|e| format!("{}: {e}", r.id))?);
        }
        let mapping = TestMapping {
            requirement_id: r.id.clone(),
            test_ids: ids.into_iter().collect(),
        };
        if r.adequacy_lists.iter().any(|l| l == "acceptance_criteria") {
            result.acceptance_criteria.push(mapping.clone());
        }
        if r.adequacy_lists.iter().any(|l| l == "changed_behaviors") {
            result.changed_behaviors.push(mapping);
        }
        if r.kind == "N" {
            result.negative_case_ids.push(r.id.clone());
        }
    }
    for s in catalog
        .required_regression_tests
        .iter()
        .filter(|s| s.suite.starts_with(&format!("{component}:")))
    {
        resolve(s, tests, suites)?;
    }
    Ok(result)
}
