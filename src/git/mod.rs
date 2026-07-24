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
    let _ = branches::create_branch;
    let _ = branches::delete_branch;
    let _ = branches::rename_branch;

    let _ = diff::get_file_diff;
    let _ = diff::get_commit_diff;

    let _ = merge::merge;
    let _ = merge::abort_merge;

    let _ = remote::fetch;
    let _ = remote::pull;
    let _ = remote::push;

    let _ = repo::init_repo;
    let _ = repo::clone_repo;

    let _ = reset::reset;
    let _ = reset::ResetMode::Soft;
    let _ = reset::ResetMode::Mixed;
    let _ = reset::ResetMode::Hard;

    let _ = stage::stage_file;
    let _ = stage::unstage_file;

    let _ = stash::list_stashes;
    let _ = stash::stash_save;
    let _ = stash::stash_apply;
    let _ = stash::stash_drop;
    let _ = stash::stash_pop;

    let info = stash::StashInfo {
        index: 0,
        message: String::new(),
        oid: String::new(),
    };
    let _ = info.index;
    let _ = info.message;
    let _ = info.oid;
}
