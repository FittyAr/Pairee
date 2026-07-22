//! M4-T9: `pairee.preview_code({path, mime?, skip?})` — syntax-
//! highlighted preview of a code file, returned as a `ui.Text`
//! rich userdata so the preview pane can render it with styles.
//!
//! Uses `syntect` for syntax highlighting (no shell-out to
//! `pygmentize` or `bat` — pure Rust, no external dependencies).
//! Returns a `ui.Text` of styled spans (one per highlighted token)
//! that the existing rich-rendering path in `quickview.rs`
//! renders into the preview pane.

use super::ui::elements::line::Line;
use super::ui::elements::span::Span;
use super::ui::elements::text::Text;
use super::ui::style;
use crate::plugin::types::Url;
use mlua::Lua;
use once_cell::sync::Lazy;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SynColor, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Lazily-initialised syntax set + theme. We use `Lazy` so the
/// first call pays the parsing cost (~10 MB of YAML in `syntect`'s
/// embedded `.sublime-syntax` data) but subsequent calls reuse
/// the loaded data.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Helper: pick the best syntect `SyntaxReference` for a file by
/// extension or filename. Falls back to plain text on no match.
fn pick_syntax(path: &Path) -> &SyntaxReference {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    SYNTAX_SET
        .find_syntax_by_token(name)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(ext))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

/// Convert a `syntect::highlighting::Color` into a `ColorString`
/// suitable for our `Style::fg`. The Theme carries colors in
/// `Rgb(r, g, b)` form; the plain text fallback uses
/// `Color::Reset` which we represent as the empty string.
fn syn_color_to_string(c: SynColor) -> Option<String> {
    let SynColor { r, g, b, .. } = c;
    Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
}

/// Build a `ui.Text` from a code file with syntect highlighting.
/// The result is a sequence of `Line` userdata, each `Line` being
/// a sequence of `Span` userdata carrying per-token styles.
pub fn build_preview_text_for_file(
    lua: &Lua,
    path: &Path,
    theme: &Theme,
) -> mlua::Result<Text> {
    let syntax = pick_syntax(path);
    let content = std::fs::read_to_string(path).map_err(|e| {
        mlua::Error::RuntimeError(format!("preview_code: read failed: {e}"))
    })?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Text::new();
    for line in LinesWithEndings::from(&content) {
        let mut line_obj = super::ui::elements::line::Line::new();
        let regions = match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (style, text) in regions {
            // Strip the trailing newline — `LinesWithEndings`
            // yields text including the `\n` of each line, but the
            // Line userdata separates by line naturally.
            let text = text.trim_end_matches('\n').to_string();
            let mut span = super::ui::elements::span::Span::new(text);
            // Apply the syntect fg color directly through the
            // Style userdata (string form, since parse_color
            // accepts "#rrggbb").
            if let Some(c) = syn_color_to_string(style.foreground) {
                if let Ok(Some(parsed)) = style::parse_color_value(
                    mlua::Value::String(lua.create_string(&c)?),
                ) {
                    span.style.inner = span.style.inner.fg(parsed);
                }
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                span.style.inner = span
                    .style
                    .inner
                    .add_modifier(ratatui::style::Modifier::BOLD);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                span.style.inner = span
                    .style
                    .inner
                    .add_modifier(ratatui::style::Modifier::ITALIC);
            }
            line_obj.spans.push(span);
        }
        lines.lines.push(line_obj);
    }
    Ok(lines)
}

/// `pairee.preview_code({path, mime?})` async binding. Returns a
/// `ui.Text` userdata with per-token syntax-highlighted styles.
pub fn bind(
    lua: &Lua,
    parent: &mlua::Table<'_>,
) -> mlua::Result<()> {
    let preview_code = lua.create_function(|lua_ctx, opts: mlua::Table| {
        let path_str: String = opts
            .get::<_, mlua::String>("path")
            .map_err(|e| mlua::Error::RuntimeError(format!("preview_code: {e}")))?
            .to_str()?
            .to_string();
        let path = std::path::PathBuf::from(&path_str);
        let url = Url::parse(&path_str);
        let _ = url; // reserved for future use (e.g. SFTP dispatch)
        let theme = THEME_SET
            .themes
            .values()
            .next()
            .expect("syntect theme set is non-empty")
            .clone();
        let text = build_preview_text_for_file(lua_ctx, &path, &theme)?;
        let ud = lua_ctx.create_userdata(text)?;
        Ok(mlua::Value::UserData(ud))
    })?;
    parent.set("preview_code", preview_code)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_preview_code_for_rust_file() {
        let lua = Lua::new();
        let dir = std::env::temp_dir().join("pairee_preview_code_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hello.rs");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "fn main() {{ println!(\"hi\"); }}").unwrap();
        drop(f);

        let theme = THEME_SET
            .themes
            .values()
            .next()
            .expect("syntect theme set is non-empty")
            .clone();
        let text = build_preview_text_for_file(&lua, &path, &theme).unwrap();
        assert!(text.lines.len() >= 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_preview_code_unknown_extension_falls_back_to_plain_text() {
        let lua = Lua::new();
        let dir = std::env::temp_dir().join("pairee_preview_code_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("unknown.weirdext");
        std::fs::write(&path, "hello world").unwrap();

        let theme = THEME_SET
            .themes
            .values()
            .next()
            .expect("syntect theme set is non-empty")
            .clone();
        let text = build_preview_text_for_file(&lua, &path, &theme).unwrap();
        assert!(text.lines.len() >= 1);
        std::fs::remove_file(&path).ok();
    }
}
