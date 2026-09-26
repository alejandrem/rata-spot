//! Primer play sin mouse: click al Play de la pagina via CDP.
//!
//! GSMTC solo controla lo que YA suena; en pagina fresca se hace click
//! al boton con Runtime.evaluate (selectores ES/EN, data-testid estables).

use std::time::Duration;

use anyhow::Result;

use super::client::cdp_call;
use super::tabs::{ensure_spotify_tab, spotify_ws_url};

/// Click al Play probando varios selectores (ES + EN + player bar).
/// Solo botones VISIBLES y habilitados: querySelector tambien encuentra
/// los ocultos y clickearlos no suena (click fantasma).
/// ORDEN: player-bar primero (retoma la cola = semantica de [space]),
/// heroes despues. Para tocar algo ESPECIFICO usar click_title (tracks.rs),
/// no esto: el primer visible es loteria con 100+ botones en el DOM.
/// pub(crate): lo reutiliza tracks.rs tras navegar a una playlist.
pub(crate) const PLAY_JS: &str = r#"(() => {
  const vis = (b) => b && !b.disabled && b.getAttribute('aria-disabled') !== 'true' && !!b.offsetParent;
  const sels = [
    '[data-testid="control-button-playpause"]',
    '[data-testid="play-button"]',
    'button[aria-label="Reproducir"]',
    'button[aria-label="Play"]'
  ];
  for (const s of sels) {
    const b = document.querySelector(s);
    if (vis(b)) { b.click(); return 'clicked:' + s + '|' + (b.getAttribute('aria-label') || '').slice(0, 60); }
  }
  const any = [...document.querySelectorAll('button[aria-label="Reproducir"],button[aria-label="Play"]')].find(vis);
  if (any) { any.click(); return 'clicked:fallback|' + (any.getAttribute('aria-label') || '').slice(0, 60); }
  return 'no-button';
})()"#;

/// Diagnostico rapido de la pagina: titulo + hay player + hay login.
const STATE_JS: &str = r#"(() => {
  const q = (s) => !!document.querySelector(s);
  return document.title + ' | play=' + q('[data-testid="control-button-playpause"]')
    + ' | login=' + (q('[data-testid="login-button"]') || q('a[href*="login"]'));
})()"#;

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
            serde_json::json!({ "expression": PLAY_JS, "returnByValue": true }),
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
                serde_json::json!({ "expression": STATE_JS, "returnByValue": true }),
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
