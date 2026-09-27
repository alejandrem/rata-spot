//! Transporte HTTP crudo hacia el servidor CDP de Brave.
//!
//! Sin reqwest a proposito: solo necesitamos GET a /json/* en localhost.
//! Lecciones aprendidas (ver bugs-resueltos-uwu.md):
//! - El servidor exige HTTP/1.1 (con 1.0 cierra sin responder, 0 bytes).
//! - NO usar read_to_end: a veces no cierra y cuelga eterno.
//!   Se lee por Content-Length con fallback a chunked.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const CDP_HOST: &str = "127.0.0.1";

/// GET a un endpoint CDP (/json/list, /json/version) con timeout global.
/// Prueba los puertos candidatos en orden: el de este arranque y luego el
/// 9222 legacy (Brave ya abierto con flags viejos o a mano). localhost
/// responde en ms; si algo cuelga, fallar rapido.
pub(crate) async fn fetch_cdp_text(path: &str) -> Result<String> {
    tokio::time::timeout(Duration::from_secs(8), async {
        let mut last_err = anyhow::anyhow!("sin puerto CDP (Brave sin --remote-debugging-port)");
        for port in crate::launcher::ports::live_candidates() {
            match fetch_from_port(path, port).await {
                Ok(body) => return Ok(body),
                Err(e) => last_err = e,
            }
        }
        Err::<String, _>(last_err)
    })
    .await
    .map_err(|_| anyhow::anyhow!("timeout hablando con CDP"))?
}

/// Un intento contra un solo puerto (sin timeout propio: lo pone el padre).
async fn fetch_from_port(path: &str, port: u16) -> Result<String> {
    let mut stream = TcpStream::connect((CDP_HOST, port))
        .await
        .with_context(|| {
            format!("sin puerto CDP en {CDP_HOST}:{port} (Brave sin --remote-debugging-port)")
        })?;

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {CDP_HOST}:{port}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
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
pub(crate) async fn fetch_json_list() -> Result<String> {
    fetch_cdp_text("/json/list").await
}

/// ¿Hay puerto CDP vivo? (diagnóstico/tests; el boot con perfil dedicado
/// reengancha por puerto guardado). Barato: un GET a /json/list.
#[allow(dead_code)]
pub async fn debug_alive() -> bool {
    fetch_json_list().await.is_ok()
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
