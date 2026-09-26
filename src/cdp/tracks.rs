//! Canciones de una playlist: leer el tracklist del DOM y tocar por track.
//!
//! La pagina de playlist tiene grid virtualizado
//! `[data-testid="playlist-tracklist"] [role="row"]` con filas como:
//! numero + `a[data-testid="internal-track-link"][href$=/track/{id}]`
//! (titulo) + links `a[href*="/artist/"]` + duracion `m:ss`.
//! Cada fila trae `button[aria-label="Reproducir ..."]` para tocarla
//! directo en contexto de la playlist.

use std::time::Duration;

use anyhow::{Context, Result};

use super::client::cdp_call;
use super::tabs::spotify_ws_url;

/// Una rola del tracklist: numero, titulo, artistas, duracion y track id.
#[derive(Debug, Clone)]
pub struct TrackItem {
    pub n: String,
    pub title: String,
    pub artist: String,
    pub duration: String,
    pub id: String,
}

/// Navega la pestana a la pagina del URI (playlist/album/...) SIN dar play.
/// Despues se leen las rolas con `playlist_tracks()`.
pub async fn open_playlist(uri: &str) -> Result<()> {
    let mut parts = uri.split(':');
    let kind = parts.nth(1).context("URI sin kind")?;
    let id = parts.next().context("URI sin id")?;
    let url = format!("https://open.spotify.com/{kind}/{id}");
    let ws_url = spotify_ws_url().await?;
    super::client::cdp_call(
        &ws_url,
        30,
        "Page.navigate",
        serde_json::json!({ "url": url }),
    )
    .await?;
    // Esperar a que el tracklist exista antes de regresar (hasta 10s).
    // Sin esto, el lector llegaba con la pagina a medias (o con otra
    // navegacion en curso) y devolvia no-tracklist.
    for _ in 0..20 {
        let v = super::client::cdp_call(
            &ws_url,
            31,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "!!document.querySelector('[data-testid=\"playlist-tracklist\"]')",
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

/// Lee TODAS las rolas del tracklist abierto (con scroll, virtualizado).
/// Reintenta si la pagina aun no renderiza el grid.
pub async fn playlist_tracks() -> Result<Vec<TrackItem>> {
    let mut last_err = String::new();
    for _ in 0..6 {
        match tracks_snapshot().await {
            Ok(items) if !items.is_empty() => return Ok(items),
            Ok(_) => last_err = "tracklist vacio".to_string(),
            Err(e) => last_err = e.to_string(),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("canciones no listas: {last_err}")
}

async fn tracks_snapshot() -> Result<Vec<TrackItem>> {
    let ws_url = spotify_ws_url().await?;
    let v = cdp_call(
        &ws_url,
        31,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": TRACKS_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("tracklist sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("tracklist JSON invalido")?;
    if let Some(err) = data.get("error").and_then(|x| x.as_str()) {
        anyhow::bail!("tracklist: {err}");
    }
    let mut items = Vec::new();
    for it in data
        .get("tracks")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let id = it.get("id").and_then(|x| x.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        items.push(TrackItem {
            n: it.get("n").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            title: it.get("title").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
            artist: it.get("artist").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            duration: it.get("duration").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            id: id.to_string(),
        });
    }
    Ok(items)
}

/// Un evaluate async: scrollea el contenedor principal leyendo filas por
/// `a[data-testid="internal-track-link"]` (estable, sin clases hash).
const TRACKS_JS: &str = r#"(async () => {
  const grid = document.querySelector('[data-testid="playlist-tracklist"]');
  if (!grid) return JSON.stringify({error:'no-tracklist'});
  const box = grid.closest('[data-overlayscrollbars-viewport]')
    || document.querySelector('.main-view-container [data-overlayscrollbars-viewport]');
  if (!box) return JSON.stringify({error:'no-scroller'});
  const total = parseInt(grid.getAttribute('aria-rowcount') || '0', 10);
  const out = new Map();
  const read = () => {
    grid.querySelectorAll('[role="row"]').forEach(r => {
      const link = r.querySelector('a[data-testid="internal-track-link"]');
      if (!link) return;
      const href = link.getAttribute('href') || '';
      const m = href.match(/\/track\/([A-Za-z0-9]+)/);
      if (!m) return;
      const id = m[1];
      if (out.has(id)) return;
      const titleEl = link.querySelector('div');
      const artists = [...r.querySelectorAll('a[href*="/artist/"]')]
        .map(a => (a.innerText || '').trim()).filter(Boolean);
      let dur = '', num = '';
      r.querySelectorAll('[role="gridcell"]').forEach((c, i) => {
        const t = (c.innerText || '').trim();
        const mm = t.match(/(\d+:\d{2})/);
        if (mm) dur = mm[1];
        if (i === 0) {
          const sp = c.querySelector('span');
          if (sp) num = (sp.innerText || '').trim();
        }
      });
      out.set(id, {n: num, title: titleEl ? titleEl.innerText.trim() : link.innerText.trim(),
        artist: artists.join(', '), duration: dur, id});
    });
  };
  read();
  box.scrollTop = 0;
  await new Promise(r => setTimeout(r, 300));
  read();
  let guard = 0, same = 0;
  while (out.size < total && guard < 60) {
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
  return JSON.stringify({total, tracks:[...out.values()]});
})()"#;

/// Toca una rola por su track id: busca su fila (scrolleando si esta
/// virtualizada), le hace scrollIntoView y click a su boton Reproducir.
/// Suena en contexto de la playlist abierta.
pub async fn play_track(track_id: &str) -> Result<String> {
    let ws_url = spotify_ws_url().await?;
    // id alfanumerico de Spotify: seguro para interpolar en el JS.
    if !track_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        anyhow::bail!("track id raro: {track_id}");
    }
    // Empezar desde arriba: el lector deja el scroll al fondo y las
    // primeras filas quedan virtualizadas (fuera del DOM).
    let _ = super::client::cdp_call(
        &ws_url,
        34,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": "(() => { const g = document.querySelector('[data-testid=\"playlist-tracklist\"]'); const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null; if (!b) return 'no-box'; b.scrollTop = 0; return 'ok'; })()",
            "returnByValue": true,
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let click_js = format!(
        r#"(() => {{
          const a = document.querySelector('a[data-testid="internal-track-link"][href*="/track/{track_id}"]');
          if (!a) return 'no-row';
          const row = a.closest('[role="row"]');
          if (!row) return 'no-row';
          row.scrollIntoView({{block:'center'}});
          const b = row.querySelector('button[aria-label^="Reproducir"]');
          if (!b) return 'no-button';
          b.click();
          return 'clicked';
        }})()"#
    );
    let scroll_js =
        r#"(() => { const g = document.querySelector('[data-testid="playlist-tracklist"]');
          const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null;
          if (!b) return 'no-box'; b.scrollTop += 800; return 'ok'; })()"#;
    for _ in 0..40 {
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
        // Fila virtualizada o aun no render: scrollear y reintentar.
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
