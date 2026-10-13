//! License of a plugin being published: a `LICENSE` file without a license
//! name in the manifest asks for one; no `LICENSE` file gets an MIT license.

use crate::plugin::loader::PluginManifest;
use std::path::Path;

/// Asks the user for a license name (the CLI reads it from the terminal;
/// the TUI has no prompt and publishes "Custom").
pub type AskLicense<'a> = Option<&'a dyn Fn() -> String>;

/// Decides the license to publish with, writing `LICENSE` when missing.
/// `None` keeps the manifest as it is.
pub(super) fn resolve_license(
    plugin_dir: &Path,
    manifest: &PluginManifest,
    ask: AskLicense<'_>,
) -> anyhow::Result<Option<String>> {
    if !has_license_file(plugin_dir) {
        // No license file present. Auto-assign MIT
        let author_name = manifest.author.as_deref().unwrap_or("unknown");
        std::fs::write(plugin_dir.join("LICENSE"), mit_license(author_name))?;
        return Ok(Some("MIT".to_string()));
    }
    let named = manifest
        .license
        .as_ref()
        .is_some_and(|l| !l.trim().is_empty());
    if named {
        return Ok(manifest.license.clone());
    }
    let answer = ask.map(|ask| ask()).unwrap_or_default();
    let answer = answer.trim();
    Ok(Some(if answer.is_empty() {
        "Custom".to_string()
    } else {
        answer.to_string()
    }))
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
