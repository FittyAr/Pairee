use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::RenMovPrompt {
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
    } = popup
    {
        let area = centered_rect_fixed(75, 17, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(t("prompt_renmov_title"))
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
        let sep_style = Style::default().fg(Color::Yellow);
        let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);

        let count = src_paths.len();
        let first_name = src_paths
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let label = if count == 1 {
            t("prompt_move_sing").replacen("{}", &first_name, 1)
        } else {
            t("prompt_move_plur").replacen("{}", &count.to_string(), 1)
        };
        let in_style = if *cursor_idx == 0 {
            act_style
        } else {
            norm_style
        };
        let mut text_lines = vec![ratatui::text::Line::from(format!(
            "{} {}",
            label,
            t("prompt_renmov_to")
        ))];
        let mut input_spans = vec![ratatui::text::Span::styled(input.clone(), in_style)];
        if *cursor_idx == 0 {
            input_spans.push(ratatui::text::Span::styled(
                "_",
                Style::default().fg(Color::Cyan),
            ));
            if !input.is_empty() {
                let history = crate::fs::transfer::history::load_history();
                if let Some(suggestion) = history
                    .destinations
                    .iter()
                    .find(|d| d.to_lowercase().starts_with(&input.to_lowercase()))
                {
                    if suggestion.len() > input.len() {
                        let suffix = &suggestion[input.len()..];
                        input_spans.push(ratatui::text::Span::styled(
                            suffix.to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
        }
        text_lines.push(ratatui::text::Line::from(input_spans));
        f.render_widget(
            Paragraph::new(ratatui::text::Text::from(text_lines)),
            chunks[0],
        );

        f.render_widget(Paragraph::new(sep_str.clone()).style(sep_style), chunks[1]);

        let exist_opts = [
            t("opt_ask"),
            t("opt_overwrite"),
            t("opt_skip"),
            t("opt_append"),
        ];
        let exist_style = if *cursor_idx == 1 {
            act_style
        } else {
            norm_style
        };
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                t("prompt_already_existing"),
                exist_opts[*already_existing]
            ))
            .style(exist_style),
            chunks[2],
        );

        let check = |b: &bool| if *b { "[x]" } else { "[ ]" };
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(process_multiple),
                t("prompt_process_multiple")
            ))
            .style(if *cursor_idx == 2 {
                act_style
            } else {
                norm_style
            }),
            chunks[3],
        );
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(copy_access_mode),
                t("prompt_copy_files_access")
            ))
            .style(if *cursor_idx == 3 {
                act_style
            } else {
                norm_style
            }),
            chunks[4],
        );
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(copy_extended_attributes),
                t("prompt_copy_ext_attr")
            ))
            .style(if *cursor_idx == 4 {
                act_style
            } else {
                norm_style
            }),
            chunks[5],
        );
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(disable_write_cache),
                t("prompt_disable_write_cache")
            ))
            .style(if *cursor_idx == 5 {
                act_style
            } else {
                norm_style
            }),
            chunks[6],
        );
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(produce_sparse_files),
                t("prompt_produce_sparse_files")
            ))
            .style(if *cursor_idx == 6 {
                act_style
            } else {
                norm_style
            }),
            chunks[7],
        );
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                check(use_copy_on_write),
                t("prompt_use_cow")
            ))
            .style(if *cursor_idx == 7 {
                act_style
            } else {
                norm_style
            }),
            chunks[8],
        );

        let sym_opts = [
            t("opt_smartly_copy"),
            t("opt_copy_link"),
            t("opt_copy_target"),
        ];
        f.render_widget(
            Paragraph::new(format!(
                "{} {}",
                t("prompt_symlinks"),
                sym_opts[*symlink_mode]
            ))
            .style(if *cursor_idx == 8 {
                act_style
            } else {
                norm_style
            }),
            chunks[9],
        );

        f.render_widget(Paragraph::new(sep_str.clone()).style(sep_style), chunks[10]);

        let filter_display = if filter_mask.is_empty() {
            String::new()
        } else {
            format!(" [{}]", filter_mask)
        };
        f.render_widget(
            Paragraph::new(format!(
                "{} {}{}",
                check(use_filter),
                t("prompt_use_filter"),
                filter_display
            ))
            .style(if *cursor_idx == 9 {
                act_style
            } else {
                norm_style
            }),
            chunks[11],
        );

        f.render_widget(Paragraph::new(sep_str.clone()).style(sep_style), chunks[12]);

        let b1 = if *cursor_idx == 10 {
            act_style
        } else {
            norm_style
        };
        let b2 = if *cursor_idx == 11 {
            act_style
        } else {
            norm_style
        };
        let b3 = if *cursor_idx == 12 {
            act_style
        } else {
            norm_style
        };
        let b4 = if *cursor_idx == 13 {
            act_style
        } else {
            norm_style
        };

        let btns = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(t("btn_rename_bracket"), b1),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(t("btn_f10_tree"), b2),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(t("btn_filter"), b3),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(t("btn_cancel_bracket"), b4),
        ]);
        f.render_widget(
            Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
            chunks[13],
        );
        true
    } else {
        false
    }
}
