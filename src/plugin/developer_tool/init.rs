use super::{TEMPLATE_BRANCH, find_pairee_repo, progress_status};
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

/// `text` escaped for the inside of a TOML basic string (`"..."`).
fn toml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn replace_placeholders(
    target_path: &std::path::Path,
    manifest_name: &str,
    description: &str,
    author: &str,
) -> anyhow::Result<()> {
    // Replace placeholders in manifest.toml
    let manifest_path = target_path.join("manifest.toml");
    if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
        // The placeholders sit inside TOML basic strings.
        let content = content
            .replace("PLUGIN_NAME", manifest_name)
            .replace("PLUGIN_DESCRIPTION", &toml_escape(description))
            .replace("PLUGIN_AUTHOR", &toml_escape(author));
        std::fs::write(&manifest_path, content)?;
    }

    // Replace placeholder in help/en.md
    let help_path = target_path.join("help").join("en.md");
    if help_path.exists() {
        let content = std::fs::read_to_string(&help_path)?;
        let content = content.replace("PLUGIN_NAME", manifest_name);
        std::fs::write(&help_path, content)?;
    }

    Ok(())
}

/// Clones the plugin template into `target_path`. The path must be
/// absolute; the caller is responsible for resolving the final
/// destination. We never change the process working directory here so
/// that other threads (the TUI loop, async tasks) are not affected.
fn clone_from_template(
    target_path: &std::path::Path,
    manifest_name: &str,
    description: &str,
    author: &str,
    progress: &Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<bool> {
    progress_status(progress, t("plugin_dev_progress_locating_template"));

    // 1. Try local repository first
    if let Some(repo_dir) = find_pairee_repo()
        && let Ok(repo) = git2::Repository::open(&repo_dir)
    {
        let branch_ref = format!("refs/heads/{}", TEMPLATE_BRANCH);
        if let Ok(reference) = repo.find_reference(&branch_ref)
            && let Ok(commit) = reference.peel_to_commit()
            && let Ok(tree) = commit.tree()
        {
            progress_status(progress, t("plugin_dev_progress_copying_template"));
            let walk_res = tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
                use git2::ObjectType;
                let rel_path = if root.is_empty() {
                    entry.name().unwrap_or("").to_string()
                } else {
                    format!("{}{}", root, entry.name().unwrap_or(""))
                };

                match entry.kind() {
                    Some(ObjectType::Blob) => {
                        let obj = match entry.to_object(&repo) {
                            Ok(o) => o,
                            Err(_) => return git2::TreeWalkResult::Ok,
                        };
                        let blob = match obj.as_blob() {
                            Some(b) => b,
                            None => return git2::TreeWalkResult::Ok,
                        };
                        // Reject entries that try to escape
                        // the target via parent-dir components.
                        // libgit2 tree paths are not user input
                        // here, but defense-in-depth is cheap.
                        if rel_path.split(['/', '\\']).any(|seg| seg == "..") {
                            log::warn!(
                                "plugin-template: refusing template entry \
                                             with parent-dir component: {}",
                                rel_path
                            );
                            return git2::TreeWalkResult::Ok;
                        }
                        let dest = target_path.join(&rel_path);
                        if let Some(parent) = dest.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&dest, blob.content());
                    }
                    Some(ObjectType::Tree) => {
                        let dir = target_path.join(&rel_path);
                        let _ = std::fs::create_dir_all(&dir);
                    }
                    _ => {}
                }
                git2::TreeWalkResult::Ok
            });

            if walk_res.is_ok() {
                progress_status(progress, t("plugin_dev_progress_replacing_placeholders"));
                replace_placeholders(target_path, manifest_name, description, author)?;
                log::info!("plugin-template: Plugin initialized from local git branch.");
                return Ok(true);
            }
        }
    }

    // 2. Fallback: Clone the remote `plugin-template` branch from GitHub
    log::debug!("plugin-template: Local repo/branch not found. Cloning from remote repository...");
    progress_status(progress, t("plugin_dev_progress_cloning_template"));
    let url = "https://github.com/FittyAr/Pairee.git";

    // Delete the directory if it already exists, as git2 clone expects the destination to not exist/be empty
    if target_path.exists() {
        let _ = std::fs::remove_dir_all(target_path);
    }

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch(TEMPLATE_BRANCH);

    // We clone the template branch directly to target_path
    match builder.clone(url, target_path) {
        Ok(_) => {
            // Remove the .git folder so it's not a git repository itself
            let git_dir = target_path.join(".git");
            if git_dir.exists() {
                let _ = std::fs::remove_dir_all(&git_dir);
            }

            progress_status(progress, t("plugin_dev_progress_replacing_placeholders"));
            replace_placeholders(target_path, manifest_name, description, author)?;
            log::info!("plugin-template: Plugin initialized from remote git branch.");
            Ok(true)
        }
        Err(e) => {
            log::warn!("plugin-template: Failed to clone remote template: {}", e);
            Ok(false)
        }
    }
}

/// CLI entry: creates the plugin in the current folder and returns the
/// success message for the caller to print.
pub fn init(name: &str, description: &str, author: &str) -> anyhow::Result<String> {
    // CLI entry: the process is single-threaded at this point, so reading
    // the cwd is safe. The interactive UI must NOT use this helper — it
    // should call `init_in` with an explicit `target_parent` so the
    // process-wide cwd is never mutated while other tasks are running.
    let target_parent = std::env::current_dir()?;
    init_in(name, description, author, None, &target_parent)
}

/// Initialises a new plugin inside `target_parent` (an absolute path).
///
/// Prefer this over [`init`] from any UI / multi-threaded context: the
/// process current working directory is never mutated, so concurrent
/// async tasks and the TUI render loop cannot accidentally observe the
/// change. The caller is responsible for passing an absolute
/// destination that the user actually wants to write into.
pub fn init_in(
    name: &str,
    description: &str,
    author: &str,
    progress: Option<UnboundedSender<DevProgress>>,
    target_parent: &std::path::Path,
) -> anyhow::Result<String> {
    let folder_name = if name.ends_with(".pairee") {
        name.to_string()
    } else {
        format!("{}.pairee", name)
    };
    let manifest_name = folder_name
        .strip_suffix(".pairee")
        .unwrap_or(&folder_name)
        .to_string();

    // The name becomes the folder and the manifest name: a plain
    // identifier, so it cannot point outside `target_parent`.
    super::check_identifier(&manifest_name)?;
    let path = target_parent.join(&folder_name);
    // Never write over (or, for the remote template, delete) existing work.
    if std::fs::read_dir(&path).is_ok_and(|mut entries| entries.next().is_some()) {
        anyhow::bail!(t("plugin_dev_err_target_not_empty").replace("{}", &path.to_string_lossy()));
    }
    std::fs::create_dir_all(&path)?;

    progress_status(&progress, t("plugin_dev_progress_creating_dir"));
    // Clone files from local `plugin-template` branch or fallback to cloning the remote branch from GitHub
    let used_template = clone_from_template(&path, &manifest_name, description, author, &progress)?;
    if !used_template {
        anyhow::bail!(t("plugin_dev_err_template_unavailable"));
    }

    Ok(t("plugin_dev_init_ok")
        .replace("{}", &manifest_name)
        .replace("{:?}", &path.to_string_lossy()))
}

/// Like [`init_in`] but always reports progress over the optional
/// `progress` channel. Currently the only call site is the
/// interactive plugin wizard, which threads the channel through to
/// the UI.
pub fn init_with_progress_in(
    name: &str,
    description: &str,
    author: &str,
    progress: Option<UnboundedSender<DevProgress>>,
    target_parent: &std::path::Path,
) -> anyhow::Result<String> {
    init_in(name, description, author, progress, target_parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_refuses_bad_names_and_existing_projects() {
        let dir = tempdir().unwrap();
        for bad in ["../evil", "a/b", "", "C:x", "with space"] {
            assert!(init_in(bad, "d", "a", None, dir.path()).is_err(), "{bad}");
        }
        let existing = dir.path().join("mine.pairee");
        std::fs::create_dir(&existing).unwrap();
        std::fs::write(existing.join("main.lua"), "work").unwrap();
        assert!(init_in("mine", "d", "a", None, dir.path()).is_err());
        assert_eq!(std::fs::read_to_string(existing.join("main.lua")).unwrap(), "work");
    }

    #[test]
    fn placeholders_are_escaped_for_toml() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("manifest.toml"),
            "name = \"PLUGIN_NAME\"\nversion = \"1\"\ndescription = \"PLUGIN_DESCRIPTION\"\nauthor = \"PLUGIN_AUTHOR\"\n",
        )
        .unwrap();
        replace_placeholders(dir.path(), "demo", "say \"hi\" \\ bye\n", "Jane \"J\" Doe").unwrap();
        let manifest =
            crate::plugin::loader::PluginManifest::read_from_dir(dir.path()).expect("valid TOML");
        assert_eq!(manifest.description.as_deref(), Some("say \"hi\" \\ bye\n"));
        assert_eq!(manifest.author.as_deref(), Some("Jane \"J\" Doe"));
    }

    #[test]
    fn test_remote_template_clone() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_remote.pairee");

        // Test remote cloning of template branch
        let ok = clone_from_template(
            &path,
            "test_remote",
            "A remote test plugin",
            "Test Author",
            &None,
        )
        .unwrap();

        if ok {
            assert!(path.join("manifest.toml").exists());
            assert!(path.join("main.lua").exists());
            assert!(path.join("lang/en.toml").exists());
            assert!(path.join("help/en.md").exists());
            assert!(path.join("icon.png").exists());
            assert!(path.join("screenshots/screenshot1.png").exists());

            // Check placeholders
            let manifest_content = std::fs::read_to_string(path.join("manifest.toml")).unwrap();
            assert!(manifest_content.contains("name = \"test_remote\""));
            assert!(manifest_content.contains("description = \"A remote test plugin\""));
            assert!(manifest_content.contains("author = \"Test Author\""));

            let help_content = std::fs::read_to_string(path.join("help/en.md")).unwrap();
            assert!(help_content.contains("# Help for test_remote Plugin"));
        } else {
            // If offline, it's expected to return false, but shouldn't panic
            tracing::info!("Offline: remote clone skipped.");
        }
    }
}
