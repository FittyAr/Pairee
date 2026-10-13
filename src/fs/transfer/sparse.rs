//! "Produce sparse files": runs of zero bytes are not written but skipped
//! with a seek, leaving holes the filesystem does not allocate. Windows only
//! makes holes in files flagged sparse, so the flag is set first.

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};

/// Granularity of hole detection (one filesystem block).
pub const HOLE_BLOCK: usize = 4096;

/// Flags `file` as sparse where the platform needs it (Windows / NTFS).
pub fn mark_sparse(file: &File) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::IO::DeviceIoControl;
        use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;
        let mut returned = 0u32;
        // SAFETY: no input or output buffers; the handle is open.
        let ok = unsafe {
            DeviceIoControl(
                file.as_raw_handle() as _,
                FSCTL_SET_SPARSE,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(not(windows))]
    let _ = file;
    Ok(())
}

/// Writes `chunk` at the current position of `file`, seeking over every
/// all-zero [`HOLE_BLOCK`] instead of writing it. The caller sets the final
/// length (a trailing hole is not materialised by a seek).
pub fn write_sparse(file: &mut File, chunk: &[u8]) -> io::Result<()> {
    let mut start = 0;
    while start < chunk.len() {
        let zero = is_zero(block_at(chunk, start));
        let mut end = start;
        while end < chunk.len() && is_zero(block_at(chunk, end)) == zero {
            end += block_at(chunk, end).len();
        }
        if zero {
            file.seek(SeekFrom::Current((end - start) as i64))?;
        } else {
            file.write_all(&chunk[start..end])?;
        }
        start = end;
    }
    Ok(())
}

fn block_at(chunk: &[u8], start: usize) -> &[u8] {
    &chunk[start..(start + HOLE_BLOCK).min(chunk.len())]
}

fn is_zero(block: &[u8]) -> bool {
    block.iter().all(|&b| b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holes_read_back_as_zeros() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sparse.bin");
        let mut data = vec![0u8; HOLE_BLOCK * 4 + 10];
        data[HOLE_BLOCK + 5] = 7;
        data[HOLE_BLOCK * 4 + 3] = 9;
        let mut file = File::create(&path).unwrap();
        mark_sparse(&file).unwrap();
        write_sparse(&mut file, &data[..HOLE_BLOCK * 3]).unwrap();
        write_sparse(&mut file, &data[HOLE_BLOCK * 3..]).unwrap();
        file.set_len(data.len() as u64).unwrap();
        drop(file);
        assert_eq!(std::fs::read(&path).unwrap(), data);
    }
}
