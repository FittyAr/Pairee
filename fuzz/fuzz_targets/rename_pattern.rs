//! Multi-rename rules: name and extension masks, search and replace (plain
//! or regex, chosen by the first byte) applied to a file name. The fields
//! are separated by NUL.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pairee::fuzz_api::RenameInput;

fuzz_target!(|data: &[u8]| {
    let Some((&flags, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let mut fields = text.split('\0');
    let mut next = || fields.next().unwrap_or("");
    pairee::fuzz_api::rename_pattern(&RenameInput {
        name_mask: next(),
        ext_mask: next(),
        search: next(),
        replace: next(),
        file_name: next(),
        regex: flags & 1 == 1,
    });
});
