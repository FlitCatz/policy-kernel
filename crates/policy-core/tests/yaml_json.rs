#[path = "../../../tests/support/yaml_json.rs"]
mod yaml_json;

use serde::Deserialize;
use serde_json::json;
use yaml_json::yaml_to_json;

#[test]
fn scalars_keep_their_core_schema_types() {
    let value = yaml_to_json(
        "int: 1\nnegative: -7\nquoted_int: \"1\"\nreal: 1.5\nyes_word: yes\n\
         t: true\nf: false\nquoted_bool: 'true'\nnull_word: null\ntilde: ~\nempty:\n\
         sha: e5f6ec8584cc91fdeca3aab40687c6cdcd303b43\ndigit_sha: 1234abcd\n\
         code: PK-PROV-001\nstatus: INCONCLUSIVE\n",
    )
    .unwrap();
    assert_eq!(
        value,
        json!({
            "int": 1, "negative": -7, "quoted_int": "1", "real": 1.5, "yes_word": "yes",
            "t": true, "f": false, "quoted_bool": "true",
            "null_word": null, "tilde": null, "empty": null,
            "sha": "e5f6ec8584cc91fdeca3aab40687c6cdcd303b43", "digit_sha": "1234abcd",
            "code": "PK-PROV-001", "status": "INCONCLUSIVE"
        })
    );
    assert!(value["int"].is_u64() && value["negative"].is_i64() && value["real"].is_f64());
}

#[test]
fn maps_sequences_block_scalars_and_aliases_convert_structurally() {
    let value = yaml_to_json(
        "runs:\n  steps:\n    - run: |\n        echo \"$A\"\n        exit 2\n      shell: bash\n\
         \x20   - [a, {b: 2}]\nbase: &base {k: v}\ncopy: *base\nempty_list: []\nempty_map: {}\n",
    )
    .unwrap();
    assert_eq!(
        value,
        json!({
            "runs": {"steps": [{"run": "echo \"$A\"\nexit 2\n", "shell": "bash"}, ["a", {"b": 2}]]},
            "base": {"k": "v"}, "copy": {"k": "v"}, "empty_list": [], "empty_map": {}
        })
    );
}

#[test]
fn ambiguous_or_unrepresentable_documents_are_rejected() {
    for (source, expected) in [
        ("a: 1\na: 2\n", "duplicated key"),
        ("outer:\n  a: 1\n  a: 1\n", "duplicated key"),
        ("", "found 0"),
        ("a: 1\n---\nb: 2\n", "found 2"),
        ("1: a\n", "is not a string"),
        ("x: .inf\n", "no JSON representation"),
        ("a: [1\n", ""),
    ] {
        let error = yaml_to_json(source).expect_err(source);
        assert!(error.contains(expected), "{source:?}: {error}");
    }
}

#[test]
fn deserialized_types_keep_deny_unknown_fields_and_field_types() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Row {
        version: u32,
        flag: bool,
        note: Option<String>,
        tags: Vec<String>,
    }
    let row: Row = serde_json::from_value(
        yaml_to_json("version: 1\nflag: false\nnote: null\ntags: [a, b]\n").unwrap(),
    )
    .unwrap();
    assert_eq!(
        row,
        Row {
            version: 1,
            flag: false,
            note: None,
            tags: vec!["a".into(), "b".into()]
        }
    );
    for source in [
        "version: 1\nflag: false\nnote: null\ntags: []\nextra: 1\n",
        "version: '1'\nflag: false\nnote: null\ntags: []\n",
        "version: 1\nflag: 'false'\nnote: null\ntags: []\n",
        "version: -1\nflag: false\nnote: null\ntags: []\n",
    ] {
        assert!(
            serde_json::from_value::<Row>(yaml_to_json(source).unwrap()).is_err(),
            "{source:?}"
        );
    }
}
