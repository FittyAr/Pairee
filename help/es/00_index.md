# 📚 Ayuda de Pairee — Índice de Documentación

> Bienvenido a **Pairee**, el gestor de archivos de terminal de doble panel.
> Este índice agrupa todos los documentos de ayuda según su propósito, para
> que vayas directo a lo que necesitás.

Los iconos de abajo marcan cada documento con el **cuadrante de Diátaxis**
al que pertenece. Elegí el que coincida con lo que querés hacer *ahora*:

| Si querés… | Cuadrante | Ir a |
| --- | --- | --- |
| Aprender Pairee desde cero, paso a paso | 🎓 **TUTORIAL** | [`10_tutorial_getting_started`](10_tutorial_getting_started.md) |
| Entender el diseño de doble panel y el sistema de pantallas | 🎓 **TUTORIAL** | [`11_tutorial_panels_and_screens`](11_tutorial_panels_and_screens.md) |
| Copiar, mover, borrar, hacer wipe, links o cambiar atributos | 🔧 **HOW-TO** | [`20_howto_file_operations`](20_howto_file_operations.md) |
| Buscar archivos, filtrar el panel, ver historial, saltar al hotlist | 🔧 **HOW-TO** | [`21_howto_search_filter_history`](21_howto_search_filter_history.md) |
| Comprimir, extraer o ejecutar comandos de archivo | 🔧 **HOW-TO** | [`22_howto_archives`](22_howto_archives.md) |
| Conectarte a un host remoto por SSH/SFTP y transferir archivos | 🔧 **HOW-TO** | [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md) |
| Administrar un repositorio Git desde el dashboard | 🔧 **HOW-TO** | [`24_howto_git_integration`](24_howto_git_integration.md) |
| Cambiar temas, layouts y grupos de colores | 🔧 **HOW-TO** | [`25_howto_appearance_themes`](25_howto_appearance_themes.md) |
| Cambiar los presets de keymap o personalizar el User Menu (`F2`) | 🔧 **HOW-TO** | [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md) |
| Mapear máscaras de archivo a comandos de apertura | 🔧 **HOW-TO** | [`27_howto_file_associations`](27_howto_file_associations.md) |
| Instalar, confiar, fijar, actualizar o escribir plugins | 🔧 **HOW-TO** | [`28_howto_plugins`](28_howto_plugins.md) |
| Compilar desde código fuente, instalar, actualizar seguro | 🔧 **HOW-TO** | [`29_howto_install_build_update`](29_howto_install_build_update.md) |
| Hacer que las teclas `Ctrl`/`Alt` funcionen sobre SSH | 🔧 **HOW-TO** | [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md) |
| Buscar un atajo de teclado o una ranura F-key | 📖 **REFERENCE** | [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md) |
| Buscar un campo de configuración, pestaña por pestaña | 📖 **REFERENCE** | [`41_reference_configuration`](41_reference_configuration.md) |
| Buscar el esquema TOML de temas o nombres de colores | 📖 **REFERENCE** | [`42_reference_themes`](42_reference_themes.md) |
| Buscar el enum completo de `Action` o escribir un keymap | 📖 **REFERENCE** | [`43_reference_actions`](43_reference_actions.md) |
| Buscar los campos del diálogo SSH u operaciones SFTP | 📖 **REFERENCE** | [`44_reference_ssh_fields`](44_reference_ssh_fields.md) |
| Buscar la API Lua `pairee.*` para autores de plugins | 📖 **REFERENCE** | [`45_reference_plugins_api`](45_reference_plugins_api.md) |
| Entender cómo funcionan el async filesystem y las pantallas | 💡 **EXPLANATION** | [`50_explanation_architecture`](50_explanation_architecture.md) |
| Entender el modelo de confianza de plugins y el sandbox Lua | 💡 **EXPLANATION** | [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md) |
| Entender el sistema de actualización y los 13 métodos de instalación | 💡 **EXPLANATION** | [`52_explanation_update_system`](52_explanation_update_system.md) |

---

## Cómo funciona el popup de ayuda

Apretá **`F1`** en cualquier parte de Pairee para abrir esta documentación
dentro de la app. El popup tiene **dos pestañas**:

- **Core Help** — todos los archivos `.md` de esta carpeta, ordenados alfabéticamente.
- **Plugins Help** — el archivo `help/<lang>.md` dentro de cada plugin instalado.

Usá **`Up` / `Down`** (o `j` / `k`) para moverte por la lista, **`Enter`**
para abrir el documento resaltado, **`PageUp` / `PageDown`** para hacer
scroll en textos largos, **`Backspace`** para volver a la lista, y **`Esc`**
para cerrar el popup.

El primer archivo que se muestra siempre es este índice (`00_index.md`).

---

## Si solo leés un documento…

…leé [`10_tutorial_getting_started`](10_tutorial_getting_started.md).
Te lleva por install → launch → primer tour en menos de diez minutos.
