//! Trust decisions against the self-signed FTPS and HTTPS test servers.

use super::*;
use crate::fs::remote::connect;
use crate::fs::remote::tests::{ftps_server, test_cert, trust_test_cert, webdavs_server};

fn ftps_url(options: &str) -> (String, crate::fs::remote::tests::Turn) {
    let server = ftps_server();
    let turn = server.turn();
    let url = format!("ftps://127.0.0.1:{}/?{options}", server.addr.port());
    (url, turn)
}

fn webdavs_url(options: &str) -> String {
    format!(
        "webdavs://127.0.0.1:{}/?{options}",
        webdavs_server().addr.port()
    )
}

#[test]
fn self_signed_servers_are_refused_as_untrusted_certificates() {
    let (url, _turn) = ftps_url("");
    let err = connect(&url).unwrap_err();
    let untrusted = untrusted(&err).unwrap_or_else(|| panic!("{err}"));
    assert_eq!(untrusted.host, "127.0.0.1");
    let err = connect(&webdavs_url("")).unwrap_err();
    assert!(super::untrusted(&err).is_some(), "{err:?}");
}

#[test]
fn a_ca_option_trusts_the_certificate() {
    let (url, _turn) = ftps_url(&trust_test_cert());
    connect(&url).unwrap();
    let remote = connect(&webdavs_url(&trust_test_cert())).unwrap();
    assert!(remote.vfs.list(std::path::Path::new("/")).is_ok());
}

#[test]
fn insecure_skips_the_check() {
    let (url, _turn) = ftps_url("insecure=true");
    connect(&url).unwrap();
    connect(&webdavs_url("insecure=true")).unwrap();
}

#[test]
fn fetched_certificates_can_be_pinned_and_are_then_trusted() {
    let _root = crate::config::paths::test_root::redirect(&tempfile::tempdir().unwrap().keep());
    let port = webdavs_server().addr.port();
    let cert = fetch_certificate("127.0.0.1", port, Handshake::Direct).unwrap();
    assert_eq!(cert, test_cert().der);
    let shown = fingerprint(&cert);
    assert_eq!(shown.len(), 32 * 3 - 1);
    assert!(shown.chars().all(|c| c.is_ascii_hexdigit() || c == ':'));

    pin("127.0.0.1", port, &cert).unwrap();
    connect(&webdavs_url("")).unwrap();

    let (_, _turn) = ftps_url("");
    let ftps = ftps_server().addr.port();
    let ftp_cert = fetch_certificate("127.0.0.1", ftps, Handshake::FtpAuthTls).unwrap();
    assert_eq!(ftp_cert, test_cert().der);
}

#[test]
fn ftp_replies_end_on_their_numbered_last_line_and_take_nothing_more() {
    let mut from: &[u8] = b"220-Welcome\r\n 220 not the end\r\nabc def\r\n220 Ready\r\n\x16\x03";
    assert_eq!(ftp_reply(&mut from).unwrap(), "220 Ready\r\n");
    assert_eq!(from, b"\x16\x03");
    let mut cut: &[u8] = b"234-Upgrading";
    assert_eq!(
        ftp_reply(&mut cut).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn a_bad_ca_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.pem");
    std::fs::write(&empty, "not a certificate").unwrap();
    let encoded: String =
        url::form_urlencoded::byte_serialize(empty.to_string_lossy().as_bytes()).collect();
    let err = connect(&webdavs_url(&format!("ca={encoded}"))).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{err}");
}
