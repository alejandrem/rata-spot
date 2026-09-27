//! Arranque con perfil dedicado (casita propia, ver launcher/config/profile):
//!
//! 1. Hay música sonando (sesión GSMTC) -> reutilizar, no lanzar nada.
//! 2. Perfil fresco (sin login) -> lanzar visible + login único (Enter) y seguir.
//! 3. Si no -> lanzar (en frío es proceso propio; si hay instancia NUESTRA
//!    viva, delega a ella, que también está bien: tiene nuestro CDP).
//!
//! Tu Brave personal es irrelevante aquí: otro perfil = otro proceso,
//! nunca delega a él ni te pide cerrarlo (eso era la sala de espera P2,
//! eliminada con el perfil dedicado).
//!
//! Regresa (was_already_running, boot_status): el status se pinta en la
//! TUI porque los println! quedan ocultos bajo la pantalla alternativa.
//! (Los println! del login SI se ven: salen antes de la TUI.)

use anyhow::{Context, Result};

use crate::{cdp, gsmtc, launcher};

pub async fn boot() -> Result<(bool, String)> {
    // Hint del historial: con varias sesiones (video + Spotify), enganchar
    // la que coincida con tu ultima rola en vez de la primera que suene.
    let hint = gsmtc::load_last_track()
        .map(|t| t.title)
        .unwrap_or_default();

    // Reenganchar instancia propia viva: su puerto quedó guardado.
    if let Some(saved) = launcher::config::profile::read_saved_port() {
        launcher::ports::set_cdp_port(saved);
    }

    // Login único: perfil fresco → abrir visible, loguearse, Enter, seguir.
    // (El lanzamiento ya fuerza visible en fresco; aquí solo la espera.)
    if launcher::config::profile::is_fresh() {
        println!("rata-spot: perfil nuevo, login único necesario.");
        println!("  1) Se abrirá Brave (perfil rata-spot): logueate en Spotify.");
        println!("  2) Vuelve aquí y pulsa Enter para seguir.");
        launcher::launch_brave_spotify().await?;
        wait_enter().await;
        println!("rata-spot: login recibido (o Enter pelado), seguimos...");
    }

    // Si ya suena algo, reutilizarlo y no lanzar nada.
    if gsmtc::pick_session(Some(&hint)).await.is_ok() {
        let dom = dom_note().await;
        let ours = launcher::config::profile::is_locked();
        let extra = if ours { " (instancia propia)" } else { "" };
        return Ok((
            true,
            format!("Reutilizando sesion existente (ya sonaba Spotify).{extra} | {dom}"),
        ));
    }

    // Sin música: lanzar y EXIGIR pestaña Spotify (ruidoso, no silencioso).
    launcher::launch_brave_spotify().await?;
    crate::cdp::ensure_spotify_tab()
        .await
        .context("la ventana no levantó pestaña Spotify (¿sin internet? ¿Widevine?)")?;
    let pid = launcher::child_pid()
        .map(|p| p.to_string())
        .unwrap_or_else(|| "?".to_string());
    let dom = dom_note().await;
    Ok((
        false,
        format!(
            "Brave propio (perfil rata-spot) PID {} | {} | dale play una vez | {dom}",
            pid,
            launcher::SPOTIFY_URL
        ),
    ))
}

/// Espera un Enter en stdin (solo se usa pre-TUI, con terminal normal).
async fn wait_enter() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut line = String::new();
    let _ = BufReader::new(tokio::io::stdin()).read_line(&mut line).await;
}

/// Health-check DOM best-effort (4s): si Spotify ya estaba abierto la
/// pestaña responde y el status dice que selectores viven; recién lanzado
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
