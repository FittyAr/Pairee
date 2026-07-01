use super::{TEMPLATE_BRANCH, find_pairee_repo};
use crate::config::localization::t;

fn clone_from_template(
    target_path: &std::path::Path,
    manifest_name: &str,
    description: &str,
    author: &str,
) -> anyhow::Result<bool> {
    let Some(repo_dir) = find_pairee_repo() else {
        log::debug!("plugin-template: Pairee repo not found; using fallback.");
        return Ok(false);
    };

    let repo = match git2::Repository::open(&repo_dir) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("plugin-template: Could not open repo: {}", e);
            return Ok(false);
        }
    };

    let branch_ref = format!("refs/heads/{}", TEMPLATE_BRANCH);
    let reference = match repo.find_reference(&branch_ref) {
        Ok(r) => r,
        Err(_) => {
            log::debug!(
                "plugin-template: Branch '{}' not found; using fallback.",
                TEMPLATE_BRANCH
            );
            return Ok(false);
        }
    };

    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    // Walk every blob in the tree and write it to target_path
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
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
    })?;

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

    log::info!(
        "plugin-template: Plugin '{}' initialized from git branch '{}'.",
        manifest_name,
        TEMPLATE_BRANCH
    );
    Ok(true)
}

pub fn init(name: &str, description: &str, author: &str, print_output: bool) -> anyhow::Result<()> {
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

    // Primary: clone files from the `plugin-template` git branch
    let used_template = clone_from_template(&path, &manifest_name, description, author)?;
    if !used_template {
        anyhow::bail!("Failed to initialize plugin: template branch unavailable.");
    }

    if print_output {
        let ok_msg = t("plugin_dev_init_ok")
            .replace("{}", &manifest_name)
            .replace("{:?}", &format!("{:?}", path));
        println!("{}", ok_msg);
    }
    Ok(())
}
