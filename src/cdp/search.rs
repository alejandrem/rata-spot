//! Buscador: de la TUI al recuadro de Spotify sin API.
//!
//! Estrategia: NO se tipea en el input (los inputs controlados de React
//! son fragiles por CDP) — se navega directo a /search/{query} y se leen
//! los resultados del DOM. Misma tecnica que biblioteca y tracklist.

use anyhow::{Context, Result};

use super::client::cdp_call;
use super::tabs::spotify_ws_url;

/// Busca y regresa items mezclados (tracks primero): LibraryItem
/// reutilizado {name, detail, uri, kind} para pintarlos igual.
pub async fn search(query: &str) -> Result<Vec<super::LibraryItem>> {
    let q = query.trim();
    if q.is_empty() {
        anyhow::bail!("escribe algo primero ( / + texto + Enter )");
    }
    let ws_url = spotify_ws_url().await?;
    super::client::cdp_call(
        &ws_url,
        40,
        "Page.navigate",
        serde_json::json!({ "url": format!("https://open.spotify.com/search/{}", encode(q)) }),
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

/// Lee anchors /track|artist|playlist|album|show fuera del sidebar y del
/// reproductor (esos contaminarian). Tracks primero, luego el resto.
const SEARCH_JS: &str = r#"(async () => {
  let w = 0;
  const count = () => document.querySelectorAll('main a[href*="/track/"],main a[href*="/artist/"],main a[href*="/playlist/"],main a[href*="/album/"]').length;
  while (count() < 3 && w < 16) { await new Promise(r => setTimeout(r, 500)); w++; }
  const out = [];
  const seen = new Set();
  const clean = (s) => (s || '').trim().split('\n')[0].trim();
  // Topes por grupo: sin esto los tracks llenan el cap y nunca se ven
  // artistas/playlists.
  let nTr = 0, nOt = 0;
  const push = (kind, uri, name, sub) => {
    if (!uri || seen.has(uri) || !name) return;
    if (kind === 'track' && nTr >= 12) return;
    if (kind !== 'track' && nOt >= 12) return;
    if (kind === 'track') nTr++; else nOt++;
    seen.add(uri);
    out.push({kind, uri, name, detail: sub || ''});
  };
  const inScope = (a) => !a.closest('#Desktop_LeftSidebar_Id')
    && !a.closest('[data-testid="now-playing-widget"]') && !a.closest('footer');
  // 1) canciones: titulo del anchor + artistas de su fila.
  document.querySelectorAll('main a[href*="/track/"]').forEach(a => {
    if (!inScope(a)) return;
    const m = a.href.match(/\/(track)\/([A-Za-z0-9]+)/);
    if (!m) return;
    const row = a.closest('[role="row"]');
    const artists = row
      ? [...row.querySelectorAll('a[href*="/artist/"]')].map(x => clean(x.innerText)).filter(Boolean)
      : [];
    push('track', 'spotify:track:' + m[2], clean(a.innerText), artists.join(', '));
  });
  // 2) artistas/playlists/albumes/shows (evitar duplicar artistas de filas).
  const groups = [['artist', '/artist/'], ['playlist', '/playlist/'], ['album', '/album/'], ['show', '/show/']];
  for (const [kind, pat] of groups) {
    document.querySelectorAll('main a[href*="' + pat + '"]').forEach(a => {
      if (!inScope(a)) return;
      const row = a.closest('[role="row"]');
      if (row && row.querySelector('a[href*="/track/"]')) return;
      const esc = pat.replace('/', '\\/');
      const m = a.href.match(new RegExp(esc + '([A-Za-z0-9]+)'));
      if (!m) return;
      push(kind, 'spotify:' + kind + ':' + m[1], clean(a.innerText) || clean(a.getAttribute('aria-label')));
    });
  }
  return JSON.stringify({items: out.slice(0, 24)});
})()"#;

/// Encode minimo para URLs (sin crate extra).
fn encode(q: &str) -> String {
    let mut out = String::with_capacity(q.len());
    for b in q.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
