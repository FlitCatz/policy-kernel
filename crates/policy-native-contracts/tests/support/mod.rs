use serde_json::{json, Value};

pub fn raw() -> Value {
    json!({"kind":"raw_byte_sha256","sha256":"a".repeat(64)})
}
pub fn digest(role: &str) -> Value {
    json!({"kind":"canonical_sha256","schema":"policy_native_v1","role":role,"sha256":"b".repeat(64)})
}
pub fn subject() -> Value {
    json!({"repository":"example/project","head":"1".repeat(40),"source_set_digest":digest("source_set"),
        "source_inventory_digest":raw(),"scope_definition_digest":digest("scope_definition")})
}
pub fn run_subject() -> Value {
    json!({"source":subject(),"run_id":"run-1","scope_id":"scope-1"})
}
pub fn expected() -> Value {
    json!({"schema_version":1,"subject":subject(),"requirements":{
        "approved_file_digest":raw(),
        "observations":[{"id":"unit","accepted_exit_codes":[0],"tests":{
            "minimum_discovered":1,"minimum_executed":1,"minimum_passed":1,
            "maximum_failed":0,"maximum_skipped":0,"maximum_retries":0}}],
        "requirements":[{"id":"AC-1","kind":"acceptance_criterion","minimum_executed_tests":1,"suites":["unit"]},
            {"id":"B-1","kind":"changed_behavior","minimum_executed_tests":1,"suites":["unit"]},
            {"id":"N-1","kind":"negative_case","minimum_executed_tests":1,"suites":["unit"]}],
        "exemptions":[]},
        "bindings":{"observations":[{"id":"unit","command_digest":digest("command"),
            "configuration_digest":digest("configuration"),"parser":{"id":"example-json","version":"1"}}],
            "evaluated_at":"2026-10-19T01:01:00Z","maximum_age_seconds":3600,"mode":"local_advisory"}})
}
pub fn run() -> Value {
    json!({"schema_version":1,"subject":run_subject(),"observations":[{
        "id":"unit","execution_id":"exec-1","subject":run_subject(),
        "command":{"argv":["native-test","--report","raw.json"],"cwd":"."},
        "configuration_digest":digest("configuration"),"started_at":"2026-10-19T01:00:00Z",
        "completed_at":"2026-10-19T01:00:01Z","observed_at":"2026-10-19T01:00:02Z",
        "completion":{"kind":"exited","code":0}}],
        "test_observations":[{"observation_id":"unit","parser":{"id":"example-json","version":"1"},
            "raw_report":{"id":"raw-1","digest":raw()},
            "counts":{"discovered":1,"executed":1,"passed":1,"failed":0,"skipped":0,"retries":0},
            "tests":[{"id":"test-1","raw_identity":"/original/checkout/tests/test-1","outcome":"passed","attempts":1}]}],
        "artifacts":[{"id":"raw-1","path":"raw.json","role":"raw_report","digest":raw()}],
        "mappings":[{"requirement_id":"AC-1","tests":[{"suite_id":"unit","test_id":"test-1"}]},
            {"requirement_id":"B-1","tests":[{"suite_id":"unit","test_id":"test-1"}]},
            {"requirement_id":"N-1","tests":[{"suite_id":"unit","test_id":"test-1"}]}],"exclusions":[]})
}
pub fn result() -> Value {
    json!({"schema_version":1,"status":"satisfied","subject":run_subject(),
        "requirements":[{"id":"AC-1","status":"satisfied","reasons":[]},
            {"id":"B-1","status":"satisfied","reasons":[]},{"id":"N-1","status":"satisfied","reasons":[]}],
        "reasons":[],"digests":{"expected_scope":digest("expected_scope"),"native_run":digest("native_run"),
            "policy":digest("policy"),"evidence":digest("evidence")},
        "evaluated_at":"2026-10-19T01:01:00Z","verifier_version":"example-verifier-1",
        "adapter_version":"example-adapter-1","trust":"local_untrusted","review":"unmet","exclusions":[]})
}

/// Patch vectors are deliberately data, reusable by consumers without Rust.
/// `schema_valid` distinguishes structural JSON Schema checks from typed invariants.
pub fn corpus() -> Value {
    json!({"valid":{"expected":expected(),"run":run(),"result":result()},"invalid":[
        {"name":"unknown-version","document":"expected","pointer":"/schema_version","value":2,"schema_valid":false},
        {"name":"unknown-field","document":"run","pointer":"/trusted","value":true,"schema_valid":false},
        {"name":"unknown-nested-field","document":"expected","pointer":"/subject/trusted","value":true,"schema_valid":false},
        {"name":"empty-required-set","document":"expected","pointer":"/requirements/observations","value":[],"schema_valid":false},
        {"name":"duplicate-required-set","document":"expected","pointer":"/requirements/observations/0/accepted_exit_codes","value":[0,0],"schema_valid":false},
        {"name":"requirement-id-collision","document":"expected","pointer":"/requirements/requirements/1/id","value":"AC-1","schema_valid":true},
        {"name":"unknown-suite","document":"expected","pointer":"/requirements/requirements/0/suites/0","value":"missing","schema_valid":true},
        {"name":"raw-in-typed-slot","document":"expected","pointer":"/subject/source_set_digest","value":raw(),"schema_valid":false},
        {"name":"typed-in-raw-slot","document":"run","pointer":"/artifacts/0/digest","value":digest("evidence"),"schema_valid":false},
        {"name":"wrong-hash-role","document":"expected","pointer":"/subject/source_set_digest/role","value":"command","schema_valid":true},
        {"name":"mismatched-child-subject","document":"run","pointer":"/observations/0/subject/source/head","value":"2".repeat(40),"schema_valid":true},
        {"name":"mismatched-expected-subject","document":"pair","pointer":"/subject/source/head","value":"2".repeat(40),"schema_valid":true},
        {"name":"artifact-hash-mismatch","document":"run","pointer":"/artifacts/0/digest/sha256","value":"c".repeat(64),"schema_valid":true},
        {"name":"unknown-artifact","document":"run","pointer":"/test_observations/0/raw_report/id","value":"missing","schema_valid":true},
        {"name":"count-mismatch","document":"run","pointer":"/test_observations/0/counts/passed","value":0,"schema_valid":true},
        {"name":"unknown-mapped-test","document":"run","pointer":"/mappings/0/tests/0/test_id","value":"missing","schema_valid":true},
        {"name":"path-traversal","document":"run","pointer":"/artifacts/0/path","value":"../raw.json","schema_valid":true},
        {"name":"empty-success","document":"result","pointer":"/requirements","value":[],"schema_valid":true},
        {"name":"success-without-evidence","document":"result","pointer":"/digests/evidence","value":null,"schema_valid":true},
        {"name":"trust-promotion","document":"result","pointer":"/trust","value":"ci_observed","schema_valid":false},
        {"name":"review-promotion","document":"result","pointer":"/review","value":"met","schema_valid":false}
    ]})
}
