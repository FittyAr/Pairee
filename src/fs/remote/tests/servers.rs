//! The in-process servers the remote tests run against, each serving a
//! temporary folder for the rest of the test process: WebDAV
//! (`dav-server`), S3 (`s3s-fs`, which checks request signatures), FTP and
//! FTPS (`libunftp`, with a certificate made at test time).

use crate::fs::archive::test_fixtures::write_tree;
use crate::fs::vfs::contract::TREE;
use rustls::pki_types::pem::PemObject;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// A server running for the rest of the test process.
pub struct Server {
    pub addr: SocketAddr,
    /// The folder it serves.
    pub root: PathBuf,
    /// Taken by a test while it uses an FTP server (see [`Server::turn`]).
    turn: Mutex<()>,
}

/// A test's exclusive use of a server.
pub type Turn = MutexGuard<'static, ()>;

impl Server {
    fn new(addr: SocketAddr, root: PathBuf) -> Self {
        Self {
            addr,
            root,
            turn: Mutex::new(()),
        }
    }

    /// Waits until no other test uses this server. libunftp binds every
    /// passive data port with `SO_REUSEADDR`, which on Windows lets two
    /// parallel sessions listen on one port and cross their data, so FTP
    /// tests take turns.
    pub fn turn(&'static self) -> Turn {
        self.turn.lock().unwrap_or_else(|e| e.into_inner())
    }
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
    Server::new(addr, root)
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

pub fn webdav_server() -> &'static Server {
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

pub const S3_KEY: &str = "pairee-test-key";
pub const S3_SECRET: &str = "pairee-test-secret";
pub const S3_BUCKET: &str = "bucket";

pub fn s3_server() -> &'static Server {
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

/// Passive data ports of one FTP test server.
const PASSIVE_PORTS: u16 = 256;

/// An FTP server; with `tls` (certificate and key PEM files) it also offers
/// explicit TLS (`AUTH TLS`). Its passive ports are a block picked at
/// random, so test processes running at once rarely share one (see
/// [`Server::turn`]); the servers of one process never do.
fn ftp_server_on(tls: Option<(PathBuf, PathBuf)>) -> Server {
    static SEED: OnceLock<u32> = OnceLock::new();
    static TRIED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let seed = *SEED.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
            ^ std::process::id()
    });
    for _ in 0..16 {
        // 19 is prime to 100: the first 100 tries of the process each get
        // a block of their own.
        let attempt = TRIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let block = ((seed % 100 + attempt * 19) % 100) as u16;
        let first = 20_000 + block * PASSIVE_PORTS;
        if let Some(server) = try_ftp_server(first..=first + PASSIVE_PORTS - 1, tls.clone()) {
            return server;
        }
    }
    panic!("no free passive ports for the FTP test server");
}

/// [`ftp_server_on`] with `ports`; `None` when the server cannot listen.
fn try_ftp_server(
    ports: std::ops::RangeInclusive<u16>,
    tls: Option<(PathBuf, PathBuf)>,
) -> Option<Server> {
    let picking = picking_ports();
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let (stopped_tx, stopped) = std::sync::mpsc::channel();
    let root = spawn_server(move |root| async move {
        let mut builder = libunftp::ServerBuilder::new(Box::new(move || {
            unftp_sbe_fs::Filesystem::new(root.clone()).unwrap()
        }))
        .passive_ports(ports);
        if let Some((cert, key)) = tls {
            builder = builder.ftps(cert, key);
        }
        let result = builder.build().unwrap().listen(addr.to_string()).await;
        let _ = stopped_tx.send(result.err().map(|e| e.to_string()));
    });
    // The port is ours once the server greets on it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !greets(addr) {
        if stopped.try_recv().is_ok() {
            return None;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "FTP server not listening"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(picking);
    Some(Server::new(addr, root))
}

/// Whether an FTP server greets at `addr`. A connection alone proves
/// nothing: on Linux, connecting to a free port of the ephemeral range can
/// connect the socket to itself, or reach another server's passive port.
fn greets(addr: SocketAddr) -> bool {
    use std::io::Read;
    let timeout = std::time::Duration::from_secs(1);
    let Ok(mut probe) = std::net::TcpStream::connect_timeout(&addr, timeout) else {
        return false;
    };
    let mut code = [0; 3];
    probe.set_read_timeout(Some(timeout)).is_ok()
        && probe.read_exact(&mut code).is_ok()
        && &code == b"220"
}

pub fn ftp_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| ftp_server_on(None))
}

/// The self-signed certificate (for `localhost` and `127.0.0.1`) the TLS
/// test servers present: its PEM file, its key's PEM file and its DER.
pub struct TestCert {
    pub cert: PathBuf,
    pub key: PathBuf,
    pub der: rustls::pki_types::CertificateDer<'static>,
}

pub fn test_cert() -> &'static TestCert {
    static CERT: OnceLock<TestCert> = OnceLock::new();
    CERT.get_or_init(|| {
        let names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let certified = rcgen::generate_simple_self_signed(names).unwrap();
        let dir = tempfile::tempdir().unwrap().keep();
        let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
        std::fs::write(&cert, certified.cert.pem()).unwrap();
        std::fs::write(&key, certified.signing_key.serialize_pem()).unwrap();
        TestCert {
            cert,
            key,
            der: certified.cert.der().clone(),
        }
    })
}

/// `ca=<the test certificate>`, the URL option that trusts it.
pub fn trust_test_cert() -> String {
    let path = test_cert().cert.to_string_lossy().into_owned();
    let encoded: String = url::form_urlencoded::byte_serialize(path.as_bytes()).collect();
    format!("ca={encoded}")
}

/// FTPS with the self-signed [`test_cert`] (not trusted unless the URL
/// says `ca=`, see [`trust_test_cert`]), behind [`relay_auth_tls`].
pub fn ftps_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| {
        let cert = test_cert();
        let ftps = ftp_server_on(Some((cert.cert.clone(), cert.key.clone())));
        let upstream = ftps.addr;
        let relay = start(move |listener, _| relay_auth_tls(listener, upstream));
        Server::new(relay.addr, ftps.root)
    })
}

/// How long [`relay_auth_tls`] holds a client's TLS handshake.
const AUTH_TLS_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// Relays the connections `listener` accepts to the FTPS server at
/// `upstream`, holding back the first bytes a client sends after
/// `AUTH TLS` (its TLS handshake) for [`AUTH_TLS_SETTLE`].
///
/// libunftp answers `234` to `AUTH TLS` and only then, from a task of its
/// own, tells the session to start TLS. Until that message arrives the
/// session keeps reading commands, so a client that starts its handshake
/// at once (as it should) can have it read as a command: the session then
/// answers `500` in plain text (`InvalidContentType` for the client) or
/// waits for a handshake it already swallowed. That happens on Linux,
/// where the task often runs late on a busy machine.
async fn relay_auth_tls(listener: tokio::net::TcpListener, upstream: SocketAddr) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let (client, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let Ok(server) = tokio::net::TcpStream::connect(upstream).await else {
                return;
            };
            let (mut from_client, mut to_client) = client.into_split();
            let (mut from_server, mut to_server) = server.into_split();
            tokio::spawn(async move {
                let _ = tokio::io::copy(&mut from_server, &mut to_client).await;
            });
            // The plain-text commands sent so far, until `AUTH TLS`.
            let mut commands = Some(Vec::new());
            let mut handshake_next = false;
            let mut buf = vec![0; 16 * 1024];
            loop {
                let n = match from_client.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                if std::mem::take(&mut handshake_next) {
                    tokio::time::sleep(AUTH_TLS_SETTLE).await;
                }
                if let Some(sent) = &mut commands {
                    sent.extend_from_slice(&buf[..n]);
                    if sent.to_ascii_uppercase().ends_with(b"AUTH TLS\r\n") {
                        commands = None;
                        handshake_next = true;
                    }
                }
                if to_server.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
        });
    }
}

/// WebDAV over HTTPS with the self-signed [`test_cert`].
pub fn webdavs_server() -> &'static Server {
    static SERVER: OnceLock<Server> = OnceLock::new();
    SERVER.get_or_init(|| {
        start(|listener, root| async move {
            let cert = test_cert();
            let key = rustls::pki_types::PrivateKeyDer::from_pem_file(&cert.key).unwrap();
            let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
            let config = rustls::ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![cert.der.clone()], key)
                .unwrap();
            let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config));
            let handler = dav_server::DavHandler::builder()
                .filesystem(dav_server::localfs::LocalFs::new(root, false, false, false))
                .locksystem(dav_server::fakels::FakeLs::new())
                .build_handler();
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let (acceptor, handler) = (acceptor.clone(), handler.clone());
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(socket).await else {
                        return;
                    };
                    let service = hyper::service::service_fn(move |req| {
                        let handler = handler.clone();
                        async move { Ok::<_, std::convert::Infallible>(handler.handle(req).await) }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(hyper_util::rt::TokioIo::new(tls), service)
                        .await;
                });
            }
        })
    })
}

/// A folder of its own below `server`'s root, loaded with [`TREE`].
pub fn folder_on(server: &Server, below: &Path) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir_in(server.root.join(below)).unwrap();
    write_tree(dir.path(), TREE);
    let name = crate::fs::file_name_lossy(dir.path());
    (dir, name)
}

/// For tests elsewhere (connection dialog, transfers): an empty folder of
/// its own on the shared FTP (`webdav`: WebDAV) test server, the server's
/// address and the folder's name.
pub(crate) fn served_folder(webdav: bool) -> (tempfile::TempDir, SocketAddr, String, Option<Turn>) {
    let server = if webdav {
        webdav_server()
    } else {
        ftp_server()
    };
    let turn = (!webdav).then(|| server.turn());
    let dir = tempfile::tempdir_in(&server.root).unwrap();
    let name = crate::fs::file_name_lossy(dir.path());
    (dir, server.addr, name, turn)
}
