use super::{TEMPLATE_BRANCH, find_pairee_repo, progress_status};
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

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
        let content = content
            .replace("PLUGIN_NAME", manifest_name)
            .replace("PLUGIN_DESCRIPTION", description)
            .replace("PLUGIN_AUTHOR", author);
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

fn clone_from_template(
    target_path: &std::path::Path,
    manifest_name: &str,
    description: &str,
    author: &str,
    progress: &Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<bool> {
    progress_status(progress, t("plugin_dev_progress_locating_template"));

    // 1. Try local repository first
    if let Some(repo_dir) = find_pairee_repo() {
        if let Ok(repo) = git2::Repository::open(&repo_dir) {
            let branch_ref = format!("refs/heads/{}", TEMPLATE_BRANCH);
            if let Ok(reference) = repo.find_reference(&branch_ref) {
                if let Ok(commit) = reference.peel_to_commit() {
                    if let Ok(tree) = commit.tree() {
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
                            log::info!(
                                "plugin-template: Plugin initialized from local git branch."
                            );
                            return Ok(true);
                        }
                    }
                }
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

pub fn init(
    name: &str,
    description: &str,
    author: &str,
    print_output: bool,
) -> anyhow::Result<()> {
    init_with_progress(name, description, author, print_output, None)
}

pub fn init_with_progress(
    name: &str,
    description: &str,
    author: &str,
    print_output: bool,
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<()> {
    let folder_name = if name.ends_with(".pairee") {
        name.to_string()
    } else {
        format!("{}.pairee", name)
    };
    let manifest_name = folder_name
        .strip_suffix(".pairee")
        .unwrap_or(&folder_name)
        .to_string();

    let path = std::env::current_dir()?.join(&folder_name);
    std::fs::create_dir_all(&path)?;

    progress_status(&progress, t("plugin_dev_progress_creating_dir"));
    // Clone files from local `plugin-template` branch or fallback to cloning the remote branch from GitHub
    let used_template = clone_from_template(&path, &manifest_name, description, author, &progress)?;
    if !used_template {
        anyhow::bail!(t("plugin_dev_err_template_unavailable"));
    }

    if print_output {
        let ok_msg = t("plugin_dev_init_ok")
            .replace("{}", &manifest_name)
            .replace("{:?}", &path.to_string_lossy());
        println!("{}", ok_msg);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
            println!("Offline: remote clone skipped.");
        }
    }
}
