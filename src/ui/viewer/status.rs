//! Bottom status line of the viewer: encoding, position, background work.

use super::state::{HEX_ROW_BYTES, ViewerMode, ViewerState};
use crate::config::localization::t;

/// Separator between status fields.
const SEP: &str = " │ ";

/// `" UTF-8 │ Line 12 of 3400 │ Indexing… 40% "` and similar.
pub(crate) fn status_text(state: &ViewerState, search_progress: Option<u8>) -> String {
    let mut fields = vec![state.doc.encoding().name().to_string()];
    let indexing = state.doc.index_progress();
    match state.mode {
        ViewerMode::Text => {
            let total = state.doc.line_count();
            let total = format!("{total}{}", if indexing.is_some() { "+" } else { "" });
            let current = (state.scroll + 1).min(state.content_rows().max(1));
            fields.push(
                t("viewer_status_line")
                    .replacen("{}", &current.to_string(), 1)
                    .replacen("{}", &total, 1),
            );
        }
        ViewerMode::Hex => fields.push(
            t("viewer_status_offset")
                .replacen("{}", &format!("{:X}", state.scroll * HEX_ROW_BYTES), 1)
                .replacen("{}", &bytesize::ByteSize::b(state.doc.len()).to_string(), 1),
        ),
        ViewerMode::Image => {}
    }
    if let Some(pct) = indexing {
        fields.push(t("viewer_indexing").replacen("{}", &pct.to_string(), 1));
    }
    if let Some(pct) = search_progress {
        fields.push(t("viewer_searching").replacen("{}", &pct.to_string(), 1));
    } else if let Some(notice) = &state.notice {
        fields.push(notice.clone());
    }
    format!(" {} ", fields.join(SEP))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_encoding_position_and_search_progress() {
        let mut state = ViewerState::from_text("a.txt".into(), vec!["x".into(), "y".into()]);
        let text = status_text(&state, None);
        assert!(text.contains("UTF-8"), "{text}");
        assert!(text.contains('2'), "{text}");
        state.notice = Some("nope".into());
        assert!(status_text(&state, None).contains("nope"));
        assert!(!status_text(&state, Some(42)).contains("nope"));
        assert!(status_text(&state, Some(42)).contains("42"));
    }
}
