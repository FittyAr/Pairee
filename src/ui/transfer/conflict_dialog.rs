use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::state::TransferUIState;
use crate::config::localization::t;
use crate::ui::panel::helpers::format_date;

pub fn render_conflict_dialog(f: &mut Frame, area: Rect, ts: &TransferUIState) {
    let (_, file_path, conflict) = match &ts.active_conflict_info {
        Some(c) => c,
        None => return,
    };

    // Crear un área centrada para el diálogo de conflicto
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(16),
            Constraint::Percentage(20),
        ])
        .split(area);

    let center_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(60),
            Constraint::Percentage(20),
        ])
        .split(popup_layout[1]);

    let dialog_area = center_row[1];

    // Limpiar el fondo
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(t("transfer_conflict_title"))
        .border_style(Style::default().fg(Color::LightRed))
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Cabecera/Instrucción
            Constraint::Length(6), // Comparación de Archivos
            Constraint::Length(6), // Opciones/Teclas
        ])
        .split(inner_area);

    // 1. Instrucción
    let title_text = t("transfer_conflict_exists").replace(
        "{}",
        &file_path.file_name().unwrap_or_default().to_string_lossy(),
    );
    f.render_widget(
        Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true }),
        content_chunks[0],
    );

    // 2. Comparación
    let src_size_str = bytesize::ByteSize(conflict.src_size).to_string();
    let dst_size_str = bytesize::ByteSize(conflict.dst_size).to_string();

    let time = |time: Option<std::time::SystemTime>| match time {
        Some(_) => format_date(time),
        None => t("transfer_unknown"),
    };
    let comparison_text = t("transfer_conflict_compare")
        .replace("{src}", &conflict.src_path.to_string_lossy())
        .replace("{src_size}", &src_size_str)
        .replace("{src_time}", &time(conflict.src_modified))
        .replace("{dst}", &conflict.dst_path.to_string_lossy())
        .replace("{dst_size}", &dst_size_str)
        .replace("{dst_time}", &time(conflict.dst_modified));

    f.render_widget(
        Paragraph::new(comparison_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true }),
        content_chunks[1],
    );

    // 3. Opciones de resolución
    let options_text = t("transfer_conflict_keys");
    f.render_widget(
        Paragraph::new(options_text)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true }),
        content_chunks[2],
    );
}
