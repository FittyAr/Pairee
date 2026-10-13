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

    fn complete(&mut self) -> io::Result<()> {
        let store = &self.store;
        let Some((upload_id, etags)) = self.multipart.take() else {
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
            &upload_id,
            etags.iter().map(String::as_str),
        );
        let url = action.sign(SIGNED_FOR);
        let request = store.client.post(url).body(action.body());
        http::send(request, "CompleteMultipartUpload").map(drop)
    }
}

impl Write for S3Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.done {
            return Err(io::Error::other("upload already finished"));
        }
        self.part.extend_from_slice(buf);
        if self.part.len() >= PART_SIZE {
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
