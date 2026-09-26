//! Sesion GSMTC de Brave: busqueda, lectura y controles de transporte.
//!
//! Nota windows 0.58: IAsyncOperation NO es Future -> se usa `.get()`
//! bloqueante (llamadas COM locales de ms, aceptable en loops 250ms/1s).

use std::time::Duration;

use anyhow::{Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

use super::track::{timespan_to_duration, Session, TrackInfo};

/// Busca la sesion GSMTC cuya app sea Brave.
/// Retorna error si Brave aun no registro sesion (no listo todavia).
pub async fn get_brave_session() -> Result<Session> {
    // windows 0.58: IAsyncOperation no es Future -> usar .get() bloqueante.
    // Son llamadas COM locales rapidas (ms), aceptable en este loop 1s/250ms.
    // Si algun dia se congela la TUI, mover a spawn_blocking.
    let manager = SessionManager::RequestAsync()
        .context("RequestAsync GSMTC fallo (¿servicio Windows inactivo?)")?
        .get()
        .context("RequestAsync.get() fallo")?;

    let sessions = manager
        .GetSessions()
        .context("GetSessions fallo (falta feature Foundation_Collections?)")?;

    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        let app_id = session
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default()
            .to_lowercase();
        if app_id.contains("brave") {
            return Ok(session);
        }
    }

    anyhow::bail!("sesion Brave no encontrada todavia")
}

/// Lee titulo/artista/album + playing/paused + progreso.
/// Nunca deberia crashear al llamador: los campos faltantes van a default.
pub async fn get_track(session: &Session) -> Result<TrackInfo> {
    let props = session
        .TryGetMediaPropertiesAsync()
        .context("TryGetMediaPropertiesAsync fallo")?
        .get()
        .context("media properties .get() fallo (¿sin metadata ahora mismo?)")?;

    let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
    let album = props
        .AlbumTitle()
        .map(|h| h.to_string())
        .unwrap_or_default();

    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    let (position, duration) = session
        .GetTimelineProperties()
        .map(|tl| {
            let pos = tl.Position().map(|t| timespan_to_duration(t.Duration)).unwrap_or(Duration::ZERO);
            let end = tl.EndTime().map(|t| timespan_to_duration(t.Duration)).unwrap_or(Duration::ZERO);
            (pos, end)
        })
        .unwrap_or((Duration::ZERO, Duration::ZERO));

    let progress = if duration.is_zero() {
        0.0
    } else {
        (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
    };

    Ok(TrackInfo {
        title: if title.is_empty() { "—".to_string() } else { title },
        artist: if artist.is_empty() { "—".to_string() } else { artist },
        album,
        playing,
        position,
        duration,
        progress,
    })
}

pub async fn play(session: &Session) -> Result<()> {
    session
        .TryPlayAsync()
        .context("TryPlayAsync fallo")?
        .get()
        .context("play .get() fallo")?;
    Ok(())
}

pub async fn pause(session: &Session) -> Result<()> {
    session
        .TryPauseAsync()
        .context("TryPauseAsync fallo")?
        .get()
        .context("pause .get() fallo")?;
    Ok(())
}

pub async fn next(session: &Session) -> Result<()> {
    session
        .TrySkipNextAsync()
        .context("TrySkipNextAsync fallo")?
        .get()
        .context("next .get() fallo")?;
    Ok(())
}

pub async fn prev(session: &Session) -> Result<()> {
    session
        .TrySkipPreviousAsync()
        .context("TrySkipPreviousAsync fallo")?
        .get()
        .context("prev .get() fallo")?;
    Ok(())
}

/// Si esta playing -> pause. Si esta paused (o cualquier otro) -> play.
pub async fn toggle(session: &Session) -> Result<()> {
    let is_playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    if is_playing {
        pause(session).await
    } else {
        play(session).await
    }
}

/// Loop reactivo (Fase 3.2): reintenta cada 1s indefinidamente.
/// No usa delay fijo de arranque; el llamador muestra "Conectando..."
/// mientras esto pende. Solo regresa cuando hay sesion Brave.
#[allow(dead_code)]
pub async fn wait_for_brave_session() -> Session {
    loop {
        match get_brave_session().await {
            Ok(session) => break session,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
