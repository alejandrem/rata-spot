//! Primer play sin mouse: click al Play de la pagina via CDP.
//!
//! GSMTC solo controla lo que YA suena; en pagina fresca se hace click
//! al boton con Runtime.evaluate (selectores ES/EN, data-testid estables).

use std::time::Duration;

use anyhow::Result;

use super::client::cdp_call;
use super::selectors::{PAGE_STATE_JS, PLAY_CLICK_JS};
use super::tabs::{ensure_spotify_tab, spotify_ws_url};

/// Selectores y health-check viven en `super::selectors` (un solo lugar).
/// Aqui solo la logica de clicks: PLAY_CLICK_JS prueba player-bar primero
/// (retoma la cola = semantica de [space]), heroes despues. Para tocar algo
/// ESPECIFICO usar click por titulo (tracks.rs), no esto: el primer visible
/// es loteria con 100+ botones en el DOM.

/// Hace click al Play de Spotify. OkClick -> la sesion GSMTC aparece sola
/// y el loop reactivo la toma.
/// La pagina fresca tarda en renderizar el player: reintenta el click
/// hasta 15s. Si no aparece, diagnostica que muestra (login vs cargando).
pub async fn play_spotify() -> Result<String> {
    for _ in 0..30 {
        // WS fresco por intento: /json/list a veces flap ea vacio un momento.
        let ws_url = match spotify_ws_url().await {
            Ok(u) => u,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let v = cdp_call(
            &ws_url,
            1,
            "Runtime.evaluate",
            serde_json::json!({ "expression": PLAY_CLICK_JS, "returnByValue": true }),
        )
        .await?;
        let out = v
            .pointer("/result/result/value")
            .and_then(|x| x.as_str())
            .unwrap_or("?")
            .to_string();
        if out.starts_with("clicked:") {
            return Ok(out);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let state = match spotify_ws_url().await {
        Ok(ws_url) => {
            cdp_call(
                &ws_url,
                3,
                "Runtime.evaluate",
                serde_json::json!({ "expression": PAGE_STATE_JS, "returnByValue": true }),
            )
            .await
            .ok()
            .and_then(|v| {
                v.pointer("/result/result/value")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "?".to_string())
        }
        Err(_) => "sin pestana al diagnosticar".to_string(),
    };
    anyhow::bail!("pagina sin boton Play visible [{state}] (¿logueado? ¿cargo?)");
}

/// Flujo completo del primer play: asegura pestana + click.
/// Esto es lo que [space] usa cuando no hay sesion GSMTC.
pub async fn play_from_scratch() -> Result<String> {
    ensure_spotify_tab().await?;
    play_spotify().await
}
