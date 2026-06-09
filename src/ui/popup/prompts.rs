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
        PopupType::MkDirPrompt { input } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("prompt_mkdir_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_mkdir_text").replacen("{}", input, 1);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::CopyPrompt {
            input,
            src_paths,
            dest_dir,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_copy_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let count = src_paths.len();
            let first_name = src_paths
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let src_label = if count == 1 {
                t("prompt_copy_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_copy_plur").replacen("{}", &count.to_string(), 1)
            };

            let text = format!(
                "\n {}\n {}\n\n > {}\n\n {}",
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
        PopupType::ConfirmDelete { paths } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_delete_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_delete_text").replacen("{}", &paths.len().to_string(), 1);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
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
            dest_dir,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_renmov_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let count = src_paths.len();
            let first_name = src_paths
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let src_label = if count == 1 {
                t("prompt_move_sing").replacen("{}", &first_name, 1)
            } else {
                t("prompt_move_plur").replacen("{}", &count.to_string(), 1)
            };

            let text = format!(
                "\n {}\n {}\n\n > {}\n\n {}",
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
