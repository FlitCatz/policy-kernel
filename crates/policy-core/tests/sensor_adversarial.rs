use policy_contracts::{
    CommitSha, Risk, SensorCondition, SensorDefinition, SensorEvidence, SensorId, SensorProfile,
    SensorStatus, Sha256Digest, TestAdequacyEvidence, TestAdequacyPolicy, TrustClass, UtcTimestamp,
};
use policy_core::{
    evaluate_sensor_evidence_for_change, resolve_required_sensors, SensorEvaluationStatus,
};

fn id(value: &str) -> SensorId {
    SensorId::parse(value).unwrap()
}

fn condition(id: &str, glob: &str) -> SensorCondition {
    SensorCondition {
        id: id.into(),
        risks: vec![],
        path_globs: vec![glob.into()],
    }
}

fn profile() -> SensorProfile {
    SensorProfile {
        schema_version: 1,
        sensors: vec![SensorDefinition {
            id: id("compile"),
            required: true,
            required_when: vec![],
            not_applicable_when: vec![condition("docs-only", "docs/**")],
            accepted_exit_codes: vec![0],
            result_requirement: None,
            result_binding: None,
        }],
        adequacy: TestAdequacyPolicy {
            acceptance_criterion_ids: vec!["AC-1".into()],
            changed_behavior_ids: vec![],
            negative_case_ids: vec![],
        },
    }
}

fn evidence(status: SensorStatus, exit_code: Option<i32>) -> SensorEvidence {
    SensorEvidence {
        sensor_id: id("compile"),
        status,
        head_sha: CommitSha::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
        command_digest: digest('b'),
        result_digest: digest('c'),
        observed_at: UtcTimestamp::parse("2026-09-04T00:00:00Z").unwrap(),
        trust: TrustClass::CiObserved,
        resolved_command: vec!["cargo".into(), "test".into()],
        source_digest: Some(digest('d')),
        configuration_digest: Some(digest('e')),
        parser_version: Some("1".into()),
        exit_code,
        discovered: None,
        executed: None,
        passed: None,
        not_applicable_condition_id: None,
        not_applicable_reason: None,
    }
}

fn digest(character: char) -> Sha256Digest {
    Sha256Digest::parse(character.to_string().repeat(64)).unwrap()
}

fn empty_adequacy() -> TestAdequacyEvidence {
    TestAdequacyEvidence {
        acceptance_criteria: vec![],
        changed_behaviors: vec![],
        negative_case_ids: vec![],
        executed_test_ids: vec![],
        discovered: 0,
        executed: 0,
        passed: 0,
        head_sha: None,
    }
}

fn adequate() -> TestAdequacyEvidence {
    TestAdequacyEvidence {
        acceptance_criteria: vec![policy_contracts::TestMapping {
            requirement_id: "AC-1".into(),
            test_ids: vec!["test-1".into()],
        }],
        executed_test_ids: vec!["test-1".into()],
        discovered: 1,
        executed: 1,
        passed: 1,
        ..empty_adequacy()
    }
}

#[test]
fn passed_status_with_unaccepted_exit_code_is_blocked() {
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[evidence(SensorStatus::Passed, Some(1))],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-SENSOR-EXIT-CODE"));
}

#[test]
fn missing_exit_code_is_inconclusive_not_blocked() {
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[evidence(SensorStatus::Passed, None)],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
    assert!(result.has_reason("PK-SENSOR-RESULT-INCOMPLETE"));
    assert!(!result.has_reason("PK-SENSOR-EXIT-CODE"));
}

#[test]
fn partially_recorded_sensor_counts_are_inconclusive() {
    let mut item = evidence(SensorStatus::Passed, Some(0));
    item.discovered = Some(1);
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[item],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
    assert!(result.has_reason("PK-SENSOR-RESULT-INCOMPLETE"));
}

#[test]
fn not_applicable_must_match_an_explicit_na_condition_for_this_change() {
    let mut item = evidence(SensorStatus::NotApplicable, None);
    item.not_applicable_condition_id = Some("docs-only".into());
    item.not_applicable_reason = Some("documentation-only change".into());

    let mismatch = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[item.clone()],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(mismatch.status, SensorEvaluationStatus::Inconclusive);

    let matched = evaluate_sensor_evidence_for_change(
        &profile(),
        &["docs/readme.md"],
        Risk::Low,
        &[item],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(matched.status, SensorEvaluationStatus::Passed);
}

#[test]
fn not_applicable_condition_must_cover_every_changed_path() {
    let mut item = evidence(SensorStatus::NotApplicable, None);
    item.not_applicable_condition_id = Some("docs-only".into());
    item.not_applicable_reason = Some("documentation-only change".into());

    for paths in [
        &["docs/readme.md", "src/lib.rs"][..],
        &["src/lib.rs", "docs/readme.md"][..],
        &[][..],
    ] {
        let mixed = evaluate_sensor_evidence_for_change(
            &profile(),
            paths,
            Risk::Low,
            &[item.clone()],
            Some(&adequate()),
        )
        .unwrap();
        assert_eq!(
            mixed.status,
            SensorEvaluationStatus::Inconclusive,
            "{paths:?}"
        );
        assert!(mixed.has_reason("PK-SENSOR-NOT-APPLICABLE-INVALID"));
    }

    let mut multi = profile();
    multi.sensors[0].not_applicable_when = vec![SensorCondition {
        id: "docs-only".into(),
        risks: vec![],
        path_globs: vec!["docs/**".into(), "*.md".into()],
    }];
    let covered = evaluate_sensor_evidence_for_change(
        &multi,
        &["docs/guide.md", "README.md"],
        Risk::Low,
        &[item.clone()],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(covered.status, SensorEvaluationStatus::Passed);

    multi.sensors[0].not_applicable_when[0].risks = vec![Risk::Low];
    let wrong_risk = evaluate_sensor_evidence_for_change(
        &multi,
        &["docs/guide.md"],
        Risk::High,
        &[item],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(wrong_risk.status, SensorEvaluationStatus::Inconclusive);
}

#[test]
fn malformed_glob_and_duplicate_sensor_ids_fail_closed() {
    let mut malformed = profile();
    malformed.sensors[0].not_applicable_when = vec![condition("bad", "[")];
    assert!(resolve_required_sensors(&malformed, &["src/lib.rs"], Risk::Low).is_err());

    let mut duplicate = profile();
    duplicate.sensors.push(duplicate.sensors[0].clone());
    assert!(resolve_required_sensors(&duplicate, &["src/lib.rs"], Risk::Low).is_err());
}

#[test]
fn duplicate_executed_test_ids_fail_adequacy() {
    let mut adequacy = empty_adequacy();
    adequacy.discovered = 2;
    adequacy.executed = 2;
    adequacy.passed = 2;
    adequacy.executed_test_ids = vec!["same".into(), "same".into()];
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[evidence(SensorStatus::Passed, Some(0))],
        Some(&adequacy),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-DUPLICATE-TEST"));
}

#[test]
fn single_star_stays_within_one_path_segment() {
    let mut profile = profile();
    profile.sensors[0].required = false;
    profile.sensors[0].required_when = vec![condition("rust-root", "src/*.rs")];
    profile.sensors[0].not_applicable_when = vec![condition("docs-top", "docs/*")];
    let required = |paths: &[&str]| {
        resolve_required_sensors(&profile, paths, Risk::Low)
            .unwrap()
            .contains(&id("compile"))
    };
    assert!(required(&["src/lib.rs"]));
    assert!(!required(&["src/nested/lib.rs"]));

    let mut recursive = profile.clone();
    recursive.sensors[0].required_when = vec![condition("rust-tree", "src/**")];
    assert!(
        resolve_required_sensors(&recursive, &["src/nested/lib.rs"], Risk::Low)
            .unwrap()
            .contains(&id("compile"))
    );

    // An exemption written for top-level docs cannot cover nested documentation.
    let mut item = evidence(SensorStatus::NotApplicable, None);
    item.not_applicable_condition_id = Some("docs-top".into());
    item.not_applicable_reason = Some("documentation-only change".into());
    let mut exempt = profile.clone();
    exempt.sensors[0].required = true;
    let covered = |paths: &[&str]| {
        evaluate_sensor_evidence_for_change(
            &exempt,
            paths,
            Risk::Low,
            &[item.clone()],
            Some(&adequate()),
        )
        .unwrap()
        .status
    };
    assert_eq!(covered(&["docs/readme.md"]), SensorEvaluationStatus::Passed);
    assert_eq!(
        covered(&["docs/guides/readme.md"]),
        SensorEvaluationStatus::Inconclusive
    );
}

#[test]
fn github_filter_subset_conformance() {
    // Accepted examples from GitHub's filter-pattern cheat sheet, plus boundaries.
    for (pattern, yes, no) in [
        ("*", "README.md", "docs/README.md"),
        ("**", "all/the/files.md", ""),
        ("*.js", "app.js", "js/app.js"),
        ("docs/*", "docs/README.md", "docs/mona/octocat.txt"),
        ("docs/**", "docs/mona/octocat.txt", "other/docs/file.txt"),
        ("docs/**/*.md", "docs/README.md", "docs/file.txt"),
        ("docs/**/*.md", "docs/a/markdown/file.md", "other/file.md"),
        ("**/docs/**", "dir/docs/my-file.txt", "dir/my-file.txt"),
        ("**/docs/**", "docs/hello.md", "mydocs/hello.md"),
        ("**/README.md", "README.md", "README.md.bak"),
        ("**/README.md", "js/README.md", "js/readme.md"),
        ("**/*src/**", "my-src/code/js/app.js", "src-other/app.js"),
        ("**/*-post.md", "path/their-post.md", "path/post.md"),
        ("**/migrate-*.sql", "db/migrate-v1.0.sql", "db/migrate.sql"),
        ("src/*.js", "src/.js", "src/page.jsx"),
        ("A_0-z.txt", "A_0-z.txt", "a_0-z.txt"),
    ] {
        let mut profile = profile();
        profile.sensors[0].required = false;
        profile.sensors[0].required_when = vec![condition("filter", pattern)];
        assert_eq!(
            resolve_required_sensors(&profile, &[yes], Risk::Low).unwrap(),
            vec![id("compile")],
            "{pattern}: {yes}"
        );
        // An empty change has no path to match, including for `**`.
        let paths = if no.is_empty() { vec![] } else { vec![no] };
        assert!(
            resolve_required_sensors(&profile, &paths, Risk::Low)
                .unwrap()
                .is_empty(),
            "{pattern}: {no}"
        );
    }
}

#[test]
fn unsupported_github_filters_fail_closed_in_both_sensor_conditions() {
    for pattern in [
        "src/*.jsx?",
        "*.jsx?",
        "v[12].[0-9]+.[0-9]+",
        "[CB]at",
        "a+",
        "**.js",
        "src/a**b",
        "***",
        "!README.md",
        "a!b",
        r"\*",
        r"src\*.js",
        "{a,b}",
        "(a)",
        "a|b",
        "a b",
        "é",
        "",
        "/src/*",
        "src/",
        "src//a",
        "./src/*",
        "src/../a",
        "src/./a",
        "src/\0a",
    ] {
        for exemption in [false, true] {
            let mut profile = profile();
            let conditions = if exemption {
                &mut profile.sensors[0].not_applicable_when
            } else {
                &mut profile.sensors[0].required_when
            };
            *conditions = vec![condition("unsupported", pattern)];
            for paths in [
                &["src/page.js"][..],
                &["src/page.jsx"],
                &["src/page.jsxa"],
                &[],
            ] {
                assert!(
                    matches!(
                        resolve_required_sensors(&profile, paths, Risk::Low),
                        Err(policy_core::SensorResolutionError::InvalidGlob(_))
                    ),
                    "{pattern}: {paths:?}"
                );
            }
            let result =
                policy_core::evaluate_sensor_evidence(&profile, &[], &[], Some(&adequate()));
            assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
            assert!(result.has_reason("PK-SENSOR-PROFILE-INVALID"));
        }
    }
}

#[test]
fn explicit_js_and_jsx_patterns_require_evidence_and_cover_only_matching_changes() {
    let mut profile = profile();
    profile.sensors[0].required = false;
    let mut filter = condition("frontend", "src/*.js");
    filter.path_globs.push("src/*.jsx".into());
    profile.sensors[0].required_when = vec![filter.clone()];
    filter.id = "frontend-exempt".into();
    profile.sensors[0].not_applicable_when = vec![filter];
    for (paths, applies) in [
        (vec!["src/page.js"], true),
        (vec!["src/page.jsx"], true),
        (vec!["src/page.js", "src/page.jsx"], true),
        (vec!["src/page.jsxa"], false),
        (vec!["src/nested/page.js"], false),
        (vec!["docs/page.js"], false),
    ] {
        let result = evaluate_sensor_evidence_for_change(
            &profile,
            &paths,
            Risk::Low,
            &[],
            Some(&adequate()),
        )
        .unwrap();
        assert_eq!(result.has_reason("PK-SENSOR-MISSING"), applies, "{paths:?}");
        let mut exempt = profile.clone();
        exempt.sensors[0].required = true;
        let mut item = evidence(SensorStatus::NotApplicable, None);
        item.not_applicable_condition_id = Some("frontend-exempt".into());
        item.not_applicable_reason = Some("declared exemption".into());
        let result = evaluate_sensor_evidence_for_change(
            &exempt,
            &paths,
            Risk::Low,
            &[item.clone()],
            Some(&adequate()),
        )
        .unwrap();
        assert_eq!(
            result.status == SensorEvaluationStatus::Passed,
            applies,
            "{paths:?}"
        );
        let mut mixed = paths;
        mixed.push("src/server.rs");
        let result = evaluate_sensor_evidence_for_change(
            &exempt,
            &mixed,
            Risk::Low,
            &[item],
            Some(&adequate()),
        )
        .unwrap();
        assert!(result.has_reason("PK-SENSOR-NOT-APPLICABLE-INVALID"));
    }
}
