//! FTP connections: how to open and log in (plain or explicit TLS), and
//! the streams of single transfers, each on a connection of its own so a
//! long download never blocks the listing connection (FTP runs one data
//! transfer per connection at a time).

use crate::fs::remote::tls::{self, Handshake, TlsTrust};
use crate::fs::remote::{credentials, invalid};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::time::Duration;
use suppaftp::types::FileType;
use suppaftp::{FtpError, Mode, RustlsConnector, RustlsFtpStream, TransferStream};
use url::Url;

/// A connection (plain until upgraded with `AUTH TLS`).
pub type Conn = RustlsFtpStream;
type Transfer = TransferStream<<Conn as Session>::Tls>;

/// The TLS stream type of [`Conn`].
pub trait Session {
    type Tls: suppaftp::TlsStream;
}

impl<T: suppaftp::TlsStream> Session for suppaftp::ImplFtpStream<T> {
    type Tls = T;
}

/// Default FTP port.
const FTP_PORT: u16 = 21;
/// Time allowed to connect to the server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// Time allowed for one network read or write.
const IO_TIMEOUT: Duration = Duration::from_secs(60);

/// An FTP error as an I/O error.
pub fn io_err(e: FtpError) -> io::Error {
    match e {
        FtpError::ConnectionError(io) => io,
        FtpError::UnexpectedResponse(response) => {
            let kind = match response.status.code() {
                550 => io::ErrorKind::NotFound,
                530 | 532 => io::ErrorKind::PermissionDenied,
                _ => io::ErrorKind::Other,
            };
            io::Error::new(
                kind,
                String::from_utf8_lossy(&response.body).trim().to_string(),
            )
        }
        other => io::Error::other(other.to_string()),
    }
}

/// How to reach and log in to one server.
pub struct Config {
    host: String,
    port: u16,
    user: String,
    password: String,
    tls: Option<Arc<rustls::ClientConfig>>,
    /// What `tls` trusts (to explain a refused certificate).
    trust: Option<TlsTrust>,
}

impl Config {
    /// The server of `url`; anonymous login without a user name.
    pub fn from_url(url: &Url, secure: bool) -> io::Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| invalid(url.as_str(), "no host"))?
            .to_string();
        let (user, password) =
            credentials(url).unwrap_or_else(|| ("anonymous".into(), "anonymous@".into()));
        let port = url.port().unwrap_or(FTP_PORT);
        let trust = match secure {
            true => Some(TlsTrust::for_server(url, &host, port)?),
            false => None,
        };
        let tls = trust.as_ref().map(TlsTrust::rustls_config).transpose()?;
        Ok(Self {
            host,
            port,
            user,
            password,
            tls,
            trust,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// `err` of the TLS upgrade, as an untrusted certificate when it is one.
    fn untrusted(&self, err: io::Error) -> io::Error {
        match &self.trust {
            Some(trust) => tls::classify(err, &self.host, self.port, Handshake::FtpAuthTls, trust),
            None => err,
        }
    }

    /// A new logged-in connection in binary, passive mode.
    pub fn open(&self) -> io::Result<Conn> {
        use std::net::ToSocketAddrs;
        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
        let mut conn = Conn::connect_timeout(addr, CONNECT_TIMEOUT).map_err(io_err)?;
        let _ = conn.get_ref().set_read_timeout(Some(IO_TIMEOUT));
        let _ = conn.get_ref().set_write_timeout(Some(IO_TIMEOUT));
        if let Some(tls) = &self.tls {
            conn = conn
                .into_secure(RustlsConnector::from(Arc::clone(tls)), &self.host)
                .map_err(|e| self.untrusted(io_err(e)))?;
        }
        conn.login(self.user.as_str(), self.password.as_str())
            .map_err(io_err)?;
        conn.transfer_type(FileType::Binary).map_err(io_err)?;
        conn.set_mode(Mode::Passive);
        conn.set_passive_nat_workaround(true);
        Ok(conn)
    }
}

/// A download on a connection of its own. The end of the data is only
/// reported once the server confirms the transfer completed (`226`): a
/// download the server aborted is an error, never a short file.
pub struct Download {
    stream: Option<Transfer>,
    _conn: Conn,
}

impl Download {
    /// `path` from byte `offset` (`REST`).
    pub fn open(config: &Config, path: &str, offset: u64) -> io::Result<Self> {
        let mut conn = config.open()?;
        if offset > 0 {
            let offset = usize::try_from(offset).map_err(io::Error::other)?;
            conn.resume_transfer(offset).map_err(io_err)?;
        }
        let stream = conn.retr_as_stream(path).map_err(io_err)?;
        Ok(Self {
            stream: Some(stream),
            _conn: conn,
        })
    }
}

impl Read for Download {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(stream) = self.stream.as_mut() else {
            return Ok(0);
        };
        let n = stream.read(buf)?;
        if n == 0 && !buf.is_empty() {
            let stream = self.stream.take().expect("checked above");
            stream.finish().map_err(io_err)?;
        }
        Ok(n)
    }
}

/// An upload on a connection of its own; `flush` completes the transfer
/// (the server's `226` reply), after which nothing more can be written.
pub struct UploadStream {
    stream: Option<Transfer>,
    _conn: Conn,
}

impl UploadStream {
    /// `STOR` (`append`: `APPE`) of `path`.
    pub fn open(config: &Config, path: &str, append: bool) -> io::Result<Self> {
        let mut conn = config.open()?;
        let stream = if append {
            conn.append_with_stream(path)
        } else {
            conn.put_with_stream(path)
        }
        .map_err(io_err)?;
        Ok(Self {
            stream: Some(stream),
            _conn: conn,
        })
    }
}

impl Write for UploadStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.stream.as_mut() {
            Some(stream) => stream.write(buf),
            None => Err(io::Error::other("upload already finished")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.stream.take() {
            Some(mut stream) => {
                stream.flush()?;
                stream.finish().map_err(io_err)
            }
            None => Ok(()),
        }
    }
}
