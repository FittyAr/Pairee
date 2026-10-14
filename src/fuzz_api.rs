//! Entry points for the `cargo fuzz` targets in `fuzz/` (feature `fuzzing`)
//! and for the bounded junk-input tests in `test_fuzz.rs`. Each one feeds
//! untrusted input to a parser; the only requirement is that it returns
//! instead of panicking, hanging or exhausting memory.
//!
//! Not a public API: it exists so the fuzz crate can reach crate-private
//! parsers through one thin, stable surface.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::fs::archive::format::{ArchiveReader, EntryMeta, Visit};
use crate::fs::multi_rename::{RenameRules, RenameSource};

/// Upper bound on the bytes read from one archive entry.
const MAX_ENTRY_BYTES: u64 = 1 << 20;
/// Upper bound on the entries visited in one archive.
const MAX_ENTRIES: usize = 4096;

/// Archive formats with a native reader.
#[derive(Debug, Clone, Copy)]
pub enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    SevenZ,
}

impl ArchiveKind {
    /// File name the reader is chosen by (readers take paths).
    fn file_name(self) -> &'static str {
        match self {
            Self::Zip => "fuzz.zip",
            Self::Tar => "fuzz.tar",
            Self::TarGz => "fuzz.tar.gz",
            Self::TarBz2 => "fuzz.tar.bz2",
            Self::TarXz => "fuzz.tar.xz",
            Self::SevenZ => "fuzz.7z",
        }
    }
}

/// Lists `data` as an archive of `kind`, then streams its entries.
pub fn archive(kind: ArchiveKind, data: &[u8]) {
    let path = scratch_file(kind.file_name());
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let Some(reader) = crate::fs::archive::browsable_reader(&path) else {
        return;
    };
    let _ = reader.entries(&path);
    visit_bounded(reader, &path);
}

fn visit_bounded(reader: &dyn ArchiveReader, path: &Path) {
    let mut seen = 0;
    let _ = reader.visit(path, &mut |_: &EntryMeta, data: &mut dyn Read| {
        std::io::copy(&mut data.take(MAX_ENTRY_BYTES), &mut std::io::sink())?;
        seen += 1;
        Ok(if seen < MAX_ENTRIES {
            Visit::Continue
        } else {
            Visit::Stop
        })
    });
}

/// A file in this process's private scratch folder (one per thread, so
/// parallel tests do not share it).
fn scratch_file(name: &str) -> PathBuf {
    static DIR: OnceLock<Option<tempfile::TempDir>> = OnceLock::new();
    let base = DIR
        .get_or_init(|| tempfile::tempdir().ok())
        .as_ref()
        .map_or_else(std::env::temp_dir, |d| d.path().to_path_buf());
    let thread = format!("{:?}", std::thread::current().id())
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    base.join(format!("{thread}-{name}"))
}

/// Builds the keymap of a preset whose TOML is `src`.
pub fn keymap_toml(src: &str) {
    use crate::config::keybindings::KeybindingsConfig;
    use crate::keybindings::loader::{KeymapLoadReport, KeymapSpec, build_keymap};

    let keybindings = KeybindingsConfig::default();
    let spec = KeymapSpec {
        preset: "fuzz",
        keybindings: &keybindings,
        yazi_letters: false,
        plugins: Vec::new(),
    };
    let source = |name: &str, _: &mut KeymapLoadReport| (name == "fuzz").then(|| src.to_string());
    let _ = build_keymap(&spec, &source);
}

/// Parses `args` as the command line of an interactive run.
pub fn launch_args(args: &[String]) {
    if let Err(e) = crate::launch_args::LaunchArgs::parse(args.iter().cloned()) {
        let _ = e.message();
    }
}

/// Renders `text` (terminal output with escape sequences) as styled text.
pub fn ansi(text: &str) {
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let _ = crate::ui::terminal_screen::terminal_output_text(&lines, 200);
}

/// Multi-rename rules applied to one file name.
pub struct RenameInput<'a> {
    pub name_mask: &'a str,
    pub ext_mask: &'a str,
    pub search: &'a str,
    pub replace: &'a str,
    pub regex: bool,
    pub file_name: &'a str,
}

/// Compiles the multi-rename rules and computes the new name.
pub fn rename_pattern(input: &RenameInput) {
    let rules = RenameRules {
        name_mask: input.name_mask.to_string(),
        ext_mask: input.ext_mask.to_string(),
        search: input.search.to_string(),
        replace: input.replace.to_string(),
        regex: input.regex,
        ..RenameRules::default()
    };
    let Ok(compiled) = rules.compile() else {
        return;
    };
    let source = RenameSource {
        path: Path::new("dir").join(input.file_name),
        is_dir: false,
        modified: None,
    };
    let _ = compiled.new_name(&source, 7);
}

/// Parses the contents of a release `.sha256` file.
pub fn sha256_file(text: &str) {
    let _ = crate::update::downloader::parse_sha256_file(text);
}

/// Compiles a user syntax definition (TOML) and highlights `text` with it;
/// the tokens of every line must be in order and on character boundaries.
pub fn syntax_definition(definition: &str, text: &str) {
    let Ok(lines) = crate::syntax::highlight_with_definition(definition, text) else {
        return;
    };
    for (tokens, line) in lines.iter().zip(text.lines()) {
        let mut end = 0;
        for token in tokens {
            assert!(token.range.start >= end && token.range.start < token.range.end);
            assert!(line.is_char_boundary(token.range.start));
            assert!(line.is_char_boundary(token.range.end));
            end = token.range.end;
        }
    }
}
