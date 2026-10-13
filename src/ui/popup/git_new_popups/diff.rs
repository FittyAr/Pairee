use crate::config::localization::t;
use crate::ui::popup::diff_view::DiffBox;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{Frame, layout::Rect};

/// Renders the diff of a file or a commit.
pub fn render_diff_view(
    f: &mut Frame,
    state: &crate::app::state::popup::GitDiffViewState,
    theme: &ResolvedTheme,
    size: Rect,
) {
    let title = if let Some(path) = &state.file_path {
        t("git_diff_view_title").replace("{}", path)
    } else if let Some(hash) = &state.commit_hash {
        t("git_diff_commit_title").replace("{}", hash)
    } else {
        t("git_diff_default_title")
    };
    DiffBox {
        title,
        summary: Vec::new(),
        viewer: &state.viewer,
        empty: t("git_diff_empty"),
        hint: t("diff_view_hint"),
    }
    .render(f, size, theme);
}
