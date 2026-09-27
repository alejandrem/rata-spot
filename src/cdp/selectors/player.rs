//! Player: aria del player + click al Play + click a fila exacta.

/// aria-label del boton play/pause ("" si no hay player).
pub(crate) const PLAYER_ARIA_JS: &str =
    "document.querySelector('[data-testid=\"control-button-playpause\"]')?.getAttribute('aria-label') || ''";

/// Click al Play probando varios selectores (ES + EN + player bar).
/// Solo botones VISIBLES y habilitados (los ocultos dan click fantasma).
/// ORDEN: player-bar primero (semantica de [space]), heroes despues.
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

/// Click a la fila exacta de un track (por id, nada de loteria).
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
