//! Bounded junk-input tests: parsers must return `Err`/`false`/`None`, never panic.
//!
//! This is the in-tree stand-in for `cargo fuzz` (stable MSRV, runs in CI).

use crate::app::state::glob_matches;
use crate::config::settings::Settings;
use crate::fs::descriptions::parse_description_line;
use crate::plugin::loader::PluginManifest;

fn junk_strings(seed: u64, count: usize) -> Vec<String> {
    let mut s = seed;
    let mut out = Vec::with_capacity(count);
    const SPECIAL: &[u8] = b"{}*?\"\\\n\r\t []=#";
    for i in 0..count {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let len = ((s >> 8) % 80) as usize;
        let mut buf = Vec::with_capacity(len);
        let mut t = s ^ (i as u64).wrapping_mul(0x9E37_79B9);
        for _ in 0..len {
            t = t.wrapping_mul(16777619).wrapping_add(2166136261);
            if t.is_multiple_of(7) {
                buf.push(SPECIAL[(t as usize) % SPECIAL.len()]);
            } else {
                buf.push(32 + (t % 95) as u8);
            }
        }
        out.push(String::from_utf8_lossy(&buf).into_owned());
    }
    out.push(String::new());
    out.push("{".repeat(200));
    out.push("*".repeat(200));
    out.push("\0\0\0".into());
    out.push("{a,b}{c,d}{e,f}{g,h}{i,j}".into());
    out
}

#[test]
fn glob_junk_does_not_panic() {
    let inputs = junk_strings(0xC0FFEE, 80);
    let long = "a".repeat(64);
    let names = ["", "a", "foo.rs", long.as_str()];
    for pat in &inputs {
        for name in &names {
            let _ = glob_matches(pat, name);
            let _ = crate::app::state::glob_matches_case(pat, name, true);
        }
    }
}

#[test]
fn descript_ion_junk_does_not_panic() {
    for line in junk_strings(0xD1, 120) {
        let _ = parse_description_line(&line);
        let quoted = format!("\"{}\" desc", line.replace('"', "\"\""));
        let _ = parse_description_line(&quoted);
    }
}

#[test]
fn plugin_manifest_junk_does_not_panic() {
    for s in junk_strings(0x51, 80) {
        let _ = PluginManifest::parse(&s);
    }
    let _ = PluginManifest::parse("");
    let _ = PluginManifest::parse("[plugin]\nname = \"x\"");
    let _ = PluginManifest::parse("name = \"ok\"\nversion = \"1\"");
}

#[test]
fn settings_toml_junk_does_not_panic() {
    for s in junk_strings(0x5E77, 80) {
        let _: Result<Settings, _> = toml::from_str(&s);
    }
    let _: Result<Settings, _> = toml::from_str("");
    let _: Result<Settings, _> = toml::from_str("show_hidden = true");
}
