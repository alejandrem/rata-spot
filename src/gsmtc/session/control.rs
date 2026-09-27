//! Control: play/pause/next/prev/toggle sobre una sesion GSMTC.

use anyhow::{Context, Result};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus;

use super::super::track::Session;

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
