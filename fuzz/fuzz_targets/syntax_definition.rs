//! A user syntax highlighting definition (TOML), compiled and used to
//! highlight some text: the definition and the text are separated by the
//! first NUL.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let (definition, sample) = text.split_once('\0').unwrap_or((text, "if x\n\"a\"\n"));
    pairee::fuzz_api::syntax_definition(definition, sample);
});
