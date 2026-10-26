//! Who and where Pairee runs: the current user and group ids and names and
//! the host name, plus the `uid → name` / `gid → name` lookups the
//! attributes dialog shows.
//!
//! On Unix the names come from `/etc/passwd` and `/etc/group`, parsed once
//! per process (static musl builds have no NSS anyway). On Windows there are
//! no numeric ids; the user and host names come from the Win32 API.

#[cfg(unix)]
use std::collections::HashMap;

/// The entries of passwd(5) or group(5) content as `(id, fields)`: both
/// keep the numeric id in the third field.
#[cfg(any(unix, test))]
fn id_entries(content: &str) -> impl Iterator<Item = (u32, Vec<&str>)> {
    content.lines().filter_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        let id = fields.get(2)?.trim().parse::<u32>().ok()?;
        Some((id, fields))
    })
}

/// `id -> name` from passwd(5) or group(5) content: both keep the name in
/// the first field (first entry wins).
#[cfg(any(unix, test))]
fn parse_id_names(content: &str) -> std::collections::HashMap<u32, String> {
    let mut map = std::collections::HashMap::new();
    for (id, fields) in id_entries(content) {
        map.entry(id).or_insert_with(|| fields[0].to_string());
    }
    map
}

/// The home folder (sixth field) of user `uid` in passwd(5) content.
#[cfg(any(unix, test))]
fn parse_home(content: &str, uid: u32) -> Option<std::path::PathBuf> {
    id_entries(content)
        .find(|(id, _)| *id == uid)
        .and_then(|(_, fields)| fields.get(5).filter(|h| h.starts_with('/')).map(Into::into))
}

/// Home folder of user `uid` as `/etc/passwd` lists it (never `HOME`).
#[cfg(unix)]
pub fn home_of(uid: u32) -> Option<std::path::PathBuf> {
    parse_home(&std::fs::read_to_string("/etc/passwd").ok()?, uid)
}

#[cfg(unix)]
fn names_from(
    file: &str,
    cache: &'static std::sync::OnceLock<HashMap<u32, String>>,
    id: u32,
) -> Option<String> {
    cache
        .get_or_init(|| {
            std::fs::read_to_string(file)
                .map(|content| parse_id_names(&content))
                .unwrap_or_default()
        })
        .get(&id)
        .cloned()
}

/// Name of user `uid`, if `/etc/passwd` lists it.
#[cfg(unix)]
pub fn user_name_of(uid: u32) -> Option<String> {
    static PASSWD: std::sync::OnceLock<HashMap<u32, String>> = std::sync::OnceLock::new();
    names_from("/etc/passwd", &PASSWD, uid)
}

/// Name of group `gid`, if `/etc/group` lists it.
#[cfg(unix)]
pub fn group_name_of(gid: u32) -> Option<String> {
    static GROUP: std::sync::OnceLock<HashMap<u32, String>> = std::sync::OnceLock::new();
    names_from("/etc/group", &GROUP, gid)
}

/// Real user id of this process (`None` on Windows).
pub fn uid() -> Option<u32> {
    #[cfg(unix)]
    {
        // SAFETY: getuid has no preconditions and cannot fail.
        Some(unsafe { libc::getuid() })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Real group id of this process (`None` on Windows).
pub fn gid() -> Option<u32> {
    #[cfg(unix)]
    {
        // SAFETY: getgid has no preconditions and cannot fail.
        Some(unsafe { libc::getgid() })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Name of the user running Pairee.
pub fn user_name() -> Option<String> {
    #[cfg(unix)]
    {
        uid().and_then(user_name_of)
    }
    #[cfg(windows)]
    {
        windows_user_name()
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Name of the primary group of the user running Pairee (`None` on
/// Windows, which has no primary group in this sense).
pub fn group_name() -> Option<String> {
    #[cfg(unix)]
    {
        gid().and_then(group_name_of)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Name of this machine.
pub fn host_name() -> Option<String> {
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        // SAFETY: the buffer is valid for `buf.len()` bytes; gethostname
        // writes at most that many and we stop at the first NUL.
        let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
        if rc != 0 {
            return None;
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let name = String::from_utf8_lossy(&buf[..end]).into_owned();
        (!name.is_empty()).then_some(name)
    }
    #[cfg(windows)]
    {
        windows_host_name()
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Reads a Win32 string through a `(buffer, &mut len)` call that reports
/// the length it wrote (without the NUL) on success.
#[cfg(windows)]
fn read_wide(call: impl Fn(*mut u16, &mut u32) -> i32) -> Option<String> {
    let mut buf = vec![0u16; 256];
    let mut len = buf.len() as u32;
    if call(buf.as_mut_ptr(), &mut len) == 0 {
        return None;
    }
    let text = &buf[..(len as usize).min(buf.len())];
    let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
    let name = String::from_utf16_lossy(&text[..end]);
    (!name.is_empty()).then_some(name)
}

#[cfg(windows)]
fn windows_user_name() -> Option<String> {
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;
    // SAFETY: the buffer pointer and length describe a live buffer.
    read_wide(|buf, len| unsafe { GetUserNameW(buf, len) })
}

#[cfg(windows)]
fn windows_host_name() -> Option<String> {
    use windows_sys::Win32::System::SystemInformation::{
        ComputerNameDnsHostname, GetComputerNameExW,
    };
    // SAFETY: the buffer pointer and length describe a live buffer.
    read_wide(|buf, len| unsafe { GetComputerNameExW(ComputerNameDnsHostname, buf, len) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwd_and_group_lines_are_parsed_into_id_maps() {
        let map = parse_id_names(
            "root:x:0:0::/root:/bin/sh
bob:x:1000:1000::/home/bob:/bin/sh
alias:x:0:0::/:/bin/sh
#bad
",
        );
        assert_eq!(map.get(&0).map(String::as_str), Some("root"));
        assert_eq!(map.get(&1000).map(String::as_str), Some("bob"));
        assert_eq!(map.len(), 2);
        let groups = parse_id_names("wheel:x:10:alice,bob\nstaff::50:\n");
        assert_eq!(groups.get(&10).map(String::as_str), Some("wheel"));
        assert_eq!(groups.get(&50).map(String::as_str), Some("staff"));
    }

    #[test]
    fn the_home_folder_comes_from_the_first_entry_of_the_user() {
        let passwd = "root:x:0:0::/root:/bin/sh\nbob:x:1000:1000::/home/bob:/bin/sh\nalias:x:0:0::/:/bin/sh\nodd:x:7:7::relative:/bin/sh\n";
        assert_eq!(parse_home(passwd, 0), Some("/root".into()));
        assert_eq!(parse_home(passwd, 1000), Some("/home/bob".into()));
        assert_eq!(parse_home(passwd, 7), None);
        assert_eq!(parse_home(passwd, 42), None);
    }

    #[test]
    fn ids_exist_only_on_unix() {
        assert_eq!(uid().is_some(), cfg!(unix));
        assert_eq!(gid().is_some(), cfg!(unix));
        assert!(cfg!(unix) || group_name().is_none());
    }

    #[test]
    fn host_name_is_known() {
        let name = host_name().expect("host name");
        assert!(!name.is_empty() && !name.contains('\0'));
    }
}
