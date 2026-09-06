use std::path::{Path, PathBuf};

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
        let raw = std::fs::read(&path).unwrap_or_default();

        let mut image_data = None;
        let mut mode = ViewerMode::Hex;

        if allow_image
            && is_image_extension(&path)
            && let Ok(img) = image::open(&path)
        {
            image_data = Some(img);
            mode = ViewerMode::Image;
        }

        let is_image = image_data.is_some();
        let is_text = std::str::from_utf8(&raw).is_ok();

        let lines = if is_text {
            std::str::from_utf8(&raw)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect()
        } else {
            String::from_utf8_lossy(&raw)
                .lines()
                .map(|l| l.to_string())
                .collect()
        };

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
        };
        state.scroll_up(3);
        assert_eq!(state.scroll, 0);
        state.scroll = 1;
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);
    }
}
