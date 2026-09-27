//! Search_js: lee anchors /track|artist|playlist|album fuera del sidebar.

/// Tracks primero (tope 12), luego resto (tope 12). Cap total 24.
pub(crate) const SEARCH_JS: &str = r#"(async () => {
  let w = 0;
  const count = () => document.querySelectorAll('main a[href*="/track/"],main a[href*="/artist/"],main a[href*="/playlist/"],main a[href*="/album/"]').length;
  while (count() < 3 && w < 16) { await new Promise(r => setTimeout(r, 500)); w++; }
  const out = [];
  const seen = new Set();
  const clean = (s) => (s || '').trim().split('\n')[0].trim();
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
