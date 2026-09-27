//! Open: navega a la pagina del URI y espera su tracklist.

use std::time::Duration;

use anyhow::{Context, Result};

use super::super::selectors::TRACKLIST_EXISTS_JS;
use super::super::tabs::{ensure_spotify_tab, spotify_ws_url};

/// Navega la pestana a la pagina del URI, sin esperar nada.
pub async fn open_page(uri: &str) -> Result<()> {
    let mut parts = uri.split(':');
    let kind = parts.nth(1).context("URI sin kind")?;
    let id = parts.next().context("URI sin id")?;
    let url = format!("https://open.spotify.com/{kind}/{id}");
    // Self-healing: si la pestana se cerro, recrearla en vez de morir.
    ensure_spotify_tab().await?;
    // SPA (no Page.navigate): no destruir el reproductor.
    super::super::tabs::spa_navigate(&url, &format!("/{kind}/{id}")).await
}

/// Abre una playlist y espera su tracklist (hasta 10s).
pub async fn open_playlist(uri: &str) -> Result<()> {
    open_page(uri).await?;
    for _ in 0..20 {
        let ws_url = match spotify_ws_url().await {
            Ok(u) => u,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let v = super::super::client::cdp_call(
            &ws_url,
            31,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": TRACKLIST_EXISTS_JS,
                "returnByValue": true,
            }),
        )
        .await?;
        if v.pointer("/result/result/value").and_then(|x| x.as_bool()) == Some(true) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    anyhow::bail!("la playlist no cargo su tracklist");
}
