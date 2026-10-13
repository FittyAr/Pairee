//! The contents of a release `.sha256` file.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    pairee::fuzz_api::sha256_file(text);
});
