//! Search_api: busqueda via Web API estable (tracks primero, cap 24).

use anyhow::{Context, Result};

use super::super::LibraryItem;
use super::fetch::page_fetch;
use super::token::{encode, web_token};

/// Busca via Web API con el token de la pagina (estable y documentada).
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
    if let Some(arr) = v.pointer("/tracks/items").and_then(|x| x.as_array()) {
        for t in arr.iter().take(12) {
            push(
                t.get("uri").and_then(|x| x.as_str()).unwrap_or(""),
                t.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                &artist_detail(t),
            );
        }
    }
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
