//! Probe: sonda de red (que endpoint existe hoy) + diagnostico API.

use super::search::search_with_token;
use super::token::web_token;

/// Nombres de recursos de red que la pagina YA pidio (sin cuerpos).
pub(crate) async fn net_probe() -> String {
    let ws_url = match super::super::tabs::spotify_ws_url().await {
        Ok(u) => u,
        Err(e) => return format!("red: sin pestana ({e:.60})"),
    };
    let v = match super::super::client::cdp_call(
        &ws_url,
        72,
        "Runtime.evaluate",
        serde_json::json!({ "expression": NET_PROBE_JS, "returnByValue": true }),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return format!("red: fallo sonda ({e:.60})"),
    };
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .unwrap_or("?");
    let short: String = payload.chars().take(900).collect();
    format!("red: {short}")
}

const NET_PROBE_JS: &str = r##"(() => {
  const urls = performance.getEntriesByType('resource').map(r => r.name);
  const hit = urls.filter(u => /get_access_token|pathfinder|spclient|searchview|api\.spotify|query/i.test(u));
  let hosts = [];
  try { hosts = [...new Set(urls.map(u => new URL(u).hostname))]; } catch (e) {}
  let lsKeys = [];
  try { lsKeys = Object.keys(localStorage).slice(0, 20); } catch (e) {}
  return JSON.stringify({total: urls.length, hit: hit.slice(0, 8), hosts: hosts.slice(0, 12), lsKeys});
})()"##;

/// Diagnostico de la capa API (para tests diag, nunca revienta).
#[allow(dead_code)]
pub async fn api_diag() -> String {
    let net = net_probe().await;
    if std::env::var("RATA_SPOT_API").as_deref() == Ok("0") {
        return format!("API: desactivada (RATA_SPOT_API=0) | {net}");
    }
    let token = match web_token().await {
        Ok(t) => t,
        Err(e) => return format!("API: sin token ({e:.100}) | {net}"),
    };
    match search_with_token("pika pika", &token).await {
        Ok(items) => {
            let n_tr = items.iter().filter(|i| i.kind == "track").count();
            format!(
                "API: token ok ({} chars) + search ok ({} items, {} tracks) | {net}",
                token.len(),
                items.len(),
                n_tr
            )
        }
        Err(e) => format!("API: token ok ({} chars) pero search fallo ({e:.100}) | {net}", token.len()),
    }
}
