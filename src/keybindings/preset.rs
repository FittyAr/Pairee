use super::actions::Action;
use std::collections::HashMap;

/// Returns the default key-to-action bindings for a given preset name.
pub fn get_preset_bindings(preset: &str) -> HashMap<String, Action> {
    let mut map = HashMap::new();

    match preset.to_lowercase().as_str() {
        "vim" => {
            insert_common_norton_bindings(&mut map);
            // Vim-style navigation overrides
            map.insert("k".to_string(), Action::MoveUp);
            map.insert("j".to_string(), Action::MoveDown);
            map.insert("Ctrl+b".to_string(), Action::PageUp);
            map.insert("Ctrl+f".to_string(), Action::PageDown);
            map.insert("g".to_string(), Action::GoToTop);
            map.insert("G".to_string(), Action::GoToBottom);
            map.insert("l".to_string(), Action::Execute);
            map.insert("h".to_string(), Action::GoParent);
            map.insert("v".to_string(), Action::SelectItem);
            map.insert("y".to_string(), Action::Copy);
            map.insert("m".to_string(), Action::Move);
            map.insert("d".to_string(), Action::Delete);
            map.insert(":".to_string(), Action::FocusCli);
        }
        "modern" => {
            insert_common_norton_bindings(&mut map);
            // Modern shortcuts overrides
            map.insert("Ctrl+c".to_string(), Action::Copy);
            map.insert("Ctrl+C".to_string(), Action::Copy);
            map.insert("Ctrl+x".to_string(), Action::Move);
            map.insert("Ctrl+X".to_string(), Action::Move);
            map.insert("Ctrl+n".to_string(), Action::MkDir);
            map.insert("Ctrl+N".to_string(), Action::MkDir);
            map.insert("Delete".to_string(), Action::Delete);
            map.insert("Ctrl+q".to_string(), Action::Quit);
            map.insert("Ctrl+Q".to_string(), Action::Quit);
        }
        _ => {
            // Default "norton" preset — pure NC/Far Manager key layout
            insert_common_norton_bindings(&mut map);
            map.insert("F5".to_string(), Action::Copy);
            map.insert("F6".to_string(), Action::Move);
        }
    }

    map
}

/// Inserts the bindings that are identical across all presets to avoid duplication.
fn insert_common_norton_bindings(map: &mut HashMap<String, Action>) {
    // ── Navigation ───────────────────────────────────────────────────────────
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

    // ── Panel view modes (Ctrl+1 … Ctrl+9) ──────────────────────────────────
    map.insert("Ctrl+1".to_string(), Action::PanelViewBrief);
    map.insert("Ctrl+2".to_string(), Action::PanelViewMedium);
    map.insert("Ctrl+3".to_string(), Action::PanelViewFull);
    map.insert("Ctrl+4".to_string(), Action::PanelViewWide);
    map.insert("Ctrl+5".to_string(), Action::PanelViewDetailed);
    map.insert("Ctrl+6".to_string(), Action::PanelViewDescriptions);
    map.insert("Ctrl+7".to_string(), Action::PanelViewFileOwners);
    map.insert("Ctrl+8".to_string(), Action::PanelViewFileLinks);
    map.insert("Ctrl+9".to_string(), Action::PanelViewAltFull);

    // ── Panel toggles ────────────────────────────────────────────────────────
    map.insert("Ctrl+F1".to_string(), Action::TogglePanelLeft);
    map.insert("Ctrl+F2".to_string(), Action::TogglePanelRight);
    map.insert("Ctrl+o".to_string(), Action::ToggleBothPanels);
    map.insert("Ctrl+O".to_string(), Action::ToggleBothPanels);
    map.insert("Ctrl+l".to_string(), Action::InfoPanel);
    map.insert("Ctrl+L".to_string(), Action::InfoPanel);
    map.insert("Ctrl+q".to_string(), Action::QuickView);
    map.insert("Ctrl+Q".to_string(), Action::QuickView);
    map.insert("Ctrl+F12".to_string(), Action::SortModes);
    map.insert("Ctrl+n".to_string(), Action::ToggleLongNames);
    map.insert("Ctrl+N".to_string(), Action::ToggleLongNames);

    // ── F-key standard actions ────────────────────────────────────────────────
    map.insert("F1".to_string(), Action::Help);
    map.insert("F2".to_string(), Action::UserMenu);
    map.insert("F3".to_string(), Action::View);
    map.insert("F4".to_string(), Action::Edit);
    map.insert("F7".to_string(), Action::MkDir);
    map.insert("F8".to_string(), Action::Delete);
    map.insert("F9".to_string(), Action::Menu);
    map.insert("F10".to_string(), Action::Quit);
    map.insert("F11".to_string(), Action::PluginMenu);
    map.insert("F12".to_string(), Action::ScreensList);

    // ── Shift+F actions ──────────────────────────────────────────────────────
    map.insert("Shift+F1".to_string(), Action::CompressFiles);
    map.insert("Shift+F2".to_string(), Action::ExtractArchive);
    map.insert("Shift+F3".to_string(), Action::ArchiveCommands);
    map.insert("Shift+F9".to_string(), Action::SaveSetup);

    // ── Alt+F actions ────────────────────────────────────────────────────────
    map.insert("Alt+F1".to_string(), Action::DriveSelectLeft);
    map.insert("Alt+F2".to_string(), Action::DriveSelectRight);
    map.insert("Alt+F6".to_string(), Action::CreateLink);
    map.insert("Alt+F7".to_string(), Action::FindFile);
    map.insert("Alt+F8".to_string(), Action::CommandHistory);
    map.insert("Alt+F9".to_string(), Action::VideoMode);
    map.insert("Alt+F10".to_string(), Action::TreeView);
    map.insert("Alt+F11".to_string(), Action::FileViewHistory);
    map.insert("Alt+F12".to_string(), Action::FoldersHistory);

    // ── File operations ──────────────────────────────────────────────────────
    map.insert("Alt+Delete".to_string(), Action::WipeFile);
    map.insert("Ctrl+a".to_string(), Action::FileAttributes);
    map.insert("Ctrl+A".to_string(), Action::FileAttributes);
    map.insert("Ctrl+g".to_string(), Action::ApplyCommand);
    map.insert("Ctrl+G".to_string(), Action::ApplyCommand);
    map.insert("Ctrl+z".to_string(), Action::DescribeFile);
    map.insert("Ctrl+Z".to_string(), Action::DescribeFile);

    // ── Bulk selection ────────────────────────────────────────────────────────
    map.insert("Gray+".to_string(), Action::SelectGroup);
    map.insert("Gray-".to_string(), Action::UnselectGroup);
    map.insert("Gray*".to_string(), Action::InvertSelection);
    map.insert("Ctrl+m".to_string(), Action::RestoreSelection);
    map.insert("Ctrl+M".to_string(), Action::RestoreSelection);

    // ── Commands ─────────────────────────────────────────────────────────────
    map.insert("Ctrl+u".to_string(), Action::SwapPanels);
    map.insert("Ctrl+U".to_string(), Action::SwapPanels);
    map.insert("Ctrl+i".to_string(), Action::FilePanelFilter);
    map.insert("Ctrl+I".to_string(), Action::FilePanelFilter);
    map.insert("Ctrl+w".to_string(), Action::TaskList);
    map.insert("Ctrl+W".to_string(), Action::TaskList);

    // ── General ──────────────────────────────────────────────────────────────
    map.insert("Ctrl+h".to_string(), Action::ToggleHidden);
    map.insert("Ctrl+H".to_string(), Action::ToggleHidden);
    map.insert("Ctrl+r".to_string(), Action::Refresh);
    map.insert("Ctrl+R".to_string(), Action::Refresh);
    map.insert("Esc".to_string(), Action::Unfocus);
    map.insert("Alt+c".to_string(), Action::CompressFiles);
    map.insert("Alt+e".to_string(), Action::ExtractArchive);
    map.insert("Menu".to_string(), Action::ContextMenu);
    map.insert("Alt+m".to_string(), Action::ContextMenu);

    // ── Folder shortcuts 1–9 (Ctrl+Alt+n) ────────────────────────────────────
    for n in 1u8..=9 {
        map.insert(format!("Ctrl+Alt+{}", n), Action::GoFolderShortcut(n));
    }
}
