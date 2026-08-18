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
| [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) | **Active** | A–B y F principal hechas; D casi cerrada; C resto / E |

---

## Product & process

| Doc | Status |
|-----|--------|
| [CHANGELOG.md](./CHANGELOG.md) | Implemented (releases) |
| [UNRELEASED.md](./UNRELEASED.md) | Implemented (staging) |
| [PRIVACY.md](./PRIVACY.md) | Implemented |
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
| [technical/plugin-dev-guide.md](../docs/plugin-dev-guide.md) | Partial | See also ES guide |
| [plugin-dev-guide.md](./plugin-dev-guide.md) | Partial | |
| [plugin-dev-guide-es.md](./plugin-dev-guide-es.md) | Partial | |
| [api/lua/README.md](./api/lua/README.md) | **Implemented** | Lua API semver **1.0.0** |
| [api/lua/v1.md](./api/lua/v1.md) | **Implemented** | Inventario estable v1 |
| [technical/installer_guide.md](./technical/installer_guide.md) | Implemented | |
| [technical/microsoft-store-publishing.md](./technical/microsoft-store-publishing.md) | Planned / process | |

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

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) and [`.agents/AGENTS.md`](../.agents/AGENTS.md) for architecture rules (SRP, no god files, decoupled state).
