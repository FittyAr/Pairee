//! Blocking preview loader for the quick-view panel (runs as a background
//! job). Reads are capped so a huge file never stalls or exhausts memory.

use super::pdf::extract_pdf_text;
use crate::config::localization::t;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// Maximum bytes read from a file for a preview (text, PDF or image).
pub const QUICK_VIEW_MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Text previews read only the start of the file (the F3 viewer pages the rest).
pub const QUICK_VIEW_TEXT_BYTES: u64 = 256 * 1024;
/// Maximum folder entries listed in a folder preview.
pub const QUICK_VIEW_MAX_DIR_ENTRIES: usize = 10_000;

const SEPARATOR: &str = "────────────────────────────────────────";
const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff", "ico", "tga",
];

/// A loaded preview: text lines and/or a decoded image.
#[derive(Debug, Default)]
pub struct QuickViewPreview {
    pub content: Vec<String>,
    pub image: Option<Arc<image::DynamicImage>>,
}

impl QuickViewPreview {
    fn text(content: Vec<String>) -> Self {
        Self {
            content,
            image: None,
        }
    }
}

fn has_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| exts.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Loads the preview for `path`, reading at most `max_bytes` from files.
pub fn load_preview(path: &Path, allow_image: bool, max_bytes: u64) -> QuickViewPreview {
    if path.is_dir() {
        return QuickViewPreview::text(folder_lines(path));
    }
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let is_image = allow_image && has_ext(path, IMAGE_EXTS);
    let is_pdf = has_ext(path, &["pdf"]);
    if (is_image || is_pdf) && size > max_bytes {
        return QuickViewPreview::text(vec![too_large(size, max_bytes)]);
    }
    if is_image && let Ok(img) = image::open(path) {
        return QuickViewPreview {
            content: Vec::new(),
            image: Some(Arc::new(img)),
        };
    }
    if is_pdf {
        return QuickViewPreview::text(pdf_lines(path));
    }
    QuickViewPreview::text(file_lines(path, size, max_bytes))
}

fn too_large(size: u64, max_bytes: u64) -> String {
    t("quickview_too_large")
        .replacen("{}", &bytesize::ByteSize::b(size).to_string(), 1)
        .replacen("{}", &bytesize::ByteSize::b(max_bytes).to_string(), 1)
}

fn pdf_lines(path: &Path) -> Vec<String> {
    match std::fs::read(path) {
        Ok(bytes) => match extract_pdf_text(&bytes) {
            Some(text) => text.lines().map(str::to_string).collect(),
            None => vec![t("quickview_pdf_no_text")],
        },
        Err(e) => vec![t("quickview_pdf_error").replacen("{}", &e.to_string(), 1)],
    }
}

fn folder_lines(path: &Path) -> Vec<String> {
    let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
    let mut lines = vec![
        t("quickview_folder").replacen("{}", &dir_name, 1),
        SEPARATOR.to_string(),
    ];
    let Ok(read) = std::fs::read_dir(path) else {
        return lines;
    };
    let mut entries: Vec<(bool, String)> = read
        .flatten()
        .take(QUICK_VIEW_MAX_DIR_ENTRIES)
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (is_dir, e.file_name().to_string_lossy().into_owned())
        })
        .collect();
    entries.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    lines.extend(
        entries
            .into_iter()
            .map(|(is_dir, name)| if is_dir { format!("{name}/") } else { name }),
    );
    lines
}

fn file_lines(path: &Path, size: u64, max_bytes: u64) -> Vec<String> {
    use crate::fs::archive::{detect_format, list_archive_files};
    if let Some(format_name) = detect_format(path).label() {
        return match list_archive_files(path) {
            Ok(files) => {
                let archive_name = path.file_name().unwrap_or_default().to_string_lossy();
                let mut lines = vec![
                    t("quickview_archive").replacen("{}", &archive_name, 1),
                    t("quickview_format").replacen("{}", format_name, 1),
                    t("quickview_files").replacen("{}", &files.len().to_string(), 1),
                    SEPARATOR.to_string(),
                ];
                lines.extend(files);
                lines
            }
            Err(e) => vec![t("quickview_error").replacen("{}", &e.to_string(), 1)],
        };
    }
    let max_bytes = max_bytes.min(QUICK_VIEW_TEXT_BYTES);
    match read_text_prefix(path, max_bytes) {
        Some(text) => {
            let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
            if size > max_bytes {
                lines.push(
                    t("preview_truncated")
                        .replacen("{}", &bytesize::ByteSize::b(max_bytes).to_string(), 1)
                        .replacen("{}", &bytesize::ByteSize::b(size).to_string(), 1),
                );
            }
            lines
        }
        None => vec![t("quickview_binary_no_preview")],
    }
}

/// Reads up to `max_bytes` and decodes them in the detected encoding
/// (UTF-8/16, legacy code pages). A character cut by the cap is dropped;
/// `None` means binary or unreadable.
pub fn read_text_prefix(path: &Path, max_bytes: u64) -> Option<String> {
    let mut buf = Vec::new();
    let mut file = std::fs::File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    (&mut file).take(max_bytes).read_to_end(&mut buf).ok()?;
    let at_eof = buf.len() as u64 >= size;
    crate::fs::text::decode_prefix(&buf, at_eof, crate::fs::text::EncodingPolicy::Detect)
}
