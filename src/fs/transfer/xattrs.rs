//! Extended attributes and POSIX ACLs of copies on Unix (FEAT-12).
//!
//! * "Copy attributes" copies the extended attributes (`user.*`, macOS
//!   `com.apple.*` such as Finder info and quarantine, `security.*` where
//!   the process may set them), except the ACLs.
//! * "Copy access control list" copies the ACLs: on Linux the POSIX ACLs,
//!   stored as the `system.posix_acl_access` / `system.posix_acl_default`
//!   attributes; on macOS the extended (NFSv4-style) ACL, kept outside the
//!   attribute namespace and copied with `acl_get_file` / `acl_set_file`
//!   of libSystem.
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

/// The ACLs of `src`: the POSIX ACLs (access, and default for folders) on
/// Linux and the other systems that keep them as attributes, the extended
/// ACL on macOS.
pub fn copy_posix_acl(src: &Path, dst: &Path) {
    #[cfg(target_os = "macos")]
    macos_acl::copy(src, dst);
    #[cfg(not(target_os = "macos"))]
    copy_matching(src, dst, is_acl);
}

/// The macOS extended ACL through libSystem (`man 3 acl_get_file`).
#[cfg(target_os = "macos")]
mod macos_acl {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// `acl_type_t ACL_TYPE_EXTENDED` of `<sys/acl.h>`.
    const ACL_TYPE_EXTENDED: c_int = 0x0000_0100;

    unsafe extern "C" {
        fn acl_get_file(path: *const c_char, kind: c_int) -> *mut c_void;
        fn acl_set_file(path: *const c_char, kind: c_int, acl: *mut c_void) -> c_int;
        fn acl_free(obj: *mut c_void) -> c_int;
    }

    fn c_path(path: &Path) -> Option<CString> {
        CString::new(path.as_os_str().as_bytes()).ok()
    }

    /// Best effort: no ACL on `src`, or one `dst` cannot take, leaves
    /// `dst` as it is.
    pub fn copy(src: &Path, dst: &Path) {
        let (Some(src), Some(dst)) = (c_path(src), c_path(dst)) else {
            return;
        };
        // SAFETY: both paths are NUL-terminated C strings that outlive the
        // calls; the ACL returned by `acl_get_file` is only used while it
        // is alive and freed once with `acl_free`.
        unsafe {
            let acl = acl_get_file(src.as_ptr(), ACL_TYPE_EXTENDED);
            if acl.is_null() {
                return;
            }
            let _ = acl_set_file(dst.as_ptr(), ACL_TYPE_EXTENDED, acl);
            acl_free(acl);
        }
    }
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
