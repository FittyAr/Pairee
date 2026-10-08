use crate::config::localization::t;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Maximum bytes the internal viewer loads; larger files are shown
/// truncated with a notice.
pub const VIEWER_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Viewing mode for the internal file viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerMode {
    Text,
    Hex,
    Image,
}

/// State for the internal F3 viewer.
#[derive(Debug, Clone)]
pub struct ViewerState {
    pub path: PathBuf,
    /// Lines of text content (used in Text mode).
    pub lines: Vec<String>,
    /// Raw bytes (used in Hex mode).
    pub raw: Vec<u8>,
    /// Loaded image data if applicable.
    pub image_data: Option<image::DynamicImage>,
    pub is_image: bool,
    pub is_text: bool,
    pub mode: ViewerMode,
    /// Vertical scroll offset (line, hex row index, or image character row).
    pub scroll: usize,
    /// Last search query
    pub last_search: Option<String>,
    pub last_case_sensitive: bool,
    /// True while the content is still being read in the background.
    pub loading: bool,
}

pub(crate) fn is_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff" | "ico" | "tga"
            )
        })
        .unwrap_or(false)
}

impl ViewerState {
    /// Load a file for viewing. Skips image decoding when `allow_image` is false.
    pub fn load_with_images(path: PathBuf, allow_image: bool) -> Self {
        Self::load_capped(path, allow_image, VIEWER_MAX_BYTES)
    }

    /// Placeholder shown while [`Self::load_with_images`] runs in the background.
    pub fn loading(path: PathBuf) -> Self {
        let mut state = Self::from_text(path, vec![t("viewer_loading")]);
        state.loading = true;
        state
    }

    /// Text-only state (listings, messages, errors).
    pub fn from_text(path: PathBuf, lines: Vec<String>) -> Self {
        Self {
            path,
            lines,
            raw: Vec::new(),
            image_data: None,
            is_image: false,
            is_text: true,
            mode: ViewerMode::Text,
            scroll: 0,
            last_search: None,
            last_case_sensitive: false,
            loading: false,
        }
    }

    /// Loads at most `max_bytes`; read errors are shown as text instead of
    /// an empty viewer.
    pub fn load_capped(path: PathBuf, allow_image: bool, max_bytes: u64) -> Self {
        let (mut raw, total) = match read_prefix(&path, max_bytes) {
            Ok(read) => read,
            Err(e) => {
                let msg = t("viewer_read_error")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                return Self::from_text(path, vec![msg]);
            }
        };
        let truncated = total > raw.len() as u64;
        if truncated {
            // Never split a UTF-8 sequence at the cap.
            if let Err(e) = std::str::from_utf8(&raw)
                && e.error_len().is_none()
            {
                raw.truncate(e.valid_up_to());
            }
        }

        let mut image_data = None;
        let mut mode = ViewerMode::Hex;

        if allow_image
            && !truncated
            && is_image_extension(&path)
            && let Ok(img) = image::open(&path)
        {
            image_data = Some(img);
            mode = ViewerMode::Image;
        }

        let is_image = image_data.is_some();
        let is_text = std::str::from_utf8(&raw).is_ok();

        let mut lines: Vec<String> = String::from_utf8_lossy(&raw)
            .lines()
            .map(|l| l.to_string())
            .collect();
        if truncated {
            lines.push(
                t("viewer_truncated")
                    .replacen("{}", &bytesize::ByteSize::b(max_bytes).to_string(), 1)
                    .replacen("{}", &bytesize::ByteSize::b(total).to_string(), 1),
            );
        }

        if !is_image {
            mode = if is_text {
                ViewerMode::Text
            } else {
                ViewerMode::Hex
            };
        }

        Self {
            path,
            lines,
            raw,
            image_data,
            is_image,
            is_text,
            mode,
            scroll: 0,
            last_search: None,
            last_case_sensitive: false,
            loading: false,
        }
    }

    pub fn toggle_mode(&mut self) {
        if self.is_image && !self.is_text {
            self.mode = match self.mode {
                ViewerMode::Image => ViewerMode::Hex,
                _ => ViewerMode::Image,
            };
        } else if self.is_text && !self.is_image {
            self.mode = match self.mode {
                ViewerMode::Text => ViewerMode::Hex,
                _ => ViewerMode::Text,
            };
        } else {
            self.mode = match self.mode {
                ViewerMode::Text => ViewerMode::Hex,
                ViewerMode::Hex => ViewerMode::Image,
                ViewerMode::Image => ViewerMode::Text,
            };
        }
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = match self.mode {
            ViewerMode::Text => self.lines.len().saturating_sub(1),
            ViewerMode::Hex => (self.raw.len() / 16).saturating_sub(1),
            ViewerMode::Image => {
                if let Some(ref img) = self.image_data {
                    (img.height() as usize / 2).saturating_sub(1)
                } else {
                    0
                }
            }
        };
        self.scroll = (self.scroll + amount).min(max);
    }
}

/// Reads up to `max_bytes` from `path`; returns the bytes and the file size.
fn read_prefix(path: &Path, max_bytes: u64) -> std::io::Result<(Vec<u8>, u64)> {
    let file = std::fs::File::open(path)?;
    let total = file.metadata()?.len();
    let mut raw = Vec::with_capacity(total.min(max_bytes) as usize);
    file.take(max_bytes).read_to_end(&mut raw)?;
    Ok((raw, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_with_images_false_skips_decode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dot.png");
        image::RgbImage::new(1, 1).save(&path).unwrap();

        let with_img = ViewerState::load_with_images(path.clone(), true);
        assert!(with_img.image_data.is_some());
        assert_eq!(with_img.mode, ViewerMode::Image);

        let no_img = ViewerState::load_with_images(path, false);
        assert!(no_img.image_data.is_none());
        assert_ne!(no_img.mode, ViewerMode::Image);
    }

    #[test]
    fn missing_file_shows_error_instead_of_empty_viewer() {
        let vs = ViewerState::load_with_images(PathBuf::from("/definitely/missing.txt"), false);
        assert_eq!(vs.lines.len(), 1);
        assert!(vs.lines[0].contains("missing.txt"));
        assert_eq!(vs.mode, ViewerMode::Text);
    }

    #[test]
    fn large_file_is_truncated_with_notice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        std::fs::write(&path, "abcdefghij\n".repeat(10)).unwrap();
        let vs = ViewerState::load_capped(path, false, 22);
        assert_eq!(vs.raw.len(), 22);
        assert_eq!(&vs.lines[..2], ["abcdefghij", "abcdefghij"]);
        assert_eq!(vs.lines.len(), 3, "notice appended");
        assert!(vs.is_text);
    }

    #[test]
    fn image_extension_recognises_common_formats() {
        assert!(is_image_extension(Path::new("a.PNG")));
        assert!(is_image_extension(Path::new("b.jpeg")));
        assert!(!is_image_extension(Path::new("c.rs")));
        assert!(!is_image_extension(Path::new("noext")));
    }

    #[test]
    fn text_mode_toggles_to_hex_and_back() {
        let mut state = ViewerState {
            path: PathBuf::from("n.txt"),
            lines: vec!["a".into(), "b".into(), "c".into()],
            raw: b"abc".to_vec(),
            image_data: None,
            is_image: false,
            is_text: true,
            mode: ViewerMode::Text,
            scroll: 2,
            last_search: None,
            last_case_sensitive: false,
            loading: false,
        };
        state.toggle_mode();
        assert_eq!(state.mode, ViewerMode::Hex);
        assert_eq!(state.scroll, 0);
        state.toggle_mode();
        assert_eq!(state.mode, ViewerMode::Text);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut state = ViewerState {
            path: PathBuf::from("n.txt"),
            lines: vec!["a".into()],
            raw: b"a".to_vec(),
            image_data: None,
            is_image: false,
            is_text: true,
            mode: ViewerMode::Text,
            scroll: 0,
            last_search: None,
            last_case_sensitive: false,
            loading: false,
        };
        state.scroll_up(3);
        assert_eq!(state.scroll, 0);
        state.scroll = 1;
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);
    }
}
