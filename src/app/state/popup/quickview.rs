use super::plugin_widget::PluginPreview;
use std::path::PathBuf;

/// Quick-view overlay (passive panel). Boxed in [super::PopupType] because of DynamicImage.
#[derive(Debug, Clone)]
pub struct QuickViewDialog {
    pub path: PathBuf,
    pub content: Vec<String>,
    pub scroll: usize,
    pub image_data: Option<std::sync::Arc<image::DynamicImage>>,
    /// What a plugin previewer drew instead of the built-in preview.
    pub plugin: Option<PluginPreview>,
}
