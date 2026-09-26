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

    // Con 2 ventanas Brave (la tuya + la nuestra) la primera sesion puede
    // ser una vieja pausada con metadata congelada -> barra estatica.
    // Se prefiere la que esta SONANDO; si ninguna, la primera.
    let mut fallback: Option<Session> = None;
    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        let app_id = session
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default()
            .to_lowercase();
        if !app_id.contains("brave") {
            continue;
        }
        let playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|s| s == PlaybackStatus::Playing)
            .unwrap_or(false);
        if playing {
            return Ok(session);
        }
        if fallback.is_none() {
            fallback = Some(session);
        }
    }

    fallback.context("sesion Brave no encontrada todavia")
}

/// TODAS las sesiones Brave (suele haber varias: tu ventana + la nuestra,
/// o un video de Facebook sonando a la vez que Spotify).
pub async fn brave_sessions() -> Result<Vec<Session>> {
    let manager = SessionManager::RequestAsync()
        .context("RequestAsync GSMTC fallo")?
        .get()
        .context("RequestAsync.get() fallo")?;
    let sessions = manager.GetSessions().context("GetSessions fallo")?;
    let mut out = Vec::new();
    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        let app_id = session
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default()
            .to_lowercase();
        if app_id.contains("brave") {
            out.push(session);
        }
    }
    if out.is_empty() {
        anyhow::bail!("sin sesiones Brave");
    }
    Ok(out)
}

/// Elige sesion: si hay pista (titulo conocido del historial), la que
/// coincida; si no, la primera SONANDO; si ninguna, la primera.
/// Evita engancharse a un video de Facebook cuando Spotify tambien suena.
pub async fn pick_session(hint_title: Option<&str>) -> Result<Session> {
    let all = brave_sessions().await?;
    if all.len() == 1 {
        return Ok(all.into_iter().next().expect("uno"));
    }
    // 1) Hint del historial rata-spot: la de Spotify casi seguro.
    if let Some(hint) = hint_title.filter(|h| !h.is_empty() && *h != "—") {
        let want = hint.to_lowercase();
        for s in &all {
            let title = get_track(s)
                .await
                .map(|t| t.title.to_lowercase())
                .unwrap_or_default();
            if !title.is_empty() && (title.contains(&want) || want.contains(&title)) {
                return Ok(s.clone());
            }
        }
    }
    // 2) Primera sonando. 3) Primera a secas.
    for s in &all {
        let playing = s
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|st| st == PlaybackStatus::Playing)
            .unwrap_or(false);
        if playing {
            return Ok(s.clone());
        }
    }
    Ok(all.into_iter().next().expect("no vacio"))
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
