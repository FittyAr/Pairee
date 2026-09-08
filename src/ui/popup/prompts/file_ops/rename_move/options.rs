use super::flags;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

pub struct CopyMoveOptionsParams<'a> {
    pub cursor_idx: usize,
    pub already_existing: usize,
    pub process_multiple: bool,
    pub copy_access_mode: bool,
    pub copy_extended_attributes: bool,
    pub disable_write_cache: bool,
    pub produce_sparse_files: bool,
    pub use_copy_on_write: bool,
    pub symlink_mode: usize,
    pub use_filter: bool,
    pub filter_mask: &'a str,
    pub theme: &'a crate::config::theme::Theme,
    pub inner_width: usize,
}

pub fn render_options(f: &mut Frame, chunks: &[Rect], params: &CopyMoveOptionsParams) {
    let CopyMoveOptionsParams {
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
        theme,
        inner_width,
    } = *params;
    let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
    let norm_style = Style::default().fg(parse_color(&theme.popup_fg));
    let sep_style = Style::default().fg(Color::Yellow);
    let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner_width);

    f.render_widget(Paragraph::new(sep_str.clone()).style(sep_style), chunks[1]);

    let exist_opts = [
        t("opt_ask"),
        t("opt_overwrite"),
        t("opt_skip"),
        t("opt_append"),
    ];
    let exist_style = if cursor_idx == 1 {
        act_style
    } else {
        norm_style
    };
    f.render_widget(
        Paragraph::new(format!(
            "{} {}",
            t("prompt_already_existing"),
            exist_opts[already_existing]
        ))
        .style(exist_style),
        chunks[2],
    );

    flags::render_flags(
        f,
        chunks,
        &flags::FlagsParams {
            cursor_idx,
            process_multiple,
            copy_access_mode,
            copy_extended_attributes,
            disable_write_cache,
            produce_sparse_files,
            use_copy_on_write,
            act_style,
            norm_style,
        },
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
            sym_opts[symlink_mode]
        ))
        .style(if cursor_idx == 8 {
            act_style
        } else {
            norm_style
        }),
        chunks[9],
    );

    f.render_widget(Paragraph::new(sep_str.clone()).style(sep_style), chunks[10]);

    let check = |b: bool| if b { "[x]" } else { "[ ]" };
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
        .style(if cursor_idx == 9 {
            act_style
        } else {
            norm_style
        }),
        chunks[11],
    );

    f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[12]);
}
