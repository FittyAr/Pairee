# Microsoft Store listing — Pairee

Product ID **9PBSDNBTP2JZ** (`30176FittyDev.Pairee`). The text Partner Center
shows on the Store page, kept here so every submission starts from the same
source. The English text goes to the English listings (en-us, en-gb, en-au,
en-ca, en-in), the Spanish text to the Spanish ones (es-es, es-mx, es-ar,
es-co, es-cl, es-us): the languages of `lang/` and the `<Resource>` entries of
`manifests/msix/AppxManifest.xml`. Before each release, add "What's new in
X.Y.Z" and "Novedades de la X.Y.Z" (from `docs/CHANGELOG.md`): the
Microsoft Store workflow copies them into every listing, by these exact
headings (`microsoft-store-publishing.md`). Limits: description 10,000 characters,
"What's new" 1,500, 20 features of 200 characters, 7 search terms of 30
characters (21 words in all).

---

## English

### Description

Pairee is a fast dual-panel file manager for the terminal, inspired by Norton Commander and Far Manager and written in Rust. Two panels side by side, the keys you already know, and every long operation in the background so the interface never freezes.

FILES, EVERYWHERE
• Copy, move, rename, delete and wipe with a transfer panel that shows progress, speed, conflicts and retries; copies resume where they stopped after a crash or power cut, and overwritten files go to the Recycle Bin so a copy can be undone.
• Connect to SFTP, FTP, FTPS, WebDAV and S3 servers from the connection manager and copy between any two places, local or remote (even FTP → S3). Passwords are kept in the system keyring.
• Browse zip, tar (gz, bz2, xz) and 7z archives like folders: copy files in and out, create folders, edit and delete entries.

BUILT-IN TOOLS
• Editor and viewer with syntax highlighting for 29 languages, find and replace with regular expressions, and encodings.
• Compare files (unified or side by side) and folders, find duplicate files, search by name or content, branch view (every file below a folder in one list) and a frecency jump to your frequent folders.
• Git: status in the panels, staging, commits, stashes, branches, and fetch / pull / push over HTTPS or SSH.
• Built-in terminal screens, a command line with completion, a process manager and a user menu.

MAKE IT YOURS
• Keyboard presets for Norton Commander / Far, Windows Explorer / VS Code / Total Commander, Neovim and yazi; any key can be changed from a searchable list.
• Themes with semantic colors, full mouse support and a configuration dialog for every setting.
• Lua plugins with a versioned API, a plugin manager and a signed plugin registry; Secure Mode sandboxes plugins you do not trust.
• Interface and help in English and Spanish.

Pairee is free and open source (GPL-3.0): https://github.com/FittyAr/Pairee. It runs in Windows Terminal or any console; type `pairee` to start it.

### Short description

Fast dual-panel terminal file manager inspired by Norton Commander and Far Manager: remote servers, archives, editor, Git and plugins.

### What's new in 0.11.0

• Archives: tar and 7z can now be changed like zip; files and folders move into and out of archives.
• Copies resume after an interruption, also to and from SSH servers; replaced files go to the Recycle Bin.
• Editor: find and replace with regular expressions; syntax highlighting for 29 languages in the editor and viewer.
• New tools: compare files, duplicate finder, branch view and frecency folder jump.
• Connection manager for SFTP, FTP, FTPS, WebDAV and S3, with password prompts and keyring storage.
• Git fetch, pull and push over SSH.
• Plugins: enable/disable, settings form, richer previews, signed registry, API 1.3.
• Terminal screens, command line editing with completion, semantic theme colors, `pairee --help`.
• Many fixes in transfers, remotes, Git, the updater and the interface.

### Product features

- Two independent panels with tabs, sorting, filters and selection
- Background copy, move and delete with progress, conflicts and retries
- Interrupted copies resume; replaced files go to the Recycle Bin
- SFTP, FTP, FTPS, WebDAV and S3 connections, copy between any two places
- Zip, tar and 7z archives browsed and edited like folders
- Editor and viewer with syntax highlighting for 29 languages
- Find and replace with regular expressions
- Compare files and folders, find duplicates, search by name or content
- Git status, commits, stashes, branches, fetch, pull and push
- Keyboard presets: Norton Commander, Far, Explorer, VS Code, Neovim, yazi
- Themes, full mouse support and a searchable shortcut list
- Sandboxed Lua plugins with a signed plugin registry
- English and Spanish interface and help

### Search terms

file manager · norton commander · far manager · terminal · dual panel · sftp · rust

---

## Español

### Descripción

Pairee es un gestor de archivos de doble panel para la terminal, rápido, inspirado en Norton Commander y Far Manager y escrito en Rust. Dos paneles lado a lado, las teclas que ya conoces, y cada operación larga en segundo plano para que la interfaz nunca se congele.

ARCHIVOS, EN TODAS PARTES
• Copia, mueve, renombra, elimina y borra de forma segura con un panel de transferencias que muestra progreso, velocidad, conflictos y reintentos; las copias continúan donde quedaron tras un cuelgue o un corte de luz, y los archivos sobrescritos van a la Papelera de reciclaje para poder deshacer la copia.
• Conéctate a servidores SFTP, FTP, FTPS, WebDAV y S3 desde el administrador de conexiones y copia entre dos lugares cualesquiera, locales o remotos (incluso FTP → S3). Las contraseñas se guardan en el almacén de credenciales del sistema.
• Recorre archivos zip, tar (gz, bz2, xz) y 7z como carpetas: copia archivos hacia adentro y hacia afuera, crea carpetas, edita y elimina entradas.

HERRAMIENTAS INCLUIDAS
• Editor y visor con resaltado de sintaxis para 29 lenguajes, buscar y reemplazar con expresiones regulares, y codificaciones.
• Comparación de archivos (unificada o lado a lado) y de carpetas, buscador de duplicados, búsqueda por nombre o contenido, vista de rama (todos los archivos debajo de una carpeta en una lista) y salto a tus carpetas frecuentes.
• Git: estado en los paneles, preparación, commits, stashes, ramas, y fetch / pull / push por HTTPS o SSH.
• Pantallas de terminal integradas, línea de comandos con autocompletado, administrador de procesos y menú de usuario.

A TU MANERA
• Esquemas de teclado de Norton Commander / Far, Explorador de Windows / VS Code / Total Commander, Neovim y yazi; cualquier tecla se cambia desde una lista con búsqueda.
• Temas con colores semánticos, soporte completo de mouse y un diálogo de configuración para cada opción.
• Plugins en Lua con una API versionada, administrador de plugins y un registro de plugins firmado; el Modo seguro aísla los plugins en los que no confías.
• Interfaz y ayuda en español e inglés.

Pairee es libre y de código abierto (GPL-3.0): https://github.com/FittyAr/Pairee. Funciona en Windows Terminal o en cualquier consola; escribe `pairee` para iniciarlo.

### Descripción breve

Gestor de archivos de doble panel para la terminal, inspirado en Norton Commander y Far Manager: servidores remotos, archivos comprimidos, editor, Git y plugins.

### Novedades de la 0.11.0

• Archivos comprimidos: tar y 7z ahora se modifican como zip; archivos y carpetas se mueven hacia adentro y hacia afuera.
• Las copias continúan tras una interrupción, también hacia y desde servidores SSH; los archivos reemplazados van a la Papelera.
• Editor: buscar y reemplazar con expresiones regulares; resaltado de sintaxis para 29 lenguajes en el editor y el visor.
• Nuevas herramientas: comparar archivos, buscador de duplicados, vista de rama y salto a carpetas frecuentes.
• Administrador de conexiones SFTP, FTP, FTPS, WebDAV y S3, con pedido de contraseña y almacén de credenciales.
• Git fetch, pull y push por SSH.
• Plugins: activar/desactivar, formulario de configuración, vistas previas más ricas, registro firmado, API 1.3.
• Pantallas de terminal, edición de la línea de comandos con autocompletado, colores de tema semánticos, `pairee --help`.
• Muchas correcciones en transferencias, conexiones remotas, Git, el actualizador y la interfaz.

### Características del producto

- Dos paneles independientes con pestañas, orden, filtros y selección
- Copiar, mover y eliminar en segundo plano con progreso, conflictos y reintentos
- Las copias interrumpidas continúan; los reemplazados van a la Papelera
- Conexiones SFTP, FTP, FTPS, WebDAV y S3, copia entre dos lugares cualesquiera
- Archivos zip, tar y 7z recorridos y editados como carpetas
- Editor y visor con resaltado de sintaxis para 29 lenguajes
- Buscar y reemplazar con expresiones regulares
- Comparar archivos y carpetas, buscar duplicados, buscar por nombre o contenido
- Git: estado, commits, stashes, ramas, fetch, pull y push
- Teclados: Norton Commander, Far, Explorador, VS Code, Neovim, yazi
- Temas, soporte completo de mouse y lista de atajos con búsqueda
- Plugins en Lua aislados con un registro de plugins firmado
- Interfaz y ayuda en español e inglés

### Términos de búsqueda

gestor de archivos · norton commander · far manager · terminal · doble panel · sftp · rust
