mod buttons;
mod flags;
mod input;
mod options;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::MovePrompt(crate::app::state::CopyMovePromptState {
        input,
        src_paths,
        dest_dir: _,
        cursor_idx,
        already_existing,
        process_multiple,
        copy_access_mode,
        copy_extended_attributes,
        disable_write_cache,
        produce_sparse_files,
        use_copy_on_write,
        symlink_mode,
        use_filter,
        filter_mask,
    }) = popup
    {
        let area = centered_rect_fixed(75, 17, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(t("prompt_move_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // 0: Input
                Constraint::Length(1), // 1: Sep
                Constraint::Length(1), // 2: Already existing
                Constraint::Length(1), // 3: Process multiple
                Constraint::Length(1), // 4: Copy access mode
                Constraint::Length(1), // 5: Extended attrs
                Constraint::Length(1), // 6: Disable write cache
                Constraint::Length(1), // 7: Sparse files
                Constraint::Length(1), // 8: COW
                Constraint::Length(1), // 9: Symlinks
                Constraint::Length(1), // 10: Sep
                Constraint::Length(1), // 11: Filter
                Constraint::Length(1), // 12: Sep
                Constraint::Length(1), // 13: Buttons
            ])
            .split(inner);

        let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

        input::render_input(
            f,
            chunks[0],
            src_paths,
            input,
            *cursor_idx,
            act_style,
            norm_style,
            true,
        );

        options::render_options(
            f,
            &chunks,
            &options::CopyMoveOptionsParams {
                cursor_idx: *cursor_idx,
                already_existing: *already_existing,
                process_multiple: *process_multiple,
                copy_access_mode: *copy_access_mode,
                copy_extended_attributes: *copy_extended_attributes,
                disable_write_cache: *disable_write_cache,
                produce_sparse_files: *produce_sparse_files,
                use_copy_on_write: *use_copy_on_write,
                symlink_mode: *symlink_mode,
                use_filter: *use_filter,
                filter_mask,
                theme,
                inner_width: inner.width as usize,
            },
        );

        buttons::render_buttons(
            f,
            chunks[13],
            *cursor_idx,
            &t("btn_move_bracket"),
            act_style,
            norm_style,
        );
        true
    } else {
        false
    }
}
