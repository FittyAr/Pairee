//! Safe ordering of a batch of renames.
//!
//! A rename may target the current name of another source (`a → b` while
//! `b → c`): it must wait until that source moved away. Chains are ordered
//! so every target is free when its rename runs; cycles (`a → b`, `b → a`)
//! are broken by first moving one source to a temporary name.

use super::names::TargetFs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One rename to perform, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Two moves share a target: the batch cannot be ordered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateTarget(pub PathBuf);

/// Orders `moves` (`(old, new)` pairs whose targets do not clash with
/// entries outside the batch) into steps. `taken` reports names that exist
/// in the folder, so temporary names never collide with them.
pub fn plan(
    moves: &[(PathBuf, PathBuf)],
    fs: TargetFs,
    taken: &dyn Fn(&Path) -> bool,
) -> Result<Vec<Step>, DuplicateTarget> {
    let mut targets = HashSet::new();
    if let Some((_, to)) = moves
        .iter()
        .find(|(_, to)| !targets.insert(fs.path_key(to)))
    {
        return Err(DuplicateTarget(to.clone()));
    }
    let mut pending: Vec<(PathBuf, PathBuf)> = moves
        .iter()
        .filter(|(from, to)| from != to)
        .cloned()
        .collect();
    let mut occupied: HashSet<String> = pending.iter().map(|(from, _)| fs.path_key(from)).collect();
    let mut steps = Vec::with_capacity(pending.len());
    let mut temp_counter = 0usize;

    while !pending.is_empty() {
        let before = pending.len();
        pending.retain(|(from, to)| {
            let from_key = fs.path_key(from);
            let to_key = fs.path_key(to);
            // A case-only rename targets its own (occupied) slot.
            if occupied.contains(&to_key) && to_key != from_key {
                return true;
            }
            occupied.remove(&from_key);
            occupied.insert(to_key);
            steps.push(Step {
                from: from.clone(),
                to: to.clone(),
            });
            false
        });
        if pending.len() == before {
            // Only cycles are left: park the first source on a free name.
            let (from, _) = &mut pending[0];
            let temp = temp_name(from, fs, &occupied, taken, &mut temp_counter);
            occupied.remove(&fs.path_key(from));
            occupied.insert(fs.path_key(&temp));
            steps.push(Step {
                from: from.clone(),
                to: temp.clone(),
            });
            *from = temp;
        }
    }
    Ok(steps)
}

/// Free temporary name next to `path`.
fn temp_name(
    path: &Path,
    fs: TargetFs,
    occupied: &HashSet<String>,
    taken: &dyn Fn(&Path) -> bool,
    counter: &mut usize,
) -> PathBuf {
    let name = crate::fs::file_name_lossy(path);
    loop {
        *counter += 1;
        let candidate = path.with_file_name(format!("{name}.pairee-tmp{counter}"));
        if !occupied.contains(&fs.path_key(&candidate)) && !taken(&candidate) {
            return candidate;
        }
    }
}
