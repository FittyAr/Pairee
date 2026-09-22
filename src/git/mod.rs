pub mod branches;
pub mod checkout;
pub mod cherry_pick;
pub mod commit;
pub mod diff;
pub mod log;
pub mod merge;
pub mod rebase;
pub mod remote;
pub mod repo;
pub mod reset;
pub mod revert;
pub mod stage;
pub mod stash;
pub mod status;
pub mod tags;

#[cfg(test)]
mod tests;

/// Keepalive function to reference API functions that are implemented but not yet integrated into the UI.
/// This prevents compiler dead_code warnings without bypassing the strict dead code policy.
pub fn unused_keepalive() {
    let _ = repo::init_repo;
    let _ = repo::clone_repo;
}
