//! Focus layouts of the inline form dialogs, shared by their key handlers and
//! renderers.

use crate::app::form::FormLayout;

/// MkDir: name, "process multiple names", OK, Cancel.
pub const MKDIR_FORM: FormLayout = FormLayout::with_input(4, 2);
/// Rename: new name, OK, Cancel.
pub const RENAME_FORM: FormLayout = FormLayout::with_input(3, 1);
