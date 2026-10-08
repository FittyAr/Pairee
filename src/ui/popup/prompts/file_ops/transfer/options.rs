use crate::app::state::CopyMovePromptState as Prompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

const EXISTING_KEYS: [&str; Prompt::EXISTING_CHOICES] =
    ["opt_ask", "opt_overwrite", "opt_skip", "opt_append"];
const SYMLINK_KEYS: [&str; Prompt::SYMLINK_CHOICES] =
    ["opt_smartly_copy", "opt_copy_link", "opt_copy_target"];

/// Option rows (chunks 1..=12): choices, checkboxes, filter and separators.
/// Chunk `n + 1` shows dialog row `n` for rows 1..=8; row 9 (filter) sits
/// below a separator in chunk 11.
pub fn render_options(
    f: &mut Frame,
    chunks: &[Rect],
    prompt: &Prompt,
    styles: FocusStyles,
    width: u16,
) {
    let focus = prompt.cursor_idx;
    let sep = Style::default().fg(Color::Yellow);
    for chunk in [1, 10, 12] {
        f.render_widget(kit::separator(width, sep), chunks[chunk]);
    }

    let choice = |label: &str, keys: &[&str], value: usize| {
        let value = keys.get(value).map(|k| t(k)).unwrap_or_default();
        format!("{} {}", t(label), value)
    };
    let mut rows = vec![(
        Prompt::ROW_EXISTING,
        choice(
            "prompt_already_existing",
            &EXISTING_KEYS,
            prompt.already_existing,
        ),
    )];
    rows.extend(
        prompt
            .flags()
            .into_iter()
            .enumerate()
            .map(|(i, (on, key))| (Prompt::ROW_FIRST_FLAG + i, kit::checkbox_row(on, &t(key)))),
    );
    rows.push((
        Prompt::ROW_SYMLINKS,
        choice("prompt_symlinks", &SYMLINK_KEYS, prompt.symlink_mode),
    ));
    for (row, text) in rows {
        f.render_widget(
            Paragraph::new(text).style(styles.row(focus, row)),
            chunks[row + 1],
        );
    }

    let mask = if prompt.filter_mask.is_empty() {
        String::new()
    } else {
        format!(" [{}]", prompt.filter_mask)
    };
    let filter = format!(
        "{}{}",
        kit::checkbox_row(prompt.use_filter, &t("prompt_use_filter")),
        mask
    );
    f.render_widget(
        Paragraph::new(filter).style(styles.row(focus, Prompt::ROW_FILTER)),
        chunks[11],
    );
}
