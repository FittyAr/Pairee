//! Off-thread directory listing: everything a panel needs after a (re)read.
//!
//! A [`ListingRequest`] is a plain snapshot built on the UI thread; [`run`]
//! executes on the blocking pool (see `app::jobs`) and returns a
//! [`PanelListing`] that the UI applies in one go.

use super::filter::{apply_masks, git_pathspec_for};
use crate::fs::FileEntry;
use crate::fs::attrs::FileAttrs;
use crate::fs::list::branch::{BranchControl, read_branch};
use crate::fs::vfs::PanelSource;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Sorting / visibility options copied from `AppState` and the panel.
pub use crate::fs::list::ListOptions as ListingOptions;

/// Everything the background listing job needs; owned so it can cross threads.
#[derive(Debug, Clone)]
pub struct ListingRequest {
    pub path: PathBuf,
    pub source: PanelSource,
    pub options: ListingOptions,
    pub filter_mask: Option<String>,
    pub quick_filter_mask: Option<String>,
    /// Read per-entry attributes (owner, mode, links) for detailed views.
    pub want_attrs: bool,
    /// Branch view: every file of the subtree, named by relative path.
    pub branch: bool,
    /// A folder restored from the session (`state::restore_check`): when
    /// it cannot be read, list its nearest readable ancestor, else home.
    pub fallback: bool,
}

/// Progress of a listing job: files found so far (branch view only).
pub type ListingProgress = u64;

/// Git information for the listed directory.
#[derive(Debug, Clone, Default)]
pub struct PanelGitInfo {
    pub branch: String,
    /// Entry name -> status label ("M", "A", "?", …).
    pub statuses: HashMap<String, String>,
    /// The status could not be read: `statuses` is empty, but the
    /// repository is not known to be clean.
    pub status_failed: bool,
}

/// Result of a listing job.
#[derive(Debug)]
pub struct PanelListing {
    pub path: PathBuf,
    pub entries: Result<Vec<FileEntry>, String>,
    pub git: Option<PanelGitInfo>,
    pub free_space: Option<u64>,
    pub attrs: HashMap<PathBuf, FileAttrs>,
    /// Hidden files were listed (for a follow-up reread).
    pub show_hidden: bool,
    /// `path` was listed instead of this unreadable restored folder.
    pub fell_back_from: Option<PathBuf>,
}

/// Executes the request. Blocking: call only from a background job.
pub fn run(
    req: &ListingRequest,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(ListingProgress),
) -> PanelListing {
    let local = req.source.is_local();
    let (path, entries, fell_back_from) = read_with_fallback(req, cancelled, progress);
    let entries = entries.map(|entries| {
        let mut entries = apply_masks(
            entries,
            req.filter_mask.as_deref(),
            req.quick_filter_mask.as_deref(),
        );
        if local && fill_sort_keys(&mut entries, req.options.sort_field) {
            req.options.sort(&mut entries);
        }
        entries
    });
    let listed = entries.as_deref().unwrap_or(&[]);
    let git = if local && !cancelled() {
        git_info(&path, listed, req.branch)
    } else {
        None
    };
    let free_space = if local {
        crate::fs::transfer::network::get_free_space(&path).ok()
    } else {
        None
    };
    let attrs = if local && req.want_attrs && !cancelled() {
        read_attrs_map(listed)
    } else {
        HashMap::new()
    };
    PanelListing {
        path,
        entries,
        git,
        free_space,
        attrs,
        show_hidden: req.options.show_hidden,
        fell_back_from,
    }
}

/// Reads the asked folder; with `req.fallback`, an unreadable one gives
/// way to its nearest readable ancestor, else home. Returns the folder
/// listed, its entries and the folder it replaced, if any.
fn read_with_fallback(
    req: &ListingRequest,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(ListingProgress),
) -> (PathBuf, Result<Vec<FileEntry>, String>, Option<PathBuf>) {
    let entries = read_entries(req, &req.path, cancelled, progress);
    if entries.is_ok() || !req.fallback || !req.source.is_local() {
        return (req.path.clone(), entries, None);
    }
    let home = crate::app::session::restore::home_dir();
    let candidates = req.path.ancestors().skip(1).map(Path::to_path_buf);
    for folder in candidates
        .filter(|p| !p.as_os_str().is_empty())
        .chain([home])
    {
        if cancelled() {
            break;
        }
        if let Ok(found) = read_entries(req, &folder, cancelled, progress) {
            log::info!(
                "Session folder {} cannot be read; opening {}",
                req.path.display(),
                folder.display()
            );
            return (folder, Ok(found), Some(req.path.clone()));
        }
    }
    (req.path.clone(), entries, None)
}

fn read_entries(
    req: &ListingRequest,
    path: &Path,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(ListingProgress),
) -> Result<Vec<FileEntry>, String> {
    let o = &req.options;
    let vfs = req.source.vfs();
    let entries = if req.branch {
        let ctl = BranchControl {
            cancelled,
            progress,
        };
        read_branch(vfs.as_ref(), path, o, &ctl)
    } else {
        vfs.read_panel(path, o)
    };
    entries.map_err(|e| e.to_string())
}

/// Fills in what sorting by owner or description needs (left out of a
/// plain listing as it costs a stat or a file read). Returns whether it
/// did, so the entries are sorted again.
fn fill_sort_keys(entries: &mut [FileEntry], field: crate::fs::list::SortField) -> bool {
    use crate::fs::descriptions::{find_description, read_descriptions_file};
    if field.needs_owner() {
        for entry in entries.iter_mut().filter(|e| e.name != "..") {
            let meta = std::fs::symlink_metadata(&entry.path).ok();
            entry.owner = meta.as_ref().and_then(crate::fs::attrs::owner_of);
        }
        return true;
    }
    if field.needs_description() {
        // One read of `descript.ion` per folder (a branch view spans many).
        let mut files: HashMap<PathBuf, Option<String>> = HashMap::new();
        for entry in entries.iter_mut().filter(|e| e.name != "..") {
            let (Some(dir), Some(name)) = (entry.path.parent(), entry.path.file_name()) else {
                continue;
            };
            let content = files
                .entry(dir.to_path_buf())
                .or_insert_with(|| read_descriptions_file(dir));
            entry.description = content
                .as_deref()
                .and_then(|c| find_description(c, &name.to_string_lossy()))
                .map(str::to_string);
        }
        return true;
    }
    false
}

fn read_attrs_map(entries: &[FileEntry]) -> HashMap<PathBuf, FileAttrs> {
    entries
        .iter()
        .filter(|e| e.name != "..")
        .filter_map(|e| {
            crate::fs::attrs::read_attrs(&e.path)
                .ok()
                .map(|a| (e.path.clone(), a))
        })
        .collect()
}

/// Branch name plus per-entry status for `path`, scoped with a pathspec to
/// the listed directory so large repositories are not scanned entirely.
/// A branch view (`branch_view`) keys the statuses by the whole relative path.
fn git_info(path: &Path, entries: &[FileEntry], branch_view: bool) -> Option<PanelGitInfo> {
    let repo = crate::git::repo::find_repo(path)?;
    let branch = branch_label(&repo);
    let workdir = repo.workdir()?.to_path_buf();
    let pathspec = git_pathspec_for(&workdir, path);
    let (statuses, status_failed) =
        match crate::git::status::get_status_scoped(&repo, pathspec.as_deref()) {
            Ok(statuses) => (statuses, false),
            Err(e) => {
                log::warn!("Reading the Git status of {} failed: {e}", path.display());
                (Vec::new(), true)
            }
        };

    let norm_path = normalize(path);
    let mut map = HashMap::new();
    for s in statuses {
        let norm_full = normalize(&workdir.join(&s.path));
        let Some(suffix) = child_suffix(&norm_full, &norm_path) else {
            continue;
        };
        let entry_name = if branch_view {
            Some(suffix.replace('/', std::path::MAIN_SEPARATOR_STR))
        } else {
            suffix.split('/').next().map(str::to_string)
        };
        let Some(entry_name) = entry_name.filter(|n| !n.is_empty()) else {
            continue;
        };
        let actual_name = entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(&entry_name))
            .map(|e| e.name.clone())
            .unwrap_or(entry_name);
        map.entry(actual_name)
            .or_insert_with(|| s.kind.label().to_string());
    }
    Some(PanelGitInfo {
        branch,
        statuses: map,
        status_failed,
    })
}

fn branch_label(repo: &git2::Repository) -> String {
    use crate::config::localization::t;
    if repo.head_detached().unwrap_or(false) {
        return t("git_detached_head");
    }
    if let Ok(head) = repo.head()
        && let Ok(name) = head.shorthand()
    {
        return name.to_string();
    }
    if let Ok(head_ref) = repo.find_reference("HEAD")
        && let Some(target) = head_ref.symbolic_target().ok().flatten()
        && let Some(b) = target.strip_prefix("refs/heads/")
    {
        return b.to_string();
    }
    t("git_detached_head")
}

/// Part of `full` below directory `dir` (both normalized), without the
/// leading slash; `None` when `full` is not inside `dir`.
pub(super) fn child_suffix<'a>(full: &'a str, dir: &str) -> Option<&'a str> {
    let suffix = full.strip_prefix(dir)?;
    if dir.ends_with('/') || suffix.is_empty() {
        Some(suffix)
    } else {
        suffix.strip_prefix('/')
    }
}

/// Lower-cased, forward-slash path string used for prefix comparisons.
pub(super) fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").to_lowercase()
}
