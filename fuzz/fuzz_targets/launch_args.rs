//! The command line of an interactive run (arguments separated by NUL).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let args: Vec<String> = text.split('\0').map(str::to_string).collect();
    pairee::fuzz_api::launch_args(&args);
});
