# 🔧 How-To: Plugins

> **Cuadrante: HOW-TO** — *orientado a problemas, enfocado en instalar y administrar plugins.*

Los plugins de Pairee son pequeños scripts en **Lua** que extienden el
gestor de archivos con comandos nuevos, visores de archivos y hooks de
ciclo de vida. Corren en un sandbox seguro (mirá
[`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)).

Esta página cubre el **lado usuario** de los plugins. Si querés
**escribir** un plugin, mirá
[`45_reference_plugins_api`](45_reference_plugins_api.md) y la
[Guía del Desarrollador de Plugins](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md).

---

## Abrir el Plugin Manager

Ya no hay un binding `F11` por default para el manager. Usá:

| Disparador | Pasos |
| --- | --- |
| Menú | `F9` → `Files` → `Plugin commands` |
| Hotkey | `Shift+F11` (después de habilitar el dev mode) o asigná la tuya |

El manager tiene **tres pestañas**. Usá `Tab` para ciclar.

---

## Pestaña 1: Instalados

Lista todos los plugins cargados actualmente, con versión, autor, y
tres badges opcionales:

| Badge | Significado |
| --- | --- |
| `[P]` (**Pinned**) | Esta versión está bloqueada; el update global la saltea. |
| `[T]` (**Trusted**) | Tiene permisos extendidos (red, shell crudo). Los plugins sin esta etiqueta corren en un sandbox estricto. |
| `[▲]` | Hay un update disponible en el registro central. |

| Tecla | Efecto |
| --- | --- |
| `t` / `T` | Alterna **trust** del plugin resaltado. |
| `p` / `P` | Alterna **pin** en `plugins.lock`. |
| `u` | **Update** del plugin resaltado en background. Un toast confirma. |
| `U` | **Update all** de los plugins no pineados en batch. |
| `Del` / `d` / `D` | **Desinstala** el plugin resaltado. |

---

## Pestaña 2: Buscar en el Registro

Navega e instala plugins desde el registro central.

| Tecla | Efecto |
| --- | --- |
| `/` | Enfoca el input de búsqueda (el borde se pone amarillo). |
| Tipeá | Filtro en vivo. |
| `Backspace` | Edita. |
| `Enter` | Envía la consulta contra el índice remoto. |
| `i` / `I` | **Instala** el resultado resaltado. Descarga en background; un toast confirma. |

El registro está hosteado en el repositorio
[`FittyAr/Pairee`](https://github.com/FittyAr/Pairee) bajo la rama
huérfana `plugin-registry`.

---

## Pestaña 3: Herramientas de Desarrollador

Esta pestaña aparece **solo cuando** `plugins_developer_mode = true`
está en `Configuration → Language & Plugins`. El flujo completo de
desarrollador está documentado en la
[Plugin Developer Guide](https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md);
la versión corta es:

| # | Opción | Efecto |
| --- | --- | --- |
| 0 | **Seleccionar plugin activo** | Modal listando cada plugin dev detectado (escanea `plugins_dev_dir` y ambos paneles buscando un `manifest.toml`). |
| 1 | **Inicializar boilerplate** | Genera `manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`, `icon.png`, `screenshots/screenshot1.png` desde la rama `plugin-template`. Se deshabilita después de un init exitoso. |
| 2 | **Auditar (Lint)** | Corre auditorías de manifest y Lua (imports inseguros, llamadas no documentadas, etc.). |
| 3 | **Package** | Prepara un clon temporal local de la rama `plugin-registry`, embebe el SHA-256 de cada archivo en el manifest, y actualiza el `registry/index.toml` maestro. Auto-asigna la licencia MIT si no hay una presente. Muestra la ruta local del caché. |
| 4 | **Instalar plugin dev local** | Copia el plugin dev activo al directorio runtime y lo registra en `plugins.lock`. |
| 5 | **Enviar plugin (PR)** | Con un token de GitHub: forkea `FittyAr/Pairee`, pushea la rama, abre un PR. Sin token: imprime los comandos exactos de `git push`. **El token se mantiene en memoria**; nunca se escribe a disco ni a variables de entorno. |

---

## Trust y pinning

- **Trust** es por plugin. Confiá en un plugin cuando hayas leído su
  fuente y entiendas qué permisos necesita. Hasta que sea trusted, el
  plugin no puede spawmear subprocesses ni abrir sockets de red.
- **Pin** es por versión. Pineá un plugin cuando dependas de una
  versión específica (ej. para reproducibilidad). `U` saltea las
  entradas pineadas.

Mirá [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
para el modelo de seguridad completo.

---

## A dónde ir ahora

- Referencia de la API de plugins: [`45_reference_plugins_api`](45_reference_plugins_api.md)
- Modelo de sandbox y trust: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
- Guía del desarrollador: https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md
