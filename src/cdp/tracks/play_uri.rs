//! Play_uri: reproduce un URI navegando a su pagina + ESPACIO real (trusted).

use std::time::Duration;

use anyhow::Result;

use super::open::open_page;
use super::super::selectors::PLAYER_ARIA_JS;
use super::super::tabs::spotify_ws_url;

/// Reproduce un URI: navega a su pagina y alterna con TECLA ESPACIO real.
/// Solo tracks matchean titulo (las playlists suenan otra rola).
pub async fn play_uri(uri: &str, title: &str) -> Result<String> {
    open_page(uri).await?;
    let want: Option<String> = uri
        .split(':')
        .nth(1)
        .filter(|k| *k == "track")
        .map(|_| title.to_lowercase());
    for _ in 0..30 {
        let ws_url = match spotify_ws_url().await {
            Ok(u) => u,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let aria = player_aria(&ws_url).await.unwrap_or_default();
        if aria == "Pausar" {
            if let Some(t) = playing_title().await {
                if want.as_ref().map(|w| t.contains(w)).unwrap_or(true) {
                    return Ok("already-playing".to_string());
                }
            }
            reload_tab().await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        }
        space_press(&ws_url).await?;
        for _ in 0..6 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Some(t) = playing_title().await {
                if want.as_ref().map(|w| t.contains(w)).unwrap_or(true) {
                    return Ok("space-playing".to_string());
                }
            }
        }
    }
    anyhow::bail!("no pude tocar '{title}' (¿saliste de su pagina?)");
}

/// aria-label del boton play/pause del player ("" si no hay player).
async fn player_aria(ws_url: &str) -> Result<String, anyhow::Error> {
    let v = super::super::client::cdp_call(
        ws_url,
        26,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": PLAYER_ARIA_JS,
            "returnByValue": true,
        }),
    )
    .await?;
    Ok(v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string())
}

/// Titulo en minusculas de lo que este sonando, o None.
async fn playing_title() -> Option<String> {
    let all = crate::gsmtc::brave_sessions().await.ok()?;
    for s in &all {
        if let Ok(t) = crate::gsmtc::get_track(s).await {
            if t.playing {
                return Some(t.title.to_lowercase());
            }
        }
    }
    None
}

/// Espacio real (trusted): keyDown + keyUp como un dedo de verdad.
async fn space_press(ws_url: &str) -> Result<()> {
    for (id, typ) in [(27, "keyDown"), (28, "keyUp")] {
        super::super::client::cdp_call(
            ws_url,
            id,
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": typ,
                "key": " ",
                "code": "Space",
                "windowsVirtualKeyCode": 32,
                "nativeVirtualKeyCode": 32,
            }),
        )
        .await?;
    }
    Ok(())
}

/// Recarga la pestana Spotify (Page.reload).
async fn reload_tab() -> Result<()> {
    let ws_url = spotify_ws_url().await?;
    super::super::client::cdp_call(
        &ws_url,
        25,
        "Page.reload",
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}
