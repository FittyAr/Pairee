//! Extended attributes and POSIX ACLs of copies on Unix (FEAT-12).
//!
//! * "Copy attributes" copies the extended attributes (`user.*`, macOS
//!   `com.apple.*` such as Finder info and quarantine, `security.*` where
//!   the process may set them), except the ACLs.
//! * "Copy access control list" copies the POSIX ACLs, which Linux stores
//!   as the `system.posix_acl_access` / `system.posix_acl_default`
//!   attributes. macOS keeps ACLs outside the attribute namespace; they are
//!   not copied there.
//!
//! Both are best effort: a file system without attributes, or one the
//! process may not set, leaves the copy without them (the data is intact).

use std::ffi::OsStr;
use std::path::Path;

/// Prefix of the attributes Linux keeps POSIX ACLs in.
const ACL_PREFIX: &str = "system.posix_acl_";

/// `true` for the attributes that hold an ACL.
fn is_acl(name: &OsStr) -> bool {
    name.to_str().is_some_and(|n| n.starts_with(ACL_PREFIX))
}

/// Copies the attributes of `src` chosen by `wanted` to `dst`.
fn copy_matching(src: &Path, dst: &Path, wanted: impl Fn(&OsStr) -> bool) {
    let Ok(names) = xattr::list(src) else {
        return;
    };
    for name in names.filter(|n| wanted(n)) {
        if let Ok(Some(value)) = xattr::get(src, &name) {
            let _ = xattr::set(dst, &name, &value);
        }
    }
}

/// The extended attributes of `src`, ACLs left out.
pub fn copy_extended_attributes(src: &Path, dst: &Path) {
    copy_matching(src, dst, |name| !is_acl(name));
}

/// The POSIX ACLs of `src` (access, and default for folders).
pub fn copy_posix_acl(src: &Path, dst: &Path) {
    copy_matching(src, dst, is_acl);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_attributes_are_told_apart() {
        assert!(is_acl(OsStr::new("system.posix_acl_access")));
        assert!(is_acl(OsStr::new("system.posix_acl_default")));
        assert!(!is_acl(OsStr::new("user.comment")));
        assert!(!is_acl(OsStr::new("security.selinux")));
    }

    /// User attributes travel with "copy attributes" (skipped where the
    /// file system of the temporary folder has none, e.g. tmpfs without
    /// `user_xattr`).
    #[test]
    fn user_attributes_are_copied() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::write(&src, b"x").unwrap();
        std::fs::write(&dst, b"x").unwrap();
        if xattr::set(&src, "user.pairee.test", b"value").is_err() {
            return;
        }
        copy_extended_attributes(&src, &dst);
        let copied = xattr::get(&dst, "user.pairee.test").unwrap();
        assert_eq!(copied.as_deref(), Some(&b"value"[..]));
    }
}
