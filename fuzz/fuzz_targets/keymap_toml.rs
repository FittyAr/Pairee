//! A keymap preset file (TOML): parsing, `extends` resolution and
//! validation of every binding.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    pairee::fuzz_api::keymap_toml(text);
});
