use super::centered_rect;
use crate::app::state::{LinkKind, PopupType, SelectMode};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph},
};

pub fn render_prompt_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::Help {
            mode,
            docs,
            cursor_idx,
            scroll_y,
            active_content,
        } => {
            let area = centered_rect(80, 80, size);
            f.render_widget(Clear, area);

            use ratatui::text::Span;

            if *mode == 0 {
                let title = format!(" {} ", t("prompt_help_title").trim());
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .title(title)
                    .style(Style::default().bg(parse_color(&theme.popup_bg)));

                let mut list_items = Vec::new();
                for (i, (doc_title, _)) in docs.iter().enumerate() {
                    let style = if i == *cursor_idx {
                        Style::default().bg(Color::Cyan).fg(Color::Black)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                        Span::styled(format!("  {}  ", doc_title), style),
                    ])));
                }

                let list = List::new(list_items)
                    .block(block)
                    .style(Style::default().bg(parse_color(&theme.popup_bg)));

                f.render_widget(list, area);

                let hint_area = Rect {
                    x: area.x + 2,
                    y: area.y + area.height - 2,
                    width: area.width.saturating_sub(4),
                    height: 1,
                };
                let hint_text = " [Up/Down] Navigate  [Enter] Open Document  [Esc] Close ";
                f.render_widget(
                    Paragraph::new(hint_text)
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(Style::default().fg(Color::DarkGray)),
                    hint_area,
                );
            } else if let Some(content) = active_content {
                let doc_title = docs
                    .get(*cursor_idx)
                    .map(|(t, _)| t.as_str())
                    .unwrap_or(" Documentation ");
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .title(format!(" {} ", doc_title))
                    .style(Style::default().bg(parse_color(&theme.popup_bg)));

                let parsed_lines = parse_markdown_to_lines(content);

                let paragraph = Paragraph::new(parsed_lines)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .scroll((*scroll_y as u16, 0))
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));

                f.render_widget(paragraph, area);

                let hint_area = Rect {
                    x: area.x + 2,
                    y: area.y + area.height - 2,
                    width: area.width.saturating_sub(4),
                    height: 1,
                };
                let hint_text = " [Up/Down/PgUp/PgDn] Scroll  [Esc/Backspace] Back to Menu ";
                f.render_widget(
                    Paragraph::new(hint_text)
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(Style::default().fg(Color::DarkGray)),
                    hint_area,
                );
            }
            true
        }
        PopupType::MkDirPrompt {
            input,
            cursor_idx,
            process_multiple,
        } => {
            let area = centered_rect(50, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("prompt_mkdir_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Length(2),
                ])
                .split(inner);

            let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

            let input_style = if *cursor_idx == 0 {
                active_style
            } else {
                normal_style
            };
            let display_input = if *cursor_idx == 0 {
                format!("{}_", input)
            } else {
                input.clone()
            };
            f.render_widget(
                Paragraph::new(format!("{}\n > {}", t("prompt_mkdir_to"), display_input))
                    .style(input_style),
                chunks[0],
            );

            let chk = if *process_multiple { "[x]" } else { "[ ]" };
            let multi_style = if *cursor_idx == 1 {
                active_style
            } else {
                normal_style
            };
            f.render_widget(
                Paragraph::new(format!("{} {}", chk, t("prompt_process_multiple_names")))
                    .style(multi_style),
                chunks[1],
            );

            let btn1 = if *cursor_idx == 2 {
                active_style
            } else {
                normal_style
            };
            let btn2 = if *cursor_idx == 3 {
                active_style
            } else {
                normal_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_ok_bracket"), btn1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[2],
            );
            true
        }
        PopupType::CopyPrompt {
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
        } => {
            let area = centered_rect(75, 45, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_copy_title"))
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
                t("prompt_copy_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_copy_plur").replacen("{}", &count.to_string(), 1)
            };
            let in_style = if *cursor_idx == 0 {
                act_style
            } else {
                norm_style
            };
            let display_input = if *cursor_idx == 0 {
                format!("{}_", input)
            } else {
                input.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{} {}\n{}",
                    label,
                    t("prompt_copy_to"),
                    display_input
                ))
                .style(in_style),
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
                ratatui::text::Span::styled(t("btn_copy_bracket"), b1),
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
        }
        PopupType::ConfirmQuit => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_exit_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_exit_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmInterrupt => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_abort_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_abort_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmOverwrite {
            src_paths,
            dest_dir,
            is_move,
            input,
        } => {
            let area = centered_rect(60, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_overwrite_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let op_name = if *is_move { t("op_move") } else { t("op_copy") };
            let first_name = src_paths
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let target_desc = if src_paths.len() == 1 {
                if let Some(inp) = input {
                    inp.clone()
                } else {
                    first_name
                }
            } else {
                t("prompt_files_count").replacen("{}", &src_paths.len().to_string(), 1)
            };

            let text = t("prompt_overwrite_text")
                .replacen("{}", &dest_dir.to_string_lossy(), 1)
                .replacen("{}", &target_desc, 1)
                .replacen("{}", &op_name, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmReload { .. } => {
            let area = centered_rect(50, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_reload_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_reload_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmClearHistory { history_type } => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_clear_history_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let hist_type_translated = match history_type.as_str() {
                "command" => t("history_type_command"),
                "view" => t("history_type_view"),
                "folder" => t("history_type_folder"),
                _ => history_type.clone(),
            };

            let text = t("prompt_clear_history_text").replacen("{}", &hist_type_translated, 1);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmDelete { paths, cursor_idx } => {
            let area = centered_rect(50, 24, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_delete_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Length(2)])
                .split(inner);

            let mut folders_count = 0;
            let mut files_count = 0;
            for p in paths {
                if p.is_dir() && !p.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                    folders_count += 1;
                } else {
                    files_count += 1;
                }
            }

            let part_files = if files_count > 0 {
                Some(t("delete_confirm_files").replacen("{}", &files_count.to_string(), 1))
            } else {
                None
            };

            let part_folders = if folders_count > 0 {
                Some(t("delete_confirm_folders").replacen("{}", &folders_count.to_string(), 1))
            } else {
                None
            };

            let target_desc = match (part_files, part_folders) {
                (Some(f), Some(d)) => format!("{}{}{}", f, t("delete_confirm_conjunction"), d),
                (Some(f), None) => f,
                (None, Some(d)) => d,
                (None, None) => "0 items".to_string(),
            };

            let text = t("delete_confirm_msg").replacen("{}", &target_desc, 1);

            f.render_widget(
                Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .style(Style::default().fg(parse_color(&theme.popup_fg))),
                chunks[0],
            );

            let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let normal_style = Style::default().fg(parse_color(&theme.popup_fg));
            let btn1 = if *cursor_idx == 0 {
                active_style
            } else {
                normal_style
            };
            let btn2 = if *cursor_idx == 1 {
                active_style
            } else {
                normal_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_delete_bracket"), btn1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[1],
            );
            true
        }
        PopupType::CopyProgress {
            current_file,
            files_copied,
            total_files,
            bytes_copied,
            total_bytes,
        } => {
            let area = centered_rect(55, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("progress_copy_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let percent = bytes_copied
                .checked_mul(100)
                .and_then(|v| v.checked_div(*total_bytes))
                .unwrap_or(0) as u16;

            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // File labels
                    Constraint::Length(3), // Progress bar
                    Constraint::Min(1),    // Size counts
                ])
                .split(block.inner(area));

            let file_label = t("progress_file_label").replacen("{}", current_file, 1);
            let paragraph =
                Paragraph::new(file_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, inner_chunks[1]);

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                .percent(percent.min(100))
                .label(format!("{}%", percent.min(100)));
            f.render_widget(gauge, inner_chunks[2]);

            let size_label = t("progress_size_label")
                .replacen("{}", &files_copied.to_string(), 1)
                .replacen("{}", &total_files.to_string(), 1)
                .replacen("{}", &(*bytes_copied / (1024 * 1024)).to_string(), 1)
                .replacen("{}", &(*total_bytes / (1024 * 1024)).to_string(), 1);
            let size_paragraph =
                Paragraph::new(size_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(size_paragraph, inner_chunks[3]);

            f.render_widget(block, area);
            true
        }
        PopupType::Error(message) => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_error_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n{}", message, t("prompt_dismiss_hint"));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::LightRed));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::Info(message) => {
            let area = centered_rect(55, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_info_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n{}", message, t("prompt_dismiss_hint"));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::RenMovPrompt {
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
        } => {
            let area = centered_rect(75, 45, size);
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
            let display_input = if *cursor_idx == 0 {
                format!("{}_", input)
            } else {
                input.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{} {}\n{}",
                    label,
                    t("prompt_renmov_to"),
                    display_input
                ))
                .style(in_style),
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
                ratatui::text::Span::styled(format!(" {{ {} }} ", t("btn_rename")), b1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(format!(" [ {} ] ", t("btn_f10_tree")), b2),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(format!(" [ {} ] ", t("btn_filter")), b3),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(format!(" [ {} ] ", t("btn_cancel")), b4),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[13],
            );

            true
        }
        PopupType::CompressPrompt {
            input,
            targets,
            dest_dir,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_compress_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let count = targets.len();
            let first_name = targets
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let src_label = if count == 1 {
                t("prompt_compress_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_compress_plur").replacen("{}", &count.to_string(), 1)
            };

            let text = format!(
                "\n {}\n {}\n\n > {}.zip\n\n {}",
                src_label,
                t("prompt_copy_dest").replacen("{}", &dest_dir.to_string_lossy(), 1),
                input,
                t("prompt_confirm_cancel_hint")
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ApplyCommandPrompt { input, targets } => {
            let area = centered_rect(65, 35, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_apply_cmd_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let first_targets = targets
                .iter()
                .take(3)
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join(", ");
            let files_label = if targets.len() > 3 {
                t("prompt_apply_cmd_plur")
                    .replacen("{}", &targets.len().to_string(), 1)
                    .replacen("{}", &first_targets, 1)
            } else {
                t("prompt_apply_cmd_sing").replacen("{}", &first_targets, 1)
            };

            let text = t("prompt_apply_cmd_text")
                .replacen("{}", &files_label, 1)
                .replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::DescribeFilePrompt {
            path,
            current_desc,
            input,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_description_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let text = t("prompt_describe_text")
                .replacen("{}", &file_name, 1)
                .replacen("{}", current_desc, 1)
                .replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::SelectGroupPrompt { mode, query } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let title = match mode {
                SelectMode::Add => t("prompt_select_group_title"),
                SelectMode::Remove => t("prompt_unselect_group_title"),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let prompt_label = match mode {
                SelectMode::Add => t("prompt_select_group_pat"),
                SelectMode::Remove => t("prompt_unselect_group_pat"),
            };

            let text = format!(
                "\n {}\n\n > {}\n\n {}",
                prompt_label,
                query,
                t("prompt_confirm_cancel_hint")
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::CreateLinkPrompt {
            src,
            dest_input,
            kind,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let title = match kind {
                LinkKind::Symbolic => t("prompt_symlink_title"),
                LinkKind::Hard => t("prompt_hardlink_title"),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let src_name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let text = t("prompt_link_text")
                .replacen("{}", &src_name, 1)
                .replacen("{}", dest_input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::FilePanelFilterPrompt { input } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_filter_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_filter_text").replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::CopyMoveFilterPrompt { input, previous: _ } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_filter_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_filter_text").replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::WipeConfirm { paths } => {
            let area = centered_rect(55, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_wipe_warn_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_wipe_warn_text").replacen("{}", &paths.len().to_string(), 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::LightRed));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::SaveSetupConfirm => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(t("prompt_save_setup_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_save_setup_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        _ => false,
    }
}

fn parse_markdown_to_lines(text: &str) -> Vec<ratatui::text::Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let parser = Parser::new(text);
    let mut lines = Vec::new();
    let mut current_spans = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let code = false;
    let mut link = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    let prefix = match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        _ => "#### ",
                    };
                    current_spans.push(Span::styled(
                        prefix,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Tag::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Link { .. } => link = true,
                Tag::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    current_spans.push(Span::styled("• ", Style::default().fg(Color::Cyan)));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if !current_spans.is_empty() {
                        for span in &mut current_spans {
                            span.style = span.style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                        }
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Link => link = false,
                TagEnd::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                let mut style = Style::default();
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if code {
                    style = style.fg(Color::Magenta);
                } else if link {
                    style = style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
                } else {
                    style = style.fg(Color::White);
                }
                current_spans.push(Span::styled(t.into_string(), style));
            }
            Event::Code(c) => {
                current_spans.push(Span::styled(
                    format!(" `{}` ", c),
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
    }

    lines
}
