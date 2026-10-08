//! A text document backed by a [`ByteStore`]: decoded on demand, line index
//! built in the background, bounded memory whatever the file size.

use super::encoding::{bom_len_for, decode_line};
use super::index::{self, IndexJob, LineIndex, SharedIndex};
use super::lines::{LineFormat, for_each_line};
use super::search::{Matcher, SearchFrom, SearchJob};
use super::store::{BLOCK_SIZE, BlockCache, ByteStore, MemStore};
use encoding_rs::{Encoding, UTF_8};
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Cancels the background index build when the last document clone drops.
#[derive(Debug, Default)]
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Lines and bytes of a file (or buffer) in a given encoding.
#[derive(Clone)]
pub struct TextDocument {
    store: Arc<dyn ByteStore>,
    cache: Arc<BlockCache>,
    encoding: &'static Encoding,
    bom_len: u64,
    index: SharedIndex,
    builder: Arc<CancelOnDrop>,
}

impl std::fmt::Debug for TextDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextDocument")
            .field("len", &self.len())
            .field("encoding", &self.encoding.name())
            .field("lines", &self.line_count())
            .finish()
    }
}

impl TextDocument {
    /// A document over `store`; the returned job builds its line index and
    /// must be run (on a worker thread for files).
    pub fn new(
        store: Arc<dyn ByteStore>,
        encoding: &'static Encoding,
        bom_len: usize,
    ) -> (Self, IndexJob) {
        let cache = Arc::new(BlockCache::new(Arc::clone(&store)));
        let mut doc = Self {
            store,
            cache,
            encoding,
            bom_len: bom_len as u64,
            index: SharedIndex::default(),
            builder: Arc::default(),
        };
        let job = doc.restart_index();
        (doc, job)
    }

    /// An in-memory UTF-8 document of `lines`, indexed immediately.
    pub fn from_lines(lines: &[String]) -> Self {
        let (doc, job) = Self::new(
            Arc::new(MemStore::new(lines.join("\n").into_bytes())),
            UTF_8,
            0,
        );
        job.run();
        doc
    }

    /// Switches the encoding; the returned job rebuilds the line index.
    pub fn set_encoding(&mut self, encoding: &'static Encoding) -> IndexJob {
        let head = self.cache.read_range(0, 4).unwrap_or_default();
        self.encoding = encoding;
        self.bom_len = bom_len_for(&head, encoding) as u64;
        self.restart_index()
    }

    fn restart_index(&mut self) -> IndexJob {
        self.builder = Arc::default();
        self.index = Arc::new(RwLock::new(LineIndex::default()));
        IndexJob {
            store: Arc::clone(&self.store),
            format: self.format(),
            start: self.bom_len,
            index: Arc::clone(&self.index),
            cancel: Arc::clone(&self.builder.0),
        }
    }

    fn format(&self) -> LineFormat {
        LineFormat::for_encoding(self.encoding)
    }

    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    /// Size in bytes.
    pub fn len(&self) -> u64 {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lines indexed so far.
    pub fn line_count(&self) -> u64 {
        index::read(&self.index).lines()
    }

    /// Indexing progress 0–99 while the background scan runs, `None` when done.
    pub fn index_progress(&self) -> Option<u8> {
        let index = index::read(&self.index);
        (!index.is_complete())
            .then(|| (index.scanned().min(self.len()) * 100 / self.len().max(1)).min(99) as u8)
    }

    /// Byte offset of `line`, once indexed.
    pub fn line_offset(&self, line: u64) -> Option<u64> {
        let mut found = None;
        self.walk_lines(line, |start, _| {
            found = Some(start);
            ControlFlow::Break(())
        });
        found
    }

    /// Up to `count` decoded lines starting at `first`.
    pub fn lines(&self, first: u64, count: usize) -> Vec<String> {
        let mut out = Vec::with_capacity(count);
        if count > 0 {
            self.walk_lines(first, |_, content| {
                out.push(decode_line(self.encoding, content));
                if out.len() >= count {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });
        }
        out
    }

    /// Calls `f` for the lines from `first` on, through the block cache.
    fn walk_lines<F>(&self, first: u64, mut f: F)
    where
        F: FnMut(u64, &[u8]) -> ControlFlow<()>,
    {
        let Some((offset, mut skip)) = index::read(&self.index).locate(first) else {
            return;
        };
        let result = for_each_line(
            self.cache.as_ref(),
            self.format(),
            offset,
            BLOCK_SIZE,
            |start, content| {
                if skip > 0 {
                    skip -= 1;
                    return ControlFlow::Continue(());
                }
                f(start, content)
            },
        );
        if let Err(e) = result {
            log::warn!("viewer read failed: {e}");
        }
    }

    /// Raw bytes `[offset, offset + len)` (hex view).
    pub fn bytes(&self, offset: u64, len: usize) -> Vec<u8> {
        self.cache
            .read_range(offset, offset.saturating_add(len as u64))
            .unwrap_or_default()
    }

    /// A search for `query` starting at line `from` (wrapping to the top);
    /// `None` while that line is not indexed yet.
    pub fn search(&self, from: u64, query: &str, case_sensitive: bool) -> Option<SearchJob> {
        let index = index::read(&self.index);
        let from = if from >= index.lines() && index.is_complete() {
            0
        } else {
            from
        };
        drop(index);
        let offset = match self.line_offset(from) {
            Some(offset) => offset,
            // An empty document: search nothing from the top.
            None if from == 0 => self.bom_len,
            None => return None,
        };
        Some(SearchJob {
            store: Arc::clone(&self.store),
            format: self.format(),
            encoding: self.encoding,
            from: SearchFrom {
                line: from,
                offset,
                top: self.bom_len,
            },
            matcher: Matcher::new(query, case_sensitive),
        })
    }
}
