//! Command-line arguments of an interactive run:
//! `pairee [--cwd-file <file>] [--print-cwd] [--no-mouse] [--standalone]
//! [--] [<left> [<right>]]`.

use crate::config::localization::t;
use std::ffi::OsString;
use std::path::PathBuf;

const CWD_FILE: &str = "--cwd-file";
const PRINT_CWD: &str = "--print-cwd";
const NO_MOUSE: &str = "--no-mouse";
/// Open Pairee in a new terminal window (see `terminal::standalone`).
pub(crate) const STANDALONE: &str = "--standalone";
const MAX_PATHS: usize = 2;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LaunchArgs {
    /// Start folders of the left and right panels.
    pub paths: Vec<PathBuf>,
    /// Where to write the focused panel's folder on exit.
    pub cwd_file: Option<PathBuf>,
    /// Print the focused panel's folder to stdout on exit.
    pub print_cwd: bool,
    /// Never capture the mouse, whatever the configuration says.
    pub no_mouse: bool,
    /// Relaunch in a new terminal window (only once the arguments parsed).
    pub standalone: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArgError {
    MissingValue(&'static str),
    UnknownOption(String),
    TooManyPaths,
}

impl ArgError {
    /// Localized message for the terminal.
    pub fn message(&self) -> String {
        match self {
            Self::MissingValue(option) => t("cli_error_missing_value").replace("{}", option),
            Self::UnknownOption(option) => t("cli_error_unknown_option").replace("{}", option),
            Self::TooManyPaths => t("cli_error_too_many_paths"),
        }
    }
}

impl LaunchArgs {
    /// Parses the arguments after the program name. Folders may be any OS
    /// string (not necessarily UTF-8); options are matched as text.
    pub fn parse<I, A>(args: I) -> Result<Self, ArgError>
    where
        I: IntoIterator<Item = A>,
        A: Into<OsString>,
    {
        let mut parsed = Self::default();
        let mut args = args.into_iter().map(Into::into);
        let mut options_done = false;
        while let Some(arg) = args.next() {
            // A non-UTF-8 argument cannot be an option: it is a folder.
            let Some(text) = arg
                .to_str()
                .filter(|a| !options_done && a.starts_with("--"))
            else {
                parsed.push_path(arg)?;
                continue;
            };
            if text == "--" {
                options_done = true;
            } else if text == CWD_FILE {
                let value = args.next().ok_or(ArgError::MissingValue(CWD_FILE))?;
                parsed.cwd_file = Some(PathBuf::from(value));
            } else if let Some(value) = text.strip_prefix("--cwd-file=") {
                if value.is_empty() {
                    return Err(ArgError::MissingValue(CWD_FILE));
                }
                parsed.cwd_file = Some(PathBuf::from(value));
            } else if text == PRINT_CWD {
                parsed.print_cwd = true;
            } else if text == NO_MOUSE {
                parsed.no_mouse = true;
            } else if text == STANDALONE {
                parsed.standalone = true;
            } else {
                return Err(ArgError::UnknownOption(text.to_string()));
            }
        }
        Ok(parsed)
    }

    fn push_path(&mut self, arg: OsString) -> Result<(), ArgError> {
        if self.paths.len() == MAX_PATHS {
            return Err(ArgError::TooManyPaths);
        }
        self.paths.push(PathBuf::from(arg));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<LaunchArgs, ArgError> {
        LaunchArgs::parse(args.iter().map(|a| a.to_string()))
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_folders_are_kept_byte_for_byte() {
        use std::os::unix::ffi::OsStringExt;
        let raw = OsString::from_vec(b"/tmp/caf\xe9".to_vec());
        let args = LaunchArgs::parse([raw.clone()]).unwrap();
        assert_eq!(args.paths, [PathBuf::from(raw)]);
    }

    #[test]
    fn no_arguments() {
        assert_eq!(parse(&[]), Ok(LaunchArgs::default()));
    }

    #[test]
    fn folders_and_options_in_any_order() {
        let args = parse(&["/a", "--cwd-file", "/tmp/cwd", "/b", "--print-cwd"]).unwrap();
        assert_eq!(args.paths, [PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(args.cwd_file, Some(PathBuf::from("/tmp/cwd")));
        assert!(args.print_cwd);
        let args = parse(&["--cwd-file=/x"]).unwrap();
        assert_eq!(args.cwd_file, Some(PathBuf::from("/x")));
    }

    #[test]
    fn double_dash_ends_options() {
        let args = parse(&["--", "--weird-folder"]).unwrap();
        assert_eq!(args.paths, [PathBuf::from("--weird-folder")]);
    }

    #[test]
    fn standalone_flag_is_an_option_before_double_dash() {
        assert!(parse(&["--no-mouse"]).unwrap().no_mouse);
        let args = parse(&["--standalone", "/a"]).unwrap();
        assert!(args.standalone);
        assert_eq!(args.paths, [PathBuf::from("/a")]);
        let args = parse(&["--", "--standalone"]).unwrap();
        assert!(!args.standalone);
        assert_eq!(args.paths, [PathBuf::from("--standalone")]);
        assert!(
            parse(&["--standalone", "--bad"]).is_err(),
            "validated first"
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            parse(&["--cwd-file"]),
            Err(ArgError::MissingValue(CWD_FILE))
        );
        assert_eq!(
            parse(&["--cwd-file="]),
            Err(ArgError::MissingValue(CWD_FILE))
        );
        assert_eq!(
            parse(&["--nope"]),
            Err(ArgError::UnknownOption("--nope".into()))
        );
        assert_eq!(parse(&["/a", "/b", "/c"]), Err(ArgError::TooManyPaths));
        assert!(!ArgError::TooManyPaths.message().is_empty());
    }
}
