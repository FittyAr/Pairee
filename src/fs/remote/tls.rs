//! Which TLS certificates FTPS and WebDAV over HTTPS trust.
//!
//! By default the system's roots (the platform verifier, as browsers do).
//! A connection can add more through its options:
//!
//! * `ca=<file.pem>`: certificates (a private CA, or a server's own
//!   self-signed certificate) trusted besides the system's roots;
//! * `insecure=true`: no certificate check at all. Anyone on the network
//!   path can then read the password; Pairee warns every time it connects.
//!
//! A certificate the user accepted when Pairee asked ("trust this
//! certificate?", with its SHA-256 fingerprint, through the credential
//! request channel) is saved as `<config>/certs/<host>_<port>.pem` and
//! trusted for that server from then on (see [`pin`]).

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Time allowed to fetch a server's certificate.
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// What a connection trusts besides (or instead of) the system's roots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TlsTrust {
    /// Extra trusted certificates (DER).
    pub extra: Vec<CertificateDer<'static>>,
    /// No certificate check at all.
    pub insecure: bool,
}

/// The server's certificate is not trusted (unknown issuer, self-signed,
/// wrong name, ...). Carried inside an `io::Error` so the connection code
/// can offer to trust it.
#[derive(Debug)]
pub struct UntrustedCertificate {
    pub host: String,
    pub port: u16,
    pub detail: String,
}

impl std::fmt::Display for UntrustedCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = crate::config::localization::t("remote_tls_untrusted")
            .replacen("{}", &format!("{}:{}", self.host, self.port), 1)
            .replacen("{}", &self.detail, 1);
        f.write_str(&text)
    }
}

impl std::error::Error for UntrustedCertificate {}

/// The [`UntrustedCertificate`] inside `err`, if that is what it is.
pub fn untrusted(err: &io::Error) -> Option<&UntrustedCertificate> {
    err.get_ref()?.downcast_ref::<UntrustedCertificate>()
}

/// After a failed connection to `host:port`: an [`UntrustedCertificate`]
/// error when the server's certificate is what `trust` refuses (checked
/// with a handshake of our own, whatever language or TLS stack reported
/// the failure), else `err` unchanged.
pub fn classify(
    err: io::Error,
    host: &str,
    port: u16,
    handshake: Handshake,
    trust: &TlsTrust,
) -> io::Error {
    if trust.insecure {
        return err;
    }
    let Ok(config) = trust.rustls_config() else {
        return err;
    };
    match shake_hands(host, port, handshake, config) {
        Err(e) if is_certificate_error(&e) => io::Error::new(
            io::ErrorKind::InvalidData,
            UntrustedCertificate {
                host: host.to_string(),
                port,
                detail: e.to_string(),
            },
        ),
        _ => err,
    }
}

/// Whether a handshake failed on the server's certificate.
fn is_certificate_error(err: &io::Error) -> bool {
    err.get_ref()
        .and_then(|e| e.downcast_ref::<rustls::Error>())
        .is_some_and(|e| {
            matches!(
                e,
                rustls::Error::InvalidCertificate(_) | rustls::Error::NoCertificatesPresented
            )
        })
}

impl TlsTrust {
    /// The trust of the server `host:port` given the URL options
    /// (`ca=`, `insecure=`) and the certificate pinned for it.
    pub fn for_server(url: &url::Url, host: &str, port: u16) -> io::Result<Self> {
        let mut trust = Self::default();
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "ca" => trust.extra.extend(read_pem(Path::new(value.as_ref()))?),
                "insecure" => trust.insecure = matches!(value.as_ref(), "true" | "1" | "yes"),
                _ => {}
            }
        }
        let pinned = pinned_path(host, port);
        if pinned.is_file() {
            trust.extra.extend(read_pem(&pinned)?);
        }
        Ok(trust)
    }

    /// The rustls configuration (FTPS) for this trust.
    pub fn rustls_config(&self) -> io::Result<Arc<rustls::ClientConfig>> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let builder = rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()
            .map_err(io::Error::other)?;
        let verifier: Arc<dyn ServerCertVerifier> = if self.insecure {
            Arc::new(AcceptAny::new(provider))
        } else {
            Arc::new(
                rustls_platform_verifier::Verifier::new_with_extra_roots(
                    self.extra.iter().cloned(),
                    provider,
                )
                .map_err(io::Error::other)?,
            )
        };
        Ok(Arc::new(
            builder
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth(),
        ))
    }

    /// `builder` (HTTPS for WebDAV) with this trust applied.
    pub fn apply(&self, builder: reqwest::ClientBuilder) -> io::Result<reqwest::ClientBuilder> {
        let certs = self
            .extra
            .iter()
            .map(|der| reqwest::Certificate::from_der(der))
            .collect::<Result<Vec<_>, _>>()
            .map_err(io::Error::other)?;
        Ok(builder
            .tls_certs_merge(certs)
            .danger_accept_invalid_certs(self.insecure))
    }
}

/// The certificates of a PEM file.
fn read_pem(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let certs = CertificateDer::pem_file_iter(path)
        .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{}: {e}", path.display()),
            )
        })?;
    if certs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: no certificate", path.display()),
        ));
    }
    Ok(certs)
}

/// Where the certificate the user trusted for `host:port` is kept.
pub fn pinned_path(host: &str, port: u16) -> PathBuf {
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    crate::config::paths::get_config_dir()
        .join("certs")
        .join(format!("{safe}_{port}.pem"))
}

/// Trusts `cert` for `host:port` from now on.
pub fn pin(host: &str, port: u16, cert: &CertificateDer<'_>) -> io::Result<()> {
    use base64::Engine as _;
    let body = base64::engine::general_purpose::STANDARD.encode(cert.as_ref());
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(line));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    let path = pinned_path(host, port);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, pem)
}

/// The SHA-256 fingerprint of `cert` (`AB:CD:...`), as browsers show it.
pub fn fingerprint(cert: &CertificateDer<'_>) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(cert.as_ref())
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// How the TLS session of a server starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handshake {
    /// TLS from the first byte (HTTPS).
    Direct,
    /// FTP's explicit TLS: `AUTH TLS` first.
    FtpAuthTls,
}

/// The certificate `host:port` presents (no check, nothing sent but the
/// handshake), to show it to the user before trusting it.
pub fn fetch_certificate(
    host: &str,
    port: u16,
    handshake: Handshake,
) -> io::Result<CertificateDer<'static>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let capture = Arc::new(AcceptAny::new(Arc::clone(&provider)));
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .dangerous()
        .with_custom_certificate_verifier(capture.clone())
        .with_no_client_auth();
    shake_hands(host, port, handshake, Arc::new(config))?;
    capture
        .seen()
        .ok_or_else(|| io::Error::other("the server sent no certificate"))
}

/// A TLS handshake with `host:port` under `config`, nothing else sent.
fn shake_hands(
    host: &str,
    port: u16,
    handshake: Handshake,
    config: Arc<rustls::ClientConfig>,
) -> io::Result<()> {
    use std::net::ToSocketAddrs;
    let addr = (host, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    let mut tcp = std::net::TcpStream::connect_timeout(&addr, PROBE_TIMEOUT)?;
    tcp.set_read_timeout(Some(PROBE_TIMEOUT))?;
    tcp.set_write_timeout(Some(PROBE_TIMEOUT))?;
    if handshake == Handshake::FtpAuthTls {
        auth_tls(&mut tcp)?;
    }
    let name = ServerName::try_from(host.to_string()).map_err(io::Error::other)?;
    let mut tls = rustls::ClientConnection::new(config, name).map_err(io::Error::other)?;
    while tls.is_handshaking() {
        tls.complete_io(&mut tcp)?;
    }
    Ok(())
}

/// Reads the FTP greeting and asks for explicit TLS.
fn auth_tls(tcp: &mut std::net::TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(tcp.try_clone()?);
    let reply = |reader: &mut BufReader<std::net::TcpStream>| -> io::Result<String> {
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            // The last line of a reply is "NNN text" (a dash continues it).
            if line.len() >= 4 && line.as_bytes()[3] == b' ' {
                return Ok(line);
            }
        }
    };
    reply(&mut reader)?;
    tcp.write_all(b"AUTH TLS\r\n")?;
    let answer = reply(&mut reader)?;
    if answer.starts_with("234") {
        Ok(())
    } else {
        Err(io::Error::other(answer.trim().to_string()))
    }
}

/// A verifier that accepts any certificate (still checking that the
/// handshake was signed by its key), remembering the last one seen: used
/// to fetch a certificate for the user and for `insecure=true`.
#[derive(Debug)]
struct AcceptAny {
    provider: Arc<CryptoProvider>,
    seen: Mutex<Option<CertificateDer<'static>>>,
}

impl AcceptAny {
    fn new(provider: Arc<CryptoProvider>) -> Self {
        Self {
            provider,
            seen: Mutex::new(None),
        }
    }

    fn seen(&self) -> Option<CertificateDer<'static>> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(end_entity.clone().into_owned());
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests;
