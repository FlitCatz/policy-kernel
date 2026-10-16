use policy_core::canonical_sha256;
use project_verification::{parse_profile, RunRequest};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, value: Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

pub fn fixture(root: &Path) -> RunRequest {
    fs::write(root.join("config.json"), b"{}").unwrap();
    write(
        root,
        "package.json",
        json!({"name":"app","version":"1.0.0","workspaces":["tools/openapi"],"devDependencies":{"typescript":"6.0.3"}}),
    );
    write(
        root,
        "tools/openapi/package.json",
        json!({"name":"generator","version":"1.0.0","devDependencies":{"typescript":"5.9.3"}}),
    );
    write(
        root,
        "package-lock.json",
        json!({"lockfileVersion":3,"packages":{
            "":{"name":"app","version":"1.0.0","workspaces":["tools/openapi"],"devDependencies":{"typescript":"6.0.3"}},
            "tools/openapi":{"name":"generator","version":"1.0.0","devDependencies":{"typescript":"5.9.3"}},
            "node_modules/generator":{"resolved":"tools/openapi","link":true},
            "node_modules/typescript":{"version":"6.0.3","resolved":"https://registry.npmjs.org/typescript/-/typescript-6.0.3.tgz"},
            "tools/openapi/node_modules/typescript":{"version":"5.9.3","resolved":"https://registry.npmjs.org/typescript/-/typescript-5.9.3.tgz"}
        }}),
    );
    for (path, version) in [
        ("node_modules/typescript/package.json", "6.0.3"),
        (
            "tools/openapi/node_modules/typescript/package.json",
            "5.9.3",
        ),
    ] {
        write(root, path, json!({"name":"typescript","version":version}));
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink("../tools/openapi", root.join("node_modules/generator")).unwrap();
    let mut profile = json!({"schema_version":4,"id":"react-typescript","version":"1.3.0",
        "components":[{"id":"frontend","root":"."}],
        "adequacy":{"acceptance_criterion_ids":[],"changed_behavior_ids":[],"negative_case_ids":[]},
        "sensors":[{"definition":{"id":"frontend:audit","required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":"process"},
            "component":"frontend","argv":["npm","audit","--audit-level=low","--include=dev","--include=optional","--include=peer","--workspaces","--include-workspace-root","--userconfig=.policy-npm-userconfig","--globalconfig=.policy-npm-globalconfig"],"version_args":["--version"],"version_stdout":"",
            "inputs":["package.json","package-lock.json","tools/openapi/package.json"],"timeout_ms":1000}],
        "selection":{"schema_version":3,"tools":{"npm":{"executable":"npm","version_args":["--version"],"version":{"normalization":"plain","acceptance":{"kind":"exact","value":"11.20.0"}},"capabilities":["process","npm_audit"]}},
            "bindings":{"frontend:audit":"npm"},"contracts":{"npm":["process/v1","npm-ci-audit/v1"]},"runtime_files":[],
            "packages":[
                {"component":"frontend","installed_manifest":"node_modules/typescript/package.json","lockfile":"package-lock.json","lock_pointer":"/packages/node_modules~1typescript/version","acceptance":{"kind":"record"}},
                {"component":"frontend","installed_manifest":"tools/openapi/node_modules/typescript/package.json","lockfile":"package-lock.json","lock_pointer":"/packages/tools~1openapi~1node_modules~1typescript/version","acceptance":{"kind":"record"}}
            ],
            "graphs":[{"id":"app","component":"frontend","role":"application","manifest":"package.json","lockfile":"package-lock.json","adapter":"npm","tool":"npm","audit_sensor":"frontend:audit","target_framework":null,
                "workspace":{"member_manifests":["tools/openapi/package.json"],"minimum_pk_revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}]}
    });
    let test = json!({"definition":{"id":"frontend:unit","required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":"test_counts"},
        "component":"frontend","argv":["this-command-must-not-exist"],"version_args":["--version"],"version_stdout":"",
        "report":{"format":"vitest_json","path":"reports/unit.json"},"inputs":["config.json"],"timeout_ms":1000});
    profile["sensors"].as_array_mut().unwrap().push(test);
    profile["selection"]["bindings"]["frontend:unit"] = json!("tests");
    profile["selection"]["tools"]["tests"] = json!({"executable":"this-command-must-not-exist","version_args":["--version"],"version":{"normalization":"plain","acceptance":{"kind":"record"}},"capabilities":["vitest_json"]});
    profile["selection"]["contracts"]["tests"] = json!(["vitest-json/v1"]);
    let profile = parse_profile(&profile.to_string()).unwrap();
    let files = vec![policy_contracts::ArtifactDigest {
        path: "config.json".into(),
        digest: project_verification::bytes_digest(b"{}"),
    }];
    serde_json::from_value(json!({"schema_version":1,"expected_profile_digest":canonical_sha256(&profile).unwrap(),"profile":profile,
        "root":root.canonicalize().unwrap(),"repository":"acceptance/npm-workspace","head_sha":"a".repeat(40),
        "source_files":files,"expected_source_digest":canonical_sha256(&files).unwrap(),"provider":"local","model":"native-workspace",
        "execution_id":"workspace-fixture","impact":{"kind":"unknown"},"mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}})).unwrap()
}
