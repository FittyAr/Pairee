# 🔧 How-To: Teclas Modificadoras Ctrl / Alt sobre SSH

> **Cuadrante: HOW-TO** — *orientado a problemas.*

Esta página explica una particularidad de las sesiones de terminal
planas: cuando te conectás a Pairee por SSH, **la terminal solo
mandá un keystroke cuando la combinación está completa** (ej.
`Ctrl+F3` se manda como un solo evento). No manda eventos
*intermedios* cuando presionás o soltás `Ctrl` o `Alt` solos. Como
resultado, la barra F-key al final de Pairee no puede cambiar
automáticamente a la vista "Ctrl" o "Alt" cuando mantenés esas
teclas.

Pairee trae dos soluciones: **ciclado manual** y **X11 forwarding**.

---

## Solución 1: Ciclado manual (`Ctrl+P`)

No hace falta software de terceros. Apretá **`Ctrl+P`** (o `Ctrl+p`)
para ciclar la barra F-key a través de tres estados:

| Apretada | La barra muestra |
| --- | --- |
| 1ª | Fila **Ctrl**: F1 Left, F2 Right, F3 Name, F4 Extens, F5 Time, F6 Size, … |
| 2ª | Fila **Alt**: F1 Left drive, F2 Right drive, F3 View alt, F4 Edit alt, F5 Print, F6 Make link, F7 Find, F8 History, F9 Video, F10 Tree, F11 View hist, F12 Folders hist |
| 3ª | Fila **Default**: F1 Help, F2 User, F3 View, F4 Edit, F5 Copy, … |

La barra es puramente un **hint visual**. Los bindings reales
funcionan sin importar la fila mostrada — `Ctrl+F3` siempre va a
ordenar por nombre, `Alt+F1` siempre va a abrir el menú de drives
izquierdo.

> Esto funciona en **todas** las terminales, incluyendo SSH plano sin
> X11.

---

## Solución 2: X11 forwarding (tracking en vivo)

Si querés que la barra se **actualice en tiempo real** cuando
mantenés `Ctrl` o `Alt`, habilitá **X11 forwarding** en tu conexión
SSH. Pairee consultará tu servidor X local para leer el estado físico
de las teclas modificadoras.

> Esto es **opt-in**. Funciona además de `Ctrl+P`, nunca en su lugar.

### Host Windows

#### MobaXterm (lo más fácil)

MobaXterm incluye un servidor X integrado. Solo creá una nueva sesión
SSH — el X11 forwarding se configura automáticamente.

#### Windows Terminal / PowerShell / CMD con VcXsrv

1. Descargá e instalá **VcXsrv** (o **Xming**).
2. Lanzá **XLaunch** con:
   - Multiple windows
   - Display number: `0`
   - **Disable access control** ← requerido para permitir conexiones
     remotas.
3. Conectá con el cliente OpenSSH integrado:

   ```cmd
   ssh -Y user@hostname -p port
   ```

#### PuTTY

1. Abrí la configuración de la sesión.
2. **Connection → SSH → X11**.
3. Tildá **Enable X11 forwarding**.
4. Seteá **X display location** a `localhost:0`.
5. Asegurate de que VcXsrv (o Xming) está corriendo de fondo antes de
   conectar.

### Host macOS

1. Descargá e instalá **XQuartz**.
2. Abrí XQuartz → **Preferences → Security** → tildá
   **Allow connections from clients**.
3. Conectá con forwarding X11:

   ```bash
   ssh -Y user@hostname -p port
   ```

### Host Linux

Linux tiene X11 integrado:

```bash
ssh -Y user@hostname -p port
```

---

## Verificar que funciona

Dentro de Pairee, mantené `Ctrl`. La barra F-key debería cambiar a la
fila **Ctrl** dentro de un refresh. Si no:

- Confirmá que tu servidor X está corriendo y tu `DISPLAY` está
  seteado (`echo $DISPLAY` debería imprimir algo como `:0`).
- Confirmá que el cliente SSH forwardeó X11 (`echo $DISPLAY` en el
  host remoto no debería estar vacío).
- Algunos emuladores de terminal de Windows eliminan el socket X11;
  probá MobaXterm si te pasa esto.

---

## Cuando todo lo demás falla

Usá **`Ctrl+P`** para lockear la barra en la fila que querés. Se
mantiene hasta que vuelvas a ciclar o reinicies Pairee.

---

## A dónde ir ahora

- Referencia de la barra F-key: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Cómo se renderea la barra F-key: [`50_explanation_architecture`](50_explanation_architecture.md)
