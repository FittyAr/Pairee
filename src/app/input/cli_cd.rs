//! `cd` typed on the command line: quoted names (as drag & drop and
//! "insert path" put them there), `~` and `~/x`, `cd -` (the folder before
//! the last `cd`), relative paths, and panels showing an SFTP server or an
//! archive (the target is resolved and listed inside them).

use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::plugin::runtime::bindings::fs::jail::normalize_lexical;
use std::path::{Path, PathBuf};

/// The argument of a `cd` command line (`cd`, `cd x`, `cd..`, `cd\`), or
/// `None` when `line` is another command.
pub fn cd_argument(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("cd")?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        return Some(rest.trim());
    }
    // cmd.exe style: `cd..`, `cd\`, `cd/`.
    (rest.starts_with("..") || rest.starts_with(['\\', '/'])).then_some(rest)
}

/// `arg` without its shell quoting: shell words on Unix (several words are
/// one name, as `cd My Docs` in cmd.exe); on Windows quotes are dropped and
/// a leading `/d` (change drive too) is ignored, as cmd.exe does.
pub fn unquote(arg: &str) -> String {
    let arg = arg.trim();
    if cfg!(windows) {
        let arg = arg
            .strip_prefix("/d ")
            .or_else(|| arg.strip_prefix("/D "))
            .unwrap_or(arg)
            .trim();
        arg.chars().filter(|c| !matches!(c, '"' | '\'')).collect()
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
    let home = if panel.source.ssh().is_some() {
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
    let path = if panel.source.ssh().is_some() {
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
    if path != current {
        state.cli.previous_dir = Some(current);
    }
    state.push_folders_history(path.clone());
    state.get_active_panel_mut().open_path(path);
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
