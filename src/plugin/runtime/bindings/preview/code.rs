//! The text of `pairee.preview_code`: the start of a file, numbered, with
//! the syntax highlighting of the built-in editor and viewer (`syntax`)
//! when the user has it on and a grammar knows the file. Tokens only carry
//! their class; the UI colors them with the active theme.

use crate::app::state::popup::plugin_widget::{WLine, WSpan, WStyle, WText};
use crate::app::state::quick_view::load::QUICK_VIEW_TEXT_BYTES;
use crate::plugin::runtime::types::LuaError;
use crate::syntax::{LineTokens, SyntaxLimits};
use std::io::Read;
use std::path::Path;

const TAB: &str = "    ";

/// The start of `path` as text, or an error for binary content.
pub fn read_text(path: &Path) -> Result<String, LuaError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(QUICK_VIEW_TEXT_BYTES)
        .read_to_end(&mut bytes)?;
    if bytes.iter().take(8192).any(|&b| b == 0) {
        return Err(LuaError {
            kind: Some("InvalidData".into()),
            code: None,
            message: format!("pairee.preview_code: {} is not text", path.display()),
        });
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// `text` with tabs expanded and control characters removed.
fn clean(text: &str) -> String {
    text.replace('\t', TAB)
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// The spans of one line: plain text between the tokens, each token with
/// its class. A token that does not fall on character boundaries leaves
/// the line plain.
fn line_spans(line: &str, tokens: Option<&LineTokens>) -> Vec<WSpan> {
    let plain = |text: &str| WSpan {
        content: clean(text),
        ..WSpan::default()
    };
    let Some(tokens) = tokens.filter(|t| !t.is_empty()) else {
        return vec![plain(line)];
    };
    let mut spans = Vec::with_capacity(tokens.len() * 2 + 1);
    let mut at = 0;
    for token in tokens {
        let (Some(before), Some(inside)) = (
            line.get(at..token.range.start),
            line.get(token.range.clone()),
        ) else {
            return vec![plain(line)];
        };
        if !before.is_empty() {
            spans.push(plain(before));
        }
        spans.push(WSpan {
            content: clean(inside),
            syntax: Some(token.kind),
            ..WSpan::default()
        });
        at = token.range.end;
    }
    match line.get(at..) {
        Some(rest) if !rest.is_empty() => spans.push(plain(rest)),
        Some(_) => {}
        None => return vec![plain(line)],
    }
    spans
}

/// Lines `skip..skip + count` of `text` (from `path`) behind a dim
/// line-number gutter, highlighted when `limits` allow it.
pub fn code_text(
    path: &Path,
    text: &str,
    (skip, count): (usize, usize),
    limits: SyntaxLimits,
) -> WText {
    let lines: Vec<&str> = text.lines().collect();
    let end = skip.saturating_add(count).min(lines.len());
    let start = skip.min(end);
    let first = lines.first().copied().unwrap_or_default();
    let tokens: Vec<LineTokens> = limits
        .cache_for(path, first, text.len() as u64)
        .map(|mut cache| {
            let fetch =
                |r: std::ops::Range<usize>| lines[r.start.min(end)..r.end.min(end)].to_vec();
            cache.highlight(start..end, fetch).to_vec()
        })
        .unwrap_or_default();
    let width = lines.len().max(1).to_string().len();
    let gutter = WStyle {
        fg: Some("darkgray".into()),
        ..WStyle::default()
    };
    let numbered = lines[start..end].iter().enumerate().map(|(offset, line)| {
        let mut spans = vec![WSpan {
            content: format!("{:>width$} │ ", start + offset + 1),
            style: gutter.clone(),
            syntax: None,
        }];
        spans.extend(line_spans(line, tokens.get(offset)));
        WLine {
            spans,
            ..WLine::default()
        }
    });
    WText::lines(numbered.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::plugin_widget::WTextBody;
    use crate::syntax::SyntaxKind;

    const OFF: SyntaxLimits = SyntaxLimits {
        enabled: false,
        max_bytes: 0,
    };
    const ON: SyntaxLimits = SyntaxLimits {
        enabled: true,
        max_bytes: 1024 * 1024,
    };

    fn lines(text: WText) -> Vec<WLine> {
        match text.body {
            WTextBody::Lines(lines) => lines,
            WTextBody::Ansi(_) => panic!("ansi"),
        }
    }

    #[test]
    fn plain_text_is_numbered_with_tabs_expanded() {
        let out = lines(code_text(
            Path::new("a.txt"),
            "a\n\tb\nc\x07\n",
            (1, 2),
            OFF,
        ));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans[0].content, "2 │ ");
        assert_eq!(out[0].spans[1].content, "    b");
        assert_eq!(out[1].spans[1].content, "c");
        assert!(lines(code_text(Path::new("a.txt"), "x", (5, 3), OFF)).is_empty());
    }

    #[test]
    fn known_languages_are_highlighted_when_enabled() {
        let source = "fn main() {\n    let s = \"hi\";\n}\n";
        let out = lines(code_text(Path::new("main.rs"), source, (0, 10), ON));
        let kinds: Vec<SyntaxKind> = out
            .iter()
            .flat_map(|l| l.spans.iter().filter_map(|s| s.syntax))
            .collect();
        assert!(kinds.contains(&SyntaxKind::Keyword), "{out:?}");
        assert!(kinds.contains(&SyntaxKind::String), "{out:?}");
        let text: String = out[1].spans[1..]
            .iter()
            .map(|s| s.content.as_str())
            .collect();
        assert_eq!(text, "    let s = \"hi\";");
        let off = lines(code_text(Path::new("main.rs"), source, (0, 10), OFF));
        assert!(
            off.iter()
                .all(|l| l.spans.iter().all(|s| s.syntax.is_none()))
        );
    }
}
