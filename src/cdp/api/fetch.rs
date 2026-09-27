//! Fetch: `fetch` DENTRO de la pagina (cookies/login incluidos).

use anyhow::{Context, Result};

/// Regresa (status_http, body). status 0 = ni salio (red/CSP/bloqueo).
pub(crate) async fn page_fetch(url: &str, bearer: Option<&str>) -> Result<(i64, String)> {
    let ws_url = super::super::tabs::spotify_ws_url().await?;
    let v = super::super::client::cdp_call_t(
        &ws_url,
        71,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": build_fetch_js(url, bearer),
            "returnByValue": true,
            "awaitPromise": true,
        }),
        15,
    )
    .await?;
    let payload = v
        .pointer("/result/result/value")
        .and_then(|x| x.as_str())
        .context("fetch sin valor")?;
    let data: serde_json::Value =
        serde_json::from_str(payload).context("fetch JSON invalido")?;
    let status = data.get("status").and_then(|x| x.as_i64()).unwrap_or(0);
    let body = data
        .get("body")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    Ok((status, body))
}

/// Arma el evaluate del fetch (puro, testeable: escapa comillas).
pub(crate) fn build_fetch_js(url: &str, bearer: Option<&str>) -> String {
    let u = js_escape(url);
    let tok = match bearer {
        Some(t) => format!("\"{}\"", js_escape(t)),
        None => "null".to_string(),
    };
    format!(
        r##"(async (url, token) => {{
      try {{
        const headers = {{'Accept': 'application/json'}};
        if (token) headers['Authorization'] = 'Bearer ' + token;
        const r = await fetch(url, {{credentials: 'include', headers}});
        const t = await r.text();
        return JSON.stringify({{status: r.status, body: t.slice(0, 30000)}});
      }} catch (e) {{ return JSON.stringify({{status: 0, body: String(e).slice(0, 300)}}); }}
    }})("{u}", {tok})"##
    )
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
