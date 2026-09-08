use crate::config::localization::t;
use ratatui::{Frame, layout::Rect, style::Style, widgets::Paragraph};

pub struct FlagsParams {
    pub cursor_idx: usize,
    pub process_multiple: bool,
    pub copy_access_mode: bool,
    pub copy_extended_attributes: bool,
    pub disable_write_cache: bool,
    pub produce_sparse_files: bool,
    pub use_copy_on_write: bool,
    pub act_style: Style,
    pub norm_style: Style,
}

pub fn render_flags(f: &mut Frame, chunks: &[Rect], params: &FlagsParams) {
    let FlagsParams {
        cursor_idx,
        process_multiple,
        copy_access_mode,
        copy_extended_attributes,
        disable_write_cache,
        produce_sparse_files,
        use_copy_on_write,
        act_style,
        norm_style,
    } = *params;

    let check = |b: bool| if b { "[x]" } else { "[ ]" };
    f.render_widget(
        Paragraph::new(format!(
            "{} {}",
            check(process_multiple),
            t("prompt_process_multiple")
        ))
        .style(if cursor_idx == 2 {
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
        .style(if cursor_idx == 3 {
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
        .style(if cursor_idx == 4 {
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
        .style(if cursor_idx == 5 {
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
        .style(if cursor_idx == 6 {
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
        .style(if cursor_idx == 7 {
            act_style
        } else {
            norm_style
        }),
        chunks[8],
    );
}
