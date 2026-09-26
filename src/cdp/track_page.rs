//! Dashboard individual de la rola: lee `section[data-testid="track-page"]`
//! (titulo, artista, album, duracion, año, reproducciones + letra).
//! A veces la pagina tarda en pintarlo: espera hasta 8s antes de rendirse.

use anyhow::{Context, Result};

use super::tabs::spotify_ws_url;
use super::selectors::DASHBOARD_JS;

/// Dashboard de la cancion individual para la TUI.
#[derive(Debug, Clone, Default)]
pub struct TrackDetail {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub year: String,
    pub playcount: String,
    pub lyrics: Vec<String>,
}

/// Lee el dashboard de la pagina de track abierta.
pub async fn track_detail() -> Result<TrackDetail> {
    // Self-healing como open_page (pestana cerrada a media sesion).
    super::tabs::ensure_spotify_tab().await?;
    let ws_url = spotify_ws_url().await?;
    let v = super::client::cdp_call_t(
        &ws_url,
        42,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": DASHBOARD_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
        30,
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("dashboard sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("dashboard JSON invalido")?;
    if let Some(err) = data.get("error").and_then(|x| x.as_str()) {
        anyhow::bail!("dashboard: {err}");
    }
    let get = |k: &str| {
        data.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    Ok(TrackDetail {
        title: get("title"),
        artist: get("artist"),
        album: get("album"),
        duration: get("duration"),
        year: get("year"),
        playcount: get("playcount"),
        lyrics: data
            .get("lyrics")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .take(40)
            .collect(),
    })
}

// Snapshot JS vive en `super::selectors` (un solo lugar).
