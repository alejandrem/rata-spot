//! Pruebas vivas contra el Brave real (requieren CDP en :9222).
//! Si pasan, el flujo completo TUI -> Brave -> musica funciona.

use super::*;
use std::time::Duration;

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

    /// Diagnostico del DOM (solo imprime, siempre pasa).
    #[tokio::test]
    async fn diag_estado() {
        match library_diag().await {
            Ok(s) => println!("DIAG: {s}"),
            Err(e) => println!("DIAG-ERR: {e}"),
        }
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
