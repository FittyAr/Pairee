use std::fs::{File, OpenOptions};
use std::path::Path;

#[cfg(target_os = "windows")]
const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
#[cfg(target_os = "windows")]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;

#[cfg(target_os = "linux")]
const O_DIRECT: i32 = 0o40000;

/// Abre un archivo para lectura, intentando usar Direct I/O (bypass de cache) si se solicita.
/// Si Direct I/O falla o no está soportado, realiza un fallback transparente a I/O estándar.
pub fn open_reader_direct(path: &Path, use_direct: bool) -> std::io::Result<File> {
    if !use_direct {
        return File::open(path);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(FILE_FLAG_NO_BUFFERING);
        
        match options.open(path) {
            Ok(file) => Ok(file),
            Err(_) => File::open(path), // Fallback
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
    if !use_direct {
        return File::create(path);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        options.custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH);
        
        match options.open(path) {
            Ok(file) => Ok(file),
            Err(_) => File::create(path), // Fallback
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        options.custom_flags(O_DIRECT);
        
        match options.open(path) {
            Ok(file) => Ok(file),
            Err(_) => File::create(path), // Fallback
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        File::create(path)
    }
}
