use super::centered_rect;
use crate::app::state::{LinkKind, PopupType, SelectMode};
use crate::ui::theme_apply::parse_color;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table, Cell},
};

pub fn render_prompt_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::Help => {
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area);

            let title = format!(" {} ", t("prompt_help_title").trim());
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let help_rows = vec![
                Row::new(vec![Cell::from(t("key_tab")), Cell::from(t("desc_tab"))]),
                Row::new(vec![Cell::from(t("key_insert")), Cell::from(t("desc_insert"))]),
                Row::new(vec![Cell::from(t("key_f3")), Cell::from(t("desc_f3"))]),
                Row::new(vec![Cell::from(t("key_f4")), Cell::from(t("desc_f4"))]),
                Row::new(vec![Cell::from(t("key_f5")), Cell::from(t("desc_f5"))]),
                Row::new(vec![Cell::from(t("key_f6")), Cell::from(t("desc_f6"))]),
                Row::new(vec![Cell::from(t("key_f7")), Cell::from(t("desc_f7"))]),
                Row::new(vec![Cell::from(t("key_f8")), Cell::from(t("desc_f8"))]),
                Row::new(vec![Cell::from(t("key_ctrl_h")), Cell::from(t("desc_ctrl_h"))]),
                Row::new(vec![Cell::from(t("key_ctrl_u")), Cell::from(t("desc_ctrl_u"))]),
                Row::new(vec![Cell::from(t("key_f10")), Cell::from(t("desc_f10"))]),
                Row::new(vec![Cell::from(t("key_esc")), Cell::from(t("desc_esc"))]),
            ];

            let table = Table::new(
                help_rows,
                [Constraint::Percentage(40), Constraint::Percentage(60)],
            )
            .block(block)
            .header(
                Row::new(vec![Cell::from(t("col_key")), Cell::from(t("col_description"))])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            );

            f.render_widget(table, area);
            true
        }
        PopupType::MkDirPrompt { input, cursor_idx, process_multiple } => {
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
                .constraints([Constraint::Length(3), Constraint::Length(2), Constraint::Length(2)])
                .split(inner);

            let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

            let input_style = if *cursor_idx == 0 { active_style } else { normal_style };
            let display_input = if *cursor_idx == 0 { format!("{}_", input) } else { input.clone() };
            f.render_widget(Paragraph::new(format!("{}\n > {}", t("prompt_mkdir_to"), display_input)).style(input_style), chunks[0]);

            let chk = if *process_multiple { "[x]" } else { "[ ]" };
            let multi_style = if *cursor_idx == 1 { active_style } else { normal_style };
            f.render_widget(Paragraph::new(format!("{} {}", chk, t("prompt_process_multiple_names"))).style(multi_style), chunks[1]);

            let btn1 = if *cursor_idx == 2 { active_style } else { normal_style };
            let btn2 = if *cursor_idx == 3 { active_style } else { normal_style };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_ok_bracket"), btn1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
            ]);
            f.render_widget(Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center), chunks[2]);
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
            let area = centered_rect(65, 45, size);
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
                    Constraint::Length(3), // Input
                    Constraint::Length(1), // Already existing
                    Constraint::Length(1), // Process multiple
                    Constraint::Length(1), // Copy access mode
                    Constraint::Length(1), // Extended attrs
                    Constraint::Length(1), // Disable write cache
                    Constraint::Length(1), // Sparse files
                    Constraint::Length(1), // COW
                    Constraint::Length(1), // Symlinks
                    Constraint::Length(2), // Filter
                    Constraint::Length(2), // Buttons
                ])
                .split(inner);

            let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            let count = src_paths.len();
            let first_name = src_paths.first().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let label = if count == 1 {
                t("prompt_copy_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_copy_plur").replacen("{}", &count.to_string(), 1)
            };
            let in_style = if *cursor_idx == 0 { act_style } else { norm_style };
            let display_input = if *cursor_idx == 0 { format!("{}_", input) } else { input.clone() };
            f.render_widget(Paragraph::new(format!("{} {}\n > {}", label, t("prompt_copy_to"), display_input)).style(in_style), chunks[0]);

            let exist_opts = [t("opt_ask"), t("opt_overwrite"), t("opt_skip"), t("opt_append")];
            let exist_style = if *cursor_idx == 1 { act_style } else { norm_style };
            f.render_widget(Paragraph::new(format!("{} {}", t("prompt_already_existing"), exist_opts[*already_existing])).style(exist_style), chunks[1]);

            let check = |b: &bool| if *b { "[x]" } else { "[ ]" };
            f.render_widget(Paragraph::new(format!("{} {}", check(process_multiple), t("prompt_process_multiple"))).style(if *cursor_idx == 2 { act_style } else { norm_style }), chunks[2]);
            f.render_widget(Paragraph::new(format!("{} {}", check(copy_access_mode), t("prompt_copy_files_access"))).style(if *cursor_idx == 3 { act_style } else { norm_style }), chunks[3]);
            f.render_widget(Paragraph::new(format!("{} {}", check(copy_extended_attributes), t("prompt_copy_ext_attr"))).style(if *cursor_idx == 4 { act_style } else { norm_style }), chunks[4]);
            f.render_widget(Paragraph::new(format!("{} {}", check(disable_write_cache), t("prompt_disable_write_cache"))).style(if *cursor_idx == 5 { act_style } else { norm_style }), chunks[5]);
            f.render_widget(Paragraph::new(format!("{} {}", check(produce_sparse_files), t("prompt_produce_sparse_files"))).style(if *cursor_idx == 6 { act_style } else { norm_style }), chunks[6]);
            f.render_widget(Paragraph::new(format!("{} {}", check(use_copy_on_write), t("prompt_use_cow"))).style(if *cursor_idx == 7 { act_style } else { norm_style }), chunks[7]);

            let sym_opts = [t("opt_smartly_copy"), t("opt_copy_link"), t("opt_copy_target")];
            f.render_widget(Paragraph::new(format!("{}          {}", t("prompt_symlinks"), sym_opts[*symlink_mode])).style(if *cursor_idx == 8 { act_style } else { norm_style }), chunks[8]);
            let filter_display = if filter_mask.is_empty() { String::new() } else { format!(" [{}]", filter_mask) };
            f.render_widget(Paragraph::new(format!("{} {}{}", check(use_filter), t("prompt_use_filter"), filter_display)).style(if *cursor_idx == 9 { act_style } else { norm_style }), chunks[9]);

            let b1 = if *cursor_idx == 10 { act_style } else { norm_style };
            let b2 = if *cursor_idx == 11 { act_style } else { norm_style };
            let b3 = if *cursor_idx == 12 { act_style } else { norm_style };
            let b4 = if *cursor_idx == 13 { act_style } else { norm_style };

            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_copy_bracket"), b1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_f10_tree"), b2),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_filter"), b3),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b4),
            ]);
            f.render_widget(Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center), chunks[10]);

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
            let area = centered_rect(45, 20, size);
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
                .constraints([Constraint::Length(3), Constraint::Length(2)])
                .split(inner);

            let text = format!("{}\n{} {}", t("prompt_delete_confirm"), paths.len(), t("label_files"));
            f.render_widget(Paragraph::new(text).alignment(ratatui::layout::Alignment::Center).style(Style::default().fg(parse_color(&theme.popup_fg))), chunks[0]);

            let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let normal_style = Style::default().fg(parse_color(&theme.popup_fg));
            let btn1 = if *cursor_idx == 0 { active_style } else { normal_style };
            let btn2 = if *cursor_idx == 1 { active_style } else { normal_style };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_delete_bracket"), btn1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
            ]);
            f.render_widget(Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center), chunks[1]);
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
            let area = centered_rect(65, 45, size);
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
                    Constraint::Length(3), // Input
                    Constraint::Length(1), // Already existing
                    Constraint::Length(1), // Process multiple
                    Constraint::Length(1), // Copy access mode
                    Constraint::Length(1), // Extended attrs
                    Constraint::Length(1), // Disable write cache
                    Constraint::Length(1), // Sparse files
                    Constraint::Length(1), // COW
                    Constraint::Length(1), // Symlinks
                    Constraint::Length(2), // Filter
                    Constraint::Length(2), // Buttons
                ])
                .split(inner);

            let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            let count = src_paths.len();
            let first_name = src_paths.first().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let label = if count == 1 {
                t("prompt_move_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_move_plur").replacen("{}", &count.to_string(), 1)
            };
            let in_style = if *cursor_idx == 0 { act_style } else { norm_style };
            let display_input = if *cursor_idx == 0 { format!("{}_", input) } else { input.clone() };
            f.render_widget(Paragraph::new(format!("{} {}\n > {}", label, t("prompt_renmov_to"), display_input)).style(in_style), chunks[0]);

            let exist_opts = [t("opt_ask"), t("opt_overwrite"), t("opt_skip"), t("opt_append")];
            let exist_style = if *cursor_idx == 1 { act_style } else { norm_style };
            f.render_widget(Paragraph::new(format!("{} {}", t("prompt_already_existing"), exist_opts[*already_existing])).style(exist_style), chunks[1]);

            let check = |b: &bool| if *b { "[x]" } else { "[ ]" };
            f.render_widget(Paragraph::new(format!("{} {}", check(process_multiple), t("prompt_process_multiple"))).style(if *cursor_idx == 2 { act_style } else { norm_style }), chunks[2]);
            f.render_widget(Paragraph::new(format!("{} {}", check(copy_access_mode), t("prompt_copy_files_access"))).style(if *cursor_idx == 3 { act_style } else { norm_style }), chunks[3]);
            f.render_widget(Paragraph::new(format!("{} {}", check(copy_extended_attributes), t("prompt_copy_ext_attr"))).style(if *cursor_idx == 4 { act_style } else { norm_style }), chunks[4]);
            f.render_widget(Paragraph::new(format!("{} {}", check(disable_write_cache), t("prompt_disable_write_cache"))).style(if *cursor_idx == 5 { act_style } else { norm_style }), chunks[5]);
            f.render_widget(Paragraph::new(format!("{} {}", check(produce_sparse_files), t("prompt_produce_sparse_files"))).style(if *cursor_idx == 6 { act_style } else { norm_style }), chunks[6]);
            f.render_widget(Paragraph::new(format!("{} {}", check(use_copy_on_write), t("prompt_use_cow"))).style(if *cursor_idx == 7 { act_style } else { norm_style }), chunks[7]);

            let sym_opts = [t("opt_smartly_copy"), t("opt_copy_link"), t("opt_copy_target")];
            f.render_widget(Paragraph::new(format!("{}          {}", t("prompt_symlinks"), sym_opts[*symlink_mode])).style(if *cursor_idx == 8 { act_style } else { norm_style }), chunks[8]);
            let filter_display = if filter_mask.is_empty() { String::new() } else { format!(" [{}]", filter_mask) };
            f.render_widget(Paragraph::new(format!("{} {}{}", check(use_filter), t("prompt_use_filter"), filter_display)).style(if *cursor_idx == 9 { act_style } else { norm_style }), chunks[9]);

            let b1 = if *cursor_idx == 10 { act_style } else { norm_style };
            let b2 = if *cursor_idx == 11 { act_style } else { norm_style };
            let b3 = if *cursor_idx == 12 { act_style } else { norm_style };
            let b4 = if *cursor_idx == 13 { act_style } else { norm_style };

            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_rename_bracket"), b1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_f10_tree"), b2),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_filter"), b3),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b4),
            ]);
            f.render_widget(Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center), chunks[10]);

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
                prompt_label, query, t("prompt_confirm_cancel_hint")
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
