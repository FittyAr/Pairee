//! `docs/UNRELEASED.md` collects the notes of the next version; a release
//! stamps them into `docs/CHANGELOG.md` and resets the template.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::Result;
use crate::text::TextFile;

const CHANGELOG: &str = "docs/CHANGELOG.md";
const UNRELEASED: &str = "docs/UNRELEASED.md";
const UNRELEASED_HEADER: &str = "## [Unreleased]";
const TEMPLATE: [&str; 13] = [
    UNRELEASED_HEADER,
    "",
    "### Added",
    "",
    "### Improved",
    "",
    "### Changed",
    "",
    "### Deprecated",
    "",
    "### Removed",
    "",
    "### Fixed",
];

/// `cargo xtask changelog <vX.Y.Z|Unreleased>`: prints the section body.
pub fn print(args: &[String]) -> Result {
    let [version] = args else {
        return Err("usage: cargo xtask changelog <vX.Y.Z|Unreleased>".into());
    };
    let (path, header) = if version.eq_ignore_ascii_case("unreleased") {
        (UNRELEASED, UNRELEASED_HEADER.to_string())
    } else {
        (
            CHANGELOG,
            format!("## [v{}]", version.trim_start_matches('v')),
        )
    };
    let file = TextFile::load(path)?;
    let body = section(&file.lines, &header)
        .ok_or_else(|| format!("section '{header}' not found or empty in {path}"))?;
    println!("{body}");
    Ok(())
}

/// Whether `docs/UNRELEASED.md` has any note below its headings.
pub fn has_unreleased_notes() -> Result<bool> {
    Ok(TextFile::load(UNRELEASED)?
        .lines
        .iter()
        .any(|l| !l.trim().is_empty() && !l.starts_with('#')))
}

/// Moves the unreleased notes into the changelog as `## [vX.Y.Z] - <today>`,
/// above the newest release, and resets `docs/UNRELEASED.md`.
pub fn stamp(version: &str) -> Result {
    let mut unreleased = TextFile::load(UNRELEASED)?;
    let mut block = drop_empty_subsections(&unreleased.lines);
    let stamped = format!("## [v{version}] - {}", today_utc());
    match block.iter().position(|l| l.starts_with(UNRELEASED_HEADER)) {
        Some(i) => block[i] = stamped,
        None => {
            block.splice(0..0, [stamped, String::new()]);
        }
    }
    let separator = ["", "---", ""].map(String::from);

    let mut changelog = TextFile::load(CHANGELOG)?;
    match changelog.lines.iter().position(|l| is_release_header(l)) {
        Some(i) => {
            block.extend(separator);
            changelog.lines.splice(i..i, block);
        }
        None => {
            changelog.lines.extend(separator);
            changelog.lines.extend(block);
        }
    }
    changelog.save()?;

    unreleased.lines = TEMPLATE.map(String::from).to_vec();
    unreleased.save()
}

/// The lines under `header` up to the next `## [` heading, without the
/// surrounding blank lines; `None` when the section is missing or empty.
fn section(lines: &[String], header: &str) -> Option<String> {
    let start = lines.iter().position(|l| is_header(l, header))? + 1;
    let body: Vec<&str> = lines[start..]
        .iter()
        .take_while(|l| !l.starts_with("## ["))
        .map(|l| l.trim_end())
        .collect();
    let first = body.iter().position(|l| !l.is_empty())?;
    let last = body.iter().rposition(|l| !l.is_empty())?;
    Some(body[first..=last].join("\n"))
}

/// `header` alone or followed by a date (`## [v0.5.1] - 2026-06-25`).
fn is_header(line: &str, header: &str) -> bool {
    line.strip_prefix(header)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '-']))
}

fn is_release_header(line: &str) -> bool {
    line.strip_prefix("## [v")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
}

/// Removes `### Heading` blocks that contain only blank lines.
fn drop_empty_subsections(lines: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with("### ") {
            let end = lines[i + 1..]
                .iter()
                .position(|l| l.starts_with('#'))
                .map_or(lines.len(), |p| i + 1 + p);
            if lines[i + 1..end].iter().all(|l| l.trim().is_empty()) {
                i = end;
                continue;
            }
        }
        out.push(lines[i].clone());
        i += 1;
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out
}

/// Today's UTC date as `YYYY-MM-DD` (civil-from-days, H. Hinnant).
fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    #[test]
    fn section_stops_at_the_next_release() {
        let log = lines("# Log\n\n## [v1.1.0] - 2026-01-02\n\n- b  \n\n## [v1.0.0]\n- a\n");
        assert_eq!(section(&log, "## [v1.1.0]").as_deref(), Some("- b"));
        assert_eq!(section(&log, "## [v1.0.0]").as_deref(), Some("- a"));
        assert_eq!(section(&log, "## [v1.0]"), None);
    }

    #[test]
    fn empty_subsections_are_dropped() {
        let notes = lines("## [Unreleased]\n\n### Added\n\n- x\n\n### Changed\n\n### Fixed\n");
        assert_eq!(
            drop_empty_subsections(&notes),
            lines("## [Unreleased]\n\n### Added\n\n- x\n")
        );
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_735), (2026, 10, 9));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }
}
