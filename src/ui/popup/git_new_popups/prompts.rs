//! Git name prompt (branch create / rename, stash save, tag create) and the
//! two-field dialogs (clone, add remote).

use crate::app::form::FieldPair;
use crate::app::state::popup::{GitNameAction, GitNamePromptState as Prompt};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier},
    widgets::Paragraph,
};

/// Title and label of a name prompt.
fn texts(action: &GitNameAction) -> (String, String) {
    match action {
        GitNameAction::CreateBranch { .. } => {
            (t("git_branch_create_title"), t("git_branch_create_prompt"))
        }
        GitNameAction::RenameBranch { old_name } => (
            t("git_branch_rename_title"),
            t("git_branch_rename_prompt").replace("{}", old_name),
        ),
        GitNameAction::SaveStash { include_untracked } => (
            t("git_stash_save_title"),
            format!(
                "{}  {} {}",
                t("git_stash_save_prompt"),
                if *include_untracked { "[X]" } else { "[ ]" },
                t("git_stash_untracked_hint")
            ),
        ),
        GitNameAction::CreateTag { .. } => (t("git_tag_create_title"), t("git_tag_create_prompt")),
    }
}

pub fn render_name_prompt(f: &mut Frame, prompt: &Prompt, theme: &Theme, size: Rect) -> bool {
    let (title, label) = texts(&prompt.action);
    let inner = kit::frame_in(
        f,
        centered_rect(60, 25, size),
        kit::accent_title(format!(" {} ", title), Color::Cyan),
        kit::fg(Color::Cyan),
        theme,
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Prompt text
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Buttons
        ])
        .split(inner);

    f.render_widget(Paragraph::new(label), chunks[0]);
    f.render_widget(
        kit::input_box(&prompt.input, prompt.cursor_idx == 0, theme),
        chunks[1],
    );
    let styles = FocusStyles {
        active: kit::selection(theme).add_modifier(Modifier::BOLD),
        normal: kit::popup_fg(theme),
    };
    let buttons = [
        format!(" [ {} ] ", t("btn_ok").trim()),
        format!(" [ {} ] ", t("btn_cancel").trim()),
    ];
    f.render_widget(
        kit::button_bar(
            &buttons,
            Prompt::FORM.focused_button(prompt.cursor_idx),
            styles,
        ),
        chunks[3],
    );
    true
}

/// Labels and hint of a two-field dialog.
pub struct PairTexts {
    pub title: String,
    pub labels: [String; 2],
    pub hint: String,
}

/// Clone / add-remote: two labelled input boxes and a hint line.
pub fn render_field_pair(
    f: &mut Frame,
    pair: &FieldPair,
    texts: PairTexts,
    theme: &Theme,
    size: Rect,
) -> bool {
    let inner = kit::frame_in(
        f,
        centered_rect(65, 38, size),
        kit::accent_title(texts.title, Color::Cyan),
        kit::fg(Color::Cyan),
        theme,
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // first label
            Constraint::Length(3), // first input box
            Constraint::Length(1), // second label
            Constraint::Length(3), // second input box
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint bar
        ])
        .split(inner);
    for (i, label) in texts.labels.into_iter().enumerate() {
        f.render_widget(
            Paragraph::new(label).style(kit::popup_fg(theme)),
            chunks[i * 2],
        );
        f.render_widget(
            kit::input_box(&pair.fields[i], pair.focus == i, theme),
            chunks[i * 2 + 1],
        );
    }
    f.render_widget(kit::hint(texts.hint), chunks[5]);
    true
}
