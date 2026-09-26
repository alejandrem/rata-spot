//! Control de la pagina Spotify via Chrome DevTools Protocol (sin API Spotify).
//!
//! Problema que resuelve: GSMTC solo controla lo que YA esta sonando.
//! En una pagina fresca (recien abierta, sin cola) `TryPlayAsync` no hace
//! nada y [space] parece muerto. Con CDP la TUI hace click al Play de la
//! pagina y la musica arranca sin tocar el mouse.
//!
//! Requiere Brave lanzado con `--remote-debugging-port` (ver launcher).
//! Si el puerto no existe (Brave abierto a mano sin el flag), regresa error
//! y la TUI muestra "dale play una vez en Brave".

use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const CDP_HOST: &str = "127.0.0.1";
pub const CDP_PORT: u16 = 9222;

/// Localiza el WebSocket de la pestana Spotify via GET /json/list
/// (HTTP crudo, sin reqwest para no engordar dependencias).
async fn spotify_ws_url() -> Result<String> {
    // Timeout global: localhost responde en ms; si algo cuelga (connect,
    // lectura sin EOF), fallar rapido en vez de trabar TUI/test eterno.
    let body = tokio::time::timeout(Duration::from_secs(8), fetch_json_list())
        .await
        .map_err(|_| anyhow::anyhow!("timeout hablando con CDP"))??;

    let list: serde_json::Value =
        serde_json::from_str(&body).context("JSON CDP invalido")?;
    let arr = list.as_array().context("lista CDP no es array")?;
    for t in arr {
        let ty = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "page" && url.contains("open.spotify") {
            return t
                .get("webSocketDebuggerUrl")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .context("tab Spotify sin WS");
        }
    }
    anyhow::bail!("no hay pestana Spotify en el Brave depurable")
}

/// GET a un endpoint CDP (/json/list, /json/version) leyendo por
/// Content-Length (NO read_to_end: el servidor a veces no cierra la
/// conexion y eso colgaba todo para siempre).
/// El servidor CDP exige HTTP/1.1 (con 1.0 cierra sin responder).
async fn fetch_cdp_text(path: &str) -> Result<String> {
    let mut stream = TcpStream::connect((CDP_HOST, CDP_PORT))
        .await
        .context("sin puerto CDP (Brave sin --remote-debugging-port)")?;

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {CDP_HOST}:{CDP_PORT}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await?;

    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut tmp).await.context("lectura CDP")?;
        if n == 0 {
            anyhow::bail!("CDP cerro sin responder");
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(p) = find_crlf2(&buf) {
            break p;
        }
        if buf.len() > 65536 {
            anyhow::bail!("headers CDP gigantes");
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
    let mut body = buf[head_end + 4..].to_vec();

    if head.contains("transfer-encoding: chunked") {
        while !body.ends_with(b"\r\n0\r\n") && !body.ends_with(b"\n0\n") {
            let n = stream.read(&mut tmp).await.context("lectura chunk")?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..n]);
            if body.len() > 4_000_000 {
                anyhow::bail!("body CDP gigante");
            }
        }
        return Ok(dechunk(&String::from_utf8_lossy(&body)));
    }

    let len: usize = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim() == "content-length" {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    while body.len() < len {
        let n = stream.read(&mut tmp).await.context("lectura body")?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(len);
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// GET /json/list via el lector robusto.
async fn fetch_json_list() -> Result<String> {
    fetch_cdp_text("/json/list").await
}

fn find_crlf2(hay: &[u8]) -> Option<usize> {
    hay.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Decodifica un body HTTP con Transfer-Encoding: chunked
/// (hex-size + datos por bloque, termina en bloque 0).
fn dechunk(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // leer linea de tamano (hasta \n), ignorar extensiones con ';'
        let mut j = i;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        let line = std::str::from_utf8(&bytes[i..j.min(bytes.len())])
            .unwrap_or("")
            .trim_end_matches(&['\r', '\n'][..])
            .trim();
        let size_str = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16).unwrap_or(0);
        i = j + 1; // saltar el \n
        if size == 0 {
            break;
        }
        let end = (i + size).min(bytes.len());
        out.extend_from_slice(&bytes[i..end]);
        i = end;
        // saltar el CRLF tras los datos
        if bytes.get(i) == Some(&b'\r') {
            i += 1;
        }
        if bytes.get(i) == Some(&b'\n') {
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Click al Play probando varios selectores (ES + EN + player bar).
const PLAY_JS: &str = r#"(() => {
  const sels = [
    '[data-testid="control-button-playpause"]',
    '[data-testid="play-button"]',
    'button[aria-label="Reproducir"]',
    'button[aria-label="Play"]'
  ];
  for (const s of sels) {
    const b = document.querySelector(s);
    if (b) { b.click(); return 'clicked:' + s; }
  }
  return 'no-button';
})()"#;

/// Hace click al Play de Spotify. OkClick -> la sesion GSMTC aparece sola
/// y el loop reactivo la toma.
/// La pagina fresca tarda en renderizar el player: reintenta el click
/// hasta 15s. Si no aparece, diagnostica que muestra (login vs cargando).
pub async fn play_spotify() -> Result<String> {
    let ws_url = spotify_ws_url().await?;
    for _ in 0..30 {
        let v = cdp_call(
            &ws_url,
            1,
            "Runtime.evaluate",
            serde_json::json!({ "expression": PLAY_JS, "returnByValue": true }),
        )
        .await?;
        let out = v
            .pointer("/result/result/value")
            .and_then(|x| x.as_str())
            .unwrap_or("?")
            .to_string();
        if out.starts_with("clicked:") {
            return Ok(out);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let state = cdp_call(
        &ws_url,
        3,
        "Runtime.evaluate",
        serde_json::json!({ "expression": STATE_JS, "returnByValue": true }),
    )
    .await
    .ok()
    .and_then(|v| {
        v.pointer("/result/result/value")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
    })
    .unwrap_or_else(|| "?".to_string());
    anyhow::bail!("pagina sin boton Play visible [{state}] (¿logueado? ¿cargo?)");
}

/// Diagnostico rapido de la pagina: titulo + hay player + hay login.
const STATE_JS: &str = r#"(() => {
  const q = (s) => !!document.querySelector(s);
  return document.title + ' | play=' + q('[data-testid="control-button-playpause"]')
    + ' | login=' + (q('[data-testid="login-button"]') || q('a[href*="login"]'));
})()"#;

/// Llamada CDP generica a un WS (de tab o de browser): envia
/// {id, method, params} y regresa el response con ese id.
/// Timeouts anti-cuelgue en connect y en lectura.
async fn cdp_call(
    ws_url: &str,
    id: i64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let (ws, _) = tokio::time::timeout(
        Duration::from_secs(5),
        tokio_tungstenite::connect_async(ws_url),
    )
    .await
    .context("timeout conectando al WS")?
    .context("WS CDP fallo")?;

    let (mut write, mut read) = ws.split();
    let cmd = serde_json::json!({ "id": id, "method": method, "params": params }).to_string();
    write
        .send(tokio_tungstenite::tungstenite::Message::Text(cmd.into()))
        .await
        .context("envio CDP fallo")?;

    Ok(tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let tokio_tungstenite::tungstenite::Message::Text(t) = msg {
                let v: serde_json::Value = serde_json::from_str(&t)?;
                if v.get("id") == Some(&serde_json::json!(id)) {
                    if let Some(err) = v.get("error") {
                        anyhow::bail!("CDP error: {err}");
                    }
                    return Ok::<_, anyhow::Error>(v);
                }
            }
        }
        anyhow::bail!("WS cerrado sin respuesta")
    })
    .await
    .context("timeout esperando respuesta CDP")??)
}

/// Crea la pestana Spotify via Target.createTarget (browser WS de /json/version).
async fn create_spotify_target() -> Result<()> {
    let version = fetch_cdp_text("/json/version").await?;
    let v: serde_json::Value =
        serde_json::from_str(&version).context("version CDP invalida")?;
    let browser_ws = v
        .get("webSocketDebuggerUrl")
        .and_then(|x| x.as_str())
        .context("version CDP sin browser WS")?;
    cdp_call(
        browser_ws,
        2,
        "Target.createTarget",
        serde_json::json!({ "url": crate::launcher::SPOTIFY_URL }),
    )
    .await?;
    Ok(())
}

/// Asegura pestana Spotify: si falta, la crea UNA vez y espera a que
/// liste (10s). Sin duplicar tabs.
pub async fn ensure_spotify_tab() -> Result<()> {
    let mut created = false;
    for _ in 0..20 {
        match spotify_ws_url().await {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("no hay pestana") => {
                if !created {
                    create_spotify_target().await?;
                    created = true;
                }
            }
            Err(e) => return Err(e),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    anyhow::bail!("pestana Spotify no aparece")
}

/// Flujo completo del primer play: asegura pestana + click.
/// Esto es lo que [space] usa cuando no hay sesion GSMTC.
pub async fn play_from_scratch() -> Result<String> {
    ensure_spotify_tab().await?;
    play_spotify().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prueba viva del primer play (lo que hace [space] sin sesion):
    /// asegura pestana Spotify, click al Play y espera sesion GSMTC (20s).
    /// Si este test pasa, el flujo completo funciona.
    #[tokio::test]
    async fn space_inicia_musica() {
        // 0. Si ya suena, no hay nada que inyectar.
        if let Ok(s) = crate::gsmtc::get_brave_session().await {
            if let Ok(t) = crate::gsmtc::get_track(&s).await {
                println!("YA SONABA: {} — {} [{}]", t.title, t.artist, t.format_time());
                return;
            }
        }
        // 1. Sin puerto CDP -> lanzar Brave como la app (abre la tab).
        let launched = if fetch_json_list().await.is_err() {
            println!("sin CDP: lanzando Brave...");
            crate::launcher::launch_brave_spotify()
                .await
                .expect("lanzar Brave");
            true
        } else {
            false
        };
        // 2+3. Pestana + click (el flujo de [space]).
        let click = play_from_scratch().await.expect("primer play fallo");
        println!("CLICK: {click}");
        assert!(
            click.starts_with("clicked:"),
            "pagina sin boton play: {click}"
        );
        // 4. Sesion GSMTC con la rola sonando.
        let mut found = None;
        for _ in 0..20 {
            if let Ok(s) = crate::gsmtc::get_brave_session().await {
                if let Ok(t) = crate::gsmtc::get_track(&s).await {
                    found = Some((s, t));
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        let (s, t) = found.expect("click OK pero sin sesion GSMTC (¿autoplay bloqueado?)");
        println!("SUENA: {} — {} [{}]", t.title, t.artist, t.format_time());
        // 5. Limpieza como la app: pausar y cerrar lo nuestro.
        let _ = crate::gsmtc::pause(&s).await;
        crate::launcher::cleanup(!launched);
    }
}
