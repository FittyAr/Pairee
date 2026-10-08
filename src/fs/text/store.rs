//! Random-access byte sources for the viewer: a file read with positioned
//! reads, an in-memory buffer, and a small LRU block cache in front of either.

use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Size of one cached block. Even, so UTF-16 code units never straddle the
/// start of a block read.
pub const BLOCK_SIZE: usize = 64 * 1024;
/// Blocks kept by a [`BlockCache`] (bounded memory: 4 MiB).
const CACHE_BLOCKS: usize = 64;

/// Bytes that can be read at any offset from several threads.
pub trait ByteStore: Send + Sync {
    /// Total length in bytes (fixed when the store was opened).
    fn len(&self) -> u64;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads up to `buf.len()` bytes at `offset`; returns fewer only at the end.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

    /// Reads `[start, end)` (clamped to the length) into a new buffer.
    fn read_range(&self, start: u64, end: u64) -> io::Result<Vec<u8>> {
        let end = end.min(self.len());
        let mut buf = vec![0; end.saturating_sub(start) as usize];
        let n = self.read_at(start, &mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }
}

/// A file read with positioned reads (the file is never loaded whole).
pub struct FileStore {
    file: std::fs::File,
    len: u64,
}

impl FileStore {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    #[cfg(unix)]
    fn pread(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        std::os::unix::fs::FileExt::read_at(&self.file, buf, offset)
    }

    #[cfg(windows)]
    fn pread(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        std::os::windows::fs::FileExt::seek_read(&self.file, buf, offset)
    }
}

impl ByteStore for FileStore {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let want = buf.len().min(self.len.saturating_sub(offset) as usize);
        let mut done = 0;
        while done < want {
            match self.pread(offset + done as u64, &mut buf[done..want]) {
                Ok(0) => break,
                Ok(n) => done += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(done)
    }
}

/// An in-memory buffer (listings, command output, messages).
pub struct MemStore(Vec<u8>);

impl MemStore {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl ByteStore for MemStore {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let start = (offset as usize).min(self.0.len());
        let n = buf.len().min(self.0.len() - start);
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

/// Most recently used blocks of a store, so repainting and scrolling do not
/// hit the disk again.
pub struct BlockCache {
    store: Arc<dyn ByteStore>,
    blocks: Mutex<VecDeque<(u64, Arc<[u8]>)>>,
}

impl BlockCache {
    pub fn new(store: Arc<dyn ByteStore>) -> Self {
        Self {
            store,
            blocks: Mutex::new(VecDeque::with_capacity(CACHE_BLOCKS)),
        }
    }

    /// Block number `no`, read from the store on a miss.
    fn block(&self, no: u64) -> io::Result<Arc<[u8]>> {
        let mut blocks = self.blocks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = blocks.iter().position(|(n, _)| *n == no) {
            let hit = blocks.remove(pos).expect("position is in range");
            blocks.push_front(hit.clone());
            return Ok(hit.1);
        }
        let start = no * BLOCK_SIZE as u64;
        let data: Arc<[u8]> = self
            .store
            .read_range(start, start + BLOCK_SIZE as u64)?
            .into();
        blocks.push_front((no, data.clone()));
        blocks.truncate(CACHE_BLOCKS);
        Ok(data)
    }
}

impl ByteStore for BlockCache {
    fn len(&self) -> u64 {
        self.store.len()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let mut done = 0;
        while done < buf.len() {
            let pos = offset + done as u64;
            if pos >= self.len() {
                break;
            }
            let block = self.block(pos / BLOCK_SIZE as u64)?;
            let within = (pos % BLOCK_SIZE as u64) as usize;
            if within >= block.len() {
                break;
            }
            let n = (buf.len() - done).min(block.len() - within);
            buf[done..done + n].copy_from_slice(&block[within..within + n]);
            done += n;
        }
        Ok(done)
    }
}
