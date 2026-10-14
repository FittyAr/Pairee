use std::fs::File;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Direct I/O alignment (offsets, lengths and buffers).
pub const ALIGN: usize = 4096;

#[cfg(target_os = "windows")]
const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
#[cfg(target_os = "windows")]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;

#[cfg(target_os = "linux")]
const O_DIRECT: i32 = 0o40000;

/// The Windows long-path (`\\?\`) form of an absolute `path`, so paths
/// past `MAX_PATH` work. Verbatim paths skip all normalization, so the path
/// is made canonical first (`/` turned into `\`, `.` and `..` resolved):
/// otherwise a typed `C:/backup` or `C:\a\..\b` would name a different,
/// invalid file. Device (`\\.\`) and verbatim paths are left alone, as is
/// everything on other platforms.
pub fn to_long_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        let starts = |prefix: &str| {
            let prefix: Vec<u16> = prefix.encode_utf16().collect();
            wide.starts_with(&prefix)
        };
        if !path.is_absolute() || starts(r"\\?\") || starts(r"\\.\") {
            return path.to_path_buf();
        }
        let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let full: Vec<u16> = full.as_os_str().encode_wide().collect();
        let unc: Vec<u16> = r"\\".encode_utf16().collect();
        let long: Vec<u16> = match full.strip_prefix(unc.as_slice()) {
            Some(share) => r"\\?\UNC\"
                .encode_utf16()
                .chain(share.iter().copied())
                .collect(),
            None => r"\\?\".encode_utf16().chain(full.iter().copied()).collect(),
        };
        PathBuf::from(OsString::from_wide(&long))
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_path_buf()
    }
}

/// Abre un archivo para lectura, intentando usar Direct I/O (bypass de cache) si se solicita.
/// Si Direct I/O falla o no está soportado, realiza un fallback transparente a I/O estándar.
pub fn open_reader_direct(path: &Path, use_direct: bool) -> std::io::Result<File> {
    let normalized = to_long_path(path);
    if !use_direct {
        return File::open(&normalized);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(FILE_FLAG_NO_BUFFERING);

        match options.open(&normalized) {
            Ok(file) => Ok(file),
            Err(_) => File::open(&normalized), // Fallback
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(O_DIRECT);

        match options.open(path) {
            Ok(file) => Ok(file),
            Err(_) => File::open(path), // Fallback
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        File::open(path)
    }
}

/// Abre un archivo para escritura, intentando usar Direct I/O (bypass de cache) si se solicita.
/// Si Direct I/O falla o no está soportado, realiza un fallback transparente a I/O estándar.
pub fn open_writer_direct(path: &Path, use_direct: bool) -> std::io::Result<File> {
    let normalized = to_long_path(path);
    if !use_direct {
        return File::create(&normalized);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        options.custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH);

        match options.open(&normalized) {
            Ok(file) => Ok(file),
            Err(_) => File::create(&normalized), // Fallback
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        options.custom_flags(O_DIRECT);

        match options.open(&normalized) {
            Ok(file) => Ok(file),
            Err(_) => File::create(&normalized), // Fallback
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        File::create(&normalized)
    }
}

/// Un buffer de bytes alineado en memoria para optimizar operaciones Direct I/O.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    size: usize,
}

impl AlignedBuffer {
    /// Crea una nueva instancia de AlignedBuffer alineado a `align` bytes.
    pub fn new(size: usize, align: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(size, align)
            .unwrap_or_else(|_| std::alloc::Layout::from_size_align(size, 4096).unwrap());
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Self { ptr, layout, size }
    }

    /// Retorna una referencia de lectura al buffer completo.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    /// Retorna una referencia mutable al buffer completo.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr, self.layout);
        }
    }
}
unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

/// Sequential reader of a file that bypasses the cache with Direct I/O for
/// every whole aligned block (when asked and supported) and reads the rest
/// — the unaligned tail, or everything after Direct I/O is refused —
/// through the page cache.
pub struct DiskReader {
    buffered: File,
    direct: Option<File>,
    buf: AlignedBuffer,
    /// File offset of the next byte read from the disk.
    pos: u64,
    /// Size of the file when opened (Direct I/O never reads past it).
    size: u64,
    filled: usize,
    consumed: usize,
}

impl DiskReader {
    /// Reader of `path` from `offset`, in blocks of `block` bytes (a
    /// multiple of [`ALIGN`]).
    pub fn open(path: &Path, direct: bool, offset: u64, block: usize) -> io::Result<Self> {
        let mut buffered = File::open(to_long_path(path))?;
        let size = buffered.metadata()?.len();
        buffered.seek(SeekFrom::Start(offset))?;
        let direct = (direct && offset.is_multiple_of(ALIGN as u64))
            .then(|| open_reader_direct(path, true).ok())
            .flatten()
            .and_then(|mut f| f.seek(SeekFrom::Start(offset)).ok().map(|_| f));
        Ok(Self {
            buffered,
            direct,
            buf: AlignedBuffer::new(block.max(ALIGN), ALIGN),
            pos: offset,
            size,
            filled: 0,
            consumed: 0,
        })
    }

    /// Reads the next block into the internal buffer.
    fn fill(&mut self) -> io::Result<()> {
        self.consumed = 0;
        self.filled = 0;
        if let Some(file) = self.direct.as_mut() {
            let remaining = self.size.saturating_sub(self.pos);
            let whole = remaining - remaining % ALIGN as u64;
            let want = whole.min(self.buf.as_slice().len() as u64) as usize;
            if want >= ALIGN
                && self.pos.is_multiple_of(ALIGN as u64)
                && let Ok(n) = file.read(&mut self.buf.as_mut_slice()[..want])
                && n > 0
            {
                self.filled = n;
                self.pos += n as u64;
                return Ok(());
            }
            // Unaligned from here on (or refused): continue buffered.
            self.direct = None;
            self.buffered.seek(SeekFrom::Start(self.pos))?;
        }
        self.filled = self.buffered.read(self.buf.as_mut_slice())?;
        self.pos += self.filled as u64;
        Ok(())
    }
}

impl Read for DiskReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.consumed == self.filled {
            self.fill()?;
        }
        let n = out.len().min(self.filled - self.consumed);
        out[..n].copy_from_slice(&self.buf.as_slice()[self.consumed..self.consumed + n]);
        self.consumed += n;
        Ok(n)
    }
}

#[cfg(all(test, target_os = "windows"))]
mod long_path_tests {
    use super::to_long_path;
    use std::path::Path;

    #[test]
    fn long_paths_are_canonical() {
        assert_eq!(
            to_long_path(Path::new("C:/data/a/../b/./c.txt")),
            Path::new(r"\\?\C:\data\b\c.txt")
        );
        assert_eq!(
            to_long_path(Path::new(r"\\server\share\dir\f.txt")),
            Path::new(r"\\?\UNC\server\share\dir\f.txt")
        );
        for kept in [r"\\?\C:\x", r"\\.\pipe\p", "relative\\f"] {
            assert_eq!(to_long_path(Path::new(kept)), Path::new(kept));
        }
    }

    /// A destination typed with `/` still works (verbatim paths take `/`
    /// as part of a name).
    #[test]
    fn forward_slash_paths_open() {
        let dir = tempfile::tempdir().unwrap();
        let typed = dir.path().to_string_lossy().replace('\\', "/") + "/sub/../f.txt";
        std::fs::write(to_long_path(Path::new(&typed)), b"x").unwrap();
        assert_eq!(std::fs::read(dir.path().join("f.txt")).unwrap(), b"x");
    }
}
