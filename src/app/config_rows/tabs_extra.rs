//! Interface, Plugins, Editor/Viewer, Colors and Git tabs.

use super::{CycleFormat, Kind, Label, Row, RowCtx, Setting, cycle, title, title_key, toggle};
use crate::app::state::ConfigDraft;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::keybindings::embedded::available_presets;
use crate::keybindings::loader::{KeymapLoadReport, KeymapSpec, load_keymap};

pub fn interface(ctx: &RowCtx) -> Vec<Row> {
    let report = keymap_report(ctx.settings, ctx.keybindings);
    let status = Label::Owned(report.summary_line());
    let status_row = if report.ok() && report.warnings.is_empty() {
        Row::Hint(status)
    } else {
        Row::Subtitle(status)
    };
    vec![
        title("General"),
        toggle!("int_clock", interface_clock),
        toggle!("int_mouse", mouse_support),
        toggle!("int_key_bar", interface_show_key_bar),
        toggle!("int_menu_bar", interface_always_show_menu_bar),
        toggle!("int_restore_session", restore_session),
        title("Keybindings"),
        cycle(
            "int_keybindings",
            0,
            CycleFormat::Angle,
            |s| s.keymap_preset.clone(),
            |s| s.keymap_preset = next_preset(&s.keymap_preset),
        ),
        status_row,
        Row::Hint(Label::Key("int_keymap_gray")),
        action("int_keymap_view", |s, context| {
            Some(PopupType::InfoPanel {
                lines: keymap_report(s, &context.config.keybindings).detail_lines(),
            })
        }),
        action("int_keymap_shortcuts", |s, context| {
            let preset = crate::keybindings::embedded::normalize_preset_name(&s.keymap_preset);
            let rows = crate::app::shortcuts::edit::rows_for(context, &preset);
            Some(PopupType::Shortcuts(Box::new(
                crate::app::shortcuts::state::ShortcutsState::new(preset, rows),
            )))
        }),
        toggle!("int_yazi_workflow", enable_yazi_workflow),
    ]
}

/// Validation report of the keymap the draft would load.
fn keymap_report(
    draft: &ConfigDraft,
    keybindings: &crate::config::keybindings::KeybindingsConfig,
) -> KeymapLoadReport {
    load_keymap(&KeymapSpec {
        preset: &draft.keymap_preset,
        keybindings,
        yazi_letters: draft.enable_yazi_workflow,
        plugins: crate::keybindings::plugin_commands::active(),
    })
    .report
}

/// The preset after `current` (built-in ones, then the user's own files;
/// wrapping, unknown names restart).
fn next_preset(current: &str) -> String {
    let names = available_presets(&crate::config::paths::get_keymaps_dir());
    let next = names.iter().position(|n| n == current).map_or(0, |i| i + 1);
    names[next % names.len()].clone()
}

pub fn plugins(settings: &Settings) -> Vec<Row> {
    let mut rows = vec![
        title("Language"),
        cycle(
            "lang_label",
            0,
            CycleFormat::ColonAngle,
            |s| s.language.clone(),
            next_language,
        ),
        title_key("plugins_manager_settings"),
        Row::Subtitle(Label::Owned(format!("  {}", t("plugin_selection")))),
        toggle!(1, "developer_mode", plugins_developer_mode),
    ];
    if settings.plugins_developer_mode {
        rows.push(Row::Setting(Setting {
            label: Label::Raw("Path"),
            indent: 2,
            kind: Kind::Edit {
                show: |s| s.plugins_dev_dir.clone(),
                get: |s| s.plugins_dev_dir.clone(),
                set: |s, v| s.plugins_dev_dir = v.to_string(),
            },
        }));
    }
    rows
}

/// Next language among the installed translation files.
fn next_language(s: &mut ConfigDraft) {
    let discovered = crate::config::localization::discover_languages();
    if discovered.is_empty() {
        return;
    }
    let next = discovered
        .iter()
        .position(|(name, _)| name == &s.language)
        .map_or(0, |idx| (idx + 1) % discovered.len());
    s.language = discovered[next].0.clone();
}

/// Tab sizes offered by the dialog: 2 → 4 → 8 → 2.
fn next_tab_size(size: u32) -> u32 {
    match size {
        2 => 4,
        4 => 8,
        _ => 2,
    }
}

pub fn editor_viewer() -> Vec<Row> {
    vec![
        title_key("ed_internal_title"),
        cycle(
            "ed_tab_size",
            1,
            CycleFormat::Bracket,
            |s| s.editor_tab_size.to_string(),
            |s| s.editor_tab_size = next_tab_size(s.editor_tab_size),
        ),
        cycle(
            "ed_expand_tabs",
            1,
            CycleFormat::Bracket,
            |s| t(s.editor_expand_tabs.label_key()),
            |s| s.editor_expand_tabs = s.editor_expand_tabs.next(),
        ),
        toggle!(1, "ed_auto_indent", editor_auto_indent),
        toggle!(1, "ed_show_line_numbers", editor_show_line_numbers),
        toggle!(1, "ed_cursor_at_end", editor_cursor_at_end),
        toggle!(1, "ed_lock_readonly", editor_lock_editing_readonly),
        toggle!(1, "ed_warn_readonly", editor_warn_opening_readonly),
        title_key("vi_settings_title"),
        toggle!(1, "vi_external", viewer_use_external),
        toggle!(1, "vi_enter_external", enter_use_external),
        Row::Subtitle(Label::Key("vi_internal_title")),
        cycle(
            "vi_tab_size",
            1,
            CycleFormat::Bracket,
            |s| s.viewer_tab_size.to_string(),
            |s| s.viewer_tab_size = next_tab_size(s.viewer_tab_size),
        ),
        toggle!(1, "vi_show_scrollbar", viewer_show_scrollbar),
        toggle!(1, "vi_autodetect_codepage", viewer_autodetect_codepage),
        cycle(
            "vi_default_codepage",
            1,
            CycleFormat::Bracket,
            |s| s.viewer_default_codepage.clone(),
            |s| s.viewer_default_codepage = next_codepage(&s.viewer_default_codepage),
        ),
    ]
}

/// The encoding after `name` in the viewer's list (wrapping; unknown → first).
fn next_codepage(name: &str) -> String {
    use crate::fs::text::{ENCODINGS, encoding_by_name};
    let current = encoding_by_name(name);
    let next = ENCODINGS
        .iter()
        .position(|e| Some(*e) == current)
        .map_or(0, |i| (i + 1) % ENCODINGS.len());
    ENCODINGS[next].name().to_string()
}

pub fn colors() -> Vec<Row> {
    vec![
        title("Appearance & Theme"),
        cycle(
            "col_theme",
            0,
            CycleFormat::ColonAngle,
            |s| s.theme.clone(),
            |s| {
                s.theme = match s.theme.as_str() {
                    "slate" => "classic_blue",
                    _ => "slate",
                }
                .to_string();
            },
        ),
        action("col_groups", |_, context| {
            Some(PopupType::ColorGroupsDialog {
                cursor_idx: 0,
                edit: None,
                theme: context.config.theme.clone(),
            })
        }),
        action("col_highlighting", |s, _| {
            Some(PopupType::FilesHighlightingDialog {
                cursor_idx: 0,
                edit: None,
                rules: s.highlight_rules.clone(),
            })
        }),
    ]
}

pub fn git() -> Vec<Row> {
    vec![
        title_key("git_section_general"),
        toggle!("git_enabled", git_enabled),
        title_key("git_section_author"),
        edit(
            "git_author_name",
            |s| or_git_config(&s.git_author_name),
            |s| s.git_author_name.clone(),
            |s, v| s.git_author_name = v.to_string(),
        ),
        edit(
            "git_author_email",
            |s| or_git_config(&s.git_author_email),
            |s| s.git_author_email.clone(),
            |s, v| s.git_author_email = v.to_string(),
        ),
        title_key("git_section_log"),
        edit(
            "git_log_limit",
            |s| s.git_log_limit.to_string(),
            |s| s.git_log_limit.to_string(),
            |s, v| {
                if let Ok(n) = v.parse::<u32>() {
                    s.git_log_limit = n.clamp(1, 10_000);
                }
            },
        ),
        Row::Hint(Label::Key("git_hint_config")),
    ]
}

/// The value, or "(from git config)" when empty.
fn or_git_config(value: &str) -> String {
    if value.is_empty() {
        format!("({})", t("git_from_git_config"))
    } else {
        value.to_string()
    }
}

fn action(
    key: &'static str,
    open: fn(&ConfigDraft, &crate::app::context::AppContext) -> Option<PopupType>,
) -> Row {
    Row::Setting(Setting {
        label: Label::Key(key),
        indent: 0,
        kind: Kind::Action(open),
    })
}

fn edit(
    key: &'static str,
    show: fn(&ConfigDraft) -> String,
    get: fn(&ConfigDraft) -> String,
    set: fn(&mut ConfigDraft, &str),
) -> Row {
    Row::Setting(Setting {
        label: Label::Key(key),
        indent: 0,
        kind: Kind::Edit { show, get, set },
    })
}
