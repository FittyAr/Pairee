//! Rename rules (masks, counter, search & replace, case) and the function
//! that turns one source file into its new name.

use super::RenameSource;
use super::mask::{DateValues, Mask, MaskValues};
use regex::{NoExpand, Regex, RegexBuilder};

/// Case transform applied last, to the whole new name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseMode {
    #[default]
    Unchanged,
    Lower,
    Upper,
    /// First letter of every word upper case, the rest lower case.
    Title,
}

impl CaseMode {
    const ALL: [Self; 4] = [Self::Unchanged, Self::Lower, Self::Upper, Self::Title];

    /// Next (`forward`) or previous mode, wrapping around.
    pub fn cycle(self, forward: bool) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        let len = Self::ALL.len();
        let next = if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        Self::ALL[next]
    }

    /// Translation key of the mode's label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Unchanged => "multi_rename_case_unchanged",
            Self::Lower => "multi_rename_case_lower",
            Self::Upper => "multi_rename_case_upper",
            Self::Title => "multi_rename_case_title",
        }
    }

    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Unchanged => text.to_string(),
            Self::Lower => text.to_lowercase(),
            Self::Upper => text.to_uppercase(),
            Self::Title => title_case(text),
        }
    }
}

fn title_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word_start = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if word_start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            word_start = false;
        } else {
            out.push(c);
            word_start = true;
        }
    }
    out
}

/// `[C]` counter: `start + index * step`, zero-padded to `digits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    pub start: i64,
    pub step: i64,
    pub digits: usize,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            start: 1,
            step: 1,
            digits: 1,
        }
    }
}

impl Counter {
    /// Largest padding accepted (keeps names sane).
    pub const MAX_DIGITS: usize = 10;

    pub fn format(self, index: usize) -> String {
        let index = i64::try_from(index).unwrap_or(i64::MAX);
        let value = self.start.saturating_add(index.saturating_mul(self.step));
        let digits = self.digits.min(Self::MAX_DIGITS);
        if value < 0 {
            format!("-{:0digits$}", value.unsigned_abs())
        } else {
            format!("{value:0digits$}")
        }
    }
}

/// Everything the user configures in the multi-rename dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRules {
    pub name_mask: String,
    pub ext_mask: String,
    pub search: String,
    pub replace: String,
    /// `search` is a regular expression (`replace` may use `$1`, `${name}`).
    pub regex: bool,
    pub ignore_case: bool,
    pub case: CaseMode,
    pub counter: Counter,
}

impl Default for RenameRules {
    fn default() -> Self {
        Self {
            name_mask: "[N]".into(),
            ext_mask: "[E]".into(),
            search: String::new(),
            replace: String::new(),
            regex: false,
            ignore_case: false,
            case: CaseMode::Unchanged,
            counter: Counter::default(),
        }
    }
}

/// Rules with masks parsed and the search pattern compiled.
#[derive(Debug, Clone)]
pub struct CompiledRules {
    name: Mask,
    ext: Mask,
    search: Option<Regex>,
    replace: String,
    regex: bool,
    case: CaseMode,
    counter: Counter,
}

impl RenameRules {
    /// Parses the masks and compiles the search pattern; the error is the
    /// regex compiler's message.
    pub fn compile(&self) -> Result<CompiledRules, String> {
        let search = if self.search.is_empty() {
            None
        } else {
            let pattern = if self.regex {
                self.search.clone()
            } else {
                regex::escape(&self.search)
            };
            let compiled = RegexBuilder::new(&pattern)
                .case_insensitive(self.ignore_case)
                .build()
                .map_err(|e| e.to_string())?;
            Some(compiled)
        };
        Ok(CompiledRules {
            name: Mask::parse(&self.name_mask),
            ext: Mask::parse(&self.ext_mask),
            search,
            replace: self.replace.clone(),
            regex: self.regex,
            case: self.case,
            counter: self.counter,
        })
    }
}

impl CompiledRules {
    /// New name of `source`, the `index`-th file of the batch (0-based):
    /// masks first, then search & replace on the whole name, then case.
    pub fn new_name(&self, source: &RenameSource, index: usize) -> String {
        let (name, ext) = source.split_name();
        let parent = source.parent_name();
        let counter = self.counter.format(index);
        let values = MaskValues {
            name: &name,
            ext: &ext,
            parent: &parent,
            counter: &counter,
            date: source.modified.map(DateValues::from_system_time),
        };
        let new_name = self.name.render(&values);
        let new_ext = self.ext.render(&values);
        let mut full = if new_ext.is_empty() {
            new_name
        } else {
            format!("{new_name}.{new_ext}")
        };
        if let Some(search) = &self.search {
            full = if self.regex {
                search
                    .replace_all(&full, self.replace.as_str())
                    .into_owned()
            } else {
                search
                    .replace_all(&full, NoExpand(&self.replace))
                    .into_owned()
            };
        }
        self.case.apply(&full)
    }
}
