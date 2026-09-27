//! Arranque en 3 estados (P2):
//!
//! 1. Hay CDP -> como siempre: reutilizar sesion GSMTC o lanzar.
//! 2. Sin CDP y sin brave.exe -> arranque en frio.
//! 3. Sin CDP pero CON brave.exe (abierto a mano, sordo) -> sala de
//!    espera: NO se spawnea un hijo inutil que solo delegaria; se pide
//!    cerrar Brave y al detectar el cierre se auto-lanza en frio y el
//!    boot continua solo (sin reiniciar la app).
//!
//! Regresa (was_already_running, boot_status): el status se pinta en la
//! TUI porque los println! quedan ocultos bajo la pantalla alternativa.
//! (Los println! de la sala de espera SI se ven: salen antes de la TUI.)

use std::time::Duration;

use anyhow::Result;

use crate::{cdp, gsmtc, launcher};

pub async fn boot() -> Result<(bool, String)> {
    // Hint del historial: con varias sesiones (video + Spotify), enganchar
    // la que coincida con tu ultima rola en vez de la primera que suene.
    let hint = gsmtc::load_last_track()
        .map(|t| t.title)
        .unwrap_or_default();

    // Estado 3: Brave abierto pero sordo (sin puerto CDP). Esperar el
    // cierre y seguir; al volver ya no hay Brave (o hay CDP).
    let mut waited = false;
    if !cdp::debug_alive().await && launcher::config::is_brave_running() {
        wait_for_brave_close().await;
        waited = true;
    }

    // Estados 1 y 2 (y salida del 3): hay CDP o ya no hay Brave.
    // Si ya suena algo, reutilizarlo y no lanzar nada.
    if gsmtc::pick_session(Some(&hint)).await.is_ok() {
        let dom = dom_note().await;
        let extra = if waited { " (tras sala de espera)" } else { "" };
        return Ok((
            true,
            format!("Reutilizando sesion Brave existente (ya sonaba Spotify).{extra} | {dom}"),
        ));
    }

    // Sin música: lanzar (en frío tras la espera → flags sí aplican;
    // con CDP vivo → delega a la instancia depurada, pestaña visible).
    let owned = launcher::launch_brave_spotify().await?;
    let pid = launcher::child_pid()
        .map(|p| p.to_string())
        .unwrap_or_else(|| "?".to_string());
    let dom = dom_note().await;
    let extra = if waited { " | sala de espera superada" } else { "" };
    Ok((
        owned,
        format!(
            "Brave ventana NUEVA visible PID {} | {} | dale play una vez | {dom}{extra}",
            pid,
            launcher::SPOTIFY_URL
        ),
    ))
}

/// Sala de espera del estado 3: pide cerrar Brave y regresa cuando ya no
/// está (para auto-lanzar en frío) o cuando aparece CDP solo (otro lo
/// depuró: se sigue sin lanzar). Sin timeout: la filosofía del proyecto
/// es reintentar indefinidamente; Ctrl+C cancela (no hay nada que limpiar:
/// no lanzamos nada aún y la música ajena sigue sonando).
async fn wait_for_brave_close() {
    println!("rata-spot: Brave esta abierto SIN depuracion (sin puerto CDP).");
    println!("  1) Cierra Brave por completo (todas las ventanas).");
    println!("  2) Yo lo reabro solo con depuracion y seguimos. (Ctrl+C para cancelar)");
    let mut ticks = 0u32;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        ticks += 1;
        if cdp::debug_alive().await {
            println!("rata-spot: aparecio CDP, seguimos sin lanzar.");
            return;
        }
        if !launcher::config::is_brave_running() {
            // Darle un respiro al SO: lanzar contra un Brave moribundo
            // delegaría al cadáver en vez de arrancar en frío.
            tokio::time::sleep(Duration::from_secs(2)).await;
            println!("rata-spot: Brave cerrado, relanzando en frio...");
            return;
        }
        if ticks % 15 == 0 {
            println!("rata-spot: sigo esperando... (cierra Brave para seguir, Ctrl+C para salir)");
        }
    }
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
