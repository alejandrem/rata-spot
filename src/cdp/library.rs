//! Lectura de "Tu biblioteca" del DOM + reproducir items por URI.
//!
//! El grid esta virtualizado (solo filas visibles en el DOM): se scrollea
//! el sidebar, se dedupea por URI y se filtra en Rust por prefijo del URI
//! (robusto ante idiomas). El URI sale del id del titulo:
//! `listrow-title-spotify:playlist:XXX` (la fila NO trae aria-labelledby).

use std::time::Duration;

use anyhow::{Context, Result};

use super::client::cdp_call;
use super::tabs::spotify_ws_url;

/// Un item de "Tu biblioteca": playlist, artista, album o podcast.
/// El `uri` (spotify:playlist:XXX) sirve para abrirlo y reproducirlo.
#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub name: String,
    pub detail: String,
    pub uri: String,
    pub kind: String,
}

/// Lee TODA la biblioteca con scroll. Reintenta si la pagina aun no
/// renderiza el grid.
pub async fn library_items() -> Result<Vec<LibraryItem>> {
    let mut last_err = String::new();
    for _ in 0..6 {
        match library_snapshot().await {
            Ok(items) if !items.is_empty() => return Ok(items),
            Ok(_) => last_err = "biblioteca vacia".to_string(),
            Err(e) => last_err = e.to_string(),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("biblioteca no lista: {last_err}")
}

async fn library_snapshot() -> Result<Vec<LibraryItem>> {
    let ws_url = spotify_ws_url().await?;
    let v = cdp_call(
        &ws_url,
        10,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": LIBRARY_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("biblioteca sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("biblioteca JSON invalido")?;
    if let Some(err) = data.get("error").and_then(|x| x.as_str()) {
        anyhow::bail!("biblioteca: {err}");
    }
    let mut items = Vec::new();
    for it in data
        .get("items")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let uri = it.get("uri").and_then(|x| x.as_str()).unwrap_or("");
        // spotify:playlist:XXX / spotify:artist:XXX / ...
        let kind = uri.split(':').nth(1).unwrap_or("?").to_string();
        if uri.is_empty() || kind == "?" {
            continue;
        }
        items.push(LibraryItem {
            name: it.get("name").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
            detail: it.get("detail").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            uri: uri.to_string(),
            kind,
        });
    }
    Ok(items)
}

/// Un evaluate async: scrollea el sidebar completo leyendo filas por
/// [id^=listrow-title-spotify:] (selectores estables, nada de clases hash).
const LIBRARY_JS: &str = r#"(async () => {
  const grid = document.querySelector('[role="grid"][aria-label="Tu biblioteca"]');
  const box = document.querySelector('#Desktop_LeftSidebar_Id [data-overlayscrollbars-viewport]');
  if (!grid || !box) return JSON.stringify({error:'no-grid'});
  const total = parseInt(grid.getAttribute('aria-rowcount') || '0', 10);
  const out = new Map();
  const read = () => {
    // El URI vive en el id del titulo (listrow-title-spotify:playlist:XXX),
    // NO en la fila: [role="row"] no trae aria-labelledby.
    document.querySelectorAll('[id^="listrow-title-spotify:"]').forEach(p => {
      const id = p.getAttribute('id') || '';
      const m = id.match(/^listrow-title-(spotify:[a-z]+:[A-Za-z0-9]+)$/);
      if (!m) return;
      const uri = m[1];
      if (out.has(uri)) return;
      const row = p.closest('[role="row"]');
      const s = row ? row.querySelector('[data-encore-id="listRowSubtitle"]') : null;
      const t = p.querySelector('span');
      out.set(uri, {name: (t ? t.innerText : p.innerText).trim(), detail: s ? s.innerText.trim() : '', uri});
    });
  };
  read();
  box.scrollTop = 0;
  await new Promise(r => setTimeout(r, 300));
  read();
  let guard = 0, same = 0;
  while (out.size < total && guard < 40) {
    box.scrollTop += Math.max(600, box.clientHeight * 0.9);
    await new Promise(r => setTimeout(r, 300));
    const before = out.size;
    read();
    guard++;
    if (out.size === before) {
      same++;
      if (same >= 3) break;
      if (box.scrollTop + box.clientHeight >= box.scrollHeight - 60) break;
    } else { same = 0; }
  }
  return JSON.stringify({total, items:[...out.values()]});
})()"#;

/// Reproduce un item por su URI: navega a open.spotify.com/{kind}/{id}
/// (misma pestana) y hace click al Play cuando renderiza.
/// Vale para playlist, artist, album y show.
pub async fn play_library_uri(uri: &str) -> Result<String> {
    let mut parts = uri.split(':');
    let kind = parts.nth(1).context("URI sin kind")?;
    let id = parts.next().context("URI sin id")?;
    if !["playlist", "artist", "album", "show"].contains(&kind) {
        anyhow::bail!("kind no reproducible: {kind}");
    }
    let url = format!("https://open.spotify.com/{kind}/{id}");
    let ws_url = spotify_ws_url().await?;
    cdp_call(
        &ws_url,
        20,
        "Page.navigate",
        serde_json::json!({ "url": url }),
    )
    .await?;
    // Esperar render y click (misma logica que el primer play).
    for _ in 0..30 {
        let v = cdp_call(
            &ws_url,
            21,
            "Runtime.evaluate",
            serde_json::json!({ "expression": super::playback::PLAY_JS, "returnByValue": true }),
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
    anyhow::bail!("la pagina {kind} no mostro Play (¿contenido no disponible?)");
}
