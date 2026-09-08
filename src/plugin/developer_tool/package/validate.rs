use super::super::{progress_progress, progress_status};
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

pub fn validate_for_publish(path: &std::path::Path) -> Result<(), String> {
    validate_for_publish_with_progress(path, None)
}

pub fn validate_for_publish_with_progress(
    path: &std::path::Path,
    progress: Option<UnboundedSender<DevProgress>>,
) -> Result<(), String> {
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(t("plugin_dev_submit_no_manifest"));
    }

    progress_status(&progress, t("plugin_dev_progress_reading_manifest"));
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(t("plugin_dev_err_read_manifest").replace("{:?}", &format!("{:?}", e)));
        }
    };

    let manifest = match crate::plugin::loader::PluginManifest::parse(&content) {
        Ok(m) => m,
        Err(e) => {
            return Err(t("plugin_dev_err_parse_manifest").replace("{:?}", &format!("{:?}", e)));
        }
    };

    // 1. Validate Icon
    progress_status(&progress, t("plugin_dev_progress_validating_icon"));
    let icon_rel = match &manifest.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => return Err(t("plugin_dev_publish_no_icon")),
    };
    let icon_path = path.join(icon_rel);
    if !icon_path.exists() || !icon_path.is_file() {
        return Err(t("plugin_dev_publish_no_icon"));
    }

    // Check dimensions: 256x256 or 512x512
    match image::image_dimensions(&icon_path) {
        Ok((w, h)) => {
            if (w != 256 || h != 256) && (w != 512 || h != 512) {
                return Err(t("plugin_dev_publish_icon_invalid_size")
                    .replace("{w}", &w.to_string())
                    .replace("{h}", &h.to_string()));
            }
        }
        Err(e) => {
            return Err(
                t("plugin_dev_publish_icon_invalid_format").replace("{:?}", &format!("{:?}", e))
            );
        }
    }

    // 2. Validate Screenshots
    progress_status(&progress, t("plugin_dev_progress_validating_screenshots"));
    let screenshots = match &manifest.screenshots {
        Some(s) if !s.is_empty() => s,
        _ => return Err(t("plugin_dev_publish_no_screenshots")),
    };

    let total_screens = screenshots.iter().filter(|s| !s.trim().is_empty()).count();
    let mut scr_idx = 0;
    for scr_rel in screenshots {
        if scr_rel.trim().is_empty() {
            continue;
        }
        scr_idx += 1;
        progress_progress(
            &progress,
            t("plugin_dev_progress_checking_screenshot")
                .replace("{}", scr_rel)
                .replace("{n}", &scr_idx.to_string())
                .replace("{t}", &total_screens.to_string()),
            scr_idx,
            total_screens.max(1),
        );
        let scr_path = path.join(scr_rel);
        if !scr_path.exists() || !scr_path.is_file() {
            return Err(t("plugin_dev_publish_screenshot_not_found").replace("{}", scr_rel));
        }

        // Validate screenshot size: minimum 640x480 pixels
        match image::image_dimensions(&scr_path) {
            Ok((w, h)) => {
                if w < 640 || h < 480 {
                    return Err(t("plugin_dev_publish_screenshot_invalid_size")
                        .replace("{}", scr_rel)
                        .replace("{w}", &w.to_string())
                        .replace("{h}", &h.to_string()));
                }
            }
            Err(e) => {
                return Err(t("plugin_dev_publish_screenshot_invalid_format")
                    .replace("{}", scr_rel)
                    .replace("{:?}", &format!("{:?}", e)));
            }
        }
    }

    Ok(())
}
