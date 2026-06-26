---
name: changelog-helper
description: Guide AI agents in documenting changes in CHANGELOG.md following the "Keep a Changelog" standard.
---

# Changelog Helper Skill

Use this skill whenever you perform code modifications, add new features, or fix bugs, to ensure that user-facing changes are correctly documented in [CHANGELOG.md](file:///home/fitty/GitHub/Pairee/CHANGELOG.md).

## Keep a Changelog Standards

The project follows the [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/) format.

All modifications must be listed under the `## [Unreleased]` section. Do not create new version sections unless explicitly requested or during a release process.

## Classification of Changes

Categorize your changes using one of the following subheadings under `## [Unreleased]`:

- **`Added`**: For new features (e.g. new views, new config options, new interactive dialogs).
- **`Changed`**: For changes in existing functionality (e.g. refactoring UI elements, renaming files, bumping dependencies).
- **`Deprecated`**: For soon-to-be removed features.
- **`Removed`**: For now removed features.
- **`Fixed`**: For bug fixes (e.g. fixing a UI layout cutting off, resolving panic states, correcting path handling).
- **`Improved`**: For performance enhancements, UX polish, or styling improvements.

## Rules for Adding Entries

1. **Be Concise and Clear**: Write short, descriptive bullet points explaining *what* changed from the user's perspective.
2. **Use Plain Language**: Avoid internal Rust jargon or overly technical implementation details (e.g. write "MkDir dialog input behavior fixed" instead of "fixed tokio channel receiver in app/state/mkdir.rs").
3. **Consistency**: Start each bullet point with a capitalized letter, and end with a period.

## Example

```markdown
## [Unreleased]

### Added

- Connection status indicator in the upper-right corner of the panels.

### Fixed

- Input field inside the MkDir dialog is now properly focused by default.
```
