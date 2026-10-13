//! A tar archive, plain or gzip / bzip2 compressed (chosen by the first
//! byte): listing and streaming every entry.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pairee::fuzz_api::ArchiveKind;

fuzz_target!(|data: &[u8]| {
    let Some((&selector, archive)) = data.split_first() else {
        return;
    };
    let kind = match selector % 3 {
        0 => ArchiveKind::Tar,
        1 => ArchiveKind::TarGz,
        _ => ArchiveKind::TarBz2,
    };
    pairee::fuzz_api::archive(kind, archive);
});
