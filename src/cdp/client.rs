//! Cliente WebSocket generico para CDP: envia {id, method, params}
//! y regresa el response con ese id. Timeouts anti-cuelgue en
//! connect y en lectura (ver bug #11).

use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};

pub(crate) async fn cdp_call(
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
