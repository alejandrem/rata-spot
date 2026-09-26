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

/// Prueba viva de lectura: la biblioteca del DOM llega a la TUI
/// (nombres UTF-8/emojis intactos). Solo lee, no reproduce.
#[tokio::test]
async fn playlists_se_listan() {
    if super::transport::fetch_json_list().await.is_err() {
        println!("sin CDP (haz cargo run primero); salto");
        return;
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
