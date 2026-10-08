//! Tar, tar.gz, tar.bz2 and tar.xz formats: reading entries (Strategy for
//! [`ArchiveReader`]). The decoders are pure Rust.

use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use tar::{Archive, EntryType};

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit, Visitor, unix_time};

/// How the tar stream is compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TarCompression {
    None,
    Gzip,
    Bzip2,
    Xz,
}

/// A tar archive, compressed as `compression` says.
pub struct TarReader {
    pub compression: TarCompression,
}

impl TarReader {
    fn open(&self, archive: &Path) -> Result<Archive<Box<dyn Read>>> {
        let file = io::BufReader::new(fs::File::open(archive)?);
        let stream: Box<dyn Read> = match self.compression {
            TarCompression::None => Box::new(file),
            TarCompression::Gzip => Box::new(GzDecoder::new(file)),
            TarCompression::Bzip2 => Box::new(bzip2::bufread::MultiBzDecoder::new(file)),
            TarCompression::Xz => Box::new(lzma_rust2::XzReader::new(file, true)),
        };
        Ok(Archive::new(stream))
    }
}

fn meta<R: Read>(entry: &tar::Entry<'_, R>) -> Result<EntryMeta> {
    let header = entry.header();
    let kind = match header.entry_type() {
        EntryType::Directory => EntryKind::Dir,
        EntryType::Regular | EntryType::Continuous => EntryKind::File,
        EntryType::Symlink => match entry.link_name()? {
            Some(target) => EntryKind::Symlink(target.into_owned()),
            None => EntryKind::Other,
        },
        _ => EntryKind::Other,
    };
    Ok(EntryMeta {
        name: entry.path()?.to_string_lossy().into_owned(),
        kind,
        size: entry.size(),
        modified: header.mtime().ok().map(unix_time),
        mode: header.mode().ok(),
    })
}

impl ArchiveReader for TarReader {
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>> {
        let mut out = Vec::new();
        self.visit(archive, &mut |meta, _| {
            out.push(meta.clone());
            Ok(Visit::Continue)
        })?;
        Ok(out)
    }

    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()> {
        let mut tar = self.open(archive)?;
        for entry in tar.entries()? {
            let mut entry = entry?;
            if visit(&meta(&entry)?, &mut entry)? == Visit::Stop {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fs::archive::extract_archive;
    use crate::fs::archive::test_fixtures::write_tar_gz;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tokio::sync::mpsc;

    #[test]
    fn extract_does_not_overwrite_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        write_tar_gz(&archive, &[("keep.txt", b"new"), ("sub/new.txt", b"hello")]);
        let dest = dir.path().join("out");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep.txt"), b"original").unwrap();

        let (tx, _rx) = mpsc::channel(64);
        extract_archive(&archive, &dest, &tx, &AtomicBool::new(false)).unwrap();
        assert_eq!(fs::read(dest.join("keep.txt")).unwrap(), b"original");
        assert_eq!(
            fs::read(dest.join("sub").join("new.txt")).unwrap(),
            b"hello"
        );
    }
}
