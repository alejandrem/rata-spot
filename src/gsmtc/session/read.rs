//! Read: lee titulo/artista/album + playing + progreso de una sesion.

use std::time::Duration;

use anyhow::{Context, Result};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus;

use super::super::track::{timespan_to_duration, Session, TrackInfo};

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
            let pos = tl
                .Position()
                .map(|t| timespan_to_duration(t.Duration))
                .unwrap_or(Duration::ZERO);
            let end = tl
                .EndTime()
                .map(|t| timespan_to_duration(t.Duration))
                .unwrap_or(Duration::ZERO);
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
