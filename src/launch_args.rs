//! Command-line arguments of an interactive run:
//! `pairee [--cwd-file <file>] [--print-cwd] [<left> [<right>]]`.

use crate::config::localization::t;
use std::path::PathBuf;

const CWD_FILE: &str = "--cwd-file";
const PRINT_CWD: &str = "--print-cwd";
/// Consumed by the standalone launcher before the arguments are parsed.
const STANDALONE: &str = "--standalone";
const MAX_PATHS: usize = 2;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LaunchArgs {
    /// Start folders of the left and right panels.
    pub paths: Vec<PathBuf>,
    /// Where to write the focused panel's folder on exit.
    pub cwd_file: Option<PathBuf>,
    /// Print the focused panel's folder to stdout on exit.
    pub print_cwd: bool,
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
    /// Parses the arguments after the program name.
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Self, ArgError> {
        let mut parsed = Self::default();
        let mut args = args.into_iter();
        let mut options_done = false;
        while let Some(arg) = args.next() {
            if options_done || !arg.starts_with("--") {
                parsed.push_path(arg)?;
            } else if arg == "--" {
                options_done = true;
            } else if arg == CWD_FILE {
                let value = args.next().ok_or(ArgError::MissingValue(CWD_FILE))?;
                parsed.cwd_file = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--cwd-file=") {
                if value.is_empty() {
                    return Err(ArgError::MissingValue(CWD_FILE));
                }
                parsed.cwd_file = Some(PathBuf::from(value));
            } else if arg == PRINT_CWD {
                parsed.print_cwd = true;
            } else if arg != STANDALONE {
                return Err(ArgError::UnknownOption(arg));
            }
        }
        Ok(parsed)
    }

    fn push_path(&mut self, arg: String) -> Result<(), ArgError> {
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
    fn standalone_flag_is_ignored() {
        let args = parse(&["--standalone", "/a"]).unwrap();
        assert_eq!(args.paths, [PathBuf::from("/a")]);
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
