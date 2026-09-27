//! Diag_media: audio/video real + espacio trusted + botones + GSMTC.
//! Solo imprimen, siempre pasan (sin pestana => lo dicen).

use std::time::Duration;

/// Elementos audio/video (paused, muted, currentTime x2 con 2s).
#[tokio::test]
async fn diag_media() {
    use crate::cdp::client::cdp_call;
    use crate::cdp::tabs::spotify_ws_url;
    let ws = match spotify_ws_url().await {
        Ok(ws) => ws,
        Err(e) => {
            println!("DIAG-MEDIA: sin Brave/CDP ({e}) — abre rata-spot y reintenta");
            return;
        }
    };
    let js = r#"(() => {
      const els = [...document.querySelectorAll('audio,video')].map(m => ({
        tag: m.tagName, paused: m.paused, muted: m.muted, vol: m.volume,
        t: m.currentTime, dur: m.duration, err: m.error ? m.error.code : 0,
        net: m.networkState, ready: m.readyState,
        src: (m.currentSrc || '').slice(-50)
      }));
      return JSON.stringify({url: location.href, vis: document.visibilityState,
        audible: document.querySelectorAll('audio,video').length, media: els});
    })()"#;
    for i in 0..2 {
        let v = match cdp_call(
            &ws,
            51,
            "Runtime.evaluate",
            serde_json::json!({ "expression": js, "returnByValue": true }),
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                println!("DIAG-MEDIA[{i}]: evaluate fallo ({e})");
                return;
            }
        };
        println!(
            "DIAG-MEDIA[{i}]: {}",
            v.pointer("/result/result/value").and_then(|x| x.as_str()).unwrap_or("?")
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Sonda: manda Espacio real (trusted) a la pagina.
#[tokio::test]
async fn diag_space() {
    use crate::cdp::client::cdp_call;
    use crate::cdp::tabs::spotify_ws_url;
    let ws = match spotify_ws_url().await {
        Ok(ws) => ws,
        Err(e) => {
            println!("DIAG-SPACE: sin Brave/CDP ({e}) — abre rata-spot y reintenta");
            return;
        }
    };
    for typ in ["keyDown", "keyUp"] {
        if let Err(e) = cdp_call(
            &ws,
            60,
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": typ,
                "key": " ",
                "code": "Space",
                "windowsVirtualKeyCode": 32,
                "nativeVirtualKeyCode": 32,
            }),
        )
        .await
        {
            println!("DIAG-SPACE: tecla {typ} fallo ({e})");
            return;
        }
    }
    println!("DIAG-SPACE: enviado");
}

/// Inventario de botones Play/Reproducir visibles.
#[tokio::test]
async fn diag_play_buttons() {
    use crate::cdp::client::cdp_call;
    use crate::cdp::tabs::spotify_ws_url;
    let ws = match spotify_ws_url().await {
        Ok(ws) => ws,
        Err(e) => {
            println!("DIAG-BTN: sin Brave/CDP ({e}) — abre rata-spot y reintenta");
            return;
        }
    };
    let js = r#"(() => {
      const vis = (b) => b && !b.disabled && b.getAttribute('aria-disabled') !== 'true' && !!b.offsetParent;
      const out = [];
      document.querySelectorAll('button').forEach((b, i) => {
        const al = b.getAttribute('aria-label') || '';
        const td = b.getAttribute('data-testid') || '';
        if (/reproducir|play/i.test(al) || td.includes('play')) {
          out.push({i, al: al.slice(0, 70), td, vis: vis(b),
            html: b.outerHTML.slice(0, 200)});
        }
      });
      return JSON.stringify({url: location.href, title: document.title.slice(0,50), buttons: out});
    })()"#;
    let v = match cdp_call(
        &ws,
        50,
        "Runtime.evaluate",
        serde_json::json!({ "expression": js, "returnByValue": true }),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            println!("DIAG-BTN: evaluate fallo ({e})");
            return;
        }
    };
    println!(
        "DIAG-BTN: {}",
        v.pointer("/result/result/value").and_then(|x| x.as_str()).unwrap_or("?")
    );
}

/// GSMTC en vivo: sesion + posicion x3 con 2s entre muestras.
#[tokio::test]
async fn diag_gsmtc() {
    use crate::gsmtc::{get_brave_session, get_track};
    let s = match get_brave_session().await {
        Ok(s) => s,
        Err(e) => {
            println!("DIAG-GSMTC: sin sesion ({e}) — pon musica y reintenta");
            return;
        }
    };
    for i in 0..3 {
        match get_track(&s).await {
            Ok(t) => println!(
                "DIAG-GSMTC[{i}]: playing={} pos={:?} dur={:?} prog={:.3} | {} — {}",
                t.playing, t.position, t.duration, t.progress, t.title, t.artist
            ),
            Err(e) => println!("DIAG-GSMTC[{i}]: get_track fallo ({e})"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
