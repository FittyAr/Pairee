//! Which host key types `known_hosts` holds for a server, so the handshake
//! asks for one of them first (as OpenSSH does). Without it libssh2 takes
//! its own order (ECDSA before Ed25519 before RSA) and a server known only
//! by its Ed25519 key hands over an ECDSA one, which `known_hosts` then
//! reports as a mismatch.

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use ssh2::{HostKeyType, MethodType, Session};
use std::path::Path;

/// Puts the host key algorithms `known_hosts` has for `host:port` ahead
/// of the others libssh2 supports. Must run before the handshake; a
/// failure only leaves libssh2's default order.
pub(super) fn prefer_known_host_keys(sess: &Session, path: &Path, host: &str, port: u16) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let known = known_key_types(&text, host, port);
    let Ok(supported) = sess.supported_algs(MethodType::HostKey) else {
        return;
    };
    if let Some(prefs) = host_key_prefs(&known, &supported)
        && let Err(e) = sess.method_pref(MethodType::HostKey, &prefs)
    {
        log::warn!("Could not set SSH host key preferences {prefs:?}: {e}");
    }
}

/// Key types (`ssh-ed25519`, `ssh-rsa`...) of the `known_hosts` lines in
/// `text` that name `host:port`, plain or hashed. `@cert-authority` and
/// `@revoked` lines are left out.
pub(super) fn known_key_types(text: &str, host: &str, port: u16) -> Vec<String> {
    let name = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };
    let mut types = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(patterns), Some(key_type)) = (fields.next(), fields.next()) else {
            continue;
        };
        if patterns.split(',').any(|p| names_host(p, &name)) && !types.iter().any(|t| t == key_type)
        {
            types.push(key_type.to_string());
        }
    }
    types
}

/// When `known_hosts` has keys for `host:port` but none of the `received`
/// type: the received type's name and the known ones. libssh2 then calls
/// the check a mismatch, though nothing could be compared (a build without
/// Ed25519 support handed an RSA key, say).
pub(super) fn unknown_key_type(
    path: &Path,
    host: &str,
    port: u16,
    received: HostKeyType,
) -> Option<(&'static str, Vec<String>)> {
    let name = key_type_name(received)?;
    let known = known_key_types(&std::fs::read_to_string(path).ok()?, host, port);
    (!known.is_empty() && !known.iter().any(|t| t == name)).then_some((name, known))
}

/// The `known_hosts` name of a host key type.
fn key_type_name(kind: HostKeyType) -> Option<&'static str> {
    Some(match kind {
        HostKeyType::Rsa => "ssh-rsa",
        HostKeyType::Dss => "ssh-dss",
        HostKeyType::Ecdsa256 => "ecdsa-sha2-nistp256",
        HostKeyType::Ecdsa384 => "ecdsa-sha2-nistp384",
        HostKeyType::Ecdsa521 => "ecdsa-sha2-nistp521",
        HostKeyType::Ed25519 => "ssh-ed25519",
        HostKeyType::Unknown => return None,
    })
}

/// Whether one host pattern of a `known_hosts` line is `name`.
fn names_host(pattern: &str, name: &str) -> bool {
    match pattern.strip_prefix("|1|") {
        Some(hashed) => hashed_names_host(hashed, name),
        None => pattern.eq_ignore_ascii_case(name),
    }
}

/// `salt|hash` (base64) of a hashed entry: the hash is HMAC-SHA1 of the
/// host name keyed by the salt.
fn hashed_names_host(hashed: &str, name: &str) -> bool {
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD;
    let Some((salt, hash)) = hashed.split_once('|') else {
        return false;
    };
    let (Ok(salt), Ok(hash)) = (b64.decode(salt), b64.decode(hash)) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(&salt) else {
        return false;
    };
    mac.update(name.as_bytes());
    mac.verify_slice(&hash).is_ok()
}

/// The host key preference list: the algorithms of the `known` key types
/// first, then every other `supported` one (so a server with a new key
/// still connects and gets the "not in known_hosts" answer). `None` when
/// nothing known is supported.
pub(super) fn host_key_prefs(known: &[String], supported: &[&str]) -> Option<String> {
    let mut first: Vec<&str> = Vec::new();
    for key_type in known {
        // An RSA key is offered under its SHA-2 signature names too.
        let algs = match key_type.as_str() {
            "ssh-rsa" => vec!["rsa-sha2-512", "rsa-sha2-256", "ssh-rsa"],
            other => vec![other],
        };
        for alg in algs {
            if supported.contains(&alg) && !first.contains(&alg) {
                first.push(alg);
            }
        }
    }
    if first.is_empty() {
        return None;
    }
    let rest = supported.iter().filter(|a| !first.contains(a));
    let all: Vec<&str> = first.iter().copied().chain(rest.copied()).collect();
    Some(all.join(","))
}
