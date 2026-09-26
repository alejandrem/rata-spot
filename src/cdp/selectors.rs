//! Selectores del DOM de Spotify Web en UN SOLO LUGAR.
//!
//! Problema: el DOM cambia cada semanas; con los JS regados en 5 archivos
//! cada cambio era caceria. Aqui vive TODO lo que toca el DOM, con su
//! cadena de fallback (estable -> menos estable):
//!
//! 1. `data-testid` (contrato interno de tests de Spotify, cambia poco)
//! 2. `role` + `href` con IDs (`/track/{id}` es contrato de URL)
//! 3. `aria-label` por substring ES/EN (cambia con idioma/rediseno)
//! 4. clases CSS con hash: PROHIBIDAS (mueren en cada deploy)
//!
//! Como agregar un selector nuevo: pon su nombre canonico arriba como
//! const + usalo en el JS de abajo + agrega un check en HEALTH_JS.
//! Los tests al fondo obligan a que el JS mencione las consts (si cambias
//! una const y el JS queda viejo, el test lo grita).
//!
//! `health_summary()` corre en boot (best-effort): dice que vive y que
//! murio sin tener que adivinar.

// ---------------------------------------------------------------------------
// Nombres canonicos (si Spotify cambia uno, se cambia AQUI)
// ---------------------------------------------------------------------------

/// Barra del player: el boton play/pause real.
#[allow(dead_code)]
pub const PLAY_TESTIDS: &[&str] = &["control-button-playpause", "play-button"];

/// Substrings de aria-label para el fallback ES/EN (case-insensitive en JS).
#[allow(dead_code)]
pub const PLAY_ARIA_NEEDLES: &[&str] = &["reproducir", "play"];

/// Sidebar izquierda de Brave/Spotify.
#[allow(dead_code)]
pub const SIDEBAR_ID: &str = "#Desktop_LeftSidebar_Id";

/// Prefijo de los ids de titulos en la biblioteca virtualizada.
#[allow(dead_code)]
pub const LIBRARY_ROW_PREFIX: &str = "listrow-title-spotify:";

/// Contenedor del tracklist de playlist.
#[allow(dead_code)]
pub const TRACKLIST_TESTID: &str = "playlist-tracklist";

/// Anchor del titulo de cada rola (estable, sin clases hash).
#[allow(dead_code)]
pub const TRACK_LINK_TESTID: &str = "internal-track-link";

/// Seccion del dashboard individual de rola.
#[allow(dead_code)]
pub const TRACK_PAGE_TESTID: &str = "track-page";

// ---------------------------------------------------------------------------
// Snippets chicos (una expresion JS cada uno)
// ---------------------------------------------------------------------------

/// Existe el tracklist de la playlist (para open_playlist).
pub(crate) const TRACKLIST_EXISTS_JS: &str =
    "!!document.querySelector('[data-testid=\"playlist-tracklist\"]')";

/// aria-label del boton play/pause ("" si no hay player).
pub(crate) const PLAYER_ARIA_JS: &str =
    "document.querySelector('[data-testid=\"control-button-playpause\"]')?.getAttribute('aria-label') || ''";

/// Scrollea el tracklist al inicio (el lector lo deja al fondo y las
/// primeras filas quedan virtualizadas, fuera del DOM).
pub(crate) const TRACKLIST_SCROLL_TOP_JS: &str = "(() => { const g = document.querySelector('[data-testid=\"playlist-tracklist\"]'); const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null; if (!b) return 'no-box'; b.scrollTop = 0; return 'ok'; })()";

/// Baja una pagina del tracklist (para filas virtualizadas).
pub(crate) const TRACKLIST_SCROLL_JS: &str = r#"(() => { const g = document.querySelector('[data-testid="playlist-tracklist"]');
          const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null;
          if (!b) return 'no-box'; b.scrollTop += 800; return 'ok'; })()"#;

/// Diagnostico rapido de la pagina: titulo + hay player + hay login.
pub(crate) const PAGE_STATE_JS: &str = r#"(() => {
  const q = (s) => !!document.querySelector(s);
  return document.title + ' | play=' + q('[data-testid="control-button-playpause"]')
    + ' | login=' + (q('[data-testid="login-button"]') || q('a[href*="login"]'));
})()"#;

/// Click a la fila exacta de un track (por id, nada de loteria).
/// `track_id` debe ser alfanumerico (lo valida el llamador).
pub(crate) fn track_row_click_js(track_id: &str) -> String {
    format!(
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
    )
}

// ---------------------------------------------------------------------------
// Bloques grandes (movidos aqui desde playback/library/search/tracks/...)
// Comportamiento IDENTICO al original; solo cambio el domicilio.
// ---------------------------------------------------------------------------

/// Click al Play probando varios selectores (ES + EN + player bar).
/// Solo botones VISIBLES y habilitados: querySelector tambien encuentra
/// los ocultos y clickearlos no suena (click fantasma).
/// ORDEN: player-bar primero (retoma la cola = semantica de [space]),
/// heroes despues. Para tocar algo ESPECIFICO usar track_row_click_js,
/// no esto: el primer visible es loteria con 100+ botones en el DOM.
pub(crate) const PLAY_CLICK_JS: &str = r#"(() => {
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

/// Un evaluate async: scrollea el sidebar completo leyendo filas por
/// [id^=listrow-title-spotify:] (selectores estables, nada de clases hash).
pub(crate) const LIBRARY_JS: &str = r#"(async () => {
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

/// Diagnostico del DOM para depurar: grid, rowcount, ids, sidebar, url.
/// Solo la usa el test diag_estado.
pub(crate) const LIBRARY_DIAG_JS: &str = r#"(() => {
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

/// Lee anchors /track|artist|playlist|album|show fuera del sidebar y del
/// reproductor (esos contaminarian). Tracks primero, luego el resto.
pub(crate) const SEARCH_JS: &str = r#"(async () => {
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

/// Un evaluate async: scrollea el contenedor principal leyendo filas por
/// `a[data-testid="internal-track-link"]` (estable, sin clases hash).
pub(crate) const TRACKS_JS: &str = r#"(async () => {
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

/// Espera la seccion track-page y cosecha header + letra con selectores
/// estables (data-testid, nada de clases hash).
pub(crate) const DASHBOARD_JS: &str = r#"(async () => {
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

// ---------------------------------------------------------------------------
// Health-check: un solo evaluate, seis sondas, cero navegacion
// ---------------------------------------------------------------------------

/// Sondas no destructivas sobre la pagina ACTUAL (sirve home, search,
/// playlist...): cada check dice ok + detalle de QUE matcheo.
const HEALTH_JS: &str = r##"(() => {
  const q = (s) => document.querySelector(s);
  const qa = (s) => [...document.querySelectorAll(s)];
  const grids = qa('[role="grid"]');
  const libGrid = q('[role="grid"][aria-label="Tu biblioteca"]');
  const libGridAny = grids.find(g => /biblioteca|library/i.test(g.getAttribute('aria-label') || ''));
  const playBar = q('[data-testid="control-button-playpause"]');
  const ariaBtns = qa('button[aria-label]').filter(b => /reproducir|play/i.test(b.getAttribute('aria-label') || ''));
  const trackAnchors = qa('main a[href*="/track/"]').length;
  const side = q('#Desktop_LeftSidebar_Id');
  const checks = [
    {name:'sidebar', ok: !!side, detail: side ? 'presente' : 'sin #Desktop_LeftSidebar_Id'},
    {name:'library_grid', ok: !!(libGrid || libGridAny), detail: libGrid ? 'exacto ES' : (libGridAny ? ('fallback: ' + (libGridAny.getAttribute('aria-label') || '').slice(0, 40)) : 'sin grid de biblioteca')},
    {name:'player_bar', ok: !!playBar, detail: playBar ? ('aria=' + (playBar.getAttribute('aria-label') || '').slice(0, 30)) : 'sin control-button-playpause'},
    {name:'play_fallback', ok: ariaBtns.length > 0, detail: ariaBtns.length + ' botones reproducir/play'},
    {name:'track_urls', ok: trackAnchors > 0, detail: trackAnchors + ' anchors /track/'},
    {name:'main', ok: !!q('main'), detail: q('main') ? 'presente' : 'sin <main>'}
  ];
  return JSON.stringify({url: location.href, checks});
})()"##;

/// Health-check best-effort: NUNCA revienta, regresa string para el status.
/// Sin pestana Spotify -> "pendiente" (no es error, es "aun cargando").
pub async fn health_summary() -> String {
    let ws_url = match super::tabs::spotify_ws_url().await {
        Ok(u) => u,
        Err(e) => return format!("DOM check pendiente: {e:.80}"),
    };
    let v = match super::client::cdp_call(
        &ws_url,
        70,
        "Runtime.evaluate",
        serde_json::json!({ "expression": HEALTH_JS, "returnByValue": true }),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return format!("DOM check pendiente: {e:.80}"),
    };
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if payload.is_empty() {
        return "DOM check: sin respuesta".to_string();
    }
    summarize_health(payload)
}

/// Resume el payload del health a una linea (puro, testeable).
pub(crate) fn summarize_health(payload: &str) -> String {
    let v: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return "DOM check: respuesta ilegible".to_string(),
    };
    let checks = v
        .get("checks")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    if checks.is_empty() {
        return "DOM check: sin checks".to_string();
    }
    let is_ok =
        |c: &serde_json::Value| c.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    let ok = checks.iter().filter(|c| is_ok(c)).count();
    let bad: Vec<String> = checks
        .iter()
        .filter(|c| !is_ok(c))
        .filter_map(|c| {
            c.get("name")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    if bad.is_empty() {
        format!("DOM {ok}/{} ok", checks.len())
    } else {
        format!("DOM {ok}/{} ok, fallan: {}", checks.len(), bad.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las consts canonicas y su JS no se pueden divorciar: si cambias una
    /// const y el JS queda viejo, esto grita antes que el usuario.
    #[test]
    fn js_menciona_sus_consts() {
        assert!(PLAY_CLICK_JS.contains(PLAY_TESTIDS[0]));
        assert!(PLAY_CLICK_JS.contains(PLAY_TESTIDS[1]));
        assert!(PLAY_CLICK_JS.contains("Reproducir"));
        assert!(LIBRARY_JS.contains(LIBRARY_ROW_PREFIX));
        assert!(LIBRARY_JS.contains(SIDEBAR_ID));
        assert!(LIBRARY_DIAG_JS.contains(LIBRARY_ROW_PREFIX));
        assert!(SEARCH_JS.contains("/track/"));
        assert!(SEARCH_JS.contains(SIDEBAR_ID));
        assert!(TRACKS_JS.contains(TRACK_LINK_TESTID));
        assert!(TRACKS_JS.contains(TRACKLIST_TESTID));
        assert!(TRACKLIST_EXISTS_JS.contains(TRACKLIST_TESTID));
        assert!(TRACKLIST_SCROLL_TOP_JS.contains(TRACKLIST_TESTID));
        assert!(TRACKLIST_SCROLL_JS.contains(TRACKLIST_TESTID));
        assert!(PLAYER_ARIA_JS.contains(PLAY_TESTIDS[0]));
        assert!(DASHBOARD_JS.contains(TRACK_PAGE_TESTID));
        assert!(track_row_click_js("abc123").contains(TRACK_LINK_TESTID));
        assert!(track_row_click_js("abc123").contains("abc123"));
    }

    /// El resumen del health es legible en una linea de status.
    #[test]
    fn resumen_health() {
        let ok = r#"{"checks":[{"name":"sidebar","ok":true},{"name":"main","ok":true}]}"#;
        assert_eq!(summarize_health(ok), "DOM 2/2 ok");
        let fail = r#"{"checks":[{"name":"sidebar","ok":true},{"name":"library_grid","ok":false},{"name":"main","ok":false}]}"#;
        assert_eq!(
            summarize_health(fail),
            "DOM 1/3 ok, fallan: library_grid,main"
        );
        assert!(summarize_health("no-json").contains("ilegible"));
        assert!(summarize_health("{}").contains("sin checks"));
    }
}
