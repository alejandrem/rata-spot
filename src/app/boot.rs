//! Arranque: si YA hay sesion GSMTC de Brave (Spotify sonando en
//! cualquier ventana), reutilizarla y no lanzar nada. Si no, abrir
//! VENTANA NUEVA del mismo Brave (tu perfil logueado), nunca pestaña
//! en tu ventana actual y nunca perfil fresh sin cuentas.
//!
//! Regresa (was_already_running, boot_status): el status se pinta en la
//! TUI porque los println! quedan ocultos bajo la pantalla alternativa.

use anyhow::Result;

use crate::{cdp, gsmtc, launcher};

/// Health-check DOM best-effort (4s): si Spotify ya estaba abierto la
/// pestana responde y el status dice que selectores viven; recién lanzado
/// aun esta cargando y queda "pendiente" (no es error).
async fn dom_note() -> String {
    match tokio::time::timeout(
        std::time::Duration::from_secs(4),
        cdp::health_summary(),
    )
    .await
    {
        Ok(s) => s,
        Err(_) => "DOM check pendiente (Spotify cargando)".to_string(),
    }
}

pub async fn boot() -> Result<(bool, String)> {
    // Hint del historial: con varias sesiones (video + Spotify), enganchar
    // la que coincida con tu ultima rola en vez de la primera que suene.
    let hint = gsmtc::load_last_track()
        .map(|t| t.title)
        .unwrap_or_default();
    let reuse_existing = gsmtc::pick_session(Some(&hint)).await.is_ok();
    if reuse_existing {
        let dom = dom_note().await;
        Ok((
            true,
            format!("Reutilizando sesion Brave existente (ya sonaba Spotify). | {dom}"),
        ))
    } else {
        let owned = launcher::launch_brave_spotify().await?;
        let pid = launcher::child_pid()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        let dom = dom_note().await;
        Ok((
            owned,
            format!(
                "Brave ventana NUEVA visible PID {} | {} | dale play una vez | {dom}",
                pid,
                launcher::SPOTIFY_URL
            ),
        ))
    }
}
