//! Terminal output with escape sequences, rendered as styled text.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    pairee::fuzz_api::ansi(text);
});
