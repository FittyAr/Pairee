use super::plugin_widget::PluginWidget;
use std::path::PathBuf;

/// Quick-view overlay (passive panel). Boxed in [super::PopupType] because of DynamicImage.
#[derive(Debug, Clone)]
pub struct QuickViewDialog {
    pub path: PathBuf,
    pub content: Vec<String>,
    pub scroll: usize,
    pub image_data: Option<image::DynamicImage>,
    pub plugin_widget: Option<PluginWidget>,
}
