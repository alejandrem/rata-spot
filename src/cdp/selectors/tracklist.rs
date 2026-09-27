//! Tracklist: existe/scroll/lectura del grid (playlists Y albumes).

/// Existe el tracklist de la colección (playlist o álbum).
pub(crate) const TRACKLIST_EXISTS_JS: &str =
    "!!(document.querySelector('[data-testid=\"playlist-tracklist\"]')||document.querySelector('[data-testid=\"track-list\"]'))";

/// Scrollea el tracklist al inicio (las primeras filas quedan virtualizadas).
pub(crate) const TRACKLIST_SCROLL_TOP_JS: &str = "(() => { const g = document.querySelector('[data-testid=\"playlist-tracklist\"],[data-testid=\"track-list\"]'); const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null; if (!b) return 'no-box'; b.scrollTop = 0; return 'ok'; })()";

/// Baja una pagina del tracklist (para filas virtualizadas).
pub(crate) const TRACKLIST_SCROLL_JS: &str = r#"(() => { const g = document.querySelector('[data-testid="playlist-tracklist"],[data-testid="track-list"]');
          const b = g ? g.closest('[data-overlayscrollbars-viewport]') : null;
          if (!b) return 'no-box'; b.scrollTop += 800; return 'ok'; })()"#;

/// Lee filas por `a[data-testid="internal-track-link"]` (sin clases hash).
pub(crate) const TRACKS_JS: &str = r#"(async () => {
  const grid = document.querySelector('[data-testid="playlist-tracklist"],[data-testid="track-list"]');
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
