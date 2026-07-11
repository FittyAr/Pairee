use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostAction {
    None,
    Shutdown,
    Sleep,
    Hibernate,
    CloseApp,
}

/// Ejecuta la acción post-procesamiento correspondiente de manera multiplataforma.
pub fn execute_post_action(action: PostAction) -> Result<(), std::io::Error> {
    match action {
        PostAction::None => Ok(()),
        PostAction::Shutdown => {
            #[cfg(target_os = "windows")]
            {
                Command::new("shutdown")
                    .args(["/s", "/t", "10", "/c", "Pairee: Transfer complete. Shutting down..."])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("shutdown")
                    .args(["-h", "+1", "Pairee: Transfer complete. Shutting down..."])
                    .spawn()?;
            }
            Ok(())
        }
        PostAction::Sleep => {
            #[cfg(target_os = "windows")]
            {
                Command::new("rundll32.exe")
                    .args(["powrprof.dll,SetSuspendState", "0", "1", "0"])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("systemctl")
                    .arg("suspend")
                    .spawn()?;
            }
            Ok(())
        }
        PostAction::Hibernate => {
            #[cfg(target_os = "windows")]
            {
                Command::new("rundll32.exe")
                    .args(["powrprof.dll,SetSuspendState", "1", "1", "0"])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("systemctl")
                    .arg("hibernate")
                    .spawn()?;
            }
            Ok(())
        }
        PostAction::CloseApp => {
            // Cerramos de forma limpia indicando salida exitosa
            std::process::exit(0);
        }
    }
}
