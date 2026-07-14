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
        let ptr = unsafe { std::alloc::alloc(layout) };
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
