use super::actions::Action;
use std::collections::HashMap;

/// Returns the default key-to-action bindings for a given preset name.
pub fn get_preset_bindings(preset: &str) -> HashMap<String, Action> {
    let mut map = HashMap::new();

    match preset.to_lowercase().as_str() {
        "vim" => {
            map.insert("k".to_string(), Action::MoveUp);
            map.insert("j".to_string(), Action::MoveDown);
            map.insert("Ctrl+b".to_string(), Action::PageUp);
            map.insert("Ctrl+f".to_string(), Action::PageDown);
            map.insert("g".to_string(), Action::GoToTop);
            map.insert("G".to_string(), Action::GoToBottom);
            map.insert("Tab".to_string(), Action::ChangePanel);
            map.insert("v".to_string(), Action::SelectItem);
            map.insert("Space".to_string(), Action::SelectItem);
            map.insert("l".to_string(), Action::Execute);
            map.insert("Enter".to_string(), Action::Execute);
            map.insert("h".to_string(), Action::GoParent);
            map.insert("Backspace".to_string(), Action::GoParent);
            map.insert("F1".to_string(), Action::Help);
            map.insert("F2".to_string(), Action::UserMenu);
            map.insert("F3".to_string(), Action::View);
            map.insert("F4".to_string(), Action::Edit);
            map.insert("y".to_string(), Action::Copy);
            map.insert("m".to_string(), Action::Move);
            map.insert("F7".to_string(), Action::MkDir);
            map.insert("d".to_string(), Action::Delete);
            map.insert("F9".to_string(), Action::Menu);
            map.insert("F10".to_string(), Action::Quit);
            map.insert("Ctrl+H".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+h".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+R".to_string(), Action::Refresh);
            map.insert("Ctrl+r".to_string(), Action::Refresh);
            map.insert("Ctrl+U".to_string(), Action::SwapPanels);
            map.insert("Ctrl+u".to_string(), Action::SwapPanels);
            map.insert("Alt+F1".to_string(), Action::DriveSelectLeft);
            map.insert("Alt+F2".to_string(), Action::DriveSelectRight);
            map.insert(":".to_string(), Action::FocusCli);
            map.insert("Esc".to_string(), Action::Unfocus);
        }
        "modern" => {
            map.insert("Up".to_string(), Action::MoveUp);
            map.insert("Down".to_string(), Action::MoveDown);
            map.insert("PageUp".to_string(), Action::PageUp);
            map.insert("PageDown".to_string(), Action::PageDown);
            map.insert("Home".to_string(), Action::GoToTop);
            map.insert("End".to_string(), Action::GoToBottom);
            map.insert("Tab".to_string(), Action::ChangePanel);
            map.insert("Space".to_string(), Action::SelectItem);
            map.insert("Enter".to_string(), Action::Execute);
            map.insert("Backspace".to_string(), Action::GoParent);
            map.insert("F1".to_string(), Action::Help);
            map.insert("Ctrl+c".to_string(), Action::Copy);
            map.insert("Ctrl+C".to_string(), Action::Copy);
            map.insert("Ctrl+x".to_string(), Action::Move);
            map.insert("Ctrl+X".to_string(), Action::Move);
            map.insert("Ctrl+n".to_string(), Action::MkDir);
            map.insert("Ctrl+N".to_string(), Action::MkDir);
            map.insert("Delete".to_string(), Action::Delete);
            map.insert("Ctrl+q".to_string(), Action::Quit);
            map.insert("Ctrl+Q".to_string(), Action::Quit);
            map.insert("Ctrl+H".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+h".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+R".to_string(), Action::Refresh);
            map.insert("Ctrl+r".to_string(), Action::Refresh);
            map.insert("Alt+F1".to_string(), Action::DriveSelectLeft);
            map.insert("Alt+F2".to_string(), Action::DriveSelectRight);
            map.insert("Esc".to_string(), Action::Unfocus);
        }
        _ => {
            // Default "norton" preset
            map.insert("Up".to_string(), Action::MoveUp);
            map.insert("Down".to_string(), Action::MoveDown);
            map.insert("PageUp".to_string(), Action::PageUp);
            map.insert("PageDown".to_string(), Action::PageDown);
            map.insert("Home".to_string(), Action::GoToTop);
            map.insert("End".to_string(), Action::GoToBottom);
            map.insert("Tab".to_string(), Action::ChangePanel);
            map.insert("Insert".to_string(), Action::SelectItem);
            map.insert("Space".to_string(), Action::SelectItem);
            map.insert("Enter".to_string(), Action::Execute);
            map.insert("Backspace".to_string(), Action::GoParent);
            map.insert("F1".to_string(), Action::Help);
            map.insert("F2".to_string(), Action::UserMenu);
            map.insert("F3".to_string(), Action::View);
            map.insert("F4".to_string(), Action::Edit);
            map.insert("F5".to_string(), Action::Copy);
            map.insert("F6".to_string(), Action::Move);
            map.insert("F7".to_string(), Action::MkDir);
            map.insert("F8".to_string(), Action::Delete);
            map.insert("F9".to_string(), Action::Menu);
            map.insert("F10".to_string(), Action::Quit);
            map.insert("Ctrl+H".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+h".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+R".to_string(), Action::Refresh);
            map.insert("Ctrl+r".to_string(), Action::Refresh);
            map.insert("Ctrl+U".to_string(), Action::SwapPanels);
            map.insert("Ctrl+u".to_string(), Action::SwapPanels);
            map.insert("Alt+F1".to_string(), Action::DriveSelectLeft);
            map.insert("Alt+F2".to_string(), Action::DriveSelectRight);
            map.insert("Esc".to_string(), Action::Unfocus);
        }
    }

    map
}
