//! Localizar y crear la pestana Spotify en el Brave depurable.

use std::time::Duration;

use anyhow::{Context, Result};

use super::client::cdp_call;
use super::transport::{fetch_cdp_text, fetch_json_list};

/// WebSocket de la pestana Spotify (type page + url open.spotify).
/// Error si no hay ninguna (la app la abre al arrancar; el test la crea).
pub(crate) async fn spotify_ws_url() -> Result<String> {
    let body = fetch_json_list().await?;

    let list: serde_json::Value =
        serde_json::from_str(&body).context("JSON CDP invalido")?;
    let arr = list.as_array().context("lista CDP no es array")?;
    for t in arr {
        let ty = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "page" && url.contains("open.spotify") {
            return t
                .get("webSocketDebuggerUrl")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .context("tab Spotify sin WS");
        }
    }
    anyhow::bail!("no hay pestana Spotify en el Brave depurable")
}

/// URL de la pestana Spotify elegida (para logs de tests).
/// Solo la usan los tests.
#[allow(dead_code)]
pub async fn spotify_tab_url() -> Result<String> {
    let body = fetch_json_list().await?;
    let list: serde_json::Value =
        serde_json::from_str(&body).context("JSON CDP invalido")?;
    for t in list.as_array().cloned().unwrap_or_default() {
        let ty = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "page" && url.contains("open.spotify") {
            return Ok(url.to_string());
        }
    }
    anyhow::bail!("no hay pestana Spotify en el Brave depurable")
}

/// Trae la pestana Spotify al frente (Page.bringToFront).
/// Desbloquea virtualizadores atorados cuando la ventana esta ocluida
/// (sin frames no se pintan filas). Puede robar el foco una vez.
pub(crate) async fn bring_spotify_front() -> Result<()> {
    let ws_url = spotify_ws_url().await?;
    cdp_call(&ws_url, 4, "Page.bringToFront", serde_json::json!({})).await?;
    Ok(())
}

/// Crea la pestana Spotify via Target.createTarget (browser WS de /json/version).
async fn create_spotify_target() -> Result<()> {
    let version = fetch_cdp_text("/json/version").await?;
    let v: serde_json::Value =
        serde_json::from_str(&version).context("version CDP invalida")?;
    let browser_ws = v
        .get("webSocketDebuggerUrl")
        .and_then(|x| x.as_str())
        .context("version CDP sin browser WS")?;
    cdp_call(
        browser_ws,
        2,
        "Target.createTarget",
        serde_json::json!({ "url": crate::launcher::SPOTIFY_URL }),
    )
    .await?;
    Ok(())
}

/// Asegura pestana Spotify: si falta, la crea UNA vez y espera a que
/// liste (10s). Sin duplicar tabs.
pub async fn ensure_spotify_tab() -> Result<()> {
    let mut created = false;
    for _ in 0..20 {
        match spotify_ws_url().await {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("no hay pestana") => {
                if !created {
                    create_spotify_target().await?;
                    created = true;
                }
            }
            Err(e) => return Err(e),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    anyhow::bail!("pestana Spotify no aparece")
}
