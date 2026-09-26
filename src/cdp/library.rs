//! Lectura de "Tu biblioteca" del DOM + reproducir items por URI.
//!
//! El grid esta virtualizado (solo filas visibles en el DOM): se scrollea
//! el sidebar, se dedupea por URI y se filtra en Rust por prefijo del URI
//! (robusto ante idiomas). El URI sale del id del titulo:
//! `listrow-title-spotify:playlist:XXX` (la fila NO trae aria-labelledby).

use std::time::Duration;

use anyhow::{Context, Result};

use super::tabs::{bring_spotify_front, spotify_ws_url};

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
    for _ in 0..3 {
        match library_snapshot().await {
            Ok((items, _)) if !items.is_empty() => return Ok(items),
            Ok((_, stuck)) => {
                if stuck {
                    // Esqueletos atorados: 1) ventana ocluida -> traer al
                    // frente y releer; 2) tab wedged por churn (navegaciones
                    // rapidas de tests) -> renavegar al home SOLO si nada
                    // suena (no arruinar reproduccion ajena).
                    let _ = bring_spotify_front().await;
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    if let Ok((items2, _)) = library_snapshot().await {
                        if !items2.is_empty() {
                            return Ok(items2);
                        }
                    }
                    if crate::gsmtc::get_brave_session().await.is_err() {
                        renavigate_home().await?;
                        tokio::time::sleep(Duration::from_secs(4)).await;
                        if let Ok((items3, _)) = library_snapshot().await {
                            if !items3.is_empty() {
                                return Ok(items3);
                            }
                        }
                    }
                }
                last_err = "biblioteca vacia (¿captcha? recarga Brave)".to_string();
            }
            Err(e) => last_err = e.to_string(),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("biblioteca no lista: {last_err}")
}

/// Renavega la pestana al home (recarga biblioteca atorada).
async fn renavigate_home() -> Result<()> {
    let ws_url = spotify_ws_url().await?;
    super::client::cdp_call(
        &ws_url,
        12,
        "Page.navigate",
        serde_json::json!({ "url": crate::launcher::SPOTIFY_URL }),
    )
    .await?;
    Ok(())
}

async fn library_snapshot() -> Result<(Vec<LibraryItem>, bool)> {
    let ws_url = spotify_ws_url().await?;
    // Snapshot con scroll: tarda 10s+ legitimos en bibliotecas grandes.
    let v = super::client::cdp_call_t(
        &ws_url,
        10,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": LIBRARY_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
        30,
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
    let total = data.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    let stuck = items.is_empty() && total > 0;
    Ok((items, stuck))
}

/// Diagnostico del DOM para depurar: grid, rowcount, ids, sidebar, url.
/// Solo la usa el test diag_estado.
#[allow(dead_code)]
pub async fn library_diag() -> Result<String> {
    let ws_url = spotify_ws_url().await?;
    let v = super::client::cdp_call(
        &ws_url,
        11,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": DIAG_JS,
            "returnByValue": true,
        }),
    )
    .await?;
    Ok(v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .unwrap_or("?")
        .to_string())
}

#[allow(dead_code)]
const DIAG_JS: &str = r#"(() => {
  const grid = document.querySelector('[role="grid"][aria-label="Tu biblioteca"]');
  const box = document.querySelector('#Desktop_LeftSidebar_Id [data-overlayscrollbars-viewport]');
  return JSON.stringify({
    url: location.href, title: document.title.slice(0, 60), ready: document.readyState,
    hasGrid: !!grid, rowcount: grid ? grid.getAttribute('aria-rowcount') : null,
    hasBox: !!box,
    titleIds: document.querySelectorAll('[id^="listrow-title-spotify:"]').length,
    libRows: grid ? grid.querySelectorAll('[role="row"]').length : -1,
    gridHtml: grid ? grid.outerHTML.slice(0, 400) : null,
    sidebars: document.querySelectorAll('#Desktop_LeftSidebar_Id').length,
    grids: [...document.querySelectorAll('[role="grid"]')].map(g => g.getAttribute('aria-label'))
  });
})()"#;

/// Un evaluate async: scrollea el sidebar completo leyendo filas por
/// [id^=listrow-title-spotify:] (selectores estables, nada de clases hash).
const LIBRARY_JS: &str = r#"(async () => {
  const grid = document.querySelector('[role="grid"][aria-label="Tu biblioteca"]');
  const box = document.querySelector('#Desktop_LeftSidebar_Id [data-overlayscrollbars-viewport]');
  if (!grid || !box) return JSON.stringify({error:'no-grid'});
  const total = parseInt(grid.getAttribute('aria-rowcount') || '0', 10);
  const out = new Map();
  // Esperar filas REALES: el grid a veces llega con esqueletos
  // (rowcount>0 pero cero titulos) mientras carga la biblioteca.
  let waited = 0;
  while (document.querySelectorAll('[id^="listrow-title-spotify:"]').length === 0 && waited < 25) {
    await new Promise(r => setTimeout(r, 300));
    waited++;
  }
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
