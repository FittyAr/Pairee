//! Writing an S3 object: data is collected into parts; an object that fits
//! in one part goes up with a single `PUT` on flush, a larger one as a
//! multipart upload (created when the first part is full, completed on
//! flush, aborted when the writer is dropped unflushed — S3 never shows a
//! half-written object).

use super::{SIGNED_FOR, Store};
use crate::fs::remote::http;
use rusty_s3::S3Action;
use rusty_s3::actions::CreateMultipartUpload;
use std::io::{self, Write};
use std::sync::Arc;

/// Size of each uploaded part (S3's minimum is 5 MiB, except the last).
const PART_SIZE: usize = 8 * 1024 * 1024;
/// Parts uploaded at each part size before it doubles: S3 takes at most
/// 10 000 parts, so a fixed size would cap objects at 80 GB.
const PARTS_PER_SIZE: usize = 1000;

/// Size of the next part once `sent` parts are up (8 MiB for the first
/// thousand, then doubling: ten sizes reach past S3's 5 TB object limit).
fn part_size(sent: usize) -> usize {
    PART_SIZE << (sent / PARTS_PER_SIZE).min(10)
}

pub struct S3Upload {
    store: Arc<Store>,
    key: String,
    part: Vec<u8>,
    /// The multipart upload and the ETags of its parts so far.
    multipart: Option<(String, Vec<String>)>,
    done: bool,
}

impl S3Upload {
    pub fn new(store: Arc<Store>, key: String) -> Self {
        Self {
            store,
            key,
            part: Vec::new(),
            multipart: None,
            done: false,
        }
    }

    fn start_multipart(&self) -> io::Result<String> {
        let store = &self.store;
        let action = store
            .bucket
            .create_multipart_upload(store.credentials.as_ref(), &self.key);
        let response = store.send(reqwest::Method::POST, &action, "CreateMultipartUpload")?;
        let xml = http::text(response)?;
        let created = CreateMultipartUpload::parse_response(&xml)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(created.upload_id().to_string())
    }

    /// Uploads the collected part.
    fn send_part(&mut self) -> io::Result<()> {
        if self.multipart.is_none() {
            self.multipart = Some((self.start_multipart()?, Vec::new()));
        }
        let (upload_id, etags) = self.multipart.as_mut().expect("multipart started above");
        let number = u16::try_from(etags.len() + 1).map_err(io::Error::other)?;
        let store = &self.store;
        let action =
            store
                .bucket
                .upload_part(store.credentials.as_ref(), &self.key, number, upload_id);
        let body = std::mem::take(&mut self.part);
        let request = store.client.put(action.sign(SIGNED_FOR)).body(body);
        let response = http::send(request, "UploadPart")?;
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "UploadPart: no ETag"))?;
        etags.push(etag.to_string());
        Ok(())
    }

    /// Puts the object, or completes the multipart upload (left for `Drop`
    /// to abort when completing fails, so no parts linger on the server).
    fn complete(&mut self) -> io::Result<()> {
        let store = &self.store;
        let Some((upload_id, etags)) = self.multipart.as_ref() else {
            let action = store
                .bucket
                .put_object(store.credentials.as_ref(), &self.key);
            let body = std::mem::take(&mut self.part);
            let request = store.client.put(action.sign(SIGNED_FOR)).body(body);
            return http::send(request, "PutObject").map(drop);
        };
        let action = store.bucket.complete_multipart_upload(
            store.credentials.as_ref(),
            &self.key,
            upload_id,
            etags.iter().map(String::as_str),
        );
        let url = action.sign(SIGNED_FOR);
        let request = store.client.post(url).body(action.body());
        http::send(request, "CompleteMultipartUpload")?;
        self.multipart = None;
        Ok(())
    }
}

impl Write for S3Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.done {
            return Err(io::Error::other("upload already finished"));
        }
        self.part.extend_from_slice(buf);
        let sent = self.multipart.as_ref().map_or(0, |(_, etags)| etags.len());
        if self.part.len() >= part_size(sent) {
            self.send_part()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        if self.multipart.is_some() && !self.part.is_empty() {
            self.send_part()?;
        }
        self.done = true;
        self.complete()
    }
}

impl Drop for S3Upload {
    fn drop(&mut self) {
        if let Some((upload_id, _)) = self.multipart.take() {
            let store = &self.store;
            let action = store.bucket.abort_multipart_upload(
                store.credentials.as_ref(),
                &self.key,
                &upload_id,
            );
            let _ = store.send(reqwest::Method::DELETE, &action, "AbortMultipartUpload");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_sizes_grow_past_the_part_count_limit() {
        assert_eq!(part_size(0), PART_SIZE);
        assert_eq!(part_size(999), PART_SIZE);
        assert_eq!(part_size(1000), 2 * PART_SIZE);
        // 10 000 parts cover the largest object S3 stores (5 TB).
        let total: u64 = (0..10_000).map(|n| part_size(n) as u64).sum();
        assert!(total > 5_000_000_000_000);
    }
}
