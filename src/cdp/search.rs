//! Buscador: de la TUI al recuadro de Spotify sin API.
//!
//! Estrategia: NO se tipea en el input (los inputs controlados de React
//! son fragiles por CDP) — se navega directo a /search/{query} y se leen
//! los resultados del DOM. Misma tecnica que biblioteca y tracklist.

use anyhow::{Context, Result};

use super::client::cdp_call;
use super::selectors::SEARCH_JS;
use super::tabs::{spa_navigate, spotify_ws_url};

/// Busca y regresa items mezclados (tracks primero): LibraryItem
/// reutilizado {name, detail, uri, kind} para pintarlos igual.
pub async fn search(query: &str) -> Result<Vec<super::LibraryItem>> {
    let q = query.trim();
    if q.is_empty() {
        anyhow::bail!("escribe algo primero ( / + texto + Enter )");
    }
    // 1) JSON/red primero: sin navegar, sin tocar la musica, mas rapido.
    // Si la API interna cambio o no hay token, cae al DOM como antes.
    let api_try = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        super::api::search_via_api(q),
    )
    .await;
    if let Ok(Ok(items)) = api_try {
        if !items.is_empty() {
            return Ok(items);
        }
    }
    // 2) DOM (fallback de siempre).
    // SPA (no Page.navigate): no cortar la musica que suena mientras buscas.
    let eq = super::api::encode(q);
    spa_navigate(
        &format!("https://open.spotify.com/search/{eq}"),
        &format!("/search/{eq}"),
    )
    .await?;
    // Esperar resultados (hasta 8s) y leerlos.
    for _ in 0..16 {
        if let Ok(items) = search_snapshot().await {
            if !items.is_empty() {
                return Ok(items);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    anyhow::bail!("sin resultados para '{q}' (¿cargo la pagina?)")
}

async fn search_snapshot() -> Result<Vec<super::LibraryItem>> {
    let ws_url = spotify_ws_url().await?;
    let v = cdp_call(
        &ws_url,
        41,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": SEARCH_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("busqueda sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("busqueda JSON invalido")?;
    let mut items = Vec::new();
    for it in data
        .get("items")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let uri = it.get("uri").and_then(|x| x.as_str()).unwrap_or("");
        let kind = uri.split(':').nth(1).unwrap_or("?").to_string();
        if uri.is_empty() || kind == "?" {
            continue;
        }
        items.push(super::LibraryItem {
            name: it.get("name").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
            detail: it.get("detail").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            uri: uri.to_string(),
            kind,
        });
    }
    Ok(items)
}

// Snapshot JS vive en `super::selectors` (un solo lugar).

