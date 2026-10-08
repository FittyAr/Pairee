//! State of the multi-rename dialog: the rule fields, the files and the live
//! preview (recomputed after every edit; no I/O).

use crate::app::form::FormLayout;
use crate::app::text_input::TextField;
use crate::fs::multi_rename::{CaseMode, Counter, Preview, RenameRules, RenameSource, TargetFs};
use crate::fs::vfs::PanelSource;

/// Text fields of the dialog, in focus order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameField {
    NameMask,
    ExtMask,
    Search,
    Replace,
    CounterStart,
    CounterStep,
    CounterDigits,
}

impl RenameField {
    pub const ALL: [Self; 7] = [
        Self::NameMask,
        Self::ExtMask,
        Self::Search,
        Self::Replace,
        Self::CounterStart,
        Self::CounterStep,
        Self::CounterDigits,
    ];

    /// Label translation key.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::NameMask => "multi_rename_name_mask",
            Self::ExtMask => "multi_rename_ext_mask",
            Self::Search => "multi_rename_search",
            Self::Replace => "multi_rename_replace",
            Self::CounterStart => "multi_rename_counter_start",
            Self::CounterStep => "multi_rename_counter_step",
            Self::CounterDigits => "multi_rename_counter_digits",
        }
    }

    /// Focus row of the field.
    pub fn row(self) -> usize {
        match self {
            Self::NameMask => 0,
            Self::ExtMask => 1,
            Self::Search => 2,
            Self::Replace => 3,
            Self::CounterStart => 7,
            Self::CounterStep => 8,
            Self::CounterDigits => 9,
        }
    }

    pub fn at_row(row: usize) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.row() == row)
    }
}

/// Multi-rename dialog (Shift+F6).
#[derive(Debug, Clone)]
pub struct MultiRenameState {
    pub sources: Vec<RenameSource>,
    /// Names of every entry of the folder (clash detection without I/O).
    pub siblings: Vec<String>,
    pub target_fs: TargetFs,
    /// Set when the files are on an SFTP panel.
    pub source: PanelSource,
    pub fields: [TextField; 7],
    pub regex: bool,
    pub ignore_case: bool,
    pub case: CaseMode,
    pub focus: usize,
    /// First preview row shown.
    pub scroll: usize,
    pub preview: Preview,
    /// Regex compile error of the current search pattern.
    pub error: Option<String>,
    /// The renames are running in the background.
    pub running: bool,
}

impl MultiRenameState {
    pub const ROW_REGEX: usize = 4;
    pub const ROW_IGNORE_CASE: usize = 5;
    pub const ROW_CASE: usize = 6;
    pub const BUTTON_RENAME: usize = 10;
    pub const BUTTON_CANCEL: usize = 11;
    pub const FORM: FormLayout = FormLayout::new(12, Self::BUTTON_RENAME);

    pub fn new(sources: Vec<RenameSource>, siblings: Vec<String>, source: PanelSource) -> Self {
        let defaults = RenameRules::default();
        let mut state = Self {
            sources,
            siblings,
            target_fs: if source.is_local() {
                TargetFs::local()
            } else {
                TargetFs::remote()
            },
            source,
            fields: [
                defaults.name_mask.as_str().into(),
                defaults.ext_mask.as_str().into(),
                TextField::default(),
                TextField::default(),
                defaults.counter.start.to_string().as_str().into(),
                defaults.counter.step.to_string().as_str().into(),
                defaults.counter.digits.to_string().as_str().into(),
            ],
            regex: defaults.regex,
            ignore_case: defaults.ignore_case,
            case: defaults.case,
            focus: 0,
            scroll: 0,
            preview: Preview::default(),
            error: None,
            running: false,
        };
        state.refresh_preview();
        state
    }

    pub fn field(&self, field: RenameField) -> &TextField {
        &self.fields[field as usize]
    }

    /// The text field on the focused row, if that row is one.
    pub fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        RenameField::at_row(self.focus).map(|f| &mut self.fields[f as usize])
    }

    /// Rules as typed; counter fields that are not numbers keep defaults.
    pub fn rules(&self) -> RenameRules {
        let text = |f: RenameField| self.field(f).text().to_string();
        let number = |f: RenameField, default: i64| text(f).trim().parse().unwrap_or(default);
        let defaults = Counter::default();
        RenameRules {
            name_mask: text(RenameField::NameMask),
            ext_mask: text(RenameField::ExtMask),
            search: text(RenameField::Search),
            replace: text(RenameField::Replace),
            regex: self.regex,
            ignore_case: self.ignore_case,
            case: self.case,
            counter: Counter {
                start: number(RenameField::CounterStart, defaults.start),
                step: number(RenameField::CounterStep, defaults.step),
                digits: usize::try_from(number(RenameField::CounterDigits, 1))
                    .unwrap_or(defaults.digits),
            },
        }
    }

    /// Recomputes the preview from the current rules.
    pub fn refresh_preview(&mut self) {
        match self.rules().compile() {
            Ok(rules) => {
                self.error = None;
                self.preview =
                    Preview::build(&self.sources, &rules, &self.siblings, self.target_fs);
            }
            Err(error) => self.error = Some(error),
        }
        self.scroll = self.scroll.min(self.sources.len().saturating_sub(1));
    }

    /// `true` when the Rename button may run.
    pub fn can_run(&self) -> bool {
        !self.running && self.error.is_none() && self.preview.is_runnable()
    }
}
