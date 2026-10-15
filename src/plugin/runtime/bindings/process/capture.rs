//! Bounded collection of what a child writes, and bounded waiting.
//!
//! A plugin could otherwise make Pairee buffer unlimited output
//! (`yes`-like tools) or wait forever on a hung program. Output past
//! [`OUTPUT_LIMIT`] or a run past its timeout kills the child and fails
//! the call.

use std::io;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Child;

/// Most bytes of stdout (and, separately, stderr) a plugin gets back.
pub const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
/// Default time `Command:output()` waits.
pub const OUTPUT_TIMEOUT: Duration = Duration::from_secs(300);
/// Default time `Command:status()` waits (it is often interactive).
pub const STATUS_TIMEOUT: Duration = Duration::from_secs(3600);
/// Longest timeout a plugin can ask for with `Command:timeout(seconds)`.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 3600);

/// All of `reader` (if any), failing once it passes `limit` bytes.
pub async fn read_limited<R: AsyncRead + Unpin>(
    reader: Option<R>,
    limit: usize,
) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(reader) = reader {
        reader
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .await?;
    }
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("output exceeds {limit} bytes"),
        ));
    }
    Ok(bytes)
}

/// Waits for `child` to exit, killing it when `timeout` passes.
pub async fn wait_within(child: &mut Child, timeout: Duration) -> io::Result<ExitStatus> {
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => status,
        Err(_) => Err(timed_out(child, timeout).await),
    }
}

/// Collects the output of `child` (stdout and stderr, each capped) and its
/// exit status, killing it when it exceeds the cap or `timeout`.
pub async fn collect(
    child: &mut Child,
    timeout: Duration,
) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    let work = async {
        let (out, err) = tokio::try_join!(
            read_limited(stdout, OUTPUT_LIMIT),
            read_limited(stderr, OUTPUT_LIMIT)
        )?;
        Ok::<_, io::Error>((out, err))
    };
    let (out, err) = match tokio::time::timeout(timeout, work).await {
        Ok(Ok(both)) => both,
        Ok(Err(e)) => {
            let _ = child.start_kill();
            return Err(e);
        }
        Err(_) => return Err(timed_out(child, timeout).await),
    };
    let status = wait_within(child, timeout).await?;
    Ok((status, out, err))
}

async fn timed_out(child: &mut Child, timeout: Duration) -> io::Error {
    let _ = child.kill().await;
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "the command did not finish within {} s and was killed",
            timeout.as_secs()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn output_past_the_limit_is_an_error() {
        let data = [b'x'; 11];
        assert!(read_limited(Some(&data[..]), 10).await.is_err());
        assert_eq!(read_limited(Some(&data[..]), 11).await.unwrap().len(), 11);
        assert!(read_limited(None::<&[u8]>, 1).await.unwrap().is_empty());
    }
}
