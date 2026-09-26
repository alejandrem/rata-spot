//! Arranque: si YA hay sesion GSMTC de Brave (Spotify sonando en
//! cualquier ventana), reutilizarla y no lanzar nada. Si no, abrir
//! VENTANA NUEVA del mismo Brave (tu perfil logueado), nunca pestaña
//! en tu ventana actual y nunca perfil fresh sin cuentas.
//!
//! Regresa (was_already_running, boot_status): el status se pinta en la
//! TUI porque los println! quedan ocultos bajo la pantalla alternativa.

use anyhow::Result;

use crate::{gsmtc, launcher};

pub async fn boot() -> Result<(bool, String)> {
    let reuse_existing = gsmtc::get_brave_session().await.is_ok();
    if reuse_existing {
        Ok((
            true,
            "Reutilizando sesion Brave existente (ya sonaba Spotify).".to_string(),
        ))
    } else {
        let owned = launcher::launch_brave_spotify().await?;
        let pid = launcher::child_pid()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        Ok((
            owned,
            format!(
                "Brave ventana NUEVA visible PID {} | {} | dale play una vez",
                pid,
                launcher::SPOTIFY_URL
            ),
        ))
    }
}
