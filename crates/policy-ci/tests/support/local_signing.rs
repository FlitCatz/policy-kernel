use base64::{engine::general_purpose::STANDARD, Engine};
use std::{fs, path::Path, process::Command};

pub const NS: &str = "policy-kernel/local-runner-attestation/v1";
pub const SK: &str = "sk-ssh-ed25519@openssh.com";

pub fn string(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend(bytes);
    out
}

pub fn openssl(executable: &Path, root: &Path, args: &[&str], input: &[u8]) -> Vec<u8> {
    let path = root.join("local-input");
    fs::write(&path, input).unwrap();
    let out = Command::new(executable)
        .args(args)
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// Software-only FIDO-format fixture, following OpenSSH PROTOCOL.sshsig and PROTOCOL.u2f.
pub struct SoftwareKey<'a> {
    pub openssl: &'a Path,
    pub root: &'a Path,
    pub blob: &'a [u8],
}

impl SoftwareKey<'_> {
    pub fn sign(&self, payload: &[u8], flags: u8, namespace: &str) -> String {
        let openssl = |args: &[&str], input: &[u8]| openssl(self.openssl, self.root, args, input);
        let hash = openssl(&["dgst", "-sha512", "-binary"], payload);
        let mut message = b"SSHSIG".to_vec();
        message.extend(string(namespace.as_bytes()));
        message.extend(string(b""));
        message.extend(string(b"sha512"));
        message.extend(string(&hash));

        let mut fido = openssl(&["dgst", "-sha256", "-binary"], b"ssh:fixture");
        fido.push(flags);
        fido.extend(1u32.to_be_bytes());
        fido.extend(openssl(&["dgst", "-sha256", "-binary"], &message));
        let key = self.root.join("local.key");
        let signature = openssl(
            &[
                "pkeyutl",
                "-sign",
                "-rawin",
                "-inkey",
                key.to_str().unwrap(),
                "-in",
            ],
            &fido,
        );
        let mut inner = string(SK.as_bytes());
        inner.extend(string(&signature));
        inner.push(flags);
        inner.extend(1u32.to_be_bytes());

        let mut outer = b"SSHSIG".to_vec();
        outer.extend(1u32.to_be_bytes());
        outer.extend(string(self.blob));
        outer.extend(string(namespace.as_bytes()));
        outer.extend(string(b""));
        outer.extend(string(b"sha512"));
        outer.extend(string(&inner));
        format!(
            "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
            STANDARD.encode(outer)
        )
    }
}
