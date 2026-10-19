use policy_report_adapters::{parse_report, ParsedReport, ReportFormat};

#[test]
fn corpus_serialization_matches_pre_extraction_bytes() {
    // Captured with project-verification at 6990b8d675a4cbd472ecc6c4e0ab6c4eabafd6af,
    // before moving the parser. These are fixed expectations, not regenerated outputs.
    for (format, version, input, expected) in [
        (
            ReportFormat::VitestJson,
            "vitest-json-3.2.4/v1",
            include_bytes!("fixtures/project-reports/vitest.json").as_slice(),
            include_bytes!("fixtures/project-reports/vitest.json.expected.json").as_slice(),
        ),
        (
            ReportFormat::PlaywrightJson,
            "playwright-json-1.55.0/v1",
            include_bytes!("fixtures/project-reports/playwright.json").as_slice(),
            include_bytes!("fixtures/project-reports/playwright.json.expected.json").as_slice(),
        ),
        (
            ReportFormat::Trx,
            "vstest-trx-17.14.1/v1",
            include_bytes!("fixtures/project-reports/vstest.trx").as_slice(),
            include_bytes!("fixtures/project-reports/vstest.trx.expected.json").as_slice(),
        ),
        (
            ReportFormat::JestJson,
            "jest-json/v1",
            include_bytes!("fixtures/project-reports/jest.json").as_slice(),
            include_bytes!("fixtures/project-reports/jest.json.expected.json").as_slice(),
        ),
    ] {
        let report = parse_report(format, "auth", input).unwrap();
        assert_eq!(serde_json::to_vec(&report).unwrap(), expected, "{format:?}");
        let decoded: ParsedReport = serde_json::from_slice(expected).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), expected);
        assert_eq!(format.parser_version(), version);
    }
}

#[test]
fn report_size_and_namespace_limits_are_preserved() {
    let mut bytes = include_bytes!("fixtures/project-reports/vitest.json").to_vec();
    bytes.resize(16 * 1024 * 1024, b' ');
    assert!(parse_report(ReportFormat::VitestJson, &"a".repeat(256), &bytes).is_ok());
    bytes.push(b' ');
    assert_eq!(
        parse_report(ReportFormat::VitestJson, "auth", &bytes)
            .unwrap_err()
            .to_string(),
        "invalid report: report exceeds 16 MiB"
    );
    for namespace in [String::new(), "a".repeat(257), "control\n".into()] {
        assert_eq!(
            parse_report(ReportFormat::VitestJson, &namespace, b"{}")
                .unwrap_err()
                .to_string(),
            "invalid report: invalid namespace"
        );
    }
}
