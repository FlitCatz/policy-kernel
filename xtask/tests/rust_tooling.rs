use std::{fs, path::Path};

#[test]
fn verification_and_test_tools_do_not_require_an_interpreter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for directory in ["scripts", "crates", ".github/workflows"] {
        let path = root.join(directory);
        if path.exists() {
            check(&path);
        }
    }
}

fn check(path: &Path) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            check(&entry.unwrap().path());
        }
    } else {
        assert_ne!(
            path.extension().and_then(|e| e.to_str()),
            Some("py"),
            "{}",
            path.display()
        );
        let contents = fs::read_to_string(path).unwrap();
        assert!(
            !contents.contains(concat!("python", "3")),
            "interpreter dependency: {}",
            path.display()
        );
    }
}
