use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"];
const BINARIES: [&str; 2] = ["policy-cli", "policy-native-importer"];

/// Package only the agreed native binaries; never checksum Cargo output indiscriminately.
pub fn assets(input: &Path, output: &Path, commit: &str) -> Result<(), String> {
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source commit must be a full lowercase Git SHA".into());
    }
    let mut files = Vec::new();
    for target in TARGETS {
        for binary in BINARIES {
            let path = input.join(target).join(binary);
            let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if bytes.is_empty() {
                return Err(format!("empty release binary: {}", path.display()));
            }
            files.push((format!("{binary}-{target}"), bytes));
        }
    }
    files.push(("SOURCE_COMMIT".into(), format!("{commit}\n").into_bytes()));
    fs::create_dir(output).map_err(|e| e.to_string())?;
    let mut sums = String::new();
    for (name, bytes) in files {
        sums.push_str(&format!("{:x}  {name}\n", Sha256::digest(&bytes)));
        fs::write(output.join(name), bytes).map_err(|e| e.to_string())?;
    }
    fs::write(output.join("SHA256SUMS"), sums).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_assets_require_both_targets_and_hash_exact_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("input");
        let output = temp.path().join("dist");
        let commit = "a".repeat(40);
        for target in TARGETS {
            fs::create_dir_all(input.join(target)).unwrap();
            for binary in BINARIES {
                fs::write(input.join(target).join(binary), b"abc").unwrap();
            }
        }
        let missing = input.join(TARGETS[1]).join(BINARIES[1]);
        fs::remove_file(&missing).unwrap();
        assert!(assets(&input, &output, &commit).is_err());
        assert!(!output.exists());
        fs::write(missing, b"abc").unwrap();
        assert!(assets(&input, &output, "bad").is_err());
        assets(&input, &output, &commit).unwrap();
        let sums = fs::read_to_string(output.join("SHA256SUMS")).unwrap();
        assert_eq!(sums.lines().count(), 5);
        for target in TARGETS {
            for binary in BINARIES {
                let name = format!("{binary}-{target}");
                assert_eq!(fs::read(output.join(&name)).unwrap(), b"abc");
                assert!(sums.contains(&format!(
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  {name}\n"
                )));
            }
        }
        assert_eq!(
            fs::read_to_string(output.join("SOURCE_COMMIT")).unwrap(),
            format!("{commit}\n")
        );
        assert!(assets(&input, &output, &commit).is_err());
    }
}
