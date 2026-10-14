# Pairee — Documentation Index

Developer and design documentation for **Pairee**.  
User-facing help loaded in-app (F1) lives under [`help/`](../help/).

**Status legend**

| Badge | Meaning |
|-------|---------|
| **Implemented** | Matches current code on `master` |
| **Partial** | Code exists; gaps remain (see notes) |
| **Planned** | Design only or incomplete |
| **Historical** | Useful context; may lag the code |

---

## Improvement tracking

| Doc | Status | Notes |
|-----|--------|-------|
| [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) | **Active** | A–F principales cerradas; checklist TTY manual WT/conhost/Linux pendiente |

---

## Product & process

| Doc | Status |
|-----|--------|
| [CHANGELOG.md](./CHANGELOG.md) | Implemented (releases) |
| [UNRELEASED.md](./UNRELEASED.md) | Implemented (staging) |
| [PRIVACY.md](./PRIVACY.md) | Implemented |
| [THREAT_MODEL.md](./THREAT_MODEL.md) | **Implemented** | Plugins, SSH, update, elevated helper |
| [i18n.md](./i18n.md) | **Implemented** | How to add a language; `scripts/check_translations.py` in lint and CI |
| [winget-submission-guide.md](./winget-submission-guide.md) | Implemented |

---

## Technical design

| Doc | Status | Notes |
|-----|--------|-------|
| [technical/architecture_en.md](./technical/architecture_en.md) | Partial | Core loop still accurate; module tree may lag |
| [technical/architecture_es.md](./technical/architecture_es.md) | Partial | Same as EN |
| [technical/transfer-engine-design.md](./technical/transfer-engine-design.md) | Partial | Engine under `src/fs/transfer/`; dual path with `ops_worker` remains |
| [technical/plugin-system-design.md](./technical/plugin-system-design.md) | Partial | Runtime exists; original banner was outdated |
| [technical/plugin-system-design-es.md](./technical/plugin-system-design-es.md) | Partial | Same as EN |
| [technical/plugin-roadmap.md](./technical/plugin-roadmap.md) | Partial | Dialogs/File/cx/Command done; G1–G14 leftovers remain |
| [technical/plugin-registry-spec.md](./technical/plugin-registry-spec.md) | Partial | |
| [plugin-dev-guide.md](./plugin-dev-guide.md) | Partial | |
| [plugin-dev-guide-es.md](./plugin-dev-guide-es.md) | Partial | |
| [api/lua/README.md](./api/lua/README.md) | **Implemented** | Lua API semver **1.3.0** |
| [api/lua/v1.md](./api/lua/v1.md) | **Implemented** | Inventario estable v1 |
| [technical/installer_guide.md](./technical/installer_guide.md) | Implemented | |
| [technical/microsoft-store-publishing.md](./technical/microsoft-store-publishing.md) | Planned / process | |
| [technical/keybindings-presets-plan.md](./technical/keybindings-presets-plan.md) | Historical | Presets, shortcuts dialog and plugin keys (ES) |
| [technical/mouse-support-plan.md](./technical/mouse-support-plan.md) | Historical | Mouse support plan (ES) |
| [technical/settings-audit.md](./technical/settings-audit.md) | Historical | Settings fields vs. runtime use |
| [technical/duplication-report.md](./technical/duplication-report.md) | Historical | How duplication is measured; what is left |
| [technical/audit-2026-10.md](./technical/audit-2026-10.md) | Historical | Full audit of 0.10.0 and its progress (ES) |

---

## In-app help (`help/`)

| Topic | English | Español |
|-------|---------|---------|
| Features | [help/en/features.md](../help/en/features.md) | [help/es/features.md](../help/es/features.md) |
| User guide | [help/en/user_guide.md](../help/en/user_guide.md) | [help/es/user_guide.md](../help/es/user_guide.md) |
| Keyboard | [help/en/keyboard_shortcuts.md](../help/en/keyboard_shortcuts.md) | [help/es/keyboard_shortcuts.md](../help/es/keyboard_shortcuts.md) |
| Plugins | [help/en/plugins.md](../help/en/plugins.md) | [help/es/plugins.md](../help/es/plugins.md) |
| Git | [help/en/git_integration.md](../help/en/git_integration.md) | [help/es/git_integration.md](../help/es/git_integration.md) |
| SSH/SFTP | [help/en/ssh_sftp.md](../help/en/ssh_sftp.md) | [help/es/ssh_sftp.md](../help/es/ssh_sftp.md) |
| Configuration | [help/en/configuration_details.md](../help/en/configuration_details.md) | [help/es/configuration_details.md](../help/es/configuration_details.md) |
| Coming from another program | [help/en/coming_from.md](../help/en/coming_from.md) | [help/es/coming_from.md](../help/es/coming_from.md) |
| Keys per preset | `help/en/keymap_<preset>.md` (norton, standard, neovim, yazi, vi) | `help/es/keymap_<preset>.md` |

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) and [`.agents/AGENTS.md`](../.agents/AGENTS.md) for architecture rules (SRP, no god files, decoupled state).
