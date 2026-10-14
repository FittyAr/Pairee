//! The [`crate::fs::vfs::contract`] suite for the remote backends, each
//! against an in-process server (WebDAV: `dav-server`, S3: `s3s-fs`, which
//! checks the request signatures, FTP: `libunftp`) serving a temporary
//! folder. Every fixture gets a folder of its own below the server's root.

mod servers;

pub(crate) use servers::{
    Turn, ftps_server, served_folder, test_cert, trust_test_cert, webdavs_server,
};

use super::{Scheme, connect, remote_path};
use crate::fs::vfs::contract::{Fixture, vfs_contract};
use servers::*;
use std::path::Path;

fn fixture(url: &str, dir: tempfile::TempDir) -> Fixture {
    let remote = connect(url).unwrap();
    Fixture {
        vfs: remote.vfs,
        root: remote.start,
        _dir: dir,
        empty_folders: true,
        _hold: None,
    }
}

fn webdav_fixture() -> Fixture {
    let server = webdav_server();
    let (dir, name) = folder_on(server, Path::new(""));
    fixture(&format!("webdav://{}/{name}", server.addr), dir)
}

fn s3_fixture() -> Fixture {
    let server = s3_server();
    let (dir, name) = folder_on(server, Path::new(S3_BUCKET));
    let endpoint = format!("http://{}", server.addr);
    let url = format!(
        "s3://{S3_KEY}:{S3_SECRET}@{S3_BUCKET}/{name}?endpoint={endpoint}&path_style=true&region=us-east-1"
    );
    // s3s-fs keeps folder markers as plain folders, which it never lists.
    Fixture {
        empty_folders: false,
        ..fixture(&url, dir)
    }
}

fn ftp_fixture() -> Fixture {
    let server = ftp_server();
    let turn = server.turn();
    let (dir, name) = folder_on(server, Path::new(""));
    Fixture {
        _hold: Some(Box::new(turn)),
        ..fixture(&format!("ftp://{}/{name}", server.addr), dir)
    }
}

fn ftps_fixture() -> Fixture {
    let server = ftps_server();
    let turn = server.turn();
    let (dir, name) = folder_on(server, Path::new(""));
    let url = format!(
        "ftps://127.0.0.1:{}/{name}?{}",
        server.addr.port(),
        servers::trust_test_cert()
    );
    Fixture {
        _hold: Some(Box::new(turn)),
        ..fixture(&url, dir)
    }
}

vfs_contract!(webdav, super::webdav_fixture);
vfs_contract!(s3, super::s3_fixture);
vfs_contract!(ftp, super::ftp_fixture);
vfs_contract!(ftps, super::ftps_fixture);

/// `stat` asks `MLST` (libunftp answers it) instead of listing the parent.
#[test]
fn ftp_stat_uses_mlst_when_the_server_has_it() {
    use crate::fs::vfs::Vfs;
    let server = ftp_server();
    let _turn = server.turn();
    let (dir, name) = folder_on(server, Path::new(""));
    std::fs::write(dir.path().join("five.txt"), b"12345").unwrap();
    let url = url::Url::parse(&format!("ftp://{}/{name}", server.addr)).unwrap();
    let ftp = super::FtpVfs::open(&url, false).unwrap();
    let folder = Path::new("/").join(&name);
    let entry = ftp.stat(&folder.join("five.txt")).unwrap();
    assert!(ftp.answers_mlst());
    assert_eq!(
        (entry.name.as_str(), entry.size, entry.is_dir),
        ("five.txt", 5, false)
    );
    assert_eq!(entry.path, folder.join("five.txt"));
    assert!(ftp.stat(&folder).unwrap().is_dir);
    let missing = ftp.stat(&folder.join("nope")).unwrap_err();
    assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
}

/// A saved FTP connection opens through `open` with its URL built from the
/// fields; the folder is the start.
#[test]
fn saved_ftp_connections_open_on_their_folder() {
    use crate::config::settings::{Connection, RemoteKind};
    use crate::credential_prompt::CredentialRequest;
    let server = ftp_server();
    let _turn = server.turn();
    let (_dir, name) = folder_on(server, Path::new(""));
    let conn = Connection {
        name: "test".into(),
        kind: RemoteKind::Ftp,
        host: server.addr.ip().to_string(),
        port: server.addr.port().to_string(),
        path: format!("/{name}"),
        ..Connection::default()
    };
    let timeout = std::time::Duration::from_secs(5);
    let never =
        |_: CredentialRequest| -> Option<String> { panic!("an anonymous login asks nothing") };
    let (remote, start) = super::open(&conn, timeout, &never).unwrap();
    assert_eq!(start, Path::new("/").join(&name));
    assert_eq!(remote.info().kind, RemoteKind::Ftp);
    assert!(
        remote
            .list(&start)
            .unwrap()
            .iter()
            .any(|e| e.name == "a.txt")
    );

    // A user without a stored password: the password is asked for.
    let with_user = Connection {
        username: "me".into(),
        ..conn
    };
    let asked = std::sync::Mutex::new(0);
    let ask = |_: CredentialRequest| {
        *asked.lock().unwrap() += 1;
        Some("typed".to_string())
    };
    let (remote, _) = super::open(&with_user, timeout, &ask).unwrap();
    assert_eq!(*asked.lock().unwrap(), 1);
    assert_eq!(remote.user_at_host(), format!("me@{}", server.addr.ip()));
}

#[test]
fn schemes_pick_their_backend() {
    assert_eq!(Scheme::parse("FTPS"), Some(Scheme::Ftp { secure: true }));
    assert_eq!(Scheme::parse("dav"), Some(Scheme::WebDav { secure: false }));
    assert_eq!(
        Scheme::parse("https"),
        Some(Scheme::WebDav { secure: true })
    );
    assert_eq!(Scheme::parse("s3"), Some(Scheme::S3));
    assert_eq!(Scheme::parse("gopher"), None);
    let err = connect("gopher://host/x").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}

#[test]
fn url_errors_never_show_the_password() {
    // An S3 URL without a bucket is refused; the message names the URL.
    for url in ["s3://key:s3cret@/prefix", "ftp://key:s3cret@[bad/x"] {
        let message = connect(url).unwrap_err().to_string();
        assert!(!message.contains("s3cret"), "{message}");
        assert!(message.contains("key@"), "{message}");
    }
}

#[test]
fn remote_paths_use_slashes() {
    let path = Path::new("/a").join("b").join("..").join("c");
    assert_eq!(remote_path(&path), "/a/c");
    assert_eq!(remote_path(Path::new("")), "/");
}

/// Streams larger than one upload chunk or S3 part, and resumed reads.
#[test]
fn large_files_stream_both_ways_and_resume() {
    let data: Vec<u8> = (0..9 * 1024 * 1024u32).map(|i| (i % 253) as u8).collect();
    for f in [webdav_fixture(), s3_fixture(), ftp_fixture()] {
        let path = f.root.join("big.bin");
        f.vfs.write_file(&path, &mut data.as_slice()).unwrap();
        assert_eq!(f.vfs.stat(&path).unwrap().size, data.len() as u64);
        let mut tail = Vec::new();
        std::io::Read::read_to_end(&mut f.vfs.open_read_at(&path, 1000).unwrap(), &mut tail)
            .unwrap();
        assert_eq!(tail, data[1000..]);
    }
}

/// A move within one server is a rename only when nothing is at the
/// target: S3 (copy + delete) and many FTP servers rename over a file, so
/// an existing one goes through the conflict setting (here: skip).
#[test]
fn same_server_moves_respect_existing_targets() {
    use crate::config::settings::RemoteKind;
    use crate::fs::transfer::job::{RemoteEndpoints, TransferJob, TransferOperation};
    use crate::fs::transfer::options::TransferOptions;
    let f = s3_fixture();
    let kept = f.root.join("sub").join("a.txt");
    f.vfs.write_file(&kept, &mut &b"keep"[..]).unwrap();
    let conn = super::RemoteConn::new(
        std::sync::Arc::clone(&f.vfs),
        super::RemoteInfo {
            kind: RemoteKind::S3,
            host: "bucket".into(),
            port: None,
            username: String::new(),
        },
    );
    let options = TransferOptions {
        conflict_resolution: "skip".into(),
        ..TransferOptions::default()
    };
    let mut job = TransferJob::new(
        TransferOperation::Move,
        vec![f.root.join("a.txt")],
        f.root.join("sub"),
        options,
    );
    job.remote = Some(RemoteEndpoints {
        src: Some(conn.clone()),
        dst: Some(conn),
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        crate::fs::transfer::backend::run_job(job, tx)
            .await
            .unwrap();
    });
    let read = |path: &Path| {
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut f.vfs.open_read(path).unwrap(), &mut out).unwrap();
        out
    };
    assert_eq!(read(&kept), b"keep");
    assert_eq!(read(&f.root.join("a.txt")), b"alpha");
}

/// Every address of a name is tried: `localhost` often resolves to `::1`
/// first, and a server listening on IPv4 only must still be reached.
#[test]
fn connections_try_every_address_of_the_host() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let timeout = std::time::Duration::from_secs(5);
    assert!(super::connect_tcp("localhost", port, timeout).is_ok());
    assert!(super::connect_tcp("127.0.0.1", port, timeout).is_ok());
}
