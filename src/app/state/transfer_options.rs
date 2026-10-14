//! Rows of the transfer panel's Options tab, in order, with what each one
//! changes and whether it reaches a job that is already running (see
//! `fs::transfer::live`). The key handler and the renderer both go through
//! [`OptionRow::ALL`], so their row numbers cannot drift apart.

use crate::fs::transfer::options::{BufferSize, TransferOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionRow {
    DirectIo,
    Verify,
    Timestamps,
    Attributes,
    PostAction,
    Buffer,
    Hash,
    Acl,
    Streams,
    SkipLinks,
    FollowLinks,
    Bandwidth,
    Conflict,
}

/// Conflict policies the Conflict row cycles through.
pub const CONFLICT_POLICIES: [&str; 5] = ["ask", "overwrite", "overwrite_older", "skip", "rename"];

/// Speed limits the Bandwidth row cycles through (bytes per second).
const BANDWIDTH_STEPS: [Option<u64>; 4] =
    [None, Some(1_048_576), Some(10_485_760), Some(52_428_800)];

impl OptionRow {
    pub const ALL: [Self; 13] = [
        Self::DirectIo,
        Self::Verify,
        Self::Timestamps,
        Self::Attributes,
        Self::PostAction,
        Self::Buffer,
        Self::Hash,
        Self::Acl,
        Self::Streams,
        Self::SkipLinks,
        Self::FollowLinks,
        Self::Bandwidth,
        Self::Conflict,
    ];

    /// Row `index` of the tab.
    pub fn at(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// Changing it affects a job that already started: the speed limit,
    /// verification and the conflict policy (shared with the worker), and
    /// what happens when the queue ends (kept by the panel). The others are
    /// read when a job starts.
    pub fn is_live(self) -> bool {
        matches!(
            self,
            Self::Verify | Self::PostAction | Self::Bandwidth | Self::Conflict
        )
    }

    /// Steps the option this row shows in `options` (the post action is
    /// the panel's, not a job option: nothing to do here).
    pub fn edit(self, options: &mut TransferOptions) {
        let o = options;
        match self {
            Self::DirectIo => o.direct_io = !o.direct_io,
            Self::Verify => o.verify_after_copy = !o.verify_after_copy,
            Self::Timestamps => o.preserve_timestamps = !o.preserve_timestamps,
            Self::Attributes => o.preserve_attributes = !o.preserve_attributes,
            Self::PostAction => {}
            Self::Buffer => {
                o.buffer_size = match o.buffer_size {
                    BufferSize::_64KB => BufferSize::_256KB,
                    BufferSize::_256KB => BufferSize::_1MB,
                    BufferSize::_1MB => BufferSize::_4MB,
                    BufferSize::_4MB => BufferSize::_64KB,
                }
            }
            Self::Hash => o.hash_algorithm = o.hash_algorithm.next(),
            Self::Acl => o.preserve_acl = !o.preserve_acl,
            Self::Streams => o.preserve_streams = !o.preserve_streams,
            Self::SkipLinks => o.skip_symlinks = !o.skip_symlinks,
            Self::FollowLinks => o.follow_symlinks = !o.follow_symlinks,
            Self::Bandwidth => {
                o.limit_bandwidth_rate = next_of(&BANDWIDTH_STEPS, o.limit_bandwidth_rate)
            }
            Self::Conflict => {
                let next = next_of(&CONFLICT_POLICIES, o.conflict_resolution.as_str());
                o.conflict_resolution = next.to_string();
            }
        }
    }
}

/// The value after `current` in `steps` (the first one after the last,
/// or for a value not in the list).
fn next_of<T: Copy + PartialEq>(steps: &[T], current: T) -> T {
    let at = steps.iter().position(|s| *s == current);
    steps[at.map_or(0, |i| (i + 1) % steps.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cycle_their_values() {
        let mut options = TransferOptions::default();
        for _ in 0..CONFLICT_POLICIES.len() {
            OptionRow::Conflict.edit(&mut options);
        }
        assert_eq!(options.conflict_resolution, "ask", "a full turn");
        OptionRow::Bandwidth.edit(&mut options);
        assert_eq!(options.limit_bandwidth_rate, Some(1_048_576));
        options.limit_bandwidth_rate = Some(7);
        OptionRow::Bandwidth.edit(&mut options);
        assert_eq!(
            options.limit_bandwidth_rate, None,
            "an unknown value restarts"
        );
        assert!(OptionRow::Bandwidth.is_live() && !OptionRow::Buffer.is_live());
    }
}
