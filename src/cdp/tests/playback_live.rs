//! Playback_live: primer play ([space]) + tocar rola de playlist.
//! Vivos y RUIDOSOS: ignorados por default (`cargo test -- --ignored`).

use std::time::Duration;

/// Primer play (lo que hace [space] sin sesion): click + sesion GSMTC (20s).
#[ignore]
#[tokio::test]
async fn space_inicia_musica() {
    use crate::cdp::{play_from_scratch, transport::fetch_json_list};
    if let Ok(s) = crate::gsmtc::get_brave_session().await {
        if let Ok(t) = crate::gsmtc::get_track(&s).await {
            println!("YA SONABA: {} — {} [{}]", t.title, t.artist, t.format_time());
            return;
        }
    }
    let launched = if fetch_json_list().await.is_err() {
        println!("sin CDP: lanzando Brave...");
        crate::launcher::launch_brave_spotify()
            .await
            .expect("lanzar Brave");
        true
    } else {
        false
    };
    let click = play_from_scratch().await.expect("primer play fallo");
    println!("CLICK: {click}");
    assert!(click.starts_with("clicked:"), "pagina sin boton play: {click}");
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
    let _ = crate::gsmtc::pause(&s).await;
    crate::launcher::cleanup(!launched);
}

/// Tocar rola: primera cancion de la primera playlist + sesion GSMTC.
#[ignore]
#[tokio::test]
async fn track_se_reproduce() {
    use crate::cdp::{ensure_spotify_tab, library_items, open_playlist, play_track, playlist_tracks};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
    if let Ok(u) = crate::cdp::spotify_tab_url().await {
        println!("TAB: {u}");
    }
    if let Ok(s) = crate::gsmtc::get_brave_session().await {
        if crate::gsmtc::get_track(&s).await.is_ok() {
            println!("YA SONABA: salto tocar rola");
            return;
        }
    }
    let items = library_items().await.expect("leer biblioteca del DOM");
    let pl = items.iter().find(|i| i.kind == "playlist").expect("sin playlists");
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
