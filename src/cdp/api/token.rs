//! Token: token web del player + encode minimo para URLs.

use anyhow::{Context, Result};

use super::fetch::page_fetch;

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

/// Encode minimo para URLs (sin crate extra).
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
