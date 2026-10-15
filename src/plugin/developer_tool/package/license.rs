//! License of a plugin being published: a `LICENSE` file without a license
//! name in the manifest asks for one; no `LICENSE` file gets an MIT license.
//! The decision never touches the author's folder: a generated `LICENSE` is
//! returned as text and goes into the registry copy only.

use crate::plugin::loader::PluginManifest;
use std::path::Path;

/// Asks the user for a license name (the CLI reads it from the terminal;
/// the TUI has no prompt and publishes "Custom").
pub type AskLicense<'a> = Option<&'a dyn Fn() -> String>;

/// The license a plugin is published with.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct License {
    /// The `license` value for the published manifest.
    pub name: String,
    /// The text of a `LICENSE` file to add to the registry copy (the
    /// plugin has none of its own).
    pub generated_file: Option<String>,
}

/// Decides the license to publish with. Nothing is written.
pub(super) fn resolve_license(
    plugin_dir: &Path,
    manifest: &PluginManifest,
    ask: AskLicense<'_>,
) -> License {
    if !has_license_file(plugin_dir) {
        // No license file present. Auto-assign MIT
        let author_name = manifest.author.as_deref().unwrap_or("unknown");
        return License {
            name: "MIT".to_string(),
            generated_file: Some(mit_license(author_name)),
        };
    }
    let named = manifest.license.as_deref().map(str::trim);
    let name = match named {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => {
            let answer = ask.map(|ask| ask()).unwrap_or_default();
            match answer.trim() {
                "" => "Custom".to_string(),
                answer => answer.to_string(),
            }
        }
    };
    License {
        name,
        generated_file: None,
    }
}

/// Check for LICENSE file (case-insensitive)
fn has_license_file(plugin_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(plugin_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name_lower = entry.file_name().to_string_lossy().to_lowercase();
        matches!(
            name_lower.as_str(),
            "license" | "license.txt" | "license.md"
        )
    })
}

/// MIT license text for `author_name`, dated this year.
fn mit_license(author_name: &str) -> String {
    let current_year = chrono::Local::now().format("%Y").to_string();
    format!(
        "MIT License\n\nCopyright (c) {} {}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.\n",
        current_year, author_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(extra: &str) -> PluginManifest {
        PluginManifest::parse(&format!("name = \"demo\"\nversion = \"1\"\n{extra}")).unwrap()
    }

    #[test]
    fn a_plugin_without_a_license_file_gets_mit_without_touching_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let license = resolve_license(dir.path(), &manifest("author = \"Ana\"\n"), None);
        assert_eq!(license.name, "MIT");
        let text = license.generated_file.unwrap();
        assert!(text.starts_with("MIT License"));
        assert!(text.contains(" Ana\n"), "{text}");
        assert!(
            !dir.path().join("LICENSE").exists(),
            "the source is untouched"
        );
    }

    #[test]
    fn a_license_file_keeps_the_named_license_or_asks_for_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("License.md"), "terms").unwrap();
        let named = manifest("license = \"Apache-2.0\"\n");
        let kept = resolve_license(dir.path(), &named, None);
        assert_eq!(kept.name, "Apache-2.0");
        assert_eq!(kept.generated_file, None);

        let unnamed = manifest("");
        let ask = || " BSD-3-Clause ".to_string();
        let asked = resolve_license(dir.path(), &unnamed, Some(&ask));
        assert_eq!(asked.name, "BSD-3-Clause");
        let silent = resolve_license(dir.path(), &unnamed, None);
        assert_eq!(silent.name, "Custom", "the TUI has no prompt");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("License.md")).unwrap(),
            "terms"
        );
    }
}
