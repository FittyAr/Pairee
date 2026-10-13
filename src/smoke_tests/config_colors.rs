//! The color dialogs of the configuration dialog open over it, and what
//! they save reaches its draft (so OK does not undo it).

use crate::app::state::ConfigurationDialogState;
use crate::app::state::popup::{ColorGroupsDialog, FilesHighlightingDialog};
use crate::test_harness::Harness;

#[test]
fn color_dialogs_open_over_the_configuration_and_update_its_draft() {
    let mut h = Harness::new();
    let config = ConfigurationDialogState::new(&h.ctx.config, 0, 0, false);
    h.state.dialogs.replace(config);
    h.state.dialogs.push(ColorGroupsDialog {
        cursor_idx: 0,
        edit: None,
        theme: Box::new(h.ctx.config.theme.clone()),
    });
    // `s` saves the colors: the theme becomes custom, in the draft too.
    h.keys("s");
    let dialog = h.state.dialogs.top_as::<ConfigurationDialogState>();
    let dialog = dialog.expect("back to the configuration dialog");
    assert_eq!(dialog.draft.theme, "custom");
    assert_eq!(h.ctx.config.settings.theme, "custom");

    let mut rules = h.ctx.config.settings.highlight_rules.clone();
    rules.push(crate::ui::highlight::HighlightRule {
        mask: "*.zz".into(),
        color: "Red".into(),
    });
    h.state.dialogs.push(FilesHighlightingDialog {
        cursor_idx: 0,
        edit: None,
        rules: rules.clone(),
    });
    h.keys("s");
    let dialog = h.state.dialogs.top_as::<ConfigurationDialogState>();
    assert_eq!(dialog.expect("configuration").draft.highlight_rules, rules);
    // Esc in a color dialog goes back without closing the configuration.
    h.state.dialogs.push(FilesHighlightingDialog {
        cursor_idx: 0,
        edit: None,
        rules,
    });
    h.keys("Esc");
    assert!(
        h.state
            .dialogs
            .top_as::<ConfigurationDialogState>()
            .is_some()
    );
}
