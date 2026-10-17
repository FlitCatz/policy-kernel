//! The project owns these catalogs; they are scope inputs, never trusted policy.
use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Catalog {
    pub catalog_version: u32,
    pub components: BTreeMap<String, CatalogComponent>,
    pub requirements: Vec<Requirement>,
    #[serde(default)]
    pub required_regression_tests: Vec<Selector>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogComponent {
    pub root: String,
    pub adequacy: Lists,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Lists {
    pub acceptance_criteria: Vec<String>,
    pub changed_behaviors: Vec<String>,
    pub negative_cases: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Requirement {
    pub id: String,
    pub parent_id: String,
    pub component: String,
    pub kind: String,
    pub adequacy_lists: Vec<String>,
    pub tests: Vec<Selector>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Selector {
    pub path: String,
    pub selector: String,
    pub selector_kind: String,
    pub suite: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Suites {
    pub version: u32,
    pub backend: SuiteComponent,
    pub frontend: SuiteComponent,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SuiteComponent {
    pub cwd: String,
    pub suites: BTreeMap<String, Suite>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Suite {
    pub report: String,
    pub format: ReportFormat,
    pub retry: u64,
    pub max_skipped: u64,
    pub argv: Vec<String>,
    #[serde(default)]
    pub classes: Vec<String>,
}
impl Suites {
    pub fn entries(&self) -> impl Iterator<Item = (String, &SuiteComponent, &Suite)> {
        [("backend", &self.backend), ("frontend", &self.frontend)]
            .into_iter()
            .flat_map(|(component, c)| {
                c.suites
                    .iter()
                    .map(move |(role, s)| (format!("{component}:{role}"), c, s))
            })
    }
}
fn unique(values: &[String]) -> bool {
    !values.is_empty()
        && values.iter().all(|v| !v.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
fn same_list(a: &[String], b: &[String]) -> bool {
    unique(a) && unique(b) && a.iter().collect::<BTreeSet<_>>() == b.iter().collect()
}

/// Check the externally approved byte digest before interpreting the catalog.
/// Unused TJ metadata (including historical execution_snapshot IDs) is not evidence.
pub fn check_catalog(
    bytes: &[u8],
    approved: &Sha256Digest,
    parents: &str,
    suites: &Suites,
    profiles: &[ProjectProfile; 2],
) -> Result<Catalog> {
    ensure(
        &bytes_digest(bytes) == approved,
        "TJ-CATALOG-DIGEST: approved catalog differs",
    )?;
    let catalog: Catalog = serde_json::from_slice(bytes)?;
    ensure(
        catalog.catalog_version == 1 && suites.version == 1,
        "TJ-CATALOG-VERSION",
    )?;
    ensure(
        catalog
            .components
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            == ["backend", "frontend"],
        "TJ-CATALOG-COMPONENTS",
    )?;
    let parent_ids: BTreeSet<_> = parents
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|s| !s.is_empty())
        .collect();
    let mut ids = BTreeSet::new();
    for r in &catalog.requirements {
        ensure(
            ids.insert(&r.id)
                && r.id
                    .starts_with(&format!("{}:{}-", r.component, r.parent_id))
                && catalog.components.contains_key(&r.component),
            "TJ-CATALOG-ID: duplicate or invalid ID",
        )?;
        ensure(
            parent_ids.contains(r.parent_id.as_str()),
            "TJ-CATALOG-PARENT: absent from REQUIREMENTS.md",
        )?;
        let expected = match r.kind.as_str() {
            "AC" => vec!["acceptance_criteria"],
            "B" => vec!["changed_behaviors"],
            "N" => vec!["changed_behaviors", "negative_cases"],
            _ => return Err("TJ-CATALOG-KIND".into()),
        };
        ensure(
            r.adequacy_lists
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                == expected,
            "TJ-CATALOG-LISTS: negative must be a changed behavior",
        )?;
        ensure(!r.tests.is_empty(), "TJ-CATALOG-SELECTOR: empty")?;
        let mut seen = BTreeSet::new();
        for s in &r.tests {
            validate_selector(s, suites)?;
            ensure(
                s.suite.starts_with(&format!("{}:", r.component))
                    && seen.insert((&s.path, &s.selector, &s.suite)),
                "TJ-CATALOG-SELECTOR: duplicate or cross-component",
            )?;
        }
    }
    for s in &catalog.required_regression_tests {
        validate_selector(s, suites)?;
    }
    for (component, c) in &catalog.components {
        ensure(c.root == *component, "TJ-CATALOG-ROOT")?;
        let profile = profiles
            .iter()
            .find(|p| {
                p.components.len() == 1
                    && p.components[0].id == *component
                    && p.components[0].root == c.root
            })
            .ok_or("TJ-PROFILE-COMPONENT: expected split profiles")?;
        profile.validate()?;
        for (list, declared, actual) in [
            (
                "acceptance_criteria",
                &c.adequacy.acceptance_criteria,
                &profile.adequacy.acceptance_criterion_ids,
            ),
            (
                "changed_behaviors",
                &c.adequacy.changed_behaviors,
                &profile.adequacy.changed_behavior_ids,
            ),
            (
                "negative_cases",
                &c.adequacy.negative_cases,
                &profile.adequacy.negative_case_ids,
            ),
        ] {
            let derived: Vec<_> = catalog
                .requirements
                .iter()
                .filter(|r| r.component == *component && r.adequacy_lists.iter().any(|l| l == list))
                .map(|r| r.id.clone())
                .collect();
            ensure(
                same_list(&derived, declared) && same_list(&derived, actual),
                "TJ-CATALOG-COMPLETENESS: each nonempty profile list must equal catalog",
            )?;
        }
    }
    for (id, c, s) in suites.entries() {
        ensure(
            c.cwd == id.split(':').next().unwrap()
                && s.retry == 0
                && s.max_skipped == 0
                && !s.argv.is_empty(),
            "TJ-SUITE-CONTRACT",
        )?;
        relative(&s.report)?;
        let sensor = profiles
            .iter()
            .flat_map(|p| &p.sensors)
            .find(|x| x.definition.id.as_str() == id)
            .ok_or("TJ-SUITE-MISSING")?;
        ensure(
            sensor.definition.required
                && sensor.argv == s.argv
                && sensor
                    .report
                    .as_ref()
                    .is_some_and(|r| r.path == s.report && r.format == s.format),
            "TJ-SUITE-PROFILE: report/argv/required differs",
        )?;
    }
    ensure(
        suites
            .backend
            .suites
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            == ["contract", "integration", "unit"].into_iter().collect()
            && suites
                .frontend
                .suites
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == ["csp", "e2e", "unit"].into_iter().collect(),
        "TJ-SUITE-COMPLETENESS",
    )?;
    let classes: Vec<_> = suites
        .backend
        .suites
        .values()
        .flat_map(|s| &s.classes)
        .collect();
    ensure(
        !classes.is_empty() && classes.iter().collect::<BTreeSet<_>>().len() == classes.len(),
        "TJ-SUITE-CLASS: empty or overlapping",
    )?;
    for id in [
        "backend:build",
        "backend:api-audit",
        "backend:modules-audit",
        "backend:tests-audit",
        "frontend:fsd",
        "frontend:build",
        "frontend:specs",
        "frontend:api",
        "frontend:audit",
    ] {
        ensure(
            profiles.iter().flat_map(|p| &p.sensors).any(|s| {
                s.definition.id.as_str() == id
                    && s.definition.required
                    && s.report.is_none()
                    && s.definition.result_requirement == Some(SensorResultRequirement::Process)
            }),
            "TJ-PROCESS-MISSING: required build/generation/FSD/audit sensor",
        )?;
    }
    Ok(catalog)
}
fn validate_selector(s: &Selector, suites: &Suites) -> Result<()> {
    relative(&s.path)?;
    let (_, c, suite) = suites
        .entries()
        .find(|(id, _, _)| id == &s.suite)
        .ok_or("TJ-SELECTOR-SUITE")?;
    let expected_kind = match suite.format {
        ReportFormat::Trx => "fully_qualified_name",
        ReportFormat::VitestJson => "fullName",
        ReportFormat::PlaywrightJson => "title",
        _ => return Err("TJ-SELECTOR-FORMAT".into()),
    };
    ensure(
        s.selector_kind == expected_kind
            && !s.selector.trim().is_empty()
            && s.path.starts_with(&format!("{}/", c.cwd)),
        "TJ-SELECTOR: kind/path/name differs",
    )
}
