//! Signature and key handling for evidence issued to the advisory PR gate.
use base64::{engine::general_purpose::STANDARD, Engine};
use policy_contracts::{Sha256Digest, UtcTimestamp};
use policy_core::canonical_json;
use policy_native_verifier::contracts::{LocalAttestationV1, LocalSignatureV1};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
    process::{Command, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const DEFAULT_SSH_KEYGEN: &str = "/usr/bin/ssh-keygen";
const NAMESPACE: &str = "policy-kernel/gate-local-attestation/v1";
const ED25519: &[u8] = b"sk-ssh-ed25519@openssh.com";
const ECDSA: &[u8] = b"sk-ecdsa-sha2-nistp256@openssh.com";
const USER_PRESENCE: u8 = 0x01;
const MAX_KEY_BYTES: usize = 16 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateStatement {
    pub pr: u64,
    pub policy_sha256: Sha256Digest,
    pub run: LocalAttestationV1,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateAttestation {
    pub statement: GateStatement,
    pub signature: LocalSignatureV1,
}

impl GateAttestation {
    /// Require a direct, exact-principal key registered by the repository owner.
    pub fn verify(&self, registry: &str, now: &UtcTimestamp) -> Result<()> {
        let statement = &self.statement;
        statement.run.validate()?;
        if self.signature.runner_id != statement.run.runner.runner_id {
            return Err("signature runner differs from statement runner".into());
        }
        let runner = statement.run.runner.runner_id.as_str();
        let mut entries = registry.lines().map(str::trim).filter(|line| {
            !line.starts_with('#') && line.split_ascii_whitespace().next() == Some(runner)
        });
        let entry = entries
            .next()
            .ok_or("local runner has no allowed-signers entry at the base commit")?;
        if entries.next().is_some() {
            return Err(
                "local runner has multiple allowed-signers entries at the base commit".into(),
            );
        }
        let key = registered_key(entry)?;
        let signature = self.signature.value.as_str();
        check_presence(signature, &key)?;

        let scratch = tempfile::tempdir()?;
        let signers = scratch.path().join("allowed_signers");
        let signature_path = scratch.path().join("signature");
        std::fs::write(&signers, format!("{entry}\n"))?;
        std::fs::write(&signature_path, signature)?;
        let mut payload = tempfile::tempfile()?;
        payload.write_all(&canonical_json(statement)?)?;
        payload.rewind()?;
        let status = Command::new(DEFAULT_SSH_KEYGEN)
            .args(["-Y", "verify", "-f"])
            .arg(signers)
            .args(["-I", runner, "-n", NAMESPACE, "-s"])
            .arg(signature_path)
            .args([
                "-O",
                &format!(
                    "verify-time={}",
                    now.truncated_to_seconds()
                        .to_string()
                        .replace(['-', ':', 'T'], "")
                ),
            ])
            .env_clear()
            .stdin(payload)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err("local attestation signature could not be verified".into());
        }
        Ok(())
    }
}

fn registered_key(entry: &str) -> Result<Vec<u8>> {
    if entry.chars().any(|c| c.is_control() && c != '\t') {
        return Err("invalid allowed-signers entry".into());
    }
    let mut fields = entry.split_ascii_whitespace();
    let runner = fields.next().ok_or("missing allowed-signers principal")?;
    if !runner
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
    {
        return Err("allowed-signers principal must be an exact runner ID".into());
    }
    let field = fields.next().ok_or("missing allowed-signers key")?;
    let algorithm = if [ED25519, ECDSA].contains(&field.as_bytes()) {
        field
    } else {
        if field.contains("no-touch-required") || field.contains("cert-authority") {
            return Err("allowed signers must use a direct key that requires touch".into());
        }
        fields.next().ok_or("missing allowed-signers key type")?
    };
    let key = STANDARD.decode(fields.next().ok_or("missing allowed-signers key bytes")?)?;
    let kind = hardware_type(&key)?;
    if kind != algorithm.as_bytes() {
        return Err("allowed-signers key type does not match key bytes".into());
    }
    Ok(key)
}

pub struct SigningKey {
    encoded: Vec<u8>,
    public_key: Vec<u8>,
}

impl SigningKey {
    /// Inspect the actual handle flags before invoking OpenSSH. A public key
    /// alone does not carry these flags.
    pub fn read(path: &Path) -> Result<Self> {
        let mut encoded = Vec::new();
        File::open(path)?
            .take((MAX_KEY_BYTES + 1) as u64)
            .read_to_end(&mut encoded)?;
        if encoded.len() > MAX_KEY_BYTES {
            return Err("OpenSSH key file is too large".into());
        }
        let decoded = unarmor(std::str::from_utf8(&encoded)?, "OPENSSH PRIVATE KEY")?;
        let public_key = touch_key(&decoded)?.to_owned();
        Ok(Self {
            encoded,
            public_key,
        })
    }

    /// Use the bytes that passed preflight, with no SSH-agent or provider override.
    pub fn command(
        &self,
        ssh_keygen: &Path,
        copy: &mut tempfile::NamedTempFile,
    ) -> Result<Command> {
        if !ssh_keygen.is_absolute() {
            return Err("ssh-keygen must be an absolute executable path".into());
        }
        copy.write_all(&self.encoded)?;
        let mut command = Command::new(ssh_keygen);
        command
            .args(["-Y", "sign", "-f"])
            .arg(copy.path())
            .args(["-n", NAMESPACE]);
        command
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("SSH_SK_PROVIDER");
        Ok(command)
    }

    pub fn check_signature(&self, signature: &str) -> Result<()> {
        check_presence(signature, &self.public_key)
    }
}

fn touch_key(mut decoded: &[u8]) -> Result<&[u8]> {
    decoded = decoded
        .strip_prefix(b"openssh-key-v1\0")
        .ok_or("expected an OpenSSH private key")?;
    let cipher = string(&mut decoded)?;
    let kdf = string(&mut decoded)?;
    let options = string(&mut decoded)?;
    if uint32(&mut decoded)? != 1 {
        return Err("expected one OpenSSH private key".into());
    }
    let public_key = string(&mut decoded)?;
    hardware_type(public_key)?;
    if cipher != b"none" || kdf != b"none" || !options.is_empty() {
        return Err("use a dedicated OpenSSH hardware key handle without a file passphrase so its touch setting can be checked".into());
    }
    let mut private = string(&mut decoded)?;
    if !decoded.is_empty() || uint32(&mut private)? != uint32(&mut private)? {
        return Err("invalid OpenSSH key encoding".into());
    }
    private = private
        .strip_prefix(public_key)
        .ok_or("OpenSSH private and public key fields differ")?;
    if byte(&mut private)? & USER_PRESENCE == 0 {
        return Err(
            "no-touch-required keys are refused. Use a hardware key that requires touch".into(),
        );
    }
    if string(&mut private)?.is_empty() {
        return Err("missing hardware key handle".into());
    }
    let _reserved = string(&mut private)?;
    let _comment = string(&mut private)?;
    if private.len() > 8 || private.iter().copied().ne((1u8..).take(private.len())) {
        return Err("invalid OpenSSH key encoding".into());
    }
    Ok(public_key)
}

fn check_presence(signature: &str, public_key: &[u8]) -> Result<()> {
    if signature.len() > MAX_KEY_BYTES {
        return Err("local SSH signature is too large".into());
    }
    let decoded = unarmor(signature, "SSH SIGNATURE")?;
    let mut data = decoded
        .strip_prefix(b"SSHSIG")
        .ok_or("invalid SSH signature")?;
    if uint32(&mut data)? != 1
        || string(&mut data)? != public_key
        || string(&mut data)? != NAMESPACE.as_bytes()
        || !string(&mut data)?.is_empty()
        || ![b"sha256".as_slice(), b"sha512".as_slice()].contains(&string(&mut data)?)
    {
        return Err("local SSH signature has the wrong key, namespace, or format".into());
    }
    let mut signature = string(&mut data)?;
    if !data.is_empty() || string(&mut signature)? != hardware_type(public_key)? {
        return Err("invalid SSH signature encoding".into());
    }
    let _crypto = string(&mut signature)?;
    let flags = byte(&mut signature)?;
    let _counter = uint32(&mut signature)?;
    if !signature.is_empty() || flags & USER_PRESENCE == 0 {
        return Err("local SSH signature requires user presence".into());
    }
    Ok(())
}

fn hardware_type(mut blob: &[u8]) -> Result<&[u8]> {
    let kind = string(&mut blob)?;
    match kind {
        ED25519 if string(&mut blob)?.len() == 32 => {}
        ECDSA if string(&mut blob)? == b"nistp256" && !string(&mut blob)?.is_empty() => {}
        _ => return Err("use an OpenSSH hardware security key (ed25519-sk or ecdsa-sk)".into()),
    }
    if string(&mut blob)?.is_empty() || !blob.is_empty() {
        return Err("invalid OpenSSH hardware public key".into());
    }
    Ok(kind)
}

fn unarmor(text: &str, label: &str) -> Result<Vec<u8>> {
    let mut lines = text.trim_end().lines();
    if lines.next() != Some(format!("-----BEGIN {label}-----").as_str())
        || lines.next_back() != Some(format!("-----END {label}-----").as_str())
    {
        return Err(format!("invalid {label} armor").into());
    }
    Ok(STANDARD.decode(lines.collect::<String>())?)
}

fn bytes<'a>(reader: &mut &'a [u8], length: usize) -> Result<&'a [u8]> {
    let (value, tail) = reader
        .split_at_checked(length)
        .ok_or("truncated OpenSSH field")?;
    *reader = tail;
    Ok(value)
}

fn byte(reader: &mut &[u8]) -> Result<u8> {
    Ok(bytes(reader, 1)?[0])
}

fn uint32(reader: &mut &[u8]) -> Result<u32> {
    Ok(u32::from_be_bytes(bytes(reader, 4)?.try_into()?))
}

fn string<'a>(reader: &mut &'a [u8]) -> Result<&'a [u8]> {
    let length = uint32(reader)? as usize;
    bytes(reader, length)
}
