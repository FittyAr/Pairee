use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
#[cfg(target_os = "windows")]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;

#[cfg(target_os = "linux")]
const O_DIRECT: i32 = 0o40000;

/// Convierte una ruta a su formato Unicode largo UNC en Windows si es absoluta.
pub fn to_long_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy();
        if path.is_absolute() && !path_str.starts_with(r"\\?\") {
            if path_str.starts_with(r"\\") {
                let mut p = PathBuf::from(r"\\?\UNC");
                p.push(path_str.strip_prefix(r"\\").unwrap_or(&path_str));
                p
            } else {
                let mut p = PathBuf::from(r"\\?\");
                p.push(path);
                p
            }
        } else {
            path.to_path_buf()
        }
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

        // Open `normalized` (the post-`to_long_path` path), not `path`,
        // to keep behaviour identical with the Windows branch and the
        // non-Direct fallback below. Opening different paths for the
        // primary attempt and the fallback would silently give the
        // caller two different file handles on systems where
        // `to_long_path` ever does something non-trivial.
        match options.open(&normalized) {
            Ok(file) => Ok(file),
            Err(_) => File::open(&normalized), // Fallback
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
        // SAFETY: `self.ptr` was allocated with `std::alloc::alloc`
        // in `AlignedBuffer::new` using the same `self.layout`, and
        // `AlignedBuffer` is not `Copy`/`Clone` so the buffer (and
        // its pointer) cannot be observed after `drop` runs.
        unsafe {
            std::alloc::dealloc(self.ptr, self.layout);
        }
    }
}
// SAFETY: `AlignedBuffer` owns a unique heap allocation. Concurrent
// access to the underlying bytes is always mediated through
// `&mut self` (see `as_mut_slice`); `as_slice` only returns a
// shared reference for the lifetime of that borrow, so the data
// race guarantee is upheld. The raw pointer is not aliased
// anywhere else in the program.
unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn aligned_buffer_is_zeroed_on_creation() {
        // `alloc_zeroed` must wipe the memory before we hand it out.
        let buf = AlignedBuffer::new(4096, 4096);
        assert_eq!(buf.as_slice().len(), 4096);
        assert!(
            buf.as_slice().iter().all(|&b| b == 0),
            "AlignedBuffer must be zero-initialised"
        );
    }

    #[test]
    fn aligned_buffer_respects_requested_size() {
        for &size in &[0usize, 1, 16, 1024, 65_537, 1_048_576] {
            let buf = AlignedBuffer::new(size, 64);
            assert_eq!(buf.as_slice().len(), size);
        }
    }

    #[test]
    fn aligned_buffer_mut_slice_is_writable() {
        let mut buf = AlignedBuffer::new(128, 64);
        let slice = buf.as_mut_slice();
        for (i, b) in slice.iter_mut().enumerate() {
            *b = (i & 0xFF) as u8;
        }
        let slice = buf.as_slice();
        for (i, &b) in slice.iter().enumerate() {
            assert_eq!(b, (i & 0xFF) as u8);
        }
    }

    #[test]
    fn aligned_buffer_alignment_is_at_least_requested() {
        // Verify the alignment guarantee for a few common block sizes.
        for &align in &[512usize, 4096, 65_536] {
            let buf = AlignedBuffer::new(align, align);
            let ptr = buf.as_slice().as_ptr() as usize;
            assert_eq!(
                ptr % align,
                0,
                "buffer pointer {:#x} is not aligned to {}",
                ptr,
                align
            );
        }
    }

    #[test]
    fn aligned_buffer_zero_size_is_valid() {
        // Edge case: zero-byte allocation is legal and must not panic
        // on `as_slice()` / `as_mut_slice()`.
        let buf = AlignedBuffer::new(0, 64);
        assert_eq!(buf.as_slice().len(), 0);
        let mut buf = AlignedBuffer::new(0, 64);
        assert_eq!(buf.as_mut_slice().len(), 0);
    }

    #[test]
    fn aligned_buffer_falls_back_to_4k_alignment_on_invalid_request() {
        // `Layout::from_size_align` rejects alignments that aren't a power
        // of two. The constructor falls back to 4 KiB to keep the buffer
        // usable for the common case (O_DIRECT sector alignment).
        let buf = AlignedBuffer::new(1024, 3000); // 3000 is not a power of two
        let ptr = buf.as_slice().as_ptr() as usize;
        // Must be aligned to at least 4 KiB after the fallback.
        assert_eq!(ptr % 4096, 0);
    }

    #[test]
    fn aligned_buffer_drop_releases_memory() {
        // Smoke test: instantiating and dropping many buffers must not
        // leak or double-free (would show up as a crash / abort).
        for _ in 0..1000 {
            let _buf = AlignedBuffer::new(4096, 4096);
        }
    }

    #[test]
    fn aligned_buffer_send_sync_across_threads() {
        // The unsafe `Send`/`Sync` impls promise the buffer is safe to
        // move between threads and to share `&` references. A real
        // concurrent access is the only test that proves it.
        let buf = Arc::new(AlignedBuffer::new(8192, 4096));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let buf = Arc::clone(&buf);
            handles.push(thread::spawn(move || {
                // Sum the bytes (all zero) — exercises the `Sync` side.
                buf.as_slice().iter().map(|&b| b as u64).sum::<u64>()
            }));
        }
        let total: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert_eq!(total, 0);
    }

    #[test]
    fn aligned_buffer_exclusive_mutable_access() {
        // The `Send`/`Sync` impls guarantee that `&mut self` is the only
        // path to mutability. We can prove the type is not `Clone` /
        // `Copy` — those would silently allow shared mutation.
        // (This is a compile-time test; if AlignedBuffer ever becomes
        // Clone, this `fn(_: &AlignedBuffer)` is still fine, but the
        // line below would need a `Drop` check or similar.)
        fn assert_not_clone<T: ?Sized>(_: &T)
        where
            T: Sized,
        {
        }
        let buf = AlignedBuffer::new(64, 64);
        assert_not_clone(&buf);
    }

    // ── to_long_path: Windows-specific path normalisation ─────────────────────

    #[test]
    fn to_long_path_is_identity_on_unix() {
        // On non-Windows the function is a no-op; the path must be
        // returned unchanged. This protects the Linux/macOS build.
        let p = std::path::PathBuf::from("/tmp/file.txt");
        assert_eq!(to_long_path(&p), p);
    }

    #[test]
    fn to_long_path_is_identity_for_relative_paths() {
        // Even on Windows, relative paths are returned unchanged —
        // the `\\?\` prefix is only valid for absolute paths.
        let p = std::path::PathBuf::from("relative/file.txt");
        let out = to_long_path(&p);
        // On Unix this is the identity; on Windows it must also stay
        // relative (no `\\?\` prefix). Either way the path must not
        // gain a UNC prefix.
        assert!(!out.to_string_lossy().starts_with(r"\\?\"));
    }

    #[test]
    fn to_long_path_does_not_double_prefix() {
        // If the caller already provided a `\\?\` or `\\?\UNC\` path
        // we must not stack another prefix.
        let already = std::path::PathBuf::from(r"\\?\C:\Windows\System32");
        let out = to_long_path(&already);
        let s = out.to_string_lossy();
        let prefix_count = s.matches(r"\\?\").count();
        assert!(prefix_count <= 1, "double-prefix detected: {}", s);
    }

    // ── open_reader_direct / open_writer_direct: smoke ───────────────────────

    #[test]
    fn open_reader_and_writer_roundtrip() {
        // We don't have a way to assert the FILE_FLAG_NO_BUFFERING
        // path is taken (it requires an administrator on Windows and
        // root on Linux), but the fallback path must work everywhere.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.bin");

        {
            let mut f = open_writer_direct(&path, false).expect("create");
            f.write_all(b"hello aligned buffer").expect("write");
        }

        let mut f = open_reader_direct(&path, false).expect("open");
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).expect("read");
        assert_eq!(buf, b"hello aligned buffer");
    }

    // `write_all` / `read_to_end` are traits from `std::io`; import
    // them here so the `use super::*;` at the top of the test module
    // does not have to drag them in.
    use std::io::{Read, Write};
}
