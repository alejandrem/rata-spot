//! Dashboard individual de la rola: lee `section[data-testid="track-page"]`
//! (titulo, artista, album, duracion, año, reproducciones + letra).
//! A veces la pagina tarda en pintarlo: espera hasta 8s antes de rendirse.

use anyhow::{Context, Result};

use super::tabs::spotify_ws_url;

/// Dashboard de la cancion individual para la TUI.
#[derive(Debug, Clone, Default)]
pub struct TrackDetail {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub year: String,
    pub playcount: String,
    pub lyrics: Vec<String>,
}

/// Lee el dashboard de la pagina de track abierta.
pub async fn track_detail() -> Result<TrackDetail> {
    // Self-healing como open_page (pestana cerrada a media sesion).
    super::tabs::ensure_spotify_tab().await?;
    let ws_url = spotify_ws_url().await?;
    let v = super::client::cdp_call_t(
        &ws_url,
        42,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": DASHBOARD_JS,
            "returnByValue": true,
            "awaitPromise": true,
        }),
        30,
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("dashboard sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("dashboard JSON invalido")?;
    if let Some(err) = data.get("error").and_then(|x| x.as_str()) {
        anyhow::bail!("dashboard: {err}");
    }
    let get = |k: &str| {
        data.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    Ok(TrackDetail {
        title: get("title"),
        artist: get("artist"),
        album: get("album"),
        duration: get("duration"),
        year: get("year"),
        playcount: get("playcount"),
        lyrics: data
            .get("lyrics")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .take(40)
            .collect(),
    })
}

/// Espera la seccion track-page y cosecha header + letra con selectores
/// estables (data-testid, nada de clases hash).
const DASHBOARD_JS: &str = r#"(async () => {
  let w = 0;
  while (!document.querySelector('section[data-testid="track-page"]') && w < 16) {
    await new Promise(r => setTimeout(r, 500)); w++;
  }
  const sec = document.querySelector('section[data-testid="track-page"]');
  if (!sec) return JSON.stringify({error:'no-track-page'});
  const head = sec.querySelector('[data-testid="entity-header"]');
  if (!head) return JSON.stringify({error:'no-entity-header'});
  const txt = (s) => { const el = head.querySelector(s); return el ? el.innerText.trim() : ''; };
  const titleEl = head.querySelector('[data-testid="entityTitle"]');
  const albumA = head.querySelector('a[href*="/album/"]');
  let duration = '';
  head.querySelectorAll('span').forEach(sp => {
    const t = (sp.innerText || '').trim();
    if (!duration && /^\d+:\d{2}$/.test(t)) duration = t;
  });
  const lyrics = [...sec.querySelectorAll('p[data-testid="lyrics-line-always-visible"]')]
    .map(p => (p.innerText || '').trim()).filter(Boolean);
  return JSON.stringify({
    title: titleEl ? titleEl.innerText.trim() : '',
    artist: txt('[data-testid="creator-link"]'),
    album: albumA ? albumA.innerText.trim() : '',
    duration,
    year: txt('[data-testid="release-date"]'),
    playcount: txt('[data-testid="playcount"]'),
    lyrics
  });
})()"#;
