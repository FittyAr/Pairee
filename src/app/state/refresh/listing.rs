//! Off-thread directory listing: everything a panel needs after a (re)read.
//!
//! A [`ListingRequest`] is a plain snapshot built on the UI thread; [`run`]
//! executes on the blocking pool (see `app::jobs`) and returns a
//! [`PanelListing`] that the UI applies in one go.

use super::filter::{apply_masks, git_pathspec_for};
use crate::app::state::SortField;
use crate::fs::FileEntry;
use crate::fs::attrs::FileAttrs;
use crate::fs::ssh::SharedSshClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Sorting / visibility options copied from `AppState` and the panel.
#[derive(Debug, Clone)]
pub struct ListingOptions {
    pub show_hidden: bool,
    pub case_sensitive: bool,
    pub natural: bool,
    pub req_admin: bool,
    pub sort_field: SortField,
    pub sort_reverse: bool,
    pub folder_by_ext: bool,
    pub show_dotdot: bool,
}

/// Everything the background listing job needs; owned so it can cross threads.
#[derive(Debug, Clone)]
pub struct ListingRequest {
    pub path: PathBuf,
    pub ssh: Option<SharedSshClient>,
    pub options: ListingOptions,
    pub filter_mask: Option<String>,
    pub quick_filter_mask: Option<String>,
    /// Read per-entry attributes (owner, mode, links) for detailed views.
    pub want_attrs: bool,
}

/// Git information for the listed directory.
#[derive(Debug, Clone, Default)]
pub struct PanelGitInfo {
    pub branch: String,
    /// Entry name -> status label ("M", "A", "?", …).
    pub statuses: HashMap<String, String>,
}

/// Result of a listing job.
#[derive(Debug)]
pub struct PanelListing {
    pub path: PathBuf,
    pub entries: Result<Vec<FileEntry>, String>,
    pub git: Option<PanelGitInfo>,
    pub free_space: Option<u64>,
    pub attrs: HashMap<PathBuf, FileAttrs>,
}

/// Executes the request. Blocking: call only from a background job.
pub fn run(req: &ListingRequest, cancelled: &dyn Fn() -> bool) -> PanelListing {
    let entries = read_entries(req).map(|entries| {
        apply_masks(
            entries,
            req.filter_mask.as_deref(),
            req.quick_filter_mask.as_deref(),
        )
    });
    let local = req.ssh.is_none();
    let listed = entries.as_deref().unwrap_or(&[]);
    let git = if local && !cancelled() {
        git_info(&req.path, listed)
    } else {
        None
    };
    let free_space = if local {
        crate::app::sys_helpers::get_free_space(&req.path)
    } else {
        None
    };
    let attrs = if local && req.want_attrs && !cancelled() {
        read_attrs_map(listed)
    } else {
        HashMap::new()
    };
    PanelListing {
        path: req.path.clone(),
        entries,
        git,
        free_space,
        attrs,
    }
}

fn read_entries(req: &ListingRequest) -> Result<Vec<FileEntry>, String> {
    let o = &req.options;
    let res = match &req.ssh {
        Some(client) => client.read_directory(
            &req.path,
            o.show_hidden,
            o.case_sensitive,
            o.natural,
            o.sort_field,
            o.sort_reverse,
            o.show_dotdot,
        ),
        None => crate::fs::read_directory_ext(
            &req.path,
            o.show_hidden,
            o.case_sensitive,
            o.natural,
            o.req_admin,
            o.sort_field,
            o.sort_reverse,
            o.folder_by_ext,
            o.show_dotdot,
        ),
    };
    res.map_err(|e| e.to_string())
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
fn git_info(path: &Path, entries: &[FileEntry]) -> Option<PanelGitInfo> {
    let repo = crate::git::repo::find_repo(path)?;
    let branch = branch_label(&repo);
    let workdir = repo.workdir()?.to_path_buf();
    let pathspec = git_pathspec_for(&workdir, path);
    let statuses = crate::git::status::get_status_scoped(&repo, pathspec.as_deref());

    let norm_path = normalize(path);
    let mut map = HashMap::new();
    for s in statuses {
        let norm_full = normalize(&workdir.join(&s.path));
        let Some(suffix) = child_suffix(&norm_full, &norm_path) else {
            continue;
        };
        let entry_name = suffix.split('/').next();
        let Some(entry_name) = entry_name.filter(|n| !n.is_empty()) else {
            continue;
        };
        let actual_name = entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(entry_name))
            .map(|e| e.name.clone())
            .unwrap_or_else(|| entry_name.to_string());
        map.entry(actual_name)
            .or_insert_with(|| s.kind.label().to_string());
    }
    Some(PanelGitInfo {
        branch,
        statuses: map,
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
