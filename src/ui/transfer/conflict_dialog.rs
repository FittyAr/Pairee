use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::app::state::TransferUIState;

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
        .title(" Conflict Detected ")
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
    let title_text = format!("File already exists in destination:\n{}", file_path.file_name().unwrap_or_default().to_string_lossy());
    f.render_widget(
        Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: true }),
        content_chunks[0]
    );

    // 2. Comparación
    let src_size_str = bytesize::ByteSize(conflict.src_size).to_string();
    let dst_size_str = bytesize::ByteSize(conflict.dst_size).to_string();
    
    let src_time_str = conflict.src_modified
        .map(|t| format!("{:?}", t))
        .unwrap_or_else(|| "Unknown".to_string());
    let dst_time_str = conflict.dst_modified
        .map(|t| format!("{:?}", t))
        .unwrap_or_else(|| "Unknown".to_string());

    let comparison_text = format!(
        "Source Path: {}\n  Size: {} | Modified: {}\nDestination Path: {}\n  Size: {} | Modified: {}",
        conflict.src_path.to_string_lossy(),
        src_size_str, src_time_str,
        conflict.dst_path.to_string_lossy(),
        dst_size_str, dst_time_str
    );

    f.render_widget(
        Paragraph::new(comparison_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true }),
        content_chunks[1]
    );

    // 3. Opciones de resolución
    let options_text = "\n\
        [o] Overwrite        [a] Overwrite Older        [s] Skip        [r] Rename Both\n\
        [O] Overwrite All    [A] Overwrite All Older    [S] Skip All    [R] Rename All\n\
        [x] Cancel Job";
    f.render_widget(
        Paragraph::new(options_text)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: true }),
        content_chunks[2]
    );
}
