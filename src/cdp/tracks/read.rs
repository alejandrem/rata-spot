//! Read: lee TODAS las rolas del tracklist (con scroll virtualizado).

use std::time::Duration;

use anyhow::{Context, Result};

use super::TrackItem;
use super::super::selectors::TRACKS_JS;
use super::super::tabs::spotify_ws_url;

/// Lee TODAS las rolas del tracklist abierto (con scroll, virtualizado).
pub async fn playlist_tracks() -> Result<Vec<TrackItem>> {
    let mut last_err = String::new();
    for _ in 0..6 {
        match tracks_snapshot().await {
            Ok(items) if !items.is_empty() => return Ok(items),
            Ok(_) => last_err = "tracklist vacio".to_string(),
            Err(e) => last_err = e.to_string(),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("canciones no listas: {last_err}")
}

async fn tracks_snapshot() -> Result<Vec<TrackItem>> {
    let ws_url = spotify_ws_url().await?;
    let v = super::super::client::cdp_call_t(
        &ws_url,
        31,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": TRACKS_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
        30,
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("tracklist sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("tracklist JSON invalido")?;
    if let Some(err) = data.get("error").and_then(|x| x.as_str()) {
        anyhow::bail!("tracklist: {err}");
    }
    let mut items = Vec::new();
    for it in data
        .get("tracks")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let id = it.get("id").and_then(|x| x.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        items.push(TrackItem {
            n: it.get("n").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            title: it.get("title").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
            artist: it.get("artist").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            duration: it.get("duration").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            id: id.to_string(),
        });
    }
    Ok(items)
}
