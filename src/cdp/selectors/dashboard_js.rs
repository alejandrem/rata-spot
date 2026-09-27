//! Dashboard_js: header + letra del track-page (selectores estables).

/// Espera la seccion track-page y cosecha header + letra.
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
