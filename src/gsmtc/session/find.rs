//! Find: localiza sesiones GSMTC cuya app sea Brave.

use anyhow::{Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

use super::super::track::Session;

/// Busca la sesion GSMTC cuya app sea Brave.
/// Retorna error si Brave aun no registro sesion (no listo todavia).
pub async fn get_brave_session() -> Result<Session> {
    // windows 0.58: IAsyncOperation no es Future -> usar .get() bloqueante.
    // Son llamadas COM locales rapidas (ms), aceptable en este loop 1s/250ms.
    let manager = SessionManager::RequestAsync()
        .context("RequestAsync GSMTC fallo (¿servicio Windows inactivo?)")?
        .get()
        .context("RequestAsync.get() fallo")?;

    let sessions = manager
        .GetSessions()
        .context("GetSessions fallo (falta feature Foundation_Collections?)")?;

    // Con 2 ventanas Brave la primera sesion puede ser una vieja pausada
    // con metadata congelada -> barra estatica. Se prefiere la que SUENA.
    let mut fallback: Option<Session> = None;
    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        if !is_brave(&session) {
            continue;
        }
        if is_playing(&session) {
            return Ok(session);
        }
        if fallback.is_none() {
            fallback = Some(session);
        }
    }

    fallback.context("sesion Brave no encontrada todavia")
}

/// TODAS las sesiones Brave (tu ventana + la nuestra, o un video sonando).
pub async fn brave_sessions() -> Result<Vec<Session>> {
    let manager = SessionManager::RequestAsync()
        .context("RequestAsync GSMTC fallo")?
        .get()
        .context("RequestAsync.get() fallo")?;
    let sessions = manager.GetSessions().context("GetSessions fallo")?;
    let mut out = Vec::new();
    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        if is_brave(&session) {
            out.push(session);
        }
    }
    if out.is_empty() {
        anyhow::bail!("sin sesiones Brave");
    }
    Ok(out)
}

pub(crate) fn is_brave(s: &Session) -> bool {
    s.SourceAppUserModelId()
        .map(|h| h.to_string())
        .unwrap_or_default()
        .to_lowercase()
        .contains("brave")
}

pub(crate) fn is_playing(s: &Session) -> bool {
    s.GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|st| st == PlaybackStatus::Playing)
        .unwrap_or(false)
}
