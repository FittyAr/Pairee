use super::AppState;
use super::types::PopupType;
use flate2::read::ZlibDecoder;
use std::io::Read;

impl AppState {
    /// Dynamically updates the quick view panel preview content.
    pub fn update_quick_view(&mut self) {
        if self.quick_view_active {
            let active = self.get_active_panel();
            let target_path = if !active.selection_order.is_empty() {
                Some(active.selection_order[0].clone())
            } else if let Some(entry) = active.entries.get(active.cursor_index) {
                Some(entry.path.clone())
            } else {
                None
            };

            if let Some(path) = target_path {
                let needs_load = match &self.active_popup {
                    Some(PopupType::QuickViewPanel {
                        path: current_path, ..
                    }) => current_path != &path,
                    _ => true,
                };

                if needs_load {
                    let is_image_ext = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| {
                            let ext_lower = ext.to_lowercase();
                            matches!(
                                ext_lower.as_str(),
                                "png"
                                    | "jpg"
                                    | "jpeg"
                                    | "bmp"
                                    | "gif"
                                    | "webp"
                                    | "tif"
                                    | "tiff"
                                    | "ico"
                                    | "tga"
                            )
                        })
                        .unwrap_or(false);

                    let mut image_data = None;
                    if is_image_ext {
                        if let Ok(img) = image::open(&path) {
                            image_data = Some(img);
                        }
                    }

                    let is_pdf = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.to_lowercase() == "pdf")
                        .unwrap_or(false);

                    let content = if is_pdf {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                if let Some(pdf_text) = extract_pdf_text(&bytes) {
                                    pdf_text.lines().map(|s| s.to_string()).collect()
                                } else {
                                    vec!["[PDF file — no extractable text found]".to_string()]
                                }
                            }
                            Err(e) => vec![format!("[Error reading PDF: {}]", e)],
                        }
                    } else if image_data.is_some() {
                        Vec::new()
                    } else {
                        crate::ui::quickview::load_quick_view_content(&path)
                    };

                    self.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content,
                        scroll: 0,
                        image_data,
                    });
                }
            } else {
                self.active_popup = None;
            }
        }
    }
}

fn extract_pdf_text(data: &[u8]) -> Option<String> {
    let mut text_content = String::new();
    let mut pos = 0;
    while let Some(stream_start) = find_subsequence(&data[pos..], b"stream") {
        let actual_start = pos + stream_start + 6;
        let mut data_start = actual_start;
        while data_start < data.len() && (data[data_start] == b'\r' || data[data_start] == b'\n') {
            data_start += 1;
        }

        if let Some(stream_end) = find_subsequence(&data[data_start..], b"endstream") {
            let actual_end = data_start + stream_end;
            let compressed_data = &data[data_start..actual_end];

            let mut decoder = ZlibDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                let mut i = 0;
                let mut in_string = false;
                let mut current_str = Vec::new();
                let mut escaped = false;
                while i < decompressed.len() {
                    let c = decompressed[i];
                    if in_string {
                        if escaped {
                            current_str.push(c);
                            escaped = false;
                        } else if c == b'\\' {
                            escaped = true;
                        } else if c == b')' {
                            in_string = false;
                            let s = String::from_utf8_lossy(&current_str);
                            text_content.push_str(&s);
                            current_str.clear();
                        } else {
                            current_str.push(c);
                        }
                    } else if c == b'(' {
                        in_string = true;
                    } else if c == b'\n' || c == b'\r' {
                        text_content.push('\n');
                    }
                    i += 1;
                }
                text_content.push('\n');
            }
            pos = actual_end + 9;
        } else {
            break;
        }
    }

    if text_content.trim().is_empty() {
        None
    } else {
        let cleaned: Vec<String> = text_content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Some(cleaned.join("\n"))
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
