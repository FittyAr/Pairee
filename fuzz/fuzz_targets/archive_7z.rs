//! A 7z archive: listing and streaming every entry.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pairee::fuzz_api::ArchiveKind;

fuzz_target!(|data: &[u8]| {
    pairee::fuzz_api::archive(ArchiveKind::SevenZ, data);
});
