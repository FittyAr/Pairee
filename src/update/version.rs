//! Lenient version parsing shared by the self-updater (is a release newer?)
//! and the plugin loader (`min_pairee`).

/// Parses a version as full semver (pre-release and build metadata
/// included), or `X` / `X.Y` padded with zeros.
pub fn parse(v: &str) -> anyhow::Result<semver::Version> {
    let v = v.trim();
    let padded = match v.split(['-', '+']).next().unwrap_or(v).matches('.').count() {
        0 => format!("{v}.0.0"),
        1 => format!("{v}.0"),
        _ => v.to_string(),
    };
    semver::Version::parse(&padded).map_err(|e| anyhow::anyhow!("invalid version {v:?}: {e}"))
}

/// `true` when the release tag `candidate` (`v` prefix optional) is a
/// strictly newer version than `current`; `false` when either cannot be
/// parsed.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let tag = |v: &str| parse(v.trim().trim_start_matches('v'));
    match (tag(candidate), tag(current)) {
        (Ok(c), Ok(cur)) => c > cur,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_short_versions_parse() {
        assert_eq!(parse("0.4.1").unwrap(), semver::Version::new(0, 4, 1));
        assert_eq!(parse("2.3").unwrap(), semver::Version::new(2, 3, 0));
        assert_eq!(parse("1").unwrap(), semver::Version::new(1, 0, 0));
        assert_eq!(parse("1.0.0-beta").unwrap().pre.as_str(), "beta");
        assert!(parse("invalid").is_err());
    }

    #[test]
    fn newer_follows_semver_precedence() {
        assert!(is_newer("v0.5.0", "0.4.1"));
        assert!(!is_newer("v0.4.1", "0.4.1"));
        assert!(!is_newer("v0.3.0", "0.4.1"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        // A pre-release build is offered the final release (a numeric-only
        // comparison calls them equal).
        assert!(is_newer("v0.11.0", "0.11.0-beta"));
        assert!(!is_newer("v0.11.0-beta", "0.11.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }
}
