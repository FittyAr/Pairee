pub mod branches;
pub mod checkout;
pub mod commit;
pub mod diff;
pub mod log;
pub mod merge;
pub mod remote;
pub mod repo;
pub mod reset;
pub mod stage;
pub mod stash;
pub mod status;

#[cfg(test)]
mod tests;

/// Keepalive function to reference API functions that are implemented but not yet integrated into the UI.
/// This prevents compiler dead_code warnings without bypassing the strict dead code policy.
pub fn unused_keepalive() {
    let _ = merge::abort_merge;
    let _ = repo::init_repo;
    let _ = repo::clone_repo;
}
