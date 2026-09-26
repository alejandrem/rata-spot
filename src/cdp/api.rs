//! Capa JSON/red (experimental, con fallback al DOM).
//!
//! Idea: el DOM cambia cada semanas; los JSON cada meses/anos. Como Brave
//! ya esta logueado, el `fetch` se hace DENTRO de la pagina (lleva cookies
//! y login): se pide el token web del propio player y con el se llama a la
//! Web API documentada (`api.spotify.com/v1/search`), que regresa items con
//! `{uri, name, artists}` sin scrollear grids virtualizados.
//!
//! Garantia: si el endpoint interno cambia o no hay token, todo regresa
//! Err y el llamador usa el DOM como antes. Lo peor que pasa es un intento
//! HTTP fallido de milisegundos. Kill-switch: `RATA_SPOT_API=0`.
//!
//! NOTA honesta: esto reusa TU sesion de Brave, no hay app OAuth propia
//! (la opcion 3 se rechazo: nada de API oficial con registro).

use anyhow::{Context, Result};

use super::LibraryItem;

/// `fetch` dentro de la pagina (cookies/login incluidos).
/// Regresa (status_http, body). status 0 = ni salio (red/CSP/bloqueo).
pub(crate) async fn page_fetch(url: &str, bearer: Option<&str>) -> Result<(i64, String)> {
    let ws_url = super::tabs::spotify_ws_url().await?;
    let v = super::client::cdp_call_t(
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

/// Token web del propio player (endpoint interno del web-player).
/// Si cambia, Err y se usa el DOM.
pub(crate) async fn web_token() -> Result<String> {
    let (status, body) = page_fetch(
        "https://open.spotify.com/get_access_token?reason=transport&productType=web-player",
        None,
    )
    .await?;
    if status != 200 {
        anyhow::bail!("token web status {status}");
    }
    let v: serde_json::Value = serde_json::from_str(&body).context("token JSON invalido")?;
    v.get("accessToken")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .context("token web sin accessToken")
}

/// Encode minimo para URLs (igual que el del DOM: sin crate extra).
pub(crate) fn encode(q: &str) -> String {
    let mut out = String::with_capacity(q.len());
    for b in q.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Busca via Web API con el token de la pagina (estable y documentada).
/// Tracks primero, luego resto, cap 24 (igual que el DOM).
pub async fn search_via_api(query: &str) -> Result<Vec<LibraryItem>> {
    if std::env::var("RATA_SPOT_API").as_deref() == Ok("0") {
        anyhow::bail!("api desactivada (RATA_SPOT_API=0)");
    }
    let token = web_token().await?;
    search_with_token(query, &token).await
}

/// Busqueda con token ya en mano (para no pedirlo dos veces en diag).
pub(crate) async fn search_with_token(query: &str, token: &str) -> Result<Vec<LibraryItem>> {
    let url = format!(
        "https://api.spotify.com/v1/search?q={}&type=track,artist,playlist,album&limit=12",
        encode(query.trim())
    );
    let (status, body) = page_fetch(&url, Some(token)).await?;
    if status != 200 {
        let hint: String = body.chars().take(120).collect();
        anyhow::bail!("search api status {status}: {hint}");
    }
    let v: serde_json::Value = serde_json::from_str(&body).context("search JSON invalido")?;
    let items = parse_search_api(&v);
    if items.is_empty() {
        anyhow::bail!("api sin items para '{query}'");
    }
    Ok(items)
}

/// Mapea la respuesta /v1/search a LibraryItems (puro, testeable).
pub(crate) fn parse_search_api(v: &serde_json::Value) -> Vec<LibraryItem> {
    let mut out: Vec<LibraryItem> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push = |uri: &str, name: &str, detail: &str| {
        if out.len() >= 24 || uri.is_empty() || name.is_empty() || !seen.insert(uri.to_string()) {
            return;
        }
        let kind = uri.split(':').nth(1).unwrap_or("?").to_string();
        if kind == "?" {
            return;
        }
        out.push(LibraryItem {
            name: name.to_string(),
            detail: detail.to_string(),
            uri: uri.to_string(),
            kind,
        });
    };
    // Tracks primero (igual que el DOM).
    if let Some(arr) = v.pointer("/tracks/items").and_then(|x| x.as_array()) {
        for t in arr.iter().take(12) {
            push(
                t.get("uri").and_then(|x| x.as_str()).unwrap_or(""),
                t.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                &artist_detail(t),
            );
        }
    }
    // Luego artistas / albumes / playlists.
    for key in ["artists", "albums", "playlists"] {
        let ptr = format!("/{key}/items");
        if let Some(arr) = v.pointer(&ptr).and_then(|x| x.as_array()) {
            for t in arr.iter().take(12) {
                let detail = if key == "playlists" {
                    t.get("owner")
                        .and_then(|o| o.get("display_name"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    artist_detail(t)
                };
                push(
                    t.get("uri").and_then(|x| x.as_str()).unwrap_or(""),
                    t.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                    &detail,
                );
            }
        }
    }
    out
}

/// "Artista1, Artista2" desde el array `artists` de un item.
fn artist_detail(t: &serde_json::Value) -> String {
    t.get("artists")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Nombres de recursos de red que la pagina YA pidio (sin cuerpos):
/// dice COMO se llama de verdad el endpoint interno hoy, sin adivinar.
pub(crate) async fn net_probe() -> String {
    let ws_url = match super::tabs::spotify_ws_url().await {
        Ok(u) => u,
        Err(e) => return format!("red: sin pestana ({e:.60})"),
    };
    let v = match super::client::cdp_call(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_espacios_y_tildes() {
        assert_eq!(encode("pika pika"), "pika%20pika");
        // Byte a byte (UTF-8): la ñ se va en %XX mayusculas.
        assert!(encode("niña").contains("%C3%B1"));
        assert_eq!(encode("abc-_.~09AZ"), "abc-_.~09AZ");
    }

    #[test]
    fn fetch_js_escapa_comillas() {
        let js = build_fetch_js("https://x/?q=\"hola\"\\", None);
        assert!(js.contains("\\\"hola\\\""));
        assert!(js.contains(", null)"));
        let js2 = build_fetch_js("https://x/", Some("tok"));
        assert!(js2.contains("\"tok\""));
    }

    /// Respuesta tipica de /v1/search -> LibraryItems con tracks primero.
    #[test]
    fn parsea_search_api() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{
              "tracks": {"items": [
                {"uri": "spotify:track:AAA", "name": "Pika Pika",
                 "artists": [{"name": "A"}, {"name": "B"}]},
                {"uri": "spotify:track:AAA", "name": "Duplicado"},
                {"uri": "", "name": "SinUri"}
              ]},
              "artists": {"items": [
                {"uri": "spotify:artist:BBB", "name": "Pikachu",
                 "artists": []}
              ]},
              "albums": {"items": []},
              "playlists": {"items": [
                {"uri": "spotify:playlist:CCC", "name": "Mix",
                 "owner": {"display_name": "rata"}}
              ]}
            }"#,
        )
        .unwrap();
        let items = parse_search_api(&v);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].kind, "track");
        assert_eq!(items[0].detail, "A, B");
        assert_eq!(items[1].uri, "spotify:artist:BBB");
        assert_eq!(items[2].detail, "rata");
    }
}
