//! Pruebas vivas contra el Brave real (requieren CDP en :9222).
//! Si pasan, el flujo completo TUI -> Brave -> musica funciona.

use super::*;
use std::time::Duration;

/// Prueba viva del primer play (lo que hace [space] sin sesion):
/// asegura pestana Spotify, click al Play y espera sesion GSMTC (20s).
/// Si este test pasa, el flujo completo funciona.
    /// Vivo y RUIDOSO (suena + navega): ignorado por default.
    /// Correr a mano con manos fuera: cargo test -- --ignored
    #[ignore]
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
    let launched = if super::transport::fetch_json_list().await.is_err() {
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

    /// Prueba viva del buscador: navega a /search y lee resultados
    /// (tracks + artistas + playlists). Solo lee, no reproduce.
    /// Vivo y RUIDOSO: ignorado por default (ver space_inicia_musica).
    #[ignore]
    #[tokio::test]
    async fn busqueda_funciona() {
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        let items = super::search("fuerza regida").await.expect("buscar");
        assert!(!items.is_empty(), "sin resultados");
        let n_tr = items.iter().filter(|i| i.kind == "track").count();
        println!("resultados={} tracks={}", items.len(), n_tr);
        for i in items.iter().take(10) {
            println!("{}: {} | {} | {}", i.kind, i.name, i.detail, i.uri);
        }
        assert!(n_tr > 0, "sin tracks en resultados");
        // Tocar el primer track por URI (lo que hace Enter en resultados).
        let first = items.iter().find(|i| i.kind == "track").expect("sin track");
        println!("TOCANDO: {} - {} [{}]", first.name, first.detail, first.uri);
        let out = super::play_uri(&first.uri, &first.name).await.expect("tocar track");
        println!("CLICK: {out}");
        let mut found = None;
        for _ in 0..20 {
            // Estricto: buscar en TODAS las sesiones la que toca Pika Pika.
            // (Un video de Facebook podria ser la primera sesion sonando.)
            if let Ok(all) = crate::gsmtc::brave_sessions().await {
                for s in &all {
                    if let Ok(t) = crate::gsmtc::get_track(s).await {
                        if t.playing && t.title.to_lowercase().contains("pika") {
                            found = Some((s.clone(), t));
                            break;
                        }
                    }
                }
            }
            if found.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let (s, t) = found.expect("click OK pero Pika Pika no suena (¿otra sesion?)");
        println!("SUENA: {} — {} [{}]", t.title, t.artist, t.format_time());
        let _ = crate::gsmtc::pause(&s).await;
    }

    /// Prueba viva del dashboard: abre la pagina del track y lee
    /// header + letra (solo lee, no reproduce).
    /// Vivo (navega): ignorado por default.
    #[ignore]
    #[tokio::test]
    async fn track_detail_se_lee() {
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        super::tracks::open_page("spotify:track:5LHPcY9yd0hWVFIW4yfOCJ")
            .await
            .expect("abrir track");
        let d = super::track_detail().await.expect("leer dashboard");
        assert!(!d.title.is_empty(), "sin titulo en dashboard");
        println!(
            "DASH: {} - {} [{}] album={} plays={} letra={}",
            d.title,
            d.artist,
            d.duration,
            d.album,
            d.playcount,
            d.lyrics.len()
        );
        for l in d.lyrics.iter().take(3) {
            println!("  LY: {l}");
        }
    }

    /// Regresion: buscar NO debe cortar la musica que suena.
    /// Flujo del bug: / + texto + Enter => Page.navigate mataba el audio.
    /// Con navegacion SPA la misma rola sigue sonando tras buscar.
    /// Vivo y RUIDOSO: ignorado por default.
    #[ignore]
    #[tokio::test]
    async fn busqueda_no_corta_musica() {
        use std::time::Duration as StdDur;
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        // 0. Entorno limpio: pausar sesiones ajenas (stories/videos) para
        // medir SOLO Spotify. Sin esto el test mide lo que miras tu.
        if let Ok(all) = crate::gsmtc::brave_sessions().await {
            for s in &all {
                if let Ok(t) = crate::gsmtc::get_track(s).await {
                    if t.playing && !t.title.to_lowercase().contains("pika") {
                        println!("pausando ajeno: {} — {}", t.title, t.artist);
                        let _ = crate::gsmtc::pause(s).await;
                    }
                }
            }
        }
        // 1. Prender Pika SIEMPRE: el test controla su musica (lo ajeno
        // como stories termina solo y falsea todo).
        async fn sonando_pika() -> Option<String> {
            let all = crate::gsmtc::brave_sessions().await.ok()?;
            for s in &all {
                if let Ok(t) = crate::gsmtc::get_track(s).await {
                    if t.playing && t.title.to_lowercase().contains("pika") {
                        return Some(t.title);
                    }
                }
            }
            None
        }
        println!("prendiendo Pika...");
        super::play_uri(
            "spotify:track:5LHPcY9yd0hWVFIW4yfOCJ",
            "Pika Pika",
        )
        .await
        .expect("prender");
        let mut antes = None;
        for _ in 0..20 {
            if let Some(t) = sonando_pika().await {
                antes = Some(t);
                break;
            }
            tokio::time::sleep(StdDur::from_secs(1)).await;
        }
        let antes = antes.expect("Pika no sono (¿audio bloqueado?)");
        println!("SONABA: {antes}");
        // 1. Buscar (esto mataba el audio con Page.navigate).
        let items = super::search("blackpink").await.expect("buscar");
        assert!(!items.is_empty(), "busqueda vacia");
        tokio::time::sleep(StdDur::from_secs(2)).await;
        // 3. La MISMA rola sigue sonando (Pika, no lo que sea).
        let despues = sonando_pika().await.expect("se corto la musica al buscar!");
        println!("SUENA: {despues}");
        let a = antes.to_lowercase();
        let d = despues.to_lowercase();
        assert!(
            a.contains(&d) || d.contains(&a),
            "cambio la rola: '{antes}' -> '{despues}'"
        );
    }

    /// Diagnostico del DOM (solo imprime, siempre pasa).
    #[tokio::test]
    async fn diag_estado() {
        match library_diag().await {
            Ok(s) => println!("DIAG: {s}"),
            Err(e) => println!("DIAG-ERR: {e}"),
        }
    }

    /// Health-check de selectores (punto 1): dice que vive y que murio.
    /// Solo imprime, siempre pasa (sin pestana => "pendiente").
    #[tokio::test]
    async fn diag_selectores() {
        println!("DIAG-SEL: {}", super::health_summary().await);
    }

    /// Diagnostico de la capa API/red (punto 2): token web + search JSON.
    /// Solo imprime, siempre pasa (sin pestana/login => lo dice).
    #[tokio::test]
    async fn diag_api() {
        println!("DIAG-API: {}", super::api_diag().await);
    }

    /// Diagnostico de media real: elementos audio/video (paused, muted,
    /// currentTime, errores) + visibilidad. Dice si el audio fluye o no.
    #[tokio::test]
    async fn diag_media() {
        use super::client::cdp_call;
        use super::tabs::spotify_ws_url;
        let ws = spotify_ws_url().await.expect("sin tab");
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
        // dos muestras con 2s para ver si currentTime avanza
        for i in 0..2 {
            let v = cdp_call(
                &ws,
                51,
                "Runtime.evaluate",
                serde_json::json!({ "expression": js, "returnByValue": true }),
            )
            .await
            .expect("evaluate");
            println!(
                "DIAG-MEDIA[{i}]: {}",
                v.pointer("/result/result/value")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
            );
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    /// Sonda: manda Espacio real (trusted) a la pagina. Lo que haria tu dedo.
    #[tokio::test]
    async fn diag_space() {
        use super::client::cdp_call;
        use super::tabs::spotify_ws_url;
        let ws = spotify_ws_url().await.expect("sin tab");
        for typ in ["keyDown", "keyUp"] {
            cdp_call(
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
            .expect("tecla");
        }
        println!("DIAG-SPACE: enviado");
    }

    /// Inventario de botones Play/Reproducir visibles en la pestana actual.
    #[tokio::test]
    async fn diag_play_buttons() {
        use super::client::cdp_call;
        use super::tabs::spotify_ws_url;
        let ws = spotify_ws_url().await.expect("sin tab");
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
        let v = cdp_call(
            &ws,
            50,
            "Runtime.evaluate",
            serde_json::json!({ "expression": js, "returnByValue": true }),
        )
        .await
        .expect("evaluate");
        println!(
            "DIAG-BTN: {}",
            v.pointer("/result/result/value")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
        );
    }

    /// Diagnostico GSMTC en vivo: sesion, playing, posicion x3 con 2s
    /// entre muestras (solo imprime, siempre pasa).
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

    /// Prueba viva de lectura: la biblioteca del DOM llega a la TUI
    /// (nombres UTF-8/emojis intactos). Solo lee, no reproduce.
    /// Autosuficiente: asegura la pestana antes de leer.
    #[tokio::test]
    async fn playlists_se_listan() {
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        if let Ok(u) = super::spotify_tab_url().await {
            println!("TAB: {u}");
        }
        let items = library_items()
            .await
            .expect("leer biblioteca del DOM");
        assert!(!items.is_empty(), "biblioteca vacia");
        let n_pl = items.iter().filter(|i| i.kind == "playlist").count();
        println!("items={} playlists={}", items.len(), n_pl);
        for i in items.iter().filter(|x| x.kind == "playlist").take(8) {
            println!("PL: {} | {} | {}", i.name, i.detail, i.uri);
        }
        assert!(n_pl > 0, "sin playlists en la biblioteca");
    }

    /// Prueba viva del tracklist: abre la primera playlist y lee sus
    /// canciones (solo dibujar, sin reproducir). Autosuficiente.
    /// Vivo (navega): ignorado por default.
    #[ignore]
    #[tokio::test]
    async fn playlist_tracks_se_leen() {
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        if let Ok(u) = super::spotify_tab_url().await {
            println!("TAB: {u}");
        }
        let items = library_items()
            .await
            .expect("leer biblioteca del DOM");
        let pl = items
            .iter()
            .find(|i| i.kind == "playlist")
            .expect("sin playlists");
        println!("PLAYLIST: {} | {}", pl.name, pl.uri);
        open_playlist(&pl.uri).await.expect("abrir playlist");
        let tracks = playlist_tracks().await.expect("leer canciones");
        assert!(!tracks.is_empty(), "tracklist vacio");
        println!("canciones={}", tracks.len());
        for t in tracks.iter().take(5) {
            println!("TR: {} | {} - {} [{}]", t.n, t.title, t.artist, t.duration);
        }
    }

    /// Prueba viva de tocar rola: reproduce la primera cancion de la
    /// primera playlist y espera sesion GSMTC. Limpia pausando al final.
    /// Vivo y RUIDOSO: ignorado por default.
    #[ignore]
    #[tokio::test]
    async fn track_se_reproduce() {
        if let Err(e) = super::ensure_spotify_tab().await {
            println!("sin pestana ({e}); salto");
            return;
        }
        if let Ok(u) = super::spotify_tab_url().await {
            println!("TAB: {u}");
        }
        if let Ok(s) = crate::gsmtc::get_brave_session().await {
            if crate::gsmtc::get_track(&s).await.is_ok() {
                println!("YA SONABA: salto tocar rola");
                return;
            }
        }
        let items = library_items()
            .await
            .expect("leer biblioteca del DOM");
        let pl = items
            .iter()
            .find(|i| i.kind == "playlist")
            .expect("sin playlists");
        open_playlist(&pl.uri).await.expect("abrir playlist");
        let tracks = playlist_tracks().await.expect("leer canciones");
        let first = tracks.first().expect("tracklist vacio");
        println!("TOCANDO: {} - {} [{}]", first.title, first.artist, first.id);
        let out = play_track(&first.id).await.expect("tocar rola");
        println!("CLICK: {out}");
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
        let (s, t) = found.expect("click OK pero sin sesion GSMTC");
        println!("SUENA: {} — {} [{}]", t.title, t.artist, t.format_time());
        let _ = crate::gsmtc::pause(&s).await;
    }
