//! Herramienta de diagnóstico para eventos de teclado de Pairee.
//!
//! ### Utilidad del archivo:
//! Este programa de diagnóstico permite verificar exactamente qué eventos de teclado
//! (`KeyEvent`) recibe la aplicación desde el terminal activo (ej. Windows Terminal, cmd).
//!
//! ### Casos de uso:
//! 1. **Comprobar si el terminal consume atajos:** Si presionas `Ctrl+1` o `Ctrl+9` y no se
//!    imprime ningún evento en pantalla, significa que el terminal los captura para sus
//!    propias funciones globales (como cambiar de pestaña) y los traga.
//! 2. **Verificar duplicaciones o traducciones:** Si presionas `Ctrl+8` y se generan dos
//!    eventos seguidos (un `Ctrl+8` y un `Backspace`), significa que el terminal emite tanto
//!    la secuencia Kitty Keyboard como el fallback de borrado antiguo (`0x08`), causando doble acción.
//!
//! Para salir del modo de diagnóstico, pulsa la tecla `Esc`.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    println!("Press keys to see their events. Press Esc to exit.");
    io::stdout().flush()?;

    loop {
        if event::poll(std::time::Duration::from_millis(500))? {
            match event::read()? {
                Event::Key(key_event) => {
                    println!("KeyEvent: {:?}", key_event);
                    io::stdout().flush()?;
                    if key_event.code == KeyCode::Esc && key_event.kind == KeyEventKind::Press {
                        break;
                    }
                }
                other => {
                    println!("Event: {:?}", other);
                    io::stdout().flush()?;
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
