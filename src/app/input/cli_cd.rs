//! `cd` typed on the command line: quoted names (as drag & drop and
//! "insert path" put them there), `~` and `~/x`, `cd -` (the folder before
//! the last `cd`), relative paths, and panels showing an SFTP server or an
//! archive (the target is resolved and listed inside them). `z <words>`
//! jumps to the best visited folder matching the words (frecency, as
//! zoxide); a bare `z` opens the folder jump list.

use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::plugin::runtime::bindings::fs::jail::normalize_lexical;
use std::path::{Path, PathBuf};

/// The argument of a `cd` command line (`cd`, `cd x`, `cd..`, `cd\`), or
/// `None` when `line` is another command.
pub fn cd_argument(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("cd")?;
    if has_shell_operator(rest) {
        return None; // `cd x && make`: the shell runs the whole line
    }
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        return Some(rest.trim());
    }
    // cmd.exe style: `cd..`, `cd\`, `cd/`.
    (rest.starts_with("..") || rest.starts_with(['\\', '/'])).then_some(rest)
}

/// Whether `text` has an unquoted shell operator (`&`, `|`, `;`, `<`, `>`),
/// i.e. is more than one plain argument. Quotes follow the platform shell:
/// only `"` on Windows, `"` and `'` (and `\` escapes) elsewhere.
fn has_shell_operator(text: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') => quote = Some('"'),
            (None, '\'') if !cfg!(windows) => quote = Some('\''),
            (None, '\\') if !cfg!(windows) => {
                chars.next();
            }
            (None, '&' | '|' | ';' | '<' | '>') => return true,
            _ => {}
        }
    }
    false
}

/// The query of a `z` command line (`z`, `z words`), or `None` when `line`
/// is another command.
pub fn z_argument(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('z')?;
    if has_shell_operator(rest) {
        return None;
    }
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

/// `arg` without its shell quoting: shell words on Unix (several words are
/// one name, as `cd My Docs` in cmd.exe); on Windows quotes are dropped and
/// a leading `/d` (change drive too) is ignored, as cmd.exe does (an
/// apostrophe inside the name stays, as in `O'Brien`).
pub fn unquote(arg: &str) -> String {
    let arg = arg.trim();
    if cfg!(windows) {
        let arg = arg
            .strip_prefix("/d ")
            .or_else(|| arg.strip_prefix("/D "))
            .unwrap_or(arg)
            .trim();
        let name: String = arg.chars().filter(|&c| c != '"').collect();
        // Single quotes only wrap a whole name (`'My Docs'`); inside it
        // they are part of the name.
        match name.strip_prefix('\'').and_then(|n| n.strip_suffix('\'')) {
            Some(inner) => inner.to_string(),
            None => name,
        }
    } else {
        shlex::split(arg).map_or_else(|| arg.to_string(), |words| words.join(" "))
    }
}

/// The folder `cd <target>` goes to from `current`. `previous` is the
/// folder before the last `cd` (for `-`); `None` when `-` has none.
pub fn resolve(
    target: &str,
    current: &Path,
    previous: Option<&Path>,
    home: &Path,
) -> Option<PathBuf> {
    let path = match target {
        "" | "~" => home.to_path_buf(),
        "-" => previous?.to_path_buf(),
        _ => match target
            .strip_prefix("~/")
            .or_else(|| target.strip_prefix("~\\"))
        {
            Some(rest) => home.join(rest),
            None => current.join(drive_root(target)),
        },
    };
    Some(normalize_lexical(&path))
}

/// `path` with `/` separators: SFTP paths are POSIX paths even when
/// Pairee runs on Windows.
fn posix_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('\\', "/");
    PathBuf::from(if text.is_empty() {
        "/".to_string()
    } else {
        text
    })
}

/// `D:` alone means the root of drive D (not its current folder, which a
/// file manager does not track).
fn drive_root(target: &str) -> PathBuf {
    let bytes = target.as_bytes();
    if cfg!(windows) && bytes.len() == 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        PathBuf::from(format!("{target}\\"))
    } else {
        PathBuf::from(target)
    }
}

/// Runs `cd <arg>` on the active panel: records the folder history and the
/// folder `cd -` returns to. A missing local folder (or no previous one for
/// `cd -`) is reported and the panel stays. The caller rereads the panels.
pub fn change_dir(state: &mut AppState, arg: &str) {
    let panel = state.get_active_panel();
    let current = panel.current_path.clone();
    let target = unquote(arg);
    let home = if panel.source.remote().is_some() {
        PathBuf::from("/")
    } else {
        crate::app::session::restore::home_dir()
    };
    let Some(path) = resolve(&target, &current, state.cli.previous_dir.as_deref(), &home) else {
        state
            .dialogs
            .replace(PopupType::info(t("cli_cd_no_previous")));
        return;
    };
    let path = if panel.source.remote().is_some() {
        posix_path(&path)
    } else {
        path
    };
    let source = panel.source.locate(&path);
    if source.is_local() && !path.is_dir() {
        let text = t("cli_cd_not_found").replacen("{}", &path.to_string_lossy(), 1);
        state.dialogs.replace(PopupType::error(text));
        return;
    }
    go_to(state, current, path);
}

/// Runs `z <query>` on the active panel: the best visited folder matching
/// `query` that still exists (and is not the current one); a bare `z`
/// opens the folder jump list. The caller rereads the panels.
pub fn jump(state: &mut AppState, query: &str) {
    if query.is_empty() {
        crate::app::folder_jump::open(state);
        return;
    }
    let current = state.get_active_panel().current_path.clone();
    let now = crate::config::frecency::now();
    let best = state
        .frecency
        .best(query, now, |path| path != current && path.is_dir());
    match best {
        Some(path) => go_to_local(state, current, path),
        None => {
            let text = t("cli_z_no_match").replacen("{}", query, 1);
            state.dialogs.replace(PopupType::info(text));
        }
    }
}

/// Moves the active panel from `current` to `path`, recording the folder
/// history and the folder `cd -` returns to.
pub fn go_to(state: &mut AppState, current: PathBuf, path: PathBuf) {
    if path != current {
        state.cli.previous_dir = Some(current);
    }
    state.push_folders_history(path.clone());
    state.get_active_panel_mut().open_path(path);
}

/// [`go_to`] for a folder of the local disk (frecency, hotlist, folder
/// shortcuts): a panel showing a server goes back to the local disk.
pub fn go_to_local(state: &mut AppState, current: PathBuf, path: PathBuf) {
    state.get_active_panel_mut().source = crate::fs::vfs::PanelSource::Local;
    go_to(state, current, path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_cd_lines() {
        assert_eq!(cd_argument("cd"), Some(""));
        assert_eq!(cd_argument("cd  docs "), Some("docs"));
        assert_eq!(cd_argument("cd.."), Some(".."));
        assert_eq!(cd_argument("cdx"), None);
        assert_eq!(cd_argument("echo cd"), None);
    }

    #[test]
    fn a_cd_chained_with_other_commands_is_left_to_the_shell() {
        for line in [
            "cd x && make",
            "cd x; ls",
            "cd x | cat",
            "cd x > f",
            "cd x &",
            "cd x || y",
        ] {
            assert_eq!(cd_argument(line), None, "{line}");
        }
        let quoted = "cd \"a && b\"";
        assert_eq!(
            cd_argument(quoted),
            Some("\"a && b\""),
            "quoted operators are a name"
        );
        assert_eq!(z_argument("z proj && ls"), None);
    }

    #[test]
    fn an_apostrophe_in_a_windows_name_is_kept() {
        if cfg!(windows) {
            assert_eq!(unquote("\"O'Brien\""), "O'Brien");
        }
    }

    #[test]
    fn recognizes_z_lines() {
        assert_eq!(z_argument("z"), Some(""));
        assert_eq!(z_argument("z  proj src "), Some("proj src"));
        assert_eq!(z_argument("zip a.zip"), None);
        assert_eq!(z_argument("cd z"), None);
    }

    #[test]
    fn removes_quotes() {
        assert_eq!(unquote("\"My Docs\""), "My Docs");
        assert_eq!(unquote("'My Docs'"), "My Docs");
        assert_eq!(unquote("My Docs"), "My Docs");
    }

    #[test]
    fn resolves_home_previous_and_relative() {
        let cur = PathBuf::from("/w/a");
        let home = PathBuf::from("/home/u");
        let r = |t: &str, prev: Option<&Path>| resolve(t, &cur, prev, &home);
        assert_eq!(r("", None), Some(home.clone()));
        assert_eq!(r("~/x", None), Some(home.join("x")));
        assert_eq!(r("..", None), Some(PathBuf::from("/w")));
        assert_eq!(r("b/../c", None), Some(cur.join("c")));
        assert_eq!(r("-", None), None);
        assert_eq!(r("-", Some(Path::new("/old"))), Some(PathBuf::from("/old")));
    }
}
