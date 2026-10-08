use super::encoding::{EncodingPolicy, decode_prefix, detect, encoding_by_name};
use super::lines::{LineFormat, MAX_LINE_BYTES, for_each_line};
use super::search::SEARCH_CHUNK;
use super::store::{BLOCK_SIZE, ByteStore, FileStore, MemStore};
use super::{IndexJob, TextDocument};
use encoding_rs::{Encoding, SHIFT_JIS, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};
use std::ops::ControlFlow;
use std::sync::Arc;

const AUTO: EncodingPolicy = EncodingPolicy::Detect;

fn utf16(text: &str, big_endian: bool) -> Vec<u8> {
    text.encode_utf16()
        .flat_map(|u| {
            if big_endian {
                u.to_be_bytes()
            } else {
                u.to_le_bytes()
            }
        })
        .collect()
}

/// Indexed document over `bytes` in `encoding` (index built inline).
fn doc(bytes: Vec<u8>, encoding: &'static Encoding) -> TextDocument {
    let bom = super::encoding::bom_len_for(&bytes, encoding);
    let (doc, job) = TextDocument::new(Arc::new(MemStore::new(bytes)), encoding, bom);
    job.run();
    doc
}

fn all_lines(doc: &TextDocument) -> Vec<String> {
    doc.lines(0, doc.line_count() as usize)
}

// ── Encoding detection ─────────────────────────────────────────────────

#[test]
fn bom_selects_utf8_and_utf16() {
    let d = detect(b"\xEF\xBB\xBFhi", true, AUTO);
    assert_eq!((d.encoding, d.bom_len, d.binary), (UTF_8, 3, false));
    let mut le = vec![0xFF, 0xFE];
    le.extend(utf16("hola", false));
    let d = detect(&le, true, AUTO);
    assert_eq!((d.encoding, d.bom_len), (UTF_16LE, 2));
    let mut be = vec![0xFE, 0xFF];
    be.extend(utf16("hola", true));
    assert_eq!(detect(&be, true, AUTO).encoding, UTF_16BE);
}

#[test]
fn utf16_without_bom_is_text_not_binary() {
    let le = utf16("Hello, world!\r\nSecond line\r\n", false);
    let d = detect(&le, true, AUTO);
    assert_eq!((d.encoding, d.bom_len, d.binary), (UTF_16LE, 0, false));
    let be = utf16("Hello, world!\nSecond line\n", true);
    assert_eq!(detect(&be, true, AUTO).encoding, UTF_16BE);
    assert_eq!(
        decode_prefix(&le, true, AUTO).as_deref(),
        Some("Hello, world!\r\nSecond line\r\n")
    );
}

#[test]
fn latin1_accents_decode_as_windows_1252() {
    let text =
        "Canción de la niña: el pingüino comió piñas en el año pasado, señor. Éxito garantizado.\n";
    let (bytes, _, _) = WINDOWS_1252.encode(text);
    let d = detect(&bytes, true, AUTO);
    assert!(!d.binary, "Latin-1 text is not binary");
    assert_eq!(d.encoding, WINDOWS_1252);
    assert_eq!(decode_prefix(&bytes, true, AUTO).as_deref(), Some(text));
}

#[test]
fn shift_jis_sample_is_detected() {
    let text =
        "日本語のテキストです。これは文字コードの自動判別のテストです。東京都の天気は晴れです。\n";
    let (bytes, _, _) = SHIFT_JIS.encode(text);
    let d = detect(&bytes, true, AUTO);
    assert_eq!(d.encoding, SHIFT_JIS);
    assert_eq!(decode_prefix(&bytes, true, AUTO).as_deref(), Some(text));
}

#[test]
fn nul_bytes_mean_binary_and_utf8_cut_is_dropped() {
    assert!(detect(b"\x7FELF\x02\x01\x01\0\0\0\0\0\x01\x02\x03", true, AUTO).binary);
    assert_eq!(decode_prefix(b"\x00\x01\x02", true, AUTO), None);
    // "añ" cut in the middle of "ñ" by the sample size: not an error.
    let cut = &"añ".as_bytes()[..2];
    assert_eq!(detect(cut, false, AUTO).encoding, UTF_8);
    assert_eq!(decode_prefix(cut, false, AUTO).as_deref(), Some("a"));
}

#[test]
fn fixed_policy_and_labels() {
    let fixed = EncodingPolicy::Fixed(WINDOWS_1252);
    assert_eq!(detect("ñ".as_bytes(), true, fixed).encoding, WINDOWS_1252);
    assert_eq!(encoding_by_name("UTF-8"), Some(UTF_8));
    assert_eq!(encoding_by_name("latin1"), Some(WINDOWS_1252));
    assert_eq!(encoding_by_name("1252"), Some(WINDOWS_1252));
    assert_eq!(encoding_by_name("no-such-thing"), None);
}

// ── Block cache, line splitting and index ──────────────────────────────

#[test]
fn block_cache_reads_across_block_boundaries() {
    let bytes: Vec<u8> = (0..BLOCK_SIZE * 3 + 17).map(|i| (i % 251) as u8).collect();
    let d = doc(bytes.clone(), WINDOWS_1252);
    let start = BLOCK_SIZE as u64 - 5;
    assert_eq!(
        d.bytes(start, 20),
        bytes[start as usize..start as usize + 20]
    );
    assert_eq!(
        d.bytes(bytes.len() as u64 - 3, 10),
        bytes[bytes.len() - 3..]
    );
    assert!(d.bytes(bytes.len() as u64 + 5, 10).is_empty());
}

#[test]
fn multibyte_char_split_across_blocks_decodes_whole() {
    // Put "ñ" (2 bytes) exactly across the first block boundary.
    let mut text = format!("{}\n", "x".repeat(99)).repeat((BLOCK_SIZE - 1) / 100);
    text.push_str(&"y".repeat(BLOCK_SIZE - 1 - text.len()));
    assert_eq!(text.len(), BLOCK_SIZE - 1);
    text.push_str("ñ\nend");
    let d = doc(text.into_bytes(), UTF_8);
    let lines = all_lines(&d);
    assert_eq!(lines.len(), (BLOCK_SIZE - 1) / 100 + 2);
    assert!(lines[lines.len() - 2].ends_with("yñ"));
    assert!(
        !lines.iter().any(|l| l.contains('\u{FFFD}')),
        "no broken chars"
    );
    assert_eq!(lines.last().map(String::as_str), Some("end"));
}

#[test]
fn crlf_split_across_chunks_is_one_terminator() {
    let mut bytes = b"x".repeat(9);
    bytes.extend_from_slice(b"\r\nnext\r\n\nlast");
    // Chunk of 10 bytes puts CR at the end of the first chunk, LF in the second.
    let store = MemStore::new(bytes);
    let mut seen = Vec::new();
    for_each_line(
        &store,
        LineFormat::for_encoding(UTF_8),
        0,
        10,
        |start, c| {
            seen.push((start, String::from_utf8(c.to_vec()).unwrap()));
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    let texts: Vec<&str> = seen.iter().map(|(_, s)| s.as_str()).collect();
    assert_eq!(texts, ["xxxxxxxxx", "next", "", "last"]);
    assert_eq!(seen[1].0, 11, "line start after CRLF");
}

#[test]
fn utf16_lines_split_on_code_units_only() {
    // U+0A0D ('\u{a0d}') has the bytes 0D 0A in LE: must not split there.
    let text = "uno\u{a0d}dos\r\ntres\n";
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(utf16(text, false));
    let d = doc(bytes, UTF_16LE);
    assert_eq!(all_lines(&d), ["uno\u{a0d}dos", "tres"]);
}

#[test]
fn long_lines_are_split_without_breaking_characters() {
    let text = "é".repeat(MAX_LINE_BYTES); // 2 bytes each, no line break
    let d = doc(text.clone().into_bytes(), UTF_8);
    let lines = all_lines(&d);
    assert!(lines.len() >= 2);
    assert!(lines.iter().all(|l| l.len() <= MAX_LINE_BYTES + 4));
    assert_eq!(lines.concat(), text);
}

#[test]
fn index_locates_lines_far_past_checkpoints() {
    let text: String = (0..2000).map(|i| format!("line {i}\n")).collect();
    let d = doc(text.into_bytes(), UTF_8);
    assert_eq!(d.line_count(), 2000);
    assert_eq!(d.index_progress(), None);
    assert_eq!(d.lines(1999, 5), ["line 1999"]);
    assert_eq!(d.lines(257, 2), ["line 257", "line 258"]);
    assert_eq!(d.line_offset(1), Some(7));
    assert!(d.lines(2000, 3).is_empty());
}

#[test]
fn empty_and_trailing_newline_match_str_lines() {
    assert_eq!(doc(Vec::new(), UTF_8).line_count(), 0);
    assert_eq!(all_lines(&doc(b"a\n".to_vec(), UTF_8)), ["a"]);
    assert_eq!(all_lines(&doc(b"\n".to_vec(), UTF_8)), [""]);
}

#[test]
fn switching_encoding_rebuilds_the_index() {
    let (bytes, _, _) = WINDOWS_1252.encode("café\nniño\n");
    let mut d = doc(bytes.into_owned(), UTF_8);
    assert!(all_lines(&d)[0].contains('\u{FFFD}'));
    d.set_encoding(WINDOWS_1252).run();
    assert_eq!(d.encoding(), WINDOWS_1252);
    assert_eq!(all_lines(&d), ["café", "niño"]);
}

#[test]
fn cancelled_index_stays_incomplete() {
    let (d, job) = TextDocument::new(Arc::new(MemStore::new(b"a\nb\n".to_vec())), UTF_8, 0);
    let IndexJob { cancel, .. } = &job;
    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    job.run();
    assert!(d.index_progress().is_some());
}

#[test]
fn file_store_pages_a_file_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.txt");
    let text: String = (0..50_000).map(|i| format!("row {i:05}\n")).collect();
    std::fs::write(&path, &text).unwrap();
    let store = FileStore::open(&path).unwrap();
    assert_eq!(store.len(), text.len() as u64);
    let (d, job) = TextDocument::new(Arc::new(store), UTF_8, 0);
    job.run();
    assert_eq!(d.line_count(), 50_000);
    assert_eq!(d.lines(49_999, 1), ["row 49999"]);
}

// ── Search ─────────────────────────────────────────────────────────────

fn run_search(d: &TextDocument, from: u64, query: &str, cs: bool) -> Option<u64> {
    d.search(from, query, cs)
        .expect("start line indexed")
        .run(&|| false, &|_| {})
}

#[test]
fn search_crosses_blocks_and_wraps() {
    // Needle placed so that it straddles a search chunk boundary.
    let fillers = (SEARCH_CHUNK - 100) / 7;
    let mut text = "filler\n".repeat(fillers);
    text.push_str(&"-".repeat(SEARCH_CHUNK - 3 - text.len()));
    text.push_str("NEEDLE here\nmore\n");
    let d = doc(text.into_bytes(), UTF_8);
    let target = fillers as u64;
    assert_eq!(run_search(&d, 0, "needle", false), Some(target));
    assert_eq!(run_search(&d, 0, "needle", true), None);
    // From past the match: wraps to the top and finds it again.
    assert_eq!(run_search(&d, target + 1, "NEEDLE", true), Some(target));
}

#[test]
fn search_decodes_with_the_document_encoding() {
    let (bytes, _, _) = SHIFT_JIS.encode("いち\nに\n東京\n");
    let d = doc(bytes.into_owned(), SHIFT_JIS);
    assert_eq!(run_search(&d, 0, "東京", true), Some(2));
}

#[test]
fn search_can_be_cancelled() {
    let d = doc("a\n".repeat(5000).into_bytes(), UTF_8);
    let job = d.search(0, "zzz", false).unwrap();
    assert_eq!(job.run(&|| true, &|_| {}), None);
}
