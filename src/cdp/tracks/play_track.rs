//! Play_track: toca una fila exacta del tracklist por track id (sin loteria).

use std::time::Duration;

use anyhow::Result;

use super::super::client::cdp_call;
use super::super::selectors::{
    TRACKLIST_SCROLL_JS, TRACKLIST_SCROLL_TOP_JS, track_row_click_js,
};
use super::super::tabs::{ensure_spotify_tab, spotify_ws_url};

pub async fn play_track(track_id: &str) -> Result<String> {
    ensure_spotify_tab().await?;
    let ws_url = spotify_ws_url().await?;
    if !track_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        anyhow::bail!("track id raro: {track_id}");
    }
    // Empezar desde arriba: el lector deja el scroll al fondo.
    let _ = super::super::client::cdp_call(
        &ws_url,
        34,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": TRACKLIST_SCROLL_TOP_JS,
            "returnByValue": true,
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let click_js = track_row_click_js(track_id);
    let scroll_js = TRACKLIST_SCROLL_JS;
    for _ in 0..40 {
        let ws_url = match spotify_ws_url().await {
            Ok(u) => u,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let v = cdp_call(
            &ws_url,
            32,
            "Runtime.evaluate",
            serde_json::json!({ "expression": click_js, "returnByValue": true }),
        )
        .await?;
        let out = v
            .pointer("/result/result/value")
            .and_then(|x| x.as_str())
            .unwrap_or("?")
            .to_string();
        if out == "clicked" {
            return Ok(out);
        }
        let _ = cdp_call(
            &ws_url,
            33,
            "Runtime.evaluate",
            serde_json::json!({ "expression": scroll_js, "returnByValue": true }),
        )
        .await;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    anyhow::bail!("no pude tocar la rola (¿saliste de la playlist?)");
}
