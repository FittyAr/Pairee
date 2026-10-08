//! The operation journal (Memento caretaker): bounded undo and redo stacks
//! plus the undo/redo runs still in flight.

use super::command::FsCommand;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// Entries kept on each stack; older ones are forgotten.
pub const JOURNAL_CAPACITY: usize = 50;

/// Remembered finished transfer jobs (a job may report its end twice).
const SEEN_JOBS: usize = 64;

/// Which stack an entry is taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

/// Who started a finished job.
#[derive(Debug, Clone)]
pub enum JobOrigin {
    /// A user operation.
    User,
    /// An undo or redo running `command`.
    Journal(Direction, FsCommand),
    /// The job was already recorded.
    Seen,
}

#[derive(Debug, Default)]
pub struct Journal {
    undo: VecDeque<FsCommand>,
    redo: VecDeque<FsCommand>,
    /// Transfer jobs running an undo/redo.
    pending_jobs: HashMap<Uuid, (Direction, FsCommand)>,
    /// The multi-rename job runs an undo/redo.
    pending_rename: Option<Direction>,
    seen_jobs: VecDeque<Uuid>,
}

impl Journal {
    /// Records a user operation; the redo stack no longer applies.
    pub fn record(&mut self, command: FsCommand) {
        if command.is_empty() {
            return;
        }
        self.redo.clear();
        push_bounded(&mut self.undo, command);
    }

    /// The entry an undo (or redo) would reverse.
    pub fn peek(&self, direction: Direction) -> Option<&FsCommand> {
        self.stack(direction).back()
    }

    /// Removes and returns the entry an undo (or redo) reverses.
    pub fn take(&mut self, direction: Direction) -> Option<FsCommand> {
        self.stack_mut(direction).pop_back()
    }

    /// Records what an undo (or redo) actually did: it becomes the entry of
    /// the opposite stack.
    pub fn applied(&mut self, direction: Direction, command: FsCommand) {
        if command.is_empty() {
            return;
        }
        let stack = match direction {
            Direction::Undo => &mut self.redo,
            Direction::Redo => &mut self.undo,
        };
        push_bounded(stack, command);
    }

    /// Files what an operation did: a user operation (`None`) is recorded,
    /// an undo/redo goes to the opposite stack.
    pub fn settle(&mut self, direction: Option<Direction>, command: FsCommand) {
        match direction {
            None => self.record(command),
            Some(direction) => self.applied(direction, command),
        }
    }

    /// Notes that transfer job `job_id` runs `command` for an undo/redo.
    pub fn begin_job(&mut self, job_id: Uuid, direction: Direction, command: FsCommand) {
        self.pending_jobs.insert(job_id, (direction, command));
    }

    /// Who started `job_id`, once per job.
    pub fn finish_job(&mut self, job_id: Uuid) -> JobOrigin {
        if self.seen_jobs.contains(&job_id) {
            return JobOrigin::Seen;
        }
        if self.seen_jobs.len() == SEEN_JOBS {
            self.seen_jobs.pop_front();
        }
        self.seen_jobs.push_back(job_id);
        match self.pending_jobs.remove(&job_id) {
            Some((direction, command)) => JobOrigin::Journal(direction, command),
            None => JobOrigin::User,
        }
    }

    /// Notes that the next multi-rename result belongs to an undo/redo.
    pub fn begin_rename(&mut self, direction: Direction) {
        self.pending_rename = Some(direction);
    }

    /// The undo/redo the finished multi-rename ran (`None`: a user rename).
    pub fn finish_rename(&mut self) -> Option<Direction> {
        self.pending_rename.take()
    }

    pub fn len(&self, direction: Direction) -> usize {
        self.stack(direction).len()
    }

    fn stack(&self, direction: Direction) -> &VecDeque<FsCommand> {
        match direction {
            Direction::Undo => &self.undo,
            Direction::Redo => &self.redo,
        }
    }

    fn stack_mut(&mut self, direction: Direction) -> &mut VecDeque<FsCommand> {
        match direction {
            Direction::Undo => &mut self.undo,
            Direction::Redo => &mut self.redo,
        }
    }
}

fn push_bounded(stack: &mut VecDeque<FsCommand>, command: FsCommand) {
    if stack.len() == JOURNAL_CAPACITY {
        stack.pop_front();
    }
    stack.push_back(command);
}
