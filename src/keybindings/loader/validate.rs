//! Whole-keymap checks run after every layer is applied.

use super::assign::Row;
use super::report::KeymapLoadReport;
use crate::config::localization::t;
use crate::keybindings::chord::{Fragility, fragility};
use crate::keybindings::registry::Bindable;
use keybinds::Match;

/// A complete chord that is also the start of a longer sequence makes the
/// sequence unreachable (`g` would fire before `g g` could).
pub fn check_prefixes<B: Bindable>(rows: &[Row<B>], report: &mut KeymapLoadReport) {
    for short in rows {
        for long in rows {
            if long.seq.match_to(short.seq.as_slice()) == Match::Prefix {
                report.errors.push(format!(
                    "'{}' ({}) hides the sequence '{}' ({})",
                    short.seq,
                    short.command.id(),
                    long.seq,
                    long.command.id()
                ));
            }
        }
    }
}

/// Essential commands need a chord every terminal delivers.
pub fn check_robustness<B: Bindable>(rows: &[Row<B>], report: &mut KeymapLoadReport) {
    for command in B::all().into_iter().filter(|c| c.essential()) {
        let reasons: Vec<Option<Fragility>> = rows
            .iter()
            .filter(|r| r.command == command)
            .map(|r| fragility(&r.seq))
            .collect();
        if reasons.is_empty() {
            report
                .robustness
                .push(format!("'{}' has no key", command.id()));
        } else if reasons.iter().all(Option::is_some)
            && let Some(Some(reason)) = reasons.first()
        {
            report.robustness.push(format!(
                "'{}' only has chords some terminals cannot send: {}",
                command.id(),
                t(reason.label_key())
            ));
        }
    }
}
