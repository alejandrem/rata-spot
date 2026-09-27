//! Health: seis sondas no destructivas + resumen en una linea.

/// Sondas sobre la pagina ACTUAL (home, search, playlist...).
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
pub async fn health_summary() -> String {
    let ws_url = match super::super::tabs::spotify_ws_url().await {
        Ok(u) => u,
        Err(e) => return format!("DOM check pendiente: {e:.80}"),
    };
    let v = match super::super::client::cdp_call(
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
