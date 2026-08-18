use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferViewMode};
use crate::config::localization::t;
use crate::ui::popup::centered_rect;

mod file_list;
mod inspector;
mod jobs;

pub fn render_transfer_panel(f: &mut Frame, state: &AppState, context: &AppContext) {
    let transfer_state = match &state.transfer {
        Some(ts) => ts,
        None => return,
    };

    if transfer_state.view_mode != TransferViewMode::Expanded {
        return;
    }

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

    // Dividir horizontalmente: Sidebar (30%) e Inspector (70%)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(inner_area);

    let jobs = transfer_state.engine.queue.get_all();

    let theme = &context.config.theme;

    // --- 1. RENDER SIDEBAR (COL IZQUIERDA) ---
    jobs::render_jobs_sidebar(
        f,
        main_layout[0],
        transfer_state,
        &jobs,
        theme,
        Some(&state.scrollbar),
    );

    // --- 2. RENDER INSPECTOR (COL DERECHA) ---
    if jobs.is_empty() {
        let empty_p =
            Paragraph::new("\n No jobs in queue.\n Press F5 to Copy or F6 to Move files.")
                .style(
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                )
                .block(
                    Block::default()
                        .borders(Borders::LEFT)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
        f.render_widget(empty_p, main_layout[1]);
    } else {
        let cursor_idx = transfer_state
            .queue_cursor
            .min(jobs.len().saturating_sub(1));
        if let Some(selected_job) = jobs.get(cursor_idx) {
            let inspector_area = main_layout[1];
            // Aseguramos una división vertical del inspector con borde izquierdo
            let inspector_block = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray));
            let inner_inspector = inspector_block.inner(inspector_area);
            f.render_widget(inspector_block, inspector_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Cabecera
                    Constraint::Length(3), // Pestañas
                    Constraint::Min(5),    // Contenido
                    Constraint::Length(3), // Footer (Acciones)
                ])
                .split(inner_inspector);

            let progress = selected_job.progress.as_ref();
            let results = &selected_job.results;
            let log_lines = &selected_job.log_lines;

            // Header
            inspector::render_header(f, chunks[0], transfer_state, progress, selected_job);

            // Tabs
            inspector::render_tabs(
                f,
                chunks[1],
                transfer_state.active_tab,
                &context.config.theme,
            );

            // Content
            match transfer_state.active_tab {
                TransferTab::FileList => file_list::render_file_list_tab(
                    f,
                    chunks[2],
                    transfer_state,
                    results,
                    theme,
                    Some(&state.scrollbar),
                ),
                TransferTab::Options => {
                    inspector::render_options_tab(f, chunks[2], transfer_state, selected_job)
                }
                TransferTab::Status => {
                    inspector::render_status_tab(f, chunks[2], transfer_state, progress, results)
                }
                TransferTab::Log => inspector::render_log_tab(
                    f,
                    chunks[2],
                    log_lines,
                    theme,
                    transfer_state,
                    Some(&state.scrollbar),
                ),
            }

            // Footer
            inspector::render_footer(f, chunks[3], selected_job);
        }
    }
}

pub fn summarize_path(path: &std::path::Path) -> String {
    if let Some(file_name) = path.file_name() {
        if let Some(parent) = path.parent()
            && let Some(parent_name) = parent.file_name()
        {
            let sep = std::path::MAIN_SEPARATOR;
            return format!(
                "..{}{}{}{}",
                sep,
                parent_name.to_string_lossy(),
                sep,
                file_name.to_string_lossy()
            );
        }
        return file_name.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}
