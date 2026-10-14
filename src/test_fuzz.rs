//! Bounded junk-input tests: parsers must return `Err`/`false`/`None`, never panic.
//!
//! This is the in-tree stand-in for `cargo fuzz` (stable MSRV, runs in CI); it
//! also drives the `crate::fuzz_api` entry points of the `fuzz/` targets with
//! junk and damaged inputs.

use crate::app::state::glob_matches;
use crate::config::settings::Settings;
use crate::fs::descriptions::parse_description_line;
use crate::fuzz_api::{self, ArchiveKind, RenameInput};
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
            let _ = crate::fs::glob::glob_matches_case(pat, name, true);
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

// ─── The `cargo fuzz` entry points (crate::fuzz_api), on stable ──────────────

/// `seed` with deterministic damage: flipped bytes, then truncations.
fn mutations(seed: &[u8]) -> Vec<Vec<u8>> {
    let mut out = vec![seed.to_vec(), Vec::new()];
    let mut s: u64 = 0x5EED;
    for _ in 0..24 {
        let mut damaged = seed.to_vec();
        for _ in 0..3 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if !damaged.is_empty() {
                let at = (s >> 16) as usize % damaged.len();
                damaged[at] ^= (s >> 8) as u8 | 1;
            }
        }
        out.push(damaged);
    }
    for cut in [1, 2, 3, 5, 8] {
        out.push(seed[..seed.len() * cut / 9].to_vec());
    }
    out
}

#[test]
fn damaged_archives_do_not_panic() {
    use crate::fs::archive::test_fixtures as fx;
    type Writer = fn(&std::path::Path, &[(&str, &[u8])]);
    let cases: [(ArchiveKind, &str, Writer); 6] = [
        (ArchiveKind::Zip, "a.zip", fx::write_zip),
        (ArchiveKind::Tar, "a.tar", fx::write_tar),
        (ArchiveKind::TarGz, "a.tar.gz", fx::write_tar_gz),
        (ArchiveKind::TarBz2, "a.tar.bz2", fx::write_tar_bz2),
        (ArchiveKind::TarXz, "a.tar.xz", fx::write_tar_xz),
        (ArchiveKind::SevenZ, "a.7z", fx::write_7z),
    ];
    let dir = tempfile::tempdir().unwrap();
    for (kind, name, write) in cases {
        let path = dir.path().join(name);
        write(
            &path,
            &[("a.txt", b"alpha"), ("sub/b.bin", &[0, 1, 2, 255])],
        );
        let seed = std::fs::read(&path).unwrap();
        for data in mutations(&seed) {
            fuzz_api::archive(kind, &data);
        }
        for junk in junk_strings(0xA2C, 10) {
            fuzz_api::archive(kind, junk.as_bytes());
        }
    }
}

#[test]
fn keymap_toml_junk_does_not_panic() {
    let shipped = crate::keybindings::embedded::preset_toml("norton").unwrap_or_default();
    for data in mutations(shipped.as_bytes()).into_iter().take(8) {
        fuzz_api::keymap_toml(&String::from_utf8_lossy(&data));
    }
    for s in junk_strings(0x4E7, 20) {
        fuzz_api::keymap_toml(&s);
        fuzz_api::keymap_toml(&format!("extends = \"fuzz\"\n[panels]\n{s} = \"quit\""));
    }
}

#[test]
fn launch_args_junk_does_not_panic() {
    let junk = junk_strings(0x1A0, 40);
    for window in junk.windows(3) {
        fuzz_api::launch_args(window);
        let flags: Vec<String> = window.iter().map(|s| format!("--{s}")).collect();
        fuzz_api::launch_args(&flags);
    }
    fuzz_api::launch_args(&["--cwd-file".into()]);
    fuzz_api::launch_args(&["--cwd-file=".into(), "a".into(), "b".into(), "c".into()]);
}

#[test]
fn ansi_junk_does_not_panic() {
    for s in junk_strings(0xA251, 60) {
        fuzz_api::ansi(&s);
        fuzz_api::ansi(&format!("\x1b[{s}m{s}\x1b[0m"));
        fuzz_api::ansi(&format!("\x1b]{s}\x07\x1b[38;5;{s}"));
    }
}

#[test]
fn rename_pattern_junk_does_not_panic() {
    let junk = junk_strings(0x2E7A, 40);
    for (i, s) in junk.iter().enumerate() {
        let other = &junk[(i + 1) % junk.len()];
        for regex in [false, true] {
            fuzz_api::rename_pattern(&RenameInput {
                name_mask: s,
                ext_mask: other,
                search: s,
                replace: other,
                regex,
                file_name: "photo.2024.jpg",
            });
        }
        fuzz_api::rename_pattern(&RenameInput {
            name_mask: &format!("[N{s}][C][E1-{other}]"),
            ext_mask: "[E]",
            search: "",
            replace: "",
            regex: false,
            file_name: s,
        });
    }
}

#[test]
fn sha256_file_junk_does_not_panic() {
    for s in junk_strings(0x5A, 60) {
        fuzz_api::sha256_file(&s);
        fuzz_api::sha256_file(&format!("{}  {s}", "a".repeat(64)));
    }
}

#[test]
fn syntax_definition_junk_does_not_panic() {
    const SEED: &str = r##"
name = "Fuzz"
extensions = ["fz"]
[[rule]]
kind = "keyword"
words = ["if", "#x"]
[[region]]
kind = "string"
start = '<<-?(\w*)'
end = '^\s*${1}$'
skip = '\\.'
after = '(?:^|[=(])\s*$'
[[region]]
kind = "tag"
start = '<(\w*)>'
end = '</>'
include = '${1}'
[[region.rule]]
kind = "constant"
pattern = '(a*)*b'
group = 1
"##;
    let text = "x = <<A\n\\A\nA\n<fz><fz><js>let r = /a/;</></> <html><script>\"\n";
    let seeded = crate::syntax::highlight_with_definition(SEED, text);
    assert!(seeded.is_ok_and(|lines| lines.len() == 4), "the seed works");
    for data in mutations(SEED.as_bytes()) {
        fuzz_api::syntax_definition(&String::from_utf8_lossy(&data), text);
    }
    for s in junk_strings(0x5A7, 40) {
        fuzz_api::syntax_definition(SEED, &s);
        let region = format!("name = \"J\"\n[[region]]\nstart = '{s}'\nend = '${{1}}{s}'");
        fuzz_api::syntax_definition(&region, &s.repeat(20));
        let include =
            format!("{SEED}\n[[region]]\nstart = '(.)'\nend = 'x'\ninclude = '{s}${{1}}'");
        fuzz_api::syntax_definition(&include, text);
    }
}
