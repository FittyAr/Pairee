//! Does a private key file need a passphrase? Read from its header, so the
//! passphrase is only asked for keys that are actually encrypted.

use base64::Engine as _;
use std::path::Path;

/// Largest key file read (keys are a few KiB).
const MAX_KEY_BYTES: u64 = 64 * 1024;
const OPENSSH_BEGIN: &str = "-----BEGIN OPENSSH PRIVATE KEY-----";
const OPENSSH_END: &str = "-----END OPENSSH PRIVATE KEY-----";
const OPENSSH_MAGIC: &[u8] = b"openssh-key-v1\0";

/// Private keys probed in `~/.ssh` when none is configured, in order of
/// preference (as `ssh` does).
const DEFAULT_KEY_NAMES: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

/// `~/.ssh/<name>` for every default key name.
pub fn default_key_paths() -> Vec<std::path::PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| {
            let ssh_dir = dirs.home_dir().join(".ssh");
            DEFAULT_KEY_NAMES.iter().map(|n| ssh_dir.join(n)).collect()
        })
        .unwrap_or_default()
}

/// `true` when the key at `path` is encrypted (OpenSSH, PEM or PKCS#8,
/// PuTTY). An unreadable file is reported as not encrypted: the login
/// attempt then shows the real error.
pub fn is_encrypted(path: &Path) -> bool {
    use std::io::Read;
    let mut text = String::new();
    let read =
        std::fs::File::open(path).and_then(|f| f.take(MAX_KEY_BYTES).read_to_string(&mut text));
    read.is_ok() && text_is_encrypted(&text)
}

/// [`is_encrypted`] on the text of a key file.
pub fn text_is_encrypted(text: &str) -> bool {
    if text.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----")
        || text.contains("Proc-Type: 4,ENCRYPTED")
    {
        return true;
    }
    if let Some(line) = text.lines().find(|l| l.starts_with("Encryption:")) {
        return line.trim_start_matches("Encryption:").trim() != "none";
    }
    openssh_cipher(text).is_some_and(|cipher| cipher != b"none")
}

/// The cipher name of an OpenSSH-format key (`none`: not encrypted).
fn openssh_cipher(text: &str) -> Option<Vec<u8>> {
    let start = text.find(OPENSSH_BEGIN)? + OPENSSH_BEGIN.len();
    let end = text[start..].find(OPENSSH_END)? + start;
    let body: String = text[start..end].split_whitespace().collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .ok()?;
    let rest = bytes.strip_prefix(OPENSSH_MAGIC)?;
    let len = u32::from_be_bytes(rest.get(..4)?.try_into().ok()?) as usize;
    rest.get(4..4 + len).map(<[u8]>::to_vec)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The text of an OpenSSH key whose cipher is `cipher` (header only:
    /// enough for detection).
    pub fn openssh_key(cipher: &str) -> String {
        let mut bytes = OPENSSH_MAGIC.to_vec();
        bytes.extend((cipher.len() as u32).to_be_bytes());
        bytes.extend(cipher.as_bytes());
        bytes.extend([0, 0, 0, 4]);
        bytes.extend(b"none");
        let body = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!("{OPENSSH_BEGIN}\n{body}\n{OPENSSH_END}\n")
    }

    #[test]
    fn openssh_keys_report_their_cipher() {
        assert!(text_is_encrypted(&openssh_key("aes256-ctr")));
        assert!(!text_is_encrypted(&openssh_key("none")));
    }

    #[test]
    fn pem_pkcs8_and_putty_keys_are_recognized() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,00\n\nAAAA\n";
        assert!(text_is_encrypted(pem));
        assert!(text_is_encrypted(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n"
        ));
        assert!(!text_is_encrypted(
            "-----BEGIN RSA PRIVATE KEY-----\nAAAA\n"
        ));
        assert!(text_is_encrypted(
            "PuTTY-User-Key-File-3: ssh-ed25519\nEncryption: aes256-cbc\n"
        ));
        assert!(!text_is_encrypted(
            "PuTTY-User-Key-File-3: ssh-ed25519\nEncryption: none\n"
        ));
    }

    #[test]
    fn missing_or_garbled_files_are_not_encrypted() {
        assert!(!is_encrypted(Path::new("/no/such/key")));
        assert!(!text_is_encrypted(&format!(
            "{OPENSSH_BEGIN}\n!!!\n{OPENSSH_END}"
        )));
    }
}
