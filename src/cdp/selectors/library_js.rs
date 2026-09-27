//! Library_js: lectura del sidebar + diagnostico del DOM.

/// Scrollea el sidebar leyendo filas por [id^=listrow-title-spotify:].
pub(crate) const LIBRARY_JS: &str = r#"(async () => {
  const grid = document.querySelector('[role="grid"][aria-label="Tu biblioteca"]');
  const box = document.querySelector('#Desktop_LeftSidebar_Id [data-overlayscrollbars-viewport]');
  if (!grid || !box) return JSON.stringify({error:'no-grid'});
  const total = parseInt(grid.getAttribute('aria-rowcount') || '0', 10);
  const out = new Map();
  let waited = 0;
  while (document.querySelectorAll('[id^="listrow-title-spotify:"]').length === 0 && waited < 25) {
    await new Promise(r => setTimeout(r, 300));
    waited++;
  }
  const read = () => {
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

/// Diagnostico del DOM: grid, rowcount, ids, sidebar, url (test diag_estado).
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
