//! The HTTP side of the WebDAV and S3 backends: the project's HTTP client
//! (`reqwest`, async) driven from the synchronous [`crate::fs::vfs::Vfs`]
//! calls.
//!
//! Requests run on one process-wide runtime of their own, and a caller
//! waits for the result on a plain channel ([`block_on`]), so the adapters
//! can be called from any thread — a background job, a blocking pool
//! thread or the async runtime's own threads — without the "cannot block
//! inside a runtime" panics of a blocking client.

use bytes::Bytes;
use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Bytes sent to the server per upload chunk.
const UPLOAD_CHUNK: usize = 256 * 1024;
/// Upload chunks queued ahead of the network.
const UPLOAD_QUEUE: usize = 4;

/// The runtime HTTP requests run on (never dropped).
fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("pairee-http")
            .enable_all()
            .build()
            .expect("HTTP runtime")
    })
}

/// Runs `future` on the HTTP runtime and waits for its output.
pub fn block_on<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    runtime().spawn(async move {
        let _ = tx.send(future.await);
    });
    rx.recv().expect("HTTP task finished without a result")
}

/// Starts `future` on the HTTP runtime without waiting.
pub fn spawn<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> tokio::task::JoinHandle<T> {
    runtime().spawn(future)
}

/// An HTTP error as an I/O error.
pub fn io_error(e: reqwest::Error) -> io::Error {
    let kind = if e.is_timeout() {
        io::ErrorKind::TimedOut
    } else if e.is_connect() {
        io::ErrorKind::ConnectionRefused
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, e)
}

/// The I/O error of a response with a failure `status`.
pub fn status_error(status: reqwest::StatusCode, what: &str) -> io::Error {
    let kind = match status {
        reqwest::StatusCode::NOT_FOUND => io::ErrorKind::NotFound,
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            io::ErrorKind::PermissionDenied
        }
        reqwest::StatusCode::CONFLICT | reqwest::StatusCode::PRECONDITION_FAILED => {
            io::ErrorKind::AlreadyExists
        }
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, format!("{what}: HTTP {status}"))
}

/// Sends `request` and fails on a non-success status.
pub fn send(request: reqwest::RequestBuilder, what: &str) -> io::Result<reqwest::Response> {
    let what = what.to_string();
    block_on(async move {
        let response = request.send().await.map_err(io_error)?;
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(status_error(response.status(), &what))
        }
    })
}

/// The body of `response` as text.
pub fn text(response: reqwest::Response) -> io::Result<String> {
    block_on(async move { response.text().await.map_err(io_error) })
}

/// A response body read as a stream, one network chunk at a time.
pub struct BodyReader {
    response: Option<reqwest::Response>,
    pending: Bytes,
}

impl BodyReader {
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            response: Some(response),
            pending: Bytes::new(),
        }
    }
}

impl Read for BodyReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.pending.is_empty() {
            let Some(mut response) = self.response.take() else {
                return Ok(0);
            };
            let (response, chunk) = block_on(async move {
                let chunk = response.chunk().await;
                (response, chunk)
            });
            match chunk.map_err(io_error)? {
                Some(bytes) => {
                    self.pending = bytes;
                    self.response = Some(response);
                }
                None => return Ok(0),
            }
        }
        let n = buf.len().min(self.pending.len());
        buf[..n].copy_from_slice(&self.pending.split_to(n));
        Ok(n)
    }
}

/// A request body fed by an [`Upload`] writer.
struct ChannelBody(mpsc::Receiver<io::Result<Bytes>>);

impl http_body::Body for ChannelBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, io::Error>>> {
        self.0
            .poll_recv(cx)
            .map(|chunk| chunk.map(|r| r.map(http_body::Frame::data)))
    }
}

/// A streamed upload: written data goes to the server as it comes (chunked
/// transfer encoding); `flush` ends the body and waits for the server's
/// answer, which makes the file durable. Dropping it unflushed aborts the
/// request.
pub struct Upload {
    tx: Option<mpsc::Sender<io::Result<Bytes>>>,
    buffer: Vec<u8>,
    response: Option<tokio::task::JoinHandle<io::Result<()>>>,
}

impl Upload {
    /// Starts the request `build` makes with a streamed body.
    pub fn start(build: impl FnOnce(reqwest::Body) -> reqwest::RequestBuilder, what: &str) -> Self {
        let (tx, rx) = mpsc::channel(UPLOAD_QUEUE);
        let request = build(reqwest::Body::wrap(ChannelBody(rx)));
        let what = what.to_string();
        let response = spawn(async move {
            let response = request.send().await.map_err(io_error)?;
            if response.status().is_success() {
                Ok(())
            } else {
                Err(status_error(response.status(), &what))
            }
        });
        Self {
            tx: Some(tx),
            buffer: Vec::with_capacity(UPLOAD_CHUNK),
            response: Some(response),
        }
    }

    fn send_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let Some(tx) = self.tx.clone() else {
            return Err(io::Error::other("upload already finished"));
        };
        let chunk = Bytes::from(std::mem::take(&mut self.buffer));
        let sent = block_on(async move { tx.send(Ok(chunk)).await.is_ok() });
        if sent {
            return Ok(());
        }
        // The request ended early: its result says why.
        self.tx = None;
        self.finish()
            .and(Err(io::Error::from(io::ErrorKind::BrokenPipe)))
    }

    fn finish(&mut self) -> io::Result<()> {
        self.tx = None;
        let Some(response) = self.response.take() else {
            return Ok(());
        };
        block_on(response).map_err(|e| io::Error::other(e.to_string()))?
    }
}

impl Write for Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.tx.is_none() {
            return Err(io::Error::other("upload already finished"));
        }
        self.buffer.extend_from_slice(buf);
        if self.buffer.len() >= UPLOAD_CHUNK {
            self.send_buffer()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.send_buffer()?;
        self.finish()
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            // An unfinished upload is aborted (the body fails and the
            // connection is dropped), never committed.
            let _ = tx.try_send(Err(io::Error::from(io::ErrorKind::Interrupted)));
            if let Some(response) = self.response.take() {
                response.abort();
            }
        }
    }
}
