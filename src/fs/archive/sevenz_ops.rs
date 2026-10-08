//! Native 7z format using `sevenz-rust2` (Strategy for [`ArchiveReader`]).

use anyhow::{Result, anyhow};
use std::path::Path;

use super::format::{ArchiveReader, EntryKind, EntryMeta, Visit, Visitor};

pub struct SevenZReader;

fn meta(entry: &sevenz_rust2::ArchiveEntry) -> EntryMeta {
    EntryMeta {
        name: entry.name().to_string(),
        kind: if entry.is_directory() {
            EntryKind::Dir
        } else {
            EntryKind::File
        },
        size: entry.size(),
        modified: entry
            .has_last_modified_date
            .then(|| entry.last_modified_date.into()),
        mode: None,
    }
}

impl ArchiveReader for SevenZReader {
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>> {
        let archive = sevenz_rust2::Archive::open(archive)
            .map_err(|e| anyhow!("Failed to open 7z: {:?}", e))?;
        Ok(archive.files.iter().map(meta).collect())
    }

    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()> {
        let mut reader =
            sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
                .map_err(|e| anyhow!("Failed to open 7z: {:?}", e))?;
        // The callback must return a `sevenz_rust2::Error`; keep the real
        // cause here so the user sees why reading stopped.
        let mut failure: Option<anyhow::Error> = None;
        let mut stopped = false;
        let result = reader.for_each_entries(|entry, data| {
            if stopped {
                return Ok(false);
            }
            match visit(&meta(entry), data) {
                Ok(Visit::Continue) => Ok(true),
                Ok(Visit::Stop) => {
                    stopped = true;
                    Ok(false)
                }
                Err(e) => {
                    let msg = e.to_string();
                    failure = Some(e);
                    Err(sevenz_rust2::Error::Other(msg.into()))
                }
            }
        });
        if let Some(e) = failure {
            return Err(e);
        }
        result.map_err(|e| anyhow!("7z extraction failed: {:?}", e))
    }

    fn count_hint(&self, archive: &Path) -> usize {
        sevenz_rust2::Archive::open(archive).map_or(0, |a| a.files.len())
    }
}
