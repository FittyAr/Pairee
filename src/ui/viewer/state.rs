use crate::app::jobs::spawn_blocking_or_inline;
use crate::config::localization::t;
use crate::fs::text::{
    self, ByteStore, DETECT_SAMPLE_BYTES, EncodingPolicy, FileStore, IndexJob, TextDocument,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Largest file read whole: images decoded by the viewer (the editor uses the
/// same cap). Text and hex are paged, whatever the size.
pub const VIEWER_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Bytes shown per hex row.
pub const HEX_ROW_BYTES: usize = 16;

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
    /// The content, paged from disk (text and hex modes).
    pub doc: TextDocument,
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
    /// True while the content is still being opened in the background.
    pub loading: bool,
    /// Message for the status line (e.g. "not found"), cleared on the next search.
    pub notice: Option<String>,
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

/// Builds a line index on Tokio's blocking pool (inline without a runtime).
fn spawn_index(job: IndexJob) {
    spawn_blocking_or_inline(move || job.run());
}

impl ViewerState {
    /// Opens `path`: detects its encoding from the first bytes and starts
    /// indexing lines in the background, so even huge files open at once.
    /// Skips image decoding when `allow_image` is false.
    pub fn load_with_images(path: PathBuf, allow_image: bool, policy: EncodingPolicy) -> Self {
        let opened = FileStore::open(&path).and_then(|store| {
            let sample = store.read_range(0, DETECT_SAMPLE_BYTES as u64)?;
            Ok((store, sample))
        });
        let (store, sample) = match opened {
            Ok(opened) => opened,
            Err(e) => {
                let msg = t("viewer_read_error")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                return Self::from_text(path, vec![msg]);
            }
        };
        let size = store.len();
        let detected = text::detect(&sample, sample.len() as u64 >= size, policy);
        let (doc, job) = TextDocument::new(Arc::new(store), detected.encoding, detected.bom_len);
        spawn_index(job);

        let image_data = (allow_image && size <= VIEWER_MAX_BYTES && is_image_extension(&path))
            .then(|| image::open(&path).ok())
            .flatten();
        let is_text = !detected.binary;
        let mode = match (&image_data, is_text) {
            (Some(_), _) => ViewerMode::Image,
            (None, true) => ViewerMode::Text,
            (None, false) => ViewerMode::Hex,
        };
        Self {
            is_image: image_data.is_some(),
            image_data,
            is_text,
            mode,
            ..Self::with_doc(path, doc)
        }
    }

    /// Placeholder shown while [`Self::load_with_images`] runs in the background.
    pub fn loading(path: PathBuf) -> Self {
        let mut state = Self::from_text(path, vec![t("viewer_loading")]);
        state.loading = true;
        state
    }

    /// Text-only state (listings, messages, errors).
    pub fn from_text(path: PathBuf, lines: Vec<String>) -> Self {
        Self::with_doc(path, TextDocument::from_lines(&lines))
    }

    fn with_doc(path: PathBuf, doc: TextDocument) -> Self {
        Self {
            path,
            doc,
            image_data: None,
            is_image: false,
            is_text: true,
            mode: ViewerMode::Text,
            scroll: 0,
            last_search: None,
            last_case_sensitive: false,
            loading: false,
            notice: None,
        }
    }

    /// Re-reads the content in `encoding` (chosen by the user) as text.
    pub fn set_encoding(&mut self, encoding: &'static encoding_rs::Encoding) {
        spawn_index(self.doc.set_encoding(encoding));
        self.is_text = true;
        self.mode = ViewerMode::Text;
        self.scroll = 0;
        self.notice = None;
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

    /// Rows of content in the current mode (lines indexed so far in text mode).
    pub fn content_rows(&self) -> usize {
        match self.mode {
            ViewerMode::Text => self.doc.line_count() as usize,
            ViewerMode::Hex => self.doc.len().div_ceil(HEX_ROW_BYTES as u64) as usize,
            ViewerMode::Image => self
                .image_data
                .as_ref()
                .map_or(0, |img| img.height() as usize / 2),
        }
    }

    /// Largest scroll offset in the current mode.
    pub fn last_scroll(&self) -> usize {
        self.content_rows().saturating_sub(1)
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = (self.scroll + amount).min(self.last_scroll());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(path: PathBuf, allow_image: bool) -> ViewerState {
        ViewerState::load_with_images(path, allow_image, EncodingPolicy::Detect)
    }

    #[test]
    fn load_with_images_false_skips_decode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dot.png");
        image::RgbImage::new(1, 1).save(&path).unwrap();

        let with_img = load(path.clone(), true);
        assert!(with_img.image_data.is_some());
        assert_eq!(with_img.mode, ViewerMode::Image);

        let no_img = load(path, false);
        assert!(no_img.image_data.is_none());
        assert_ne!(no_img.mode, ViewerMode::Image);
    }

    #[test]
    fn missing_file_shows_error_instead_of_empty_viewer() {
        let vs = load(PathBuf::from("/definitely/missing.txt"), false);
        let lines = vs.doc.lines(0, 10);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("missing.txt"));
        assert_eq!(vs.mode, ViewerMode::Text);
    }

    #[test]
    fn non_utf8_text_opens_as_text_and_encoding_can_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("latin1.txt");
        let (bytes, _, _) = encoding_rs::WINDOWS_1252
            .encode("El pingüino comió piñas en la montaña con su señora.\n");
        std::fs::write(&path, &bytes).unwrap();
        let mut vs = load(path, false);
        assert!(vs.is_text);
        assert_eq!(vs.mode, ViewerMode::Text);
        assert_eq!(vs.doc.encoding(), encoding_rs::WINDOWS_1252);
        assert!(vs.doc.lines(0, 1)[0].contains("pingüino"));
        vs.set_encoding(encoding_rs::UTF_8);
        assert!(vs.doc.lines(0, 1)[0].contains('\u{FFFD}'));
    }

    #[test]
    fn binary_opens_in_hex_with_rows_from_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.bin");
        std::fs::write(&path, [0u8, 1, 2, 3].repeat(10)).unwrap();
        let mut vs = load(path, false);
        assert!(!vs.is_text);
        assert_eq!(vs.mode, ViewerMode::Hex);
        assert_eq!(vs.content_rows(), 3);
        vs.scroll_down(10);
        assert_eq!(vs.scroll, 2);
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
        let mut state = ViewerState::from_text("n.txt".into(), vec!["a".into(), "b".into()]);
        state.scroll = 1;
        state.toggle_mode();
        assert_eq!(state.mode, ViewerMode::Hex);
        assert_eq!(state.scroll, 0);
        state.toggle_mode();
        assert_eq!(state.mode, ViewerMode::Text);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut state = ViewerState::from_text("n.txt".into(), vec!["a".into()]);
        state.scroll_up(3);
        assert_eq!(state.scroll, 0);
        state.scroll = 1;
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);
    }
}
