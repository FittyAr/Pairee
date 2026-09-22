/// Creates a new tag pointing to `target` (commit hash, branch, or HEAD).
pub fn create_tag(
    repo: &git2::Repository,
    tag_name: &str,
    target: &str,
    message: Option<&str>,
) -> anyhow::Result<git2::Oid> {
    let obj = repo.revparse_single(target)?;
    if let Some(msg) = message {
        let sig = match repo.signature() {
            Ok(s) => s,
            Err(_) => git2::Signature::now("Pairee User", "pairee@localhost")?,
        };
        let oid = repo.tag(tag_name, &obj, &sig, msg, false)?;
        Ok(oid)
    } else {
        let oid = repo.tag_lightweight(tag_name, &obj, false)?;
        Ok(oid)
    }
}

/* Note: TagInfo, list_tags, and delete_tag will be activated in Phase 5 (Tags Tab)
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub target_oid: String,
    pub message: Option<String>,
}

pub fn list_tags(repo: &git2::Repository) -> anyhow::Result<Vec<TagInfo>> {
    let tag_names = repo.tag_names(None)?;
    let mut result = Vec::new();
    for name_opt in &tag_names {
        if let Ok(Some(name)) = name_opt {
            let refname = format!("refs/tags/{}", name);
            if let Ok(reference) = repo.find_reference(&refname) {
                let target_oid = reference
                    .target()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| {
                        reference
                            .peel_to_commit()
                            .map(|c| c.id().to_string())
                            .unwrap_or_default()
                    });
                let message = reference
                    .peel(git2::ObjectType::Tag)
                    .ok()
                    .and_then(|obj| {
                        obj.as_tag().and_then(|t| {
                            t.message().ok().flatten().map(ToString::to_string)
                        })
                    });
                result.push(TagInfo {
                    name: name.to_string(),
                    target_oid,
                    message,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

pub fn delete_tag(repo: &git2::Repository, tag_name: &str) -> anyhow::Result<()> {
    repo.tag_delete(tag_name)?;
    Ok(())
}
*/
