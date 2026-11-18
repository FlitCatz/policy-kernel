#![forbid(unsafe_code)]

mod local_signing;
#[path = "../../../policy-local-attest/tests/support/native_runner.rs"]
mod native_runner;

use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let root = PathBuf::from(&args[1]);
    let openssl = Path::new(&args[2]);
    match args[0].to_str().unwrap() {
        "run" => {
            assert_eq!(args.len(), 4);
            assert_eq!(args[3], "literal spaces; and $metacharacters");
            native_runner::emit(&root, "pass", |bytes| {
                let digest =
                    local_signing::openssl(openssl, &root, &["dgst", "-sha256", "-r"], bytes);
                String::from_utf8(digest)
                    .unwrap()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .to_owned()
            });
        }
        "sign" => {
            assert_eq!(args.len(), 3);
            let key_blob = fs::read(root.join("local-key-blob")).unwrap();
            let mut payload = Vec::new();
            io::stdin().read_to_end(&mut payload).unwrap();
            fs::write(root.join("attester-signing-input"), &payload).unwrap();
            let key = local_signing::SoftwareKey {
                openssl,
                root: &root,
                blob: &key_blob,
            };
            print!("{}", key.sign(&payload, 1, local_signing::NS));
        }
        _ => panic!("invalid fixture command"),
    }
}
