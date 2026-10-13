//! 7z rewriting for [`super::edit`]: compressed blocks whose entries are all
//! kept are copied raw (no recompression); a block that loses an entry is
//! decoded and its other entries are compressed again. Added files keep
//! their time and, on Unix, their permissions (p7zip's attribute
//! extension). Encrypted archives are not rewritten (no password is known).

use anyhow::Result;
use sevenz_rust2::{Archive, ArchiveEntry, ArchiveWriter, BlockDecoder, Password};
use std::fs;
use std::io::{self, Read, Seek};
use std::path::Path;

use super::edit::{ArchiveEdit, Dropper, EntrySource, OnAdded, entry_name, replace_archive};

/// Windows attribute of a folder.
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
/// Windows attribute of a read-only entry.
const FILE_ATTRIBUTE_READONLY: u32 = 0x01;
/// p7zip: the high 16 bits of the attributes hold the Unix mode.
pub(super) const UNIX_EXTENSION: u32 = 0x8000;

/// Applies `edit` to the 7z file `archive`.
pub fn rewrite_7z(archive: &Path, edit: &ArchiveEdit, on_added: &mut OnAdded) -> Result<()> {
    replace_archive(archive, |tmp| {
        let mut source = fs::File::open(archive)?;
        let old = Archive::read(&mut source, &Password::empty())?;
        let mut out = ArchiveWriter::new(tmp)?;
        copy_kept(&old, &mut source, &mut out, &edit.dropper())?;
        for (rel, entry) in &edit.add {
            add_entry(&mut out, rel, entry)?;
            on_added(rel)?;
        }
        out.finish()?;
        Ok(())
    })
}

/// Copies the entries of `old` the edit keeps.
fn copy_kept<R: Read + Seek, W: io::Write + Seek>(
    old: &Archive,
    source: &mut R,
    out: &mut ArchiveWriter<W>,
    dropper: &Dropper,
) -> Result<()> {
    let block_of = |file: usize| old.stream_map.file_block_index.get(file).copied().flatten();
    // Folders and empty files belong to no block.
    for (index, entry) in old.files.iter().enumerate() {
        if block_of(index).is_none() && !dropper.drops(&entry.name) {
            out.push_archive_entry(entry.clone(), None::<&[u8]>)?;
        }
    }
    for block in 0..old.blocks.len() {
        let all_kept = (0..old.files.len())
            .filter(|&f| block_of(f) == Some(block))
            .all(|f| !dropper.drops(&old.files[f].name));
        if all_kept {
            out.push_raw_block(old, block, source, |entry| entry.name.clone())?;
            continue;
        }
        let password = Password::empty();
        let decoder = BlockDecoder::new(1, block, old, &password, source);
        decoder.for_each_entries(&mut |entry, data| {
            if entry.has_stream && !dropper.drops(&entry.name) {
                out.push_archive_entry(entry.clone(), Some(data))?;
            } else {
                // The block is one stream: skip the data to reach the next.
                io::copy(data, &mut io::sink())?;
            }
            Ok(true)
        })?;
    }
    Ok(())
}

/// The attributes stored for a local entry: folder and read-only flags,
/// and the Unix mode where there is one.
fn attributes(meta: &fs::Metadata) -> u32 {
    let mut attributes = if meta.is_dir() {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        0
    };
    if meta.permissions().readonly() {
        attributes |= FILE_ATTRIBUTE_READONLY;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        attributes |= UNIX_EXTENSION | ((meta.permissions().mode() & 0xFFFF) << 16);
    }
    attributes
}

fn add_entry<W: io::Write + Seek>(
    out: &mut ArchiveWriter<W>,
    rel: &Path,
    source: &EntrySource,
) -> Result<()> {
    let name = entry_name(rel, false);
    match source {
        EntrySource::Dir => {
            out.push_archive_entry(ArchiveEntry::new_directory(&name), None::<&[u8]>)?;
        }
        EntrySource::Bytes(bytes) => {
            out.push_archive_entry(ArchiveEntry::new_file(&name), Some(bytes.as_slice()))?;
        }
        EntrySource::File(path) => {
            let meta = fs::metadata(path)?;
            let mut entry = ArchiveEntry::from_path(path, name);
            entry.has_windows_attributes = true;
            entry.windows_attributes = attributes(&meta);
            let data = (!meta.is_dir()).then(|| fs::File::open(path)).transpose()?;
            out.push_archive_entry(entry, data)?;
        }
    }
    Ok(())
}
