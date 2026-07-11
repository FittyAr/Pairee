use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, BorderType, Gauge, Paragraph};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferViewMode};
use crate::config::localization::t;

pub fn render_transfer_bar(f: &mut Frame, area: Rect, state: &AppState, _context: &AppContext) {
    let transfer_state = match &state.transfer {
        Some(ts) => ts,
        None => return,
    };

    if transfer_state.view_mode != TransferViewMode::Minimized {
        return;
    }

    let jobs = transfer_state.engine.queue.get_all();
    let active_job = jobs.iter().find(|j| {
        matches!(
            j.status,
            crate::fs::transfer::job::TransferJobStatus::Scanning
                | crate::fs::transfer::job::TransferJobStatus::Transferring
                | crate::fs::transfer::job::TransferJobStatus::Verifying
                | crate::fs::transfer::job::TransferJobStatus::Paused
        )
    }).or_else(|| jobs.first());

    let job = match active_job {
        Some(j) => j,
        None => return,
    };

    let progress = match &job.progress {
        Some(p) => p,
        None => return,
    };

    // Color según estado
    let bar_color = if job.status == crate::fs::transfer::job::TransferJobStatus::Paused {
        Color::Yellow
    } else if progress.files_failed > 0 {
        Color::Red
    } else {
        Color::Green
    };

    let percent = progress.percent_bytes() as u16;
    
    // Crear bloque contenedor
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", t("transfer_title")));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Dividir en secciones horizontales
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Info: "Copying 3/10 files"
            Constraint::Min(10),    // Gauge: [███████░░░] 45%
            Constraint::Length(20), // Speed & ETA
            Constraint::Length(18), // Actions info: [Ctrl+T] Expand
        ])
        .split(inner_area);

    // 1. Info de archivos
    let info_text = format!(
        "📋 {} {}/{} files",
        t("transfer_copying"),
        progress.files_completed,
        progress.files_total
    );
    f.render_widget(Paragraph::new(info_text).style(Style::default().fg(Color::White)), chunks[0]);

    // 2. Gauge de progreso
    let label = format!("{}%", percent);
    let gauge = Gauge::default()
        .percent(percent)
        .label(label)
        .gauge_style(Style::default().fg(bar_color).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    f.render_widget(gauge, chunks[1]);

    // 3. Velocidad y ETA
    let speed_formatted = if job.status == crate::fs::transfer::job::TransferJobStatus::Paused {
        "0 B".to_string()
    } else {
        bytesize::ByteSize(transfer_state.speed_info.0 as u64).to_string()
    };
    let eta_text = if job.status == crate::fs::transfer::job::TransferJobStatus::Paused {
        "ETA --".to_string()
    } else {
        match transfer_state.speed_info.1 {
            Some(secs) => format!("ETA {}s", secs),
            None => "ETA --".to_string(),
        }
    };
    let speed_eta = format!(" {}/s | {}", speed_formatted, eta_text);
    f.render_widget(Paragraph::new(speed_eta).style(Style::default().fg(Color::Yellow)), chunks[2]);

    // 4. Atajo de ayuda compacta
    let action_text = " [Ctrl+T] Expand ";
    f.render_widget(
        Paragraph::new(action_text).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)),
        chunks[3]
    );
}
