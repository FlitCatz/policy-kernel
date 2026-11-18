use serde_json::{json, Value};

const TEST_ID: &str = "unit:74bc59bfb3eae7fa59dc30d61a5b5237be4bbba30d8a242c4d39061450ead847";

pub fn prepare(expected: &mut Value, run: &mut Value) {
    let parser = json!({"id": "vitest_json", "version": "vitest-json-3.2.4/v1"});
    expected["bindings"]["observations"][0]["parser"] = parser.clone();
    let report = &mut run["test_observations"][0];
    report["parser"] = parser;
    report["tests"][0]["id"] = json!(TEST_ID);
    report["tests"][0]["raw_identity"] = json!(json!([
        "/repo/test/login.test.ts",
        "login accepts valid credentials"
    ])
    .to_string());
    for mapping in run["mappings"].as_array_mut().unwrap() {
        mapping["tests"][0]["test_id"] = json!(TEST_ID);
    }
}
