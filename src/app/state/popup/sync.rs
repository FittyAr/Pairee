//! State of the Synchronize folders dialog: the options form, then the
//! list of differences with one action each.

use crate::app::form::FormLayout;
use crate::app::text_input::TextField;
use crate::fs::sync::plan::apply_direction;
use crate::fs::sync::{DiffKind, SyncItem, SyncOptions, SyncSummary};
use crate::fs::transfer::options::HashAlgorithm;
use std::path::PathBuf;

/// Rows of the options form.
pub mod rows {
    pub const DIRECTION: usize = 0;
    pub const CONTENT: usize = 1;
    pub const HIDDEN: usize = 2;
    pub const MASK: usize = 3;
    pub const COMPARE_BUTTON: usize = 4;
}

/// Options form: four option rows, then the Compare and Cancel buttons.
pub const OPTIONS_FORM: FormLayout =
    FormLayout::new(rows::COMPARE_BUTTON + 2, rows::COMPARE_BUTTON);

#[derive(Debug, Clone)]
pub struct SyncDialog {
    pub left: PathBuf,
    pub right: PathBuf,
    pub options: SyncOptions,
    /// Algorithm used when content comparison is on (transfer setting).
    pub algorithm: HashAlgorithm,
    pub mask: TextField,
    pub focus: usize,
    /// `Some` once a comparison delivered its result.
    pub review: Option<SyncReview>,
}

/// The differences found and the action chosen for each.
#[derive(Debug, Clone, Default)]
pub struct SyncReview {
    pub items: Vec<SyncItem>,
    /// Index into [`Self::visible`].
    pub cursor: usize,
    pub show_equal: bool,
    /// Waiting for the user to confirm a plan that deletes files.
    pub confirming: bool,
}

impl SyncDialog {
    pub fn new(
        left: PathBuf,
        right: PathBuf,
        options: SyncOptions,
        algorithm: HashAlgorithm,
    ) -> Self {
        Self {
            left,
            right,
            mask: TextField::new(options.mask.clone()),
            options,
            algorithm,
            focus: rows::DIRECTION,
            review: None,
        }
    }

    pub fn toggle_content(&mut self) {
        self.options.content_hash = match self.options.content_hash {
            Some(_) => None,
            None => Some(self.algorithm),
        };
    }

    /// Options of the next run (mask taken from the text field).
    pub fn run_options(&self) -> SyncOptions {
        SyncOptions {
            mask: self.mask.text().to_owned(),
            ..self.options.clone()
        }
    }

    pub fn show_review(&mut self, items: Vec<SyncItem>) {
        self.review = Some(SyncReview {
            items,
            ..SyncReview::default()
        });
    }

    /// Changes the direction; in the review the default actions follow it.
    pub fn cycle_direction(&mut self) {
        self.options.direction = self.options.direction.next();
        if let Some(review) = &mut self.review {
            apply_direction(&mut review.items, self.options.direction);
        }
    }
}

impl SyncReview {
    /// Indices of the items listed: differences (and equal files when
    /// shown); folder pairs are context only.
    pub fn visible(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, i)| !i.is_dir_pair() && (self.show_equal || i.kind != DiffKind::Equal))
            .map(|(idx, _)| idx)
            .collect()
    }

    /// The item under the cursor.
    pub fn current_mut(&mut self) -> Option<&mut SyncItem> {
        let idx = *self.visible().get(self.cursor)?;
        self.items.get_mut(idx)
    }

    pub fn toggle_equal(&mut self) {
        self.show_equal = !self.show_equal;
        self.cursor = self.cursor.min(self.visible().len().saturating_sub(1));
    }

    pub fn summary(&self) -> SyncSummary {
        SyncSummary::of(&self.items)
    }
}
