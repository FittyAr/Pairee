pub mod branches;
pub mod checkout;
pub mod cherry_pick;
pub mod commit;
pub mod credentials;
pub mod diff;
pub mod log;
pub mod merge;
pub mod outcome;
pub mod pull;
pub mod push;
pub mod rebase;
pub mod remote;
pub mod repo;
pub mod reset;
pub mod revert;
pub mod signature;
pub mod snapshot;
pub mod stage;
pub mod stash;
pub mod status;
pub mod tags;
pub mod worktree;

#[cfg(test)]
mod tests;
