use crate::app::context::AppContext;
use crate::app::state::PopupType;
use crate::config::settings::Settings;
use crate::keybindings::loader::load_keybinds;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    context: &AppContext,
) -> Option<PopupType> {
    match cursor_idx {
        0 => settings.interface_clock = !settings.interface_clock,
        1 => settings.mouse_support = !settings.mouse_support,
        2 => settings.interface_show_key_bar = !settings.interface_show_key_bar,
        3 => {
            settings.interface_always_show_menu_bar = !settings.interface_always_show_menu_bar;
        }
        36 => {
            settings.keybinding_preset = match settings.keybinding_preset.as_str() {
                "norton" => "neovim".to_string(),
                "neovim" => "vscode".to_string(),
                _ => "norton".to_string(),
            };
        }
        37 => {
            settings.enable_yazi_workflow = !settings.enable_yazi_workflow;
        }
        38 => {
            let (_, report) = load_keybinds(
                &settings.keybinding_preset,
                &context.config.keybindings.custom_bindings,
            );
            return Some(PopupType::InfoPanel {
                lines: report.detail_lines(),
            });
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, settings::Settings};

    #[test]
    fn view_keymap_issues_opens_info_panel() {
        let context = AppContext::new(AppConfig::default());
        let mut settings = Settings::default();
        match handle_row(38, &mut settings, &context) {
            Some(PopupType::InfoPanel { lines }) => {
                assert!(!lines.is_empty());
                assert!(
                    lines
                        .iter()
                        .any(|l| l.contains("Gray+") || l.contains("Plus")),
                    "expected Gray+/Plus hint, got {lines:?}"
                );
            }
            other => panic!("expected InfoPanel, got {other:?}"),
        }
    }
}
