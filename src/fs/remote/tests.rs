//! The [`crate::fs::vfs::contract`] suite for the remote backends, each
//! against an in-process server (WebDAV: `dav-server`, S3: `s3s-fs`, which
//! checks the request signatures, FTP: `libunftp`) serving a temporary
//! folder. Every fixture gets a folder of its own below the server's root.

use super::{Scheme, connect, remote_path};
use crate::fs::archive::test_fixtures::write_tree;
use crate::fs::vfs::contract::{Fixture, TREE, vfs_contract};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A server running for the rest of the test process.
struct Server {
    addr: SocketAddr,
    /// The folder it serves.
    root: PathBuf,
}

/// Held while a port is picked: libunftp binds its own address, so an FTP
/// server picks a free port, releases it and listens there; meanwhile no
/// other test server may bind (it could be handed the same port).
fn picking_ports() -> std::sync::MutexGuard<'static, ()> {
    static PICKING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    PICKING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `serve` on a thread of its own with a runtime, on a folder that
/// lives as long as the process.
fn spawn_server<F, Fut>(serve: F) -> PathBuf
where
    F: FnOnce(PathBuf) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()>,
{
    let root = tempfile::tempdir().unwrap().keep();
    let served = root.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(serve(served));
    });
    root
}

/// Starts `serve` on a listener of its own.
fn start<F, Fut>(serve: F) -> Server
where
    F: FnOnce(tokio::net::TcpListener, PathBuf) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()>,
{
    let listener = {
        let _picking = picking_ports();
        std::net::TcpListener::bind("127.0.0.1:0").unwrap()
    };
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let root = spawn_server(move |root| async move {
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        serve(listener, root).await;
    });
    Server { addr, root }
}

/// Serves `service` on every connection `listener` accepts (HTTP/1).
async fn serve_http<S, B>(listener: tokio::net::TcpListener, service: S)
where
    S: hyper::service::Service<
            hyper::Request<hyper::body::Incoming>,
            Response = hyper::Response<B>,
        > + Clone
        + Send
        + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let service = service.clone();
        tokio::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(socket);
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await;
        });
    }
}

fn webdav_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| {
        start(|listener, root| async move {
            let handler = dav_server::DavHandler::builder()
                .filesystem(dav_server::localfs::LocalFs::new(root, false, false, false))
                .locksystem(dav_server::fakels::FakeLs::new())
                .build_handler();
            let service = hyper::service::service_fn(move |req| {
                let handler = handler.clone();
                async move { Ok::<_, std::convert::Infallible>(handler.handle(req).await) }
            });
            serve_http(listener, service).await;
        })
    })
}

const S3_KEY: &str = "pairee-test-key";
const S3_SECRET: &str = "pairee-test-secret";
const S3_BUCKET: &str = "bucket";

fn s3_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| {
        let server = start(|listener, root| async move {
            let fs = s3s_fs::FileSystem::new(&root).unwrap();
            let mut builder = s3s::service::S3ServiceBuilder::new(fs);
            builder.set_auth(s3s::auth::SimpleAuth::from_single(S3_KEY, S3_SECRET));
            serve_http(listener, builder.build()).await;
        });
        std::fs::create_dir(server.root.join(S3_BUCKET)).unwrap();
        server
    })
}

/// An FTP server with passive data ports in `ports`; with `tls` (certificate
/// and key PEM files) it also offers explicit TLS (`AUTH TLS`). libunftp
/// binds data ports with `SO_REUSEADDR`, which on Windows lets two sessions
/// share one: wide ranges keep that unlikely while tests run in parallel.
fn ftp_server_on(ports: std::ops::RangeInclusive<u16>, tls: Option<(PathBuf, PathBuf)>) -> Server {
    let picking = picking_ports();
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let root = spawn_server(move |root| async move {
        let mut builder = libunftp::ServerBuilder::new(Box::new(move || {
            unftp_sbe_fs::Filesystem::new(root.clone()).unwrap()
        }))
        .passive_ports(ports);
        if let Some((cert, key)) = tls {
            builder = builder.ftps(cert, key);
        }
        let server = builder.build().unwrap();
        server.listen(addr.to_string()).await.unwrap();
    });
    // The port is ours once the server accepts on it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::net::TcpStream::connect(addr).is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "FTP server not listening"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(picking);
    Server { addr, root }
}

fn ftp_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| ftp_server_on(20_000..=34_999, None))
}

/// FTPS with a self-signed certificate made now, which the client trusts
/// instead of the system's roots (see `ftp::TEST_ROOT`).
fn ftps_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| {
        let names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let certified = rcgen::generate_simple_self_signed(names).unwrap();
        let dir = tempfile::tempdir().unwrap().keep();
        let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
        std::fs::write(&cert, certified.cert.pem()).unwrap();
        std::fs::write(&key, certified.signing_key.serialize_pem()).unwrap();
        let _ = super::ftp::TEST_ROOT.set(certified.cert.der().clone());
        ftp_server_on(35_000..=49_151, Some((cert, key)))
    })
}

/// A folder of its own below `server`'s root, loaded with [`TREE`].
fn folder_on(server: &Server, below: &Path) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir_in(server.root.join(below)).unwrap();
    write_tree(dir.path(), TREE);
    let name = crate::fs::file_name_lossy(dir.path());
    (dir, name)
}

fn fixture(url: &str, dir: tempfile::TempDir) -> Fixture {
    let remote = connect(url).unwrap();
    Fixture {
        vfs: remote.vfs,
        root: remote.start,
        _dir: dir,
        empty_folders: true,
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
    let (dir, name) = folder_on(server, Path::new(""));
    fixture(&format!("ftp://{}/{name}", server.addr), dir)
}

/// For tests elsewhere (connection dialog, transfers): an empty folder of
/// its own on the shared FTP (`webdav`: WebDAV) test server, the server's
/// address and the folder's name.
pub(crate) fn served_folder(webdav: bool) -> (tempfile::TempDir, SocketAddr, String) {
    let server = if webdav {
        webdav_server()
    } else {
        ftp_server()
    };
    let dir = tempfile::tempdir_in(&server.root).unwrap();
    let name = crate::fs::file_name_lossy(dir.path());
    (dir, server.addr, name)
}

fn ftps_fixture() -> Fixture {
    let server = ftps_server();
    let (dir, name) = folder_on(server, Path::new(""));
    fixture(
        &format!("ftps://127.0.0.1:{}/{name}", server.addr.port()),
        dir,
    )
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
