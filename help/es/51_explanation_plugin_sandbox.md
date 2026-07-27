# 💡 Explicación: Sandbox de Plugins y Modelo de Confianza

> **Cuadrante: EXPLANATION** — *orientado a entender.*

El sistema de plugins de Pairee está construido alrededor de un solo
principio: **los plugins no deberían poder hacer nada que el usuario no
haya permitido explícitamente**. Esta página explica los tres anillos
concéntricos de confianza, y qué permite cada uno.

---

## 1. Los tres anillos

```
┌──────────────────────────────────────────────────────────┐
│  Anillo 0: no confiable (default)                        │
│  - base, table, string, utf8, math                       │
│  - sin io, sin os, sin package, sin require              │
│  - sin spawn                                             │
│  - lecturas de archivos limitadas a dir del plugin +      │
│    workspace activo                                      │
└──────────────────────────────────────────────────────────┘
                          ▲  user clicks [T] (trust)
                          │
┌──────────────────────────────────────────────────────────┐
│  Anillo 1: confiable (por plugin, opt-in)                │
│  - todo lo del Anillo 0                                  │
│  - fs.spawn (sujeto a blacklist cuando Secure Mode)      │
│  - acceso a red a través de la API pública               │
│  - clipboard, diálogos, notificaciones                   │
└──────────────────────────────────────────────────────────┘
                          ▲  user enables Secure Mode
                          │
┌──────────────────────────────────────────────────────────┐
│  Anillo 2: Secure Mode (global)                          │
│  - blacklist de 27 comandos aplicada a fs.spawn          │
│  - acceso a filesystem restringido a workspace +          │
│    config + cache                                        │
│  - los plugins no pueden alcanzar paths arbitrarios      │
└──────────────────────────────────────────────────────────┘
```

El usuario controla cada anillo explícitamente:

- **Trust** es por plugin, se togglea apretando `T` en el Plugin
  Manager. Es sticky: una vez confiableado, el plugin se queda
  confiableado hasta que le saqués el tilde.
- **Secure Mode** es un flag global (`secure_mode = true` en
  `settings.toml` o en `Configuration → Plugins`). Aplica a todos los
  plugins, confiables o no.

---

## 2. Qué significa "no confiable"

Un plugin que **no** fue marcado como confiable corre con:

- Las libraries de Lua **base**, **table**, **string**, **utf8**, y
  **math**.
- Un `require` **acotado** que solo puede cargar módulos dentro del
  propio directorio del plugin.
- Llamadas a `pairee.*` que no tocan la red ni spawnean procesos.
- **Sin** `io`, **sin** `os`, **sin** `package`, **sin** `load`,
  **sin** `loadstring`, **sin** `dofile`, **sin** `loadfile`.
- **Sin** `pairee.fs.spawn`.
- **Sin** APIs de clipboard / notificación / diálogo (usá las formas
  estructuradas en `pairee.*` para esas — son seguras
  independientemente del trust).

La intención: un plugin malformado o malicioso puede leer y escribir
adentro de su propio directorio y del workspace activo, pero no puede
exfiltrar datos ni tomar el control del sistema.

---

## 3. Qué significa "confiable"

Apretar `T` sobre un plugin le otorga el **Anillo 1**:

- `pairee.fs.spawn(cmd, args, opts)` — corre un proceso hijo.
- `pairee.utils.target_os()` / `target_family()` — informativo.
- Acceso completo a los widgets de `pairee.ui.*`.
- Clipboard vía `pairee.clipboard.*`.
- Notificaciones y diálogos estructurados.

Un plugin confiable aún **no** puede leer o escribir fuera del
workspace + config + cache a menos que Secure Mode esté desactivado.

> Trust es por plugin. El Plugin Manager muestra un badge `[T]` al
> lado de los plugins confiables así la lista es auditable de un
> vistazo.

---

## 4. Qué significa "Secure Mode"

Cuando `secure_mode = true`:

- `pairee.fs.spawn` se chequea contra una **blacklist de 27 comandos**
  de herramientas de red, shells y runtimes de script. Ejemplos:
  `bash`, `sh`, `zsh`, `cmd`, `powershell`, `ssh`, `scp`, `sftp`,
  `nc`, `ncat`, `curl`, `wget`, `python`, `python3`, `ruby`, `perl`,
  `node`, `php`, `lua`, `lua5.x`, `awk`, `gawk`, `tclsh`, `wish`,
  `expect`, `socat`, `telnet`. Si el comando está en la lista, el
  spawn se rechaza y se escribe una línea de log.
- El acceso al filesystem está **restringido por path** al workspace
  activo + los directorios de config y cache del usuario. Reads y
  writes fuera de esos paths se deniegan.
- Incluso los plugins **confiables** están sujetos a estas reglas.
  Secure Mode es la capa "cinturón y tirantes".

> La blacklist es intencionalmente conservadora. Podés extenderla
> editando la configuración de Secure Mode en el source si lo
> necesitás (avanzado; consultá la guía de developer primero).

---

## 5. Por qué importa pinear

Pinear (`P` en el Plugin Manager) escribe la versión actual del plugin
en `plugins.lock`. Cuando corrés `U` (update all), las entradas
pineadas se saltean.

Esto te protege contra dos escenarios:

1. **Breaking changes.** Una nueva major version de un plugin cambia
   un comportamiento del que dependés. Pineá la versión en la que
   confiás; el pase de update no la puede tocar.
2. **Ataque a la supply chain.** Un registro de plugins es
   comprometido y pushea una versión con backdoor. Las entradas
   pineadas se quedan en la versión que revisaste; vos decidís
   cuándo upgredeás.

`plugins.lock` es un archivo TOML plano en tu carpeta de config;
podés editarlo a mano para control más fino.

---

## 6. Modelo de amenaza

| Amenaza | Mitigación |
| --- | --- |
| Plugin malicioso en el registro | Trust es opt-in; el Plugin Manager muestra el repo fuente. |
| Bugs de plugin que corrompen el workspace | Las operaciones de archivo sobre el workspace se loguean en `app.log`. |
| Plugin que exfiltra datos por red | `pairee.fs.spawn` y `pairee.net.*` requieren trust; Secure Mode agrega una blacklist de comandos. |
| Canal de update comprometido | Los updates están gateados por SHA-256 (mirá [`52_explanation_update_system`](52_explanation_update_system.md)). |
| Plugin se rompe entre versiones de Pairee | Cada plugin declara un `pairee_api_version`; los mismatches previenen la carga. |

---

## 7. Lo que podés hacer para hardenear tu setup

1. **No confíes** en plugins cuya fuente no hayas leído.
2. **Habilitá Secure Mode** si manejás datos sensibles.
3. **Pineá** las versiones de los plugins de los que dependés.
4. **Revisá** el Plugin Manager de vez en cuando para ver si hay
   badges `[T]` que ya no necesitás.
5. **Inspeccioná** `app.log` en la carpeta de cache si un plugin se
   comporta raro.

---

## A dónde ir ahora

- Guía de usuario de plugins: [`28_howto_plugins`](28_howto_plugins.md)
- Referencia de la API de plugins: [`45_reference_plugins_api`](45_reference_plugins_api.md)
- Arquitectura de plugins: https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md
