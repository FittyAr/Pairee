use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, BorderType, Clear, Gauge, List, ListItem, Paragraph, Row, Table};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferViewMode};
use crate::config::localization::t;
use crate::ui::popup::centered_rect;

pub fn render_transfer_panel(f: &mut Frame, state: &AppState, _context: &AppContext) {
    let transfer_state = match &state.transfer {
        Some(ts) => ts,
        None => return,
    };

    if transfer_state.view_mode != TransferViewMode::Expanded {
        return;
    }

    let progress_opt = transfer_state.current_progress.as_ref();

    let size = f.area();
    // Popup centrado: 80% ancho, 75% alto
    let popup_area = centered_rect(80, 75, size);
    
    // Clear para tapar los paneles debajo
    f.render_widget(Clear, popup_area);

    // Contenedor principal con borde redondeado
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", t("transfer_title")));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Layout principal: Cabecera, Pestañas, Contenido de Pestaña, Cola, Botones
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Cabecera (Velocidad, ETA, Gauge principal)
            Constraint::Length(3), // Pestañas (File List, Options, Status, Log)
            Constraint::Min(5),    // Contenido
            Constraint::Length(3), // Botones de Acción (Footer)
        ])
        .split(inner_area);

    // --- 1. CABECERA ---
    render_header(f, chunks[0], transfer_state, progress_opt);

    // --- 2. PESTAÑAS (TABS) ---
    render_tabs(f, chunks[1], transfer_state.active_tab);

    // --- 3. CONTENIDO ---
    match transfer_state.active_tab {
        TransferTab::FileList => render_file_list_tab(f, chunks[2], transfer_state),
        TransferTab::Options => render_options_tab(f, chunks[2], transfer_state),
        TransferTab::Status => render_status_tab(f, chunks[2], transfer_state, progress_opt),
        TransferTab::Log => render_log_tab(f, chunks[2], transfer_state),
        TransferTab::Queue => super::queue_view::render_queue_view(f, chunks[2], transfer_state),
    }

    // --- 4. FOOTER (ACCIONES) ---
    render_footer(f, chunks[3], transfer_state);
}

fn render_header(f: &mut Frame, area: Rect, ts: &crate::app::state::TransferUIState, prog: Option<&crate::fs::transfer::job::TransferProgress>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Texto: DJI_0427.MP4
            Constraint::Length(2), // Gauge + Speed/ETA info
        ])
        .split(area);

    let file_text = match prog {
        Some(p) => format!("Current File: {}", p.current_file),
        None => "No active transfer / Finished".to_string(),
    };
    f.render_widget(Paragraph::new(file_text).style(Style::default().fg(Color::White)), chunks[0]);

    let percent = prog.map(|p| p.percent_bytes() as u16).unwrap_or(0);
    let label = if prog.is_some() {
        format!("{}%", percent)
    } else {
        "Finished / Idle".to_string()
    };
    let speed_formatted = if prog.is_some() {
        bytesize::ByteSize(ts.speed_info.0 as u64).to_string()
    } else {
        "0 B".to_string()
    };
    let eta_text = match prog.and_then(|_| ts.speed_info.1) {
        Some(secs) => format!("ETA {}s", secs),
        None => "ETA --".to_string(),
    };

    let gauge_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(30),
        ])
        .split(chunks[1]);

    let gauge = Gauge::default()
        .percent(percent)
        .label(label)
        .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    f.render_widget(gauge, gauge_chunk[0]);

    let info_text = format!(" {}/s | {} ", speed_formatted, eta_text);
    f.render_widget(Paragraph::new(info_text).style(Style::default().fg(Color::Yellow)), gauge_chunk[1]);
}

fn render_tabs(f: &mut Frame, area: Rect, active_tab: TransferTab) {
    let tab_titles = vec!["[1] File List", "[2] Options", "[3] Status", "[4] Log", "[5] Queue"];
    let tab_area = Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray));
    let inner_area = tab_area.inner(area);
    f.render_widget(tab_area, area);

    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(inner_area);

    for (idx, title) in tab_titles.into_iter().enumerate() {
        let is_active = idx == active_tab as usize;
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let p = Paragraph::new(format!("  {}  ", title))
            .block(Block::default())
            .style(style);
        f.render_widget(p, tab_chunks[idx]);
    }
}

fn render_file_list_tab(f: &mut Frame, area: Rect, ts: &crate::app::state::TransferUIState) {
    let total_files = if let Some(ref res) = ts.current_results {
        res.failed_files.len() + res.skipped_files.len() + res.completed_files.len()
    } else {
        0
    };

    if total_files == 0 {
        let empty_p = Paragraph::new("\n No files transferred yet.")
            .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC));
        f.render_widget(empty_p, area);
        return;
    }

    let height = area.height.saturating_sub(3) as usize;
    let cursor = ts.file_list_cursor;

    let start = if cursor > height / 2 {
        cursor.saturating_sub(height / 2)
    } else {
        0
    };
    let start = if start + height > total_files {
        total_files.saturating_sub(height)
    } else {
        start
    };
    let end = start + height.min(total_files.saturating_sub(start));

    let mut rows = Vec::new();
    if let Some(ref res) = ts.current_results {
        let f_len = res.failed_files.len();
        let s_len = res.skipped_files.len();

        for i in start..end {
            let is_selected = i == cursor;
            let mut style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            if i < f_len {
                let f = &res.failed_files[i];
                if !is_selected {
                    style = style.fg(Color::Red);
                }
                rows.push(Row::new(vec![
                    " ✗ FAIL ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    "-".to_string(),
                    f.error.clone(),
                ]).style(style));
            } else if i < f_len + s_len {
                let f = &res.skipped_files[i - f_len];
                if !is_selected {
                    style = style.fg(Color::Yellow);
                }
                rows.push(Row::new(vec![
                    " ⚠ SKIP ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    "-".to_string(),
                    f.reason.clone(),
                ]).style(style));
            } else {
                let f = &res.completed_files[i - f_len - s_len];
                if !is_selected {
                    style = style.fg(Color::Green);
                }
                let src_hash = f.src_hash.as_deref().unwrap_or("-");
                let dst_hash = f.dst_hash.as_deref().unwrap_or("-");
                let hash_text = format!("{} : {}", &src_hash[..src_hash.len().min(4)], &dst_hash[..dst_hash.len().min(4)]);

                rows.push(Row::new(vec![
                    " ✓ OK ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    bytesize::ByteSize(f.size).to_string(),
                    hash_text,
                ]).style(style));
            }
        }
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(10), // Estado
            Constraint::Min(30),    // Archivo
            Constraint::Length(12), // Tamaño
            Constraint::Length(15), // Hash (Src:Dst)
        ]
    )
    .header(Row::new(vec!["Status", "File Path", "Size", "Hashes"]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(cursor.saturating_sub(start)));

    f.render_stateful_widget(table, area, &mut table_state);
}

fn render_options_tab(f: &mut Frame, area: Rect, ts: &crate::app::state::TransferUIState) {
    let options = &ts.engine.queue.get_active().map(|j| j.options.clone()).unwrap_or_default();
    
    let opt_labels = vec![
        format!("Direct I/O (bypass cache): {}", if options.direct_io { "Yes" } else { "No" }),
        format!("Verify integrity after transfer: {}", if options.verify_after_copy { "Yes" } else { "No" }),
        format!("Preserve timestamps (created, modified): {}", if options.preserve_timestamps { "Yes" } else { "No" }),
        format!("Preserve attributes and permissions: {}", if options.preserve_attributes { "Yes" } else { "No" }),
        format!("Post-Action (On Finish): {:?}", ts.post_action),
        format!("Buffer size: {}", bytesize::ByteSize(options.buffer_size.to_bytes() as u64).to_string()),
        format!("Hash algorithm: {}", options.hash_algorithm.as_str()),
        format!("Preserve Security / ACLs: {}", if options.preserve_acl { "Yes" } else { "No" }),
        format!("Preserve Alternate Data Streams: {}", if options.preserve_streams { "Yes" } else { "No" }),
        format!("Skip symbolic links: {}", if options.skip_symlinks { "Yes" } else { "No" }),
        format!("Follow symbolic links: {}", if options.follow_symlinks { "Yes" } else { "No" }),
        format!("Limit bandwidth: {}", if let Some(rate) = options.limit_bandwidth_rate { format!("{} /s", bytesize::ByteSize(rate)) } else { "No limit".to_string() }),
    ];

    let mut lines = Vec::new();
    lines.push(ratatui::text::Line::from("")); // Margen superior
    for (idx, label) in opt_labels.iter().enumerate() {
        let is_selected = idx == ts.options_cursor;
        if is_selected {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("  ▶  {}  ", label),
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            )));
        } else {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("     {}  ", label),
                Style::default().fg(Color::Gray)
            )));
        }
        lines.push(ratatui::text::Line::from("")); // Espaciado entre items
    }

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Active Transfer Settings (Use Up/Down + Enter to toggle) ").border_type(BorderType::Rounded));
    f.render_widget(p, area);
}

fn render_status_tab(f: &mut Frame, area: Rect, ts: &crate::app::state::TransferUIState, prog: Option<&crate::fs::transfer::job::TransferProgress>) {
    let (files_total, files_completed, files_failed, files_skipped, bytes_total, bytes_transferred, speed, eta) = match prog {
        Some(p) => {
            (
                p.files_total,
                p.files_completed,
                p.files_failed,
                p.files_skipped,
                bytesize::ByteSize(p.bytes_total).to_string(),
                bytesize::ByteSize(p.bytes_transferred).to_string(),
                format!("{}/s", bytesize::ByteSize(ts.speed_info.0 as u64).to_string()),
                match ts.speed_info.1 {
                    Some(secs) => format!("{} seconds", secs),
                    None => "Calculating...".to_string(),
                }
            )
        }
        None => {
            if let Some(ref res) = ts.current_results {
                let completed = res.completed_files.len();
                let failed = res.failed_files.len();
                let skipped = res.skipped_files.len();
                let total = completed + failed + skipped;
                let bytes: u64 = res.completed_files.iter().map(|f| f.size).sum();
                (
                    total,
                    completed,
                    failed,
                    skipped,
                    bytesize::ByteSize(bytes).to_string(),
                    bytesize::ByteSize(bytes).to_string(),
                    "0 B/s".to_string(),
                    "Finished".to_string(),
                )
            } else {
                (0, 0, 0, 0, "0 B".to_string(), "0 B".to_string(), "0 B/s".to_string(), "Idle".to_string())
            }
        }
    };

    let text = format!(
        r#"  - Total Files: {}
  - Files Completed: {}
  - Files Failed: {}
  - Files Skipped: {}
  
  - Total Size: {}
  - Bytes Copied: {}
  - Current Speed: {}
  - Estimated Time (ETA): {}"#,
        files_total,
        files_completed,
        files_failed,
        files_skipped,
        bytes_total,
        bytes_transferred,
        speed,
        eta
    );

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Statistics ").border_type(BorderType::Rounded))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(p, area);
}

fn render_log_tab(f: &mut Frame, area: Rect, ts: &crate::app::state::TransferUIState) {
    let items: Vec<ListItem> = ts.log_lines
        .iter()
        .rev() // Mostrar logs más nuevos arriba
        .take(15)
        .map(|line| ListItem::new(line.as_str()).style(Style::default().fg(Color::Gray)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Event Log ").border_type(BorderType::Rounded));
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect, _ts: &crate::app::state::TransferUIState) {
    let footer_text = " [p] Pause/Resume  [s] Skip File  [x] Cancel Job  [Del] Remove Job  [Esc] Minimize ";
    let p = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(p, area);
}
