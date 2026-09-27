//! Library_live: biblioteca del DOM llega a la TUI + tracklist se lee.

/// Lectura de biblioteca (UTF-8/emojis intactos). Autosuficiente.
#[tokio::test]
async fn playlists_se_listan() {
    use crate::cdp::{ensure_spotify_tab, library_items};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
    if let Ok(u) = crate::cdp::spotify_tab_url().await {
        println!("TAB: {u}");
    }
    let items = library_items().await.expect("leer biblioteca del DOM");
    assert!(!items.is_empty(), "biblioteca vacia");
    let n_pl = items.iter().filter(|i| i.kind == "playlist").count();
    println!("items={} playlists={}", items.len(), n_pl);
    for i in items.iter().filter(|x| x.kind == "playlist").take(8) {
        println!("PL: {} | {} | {}", i.name, i.detail, i.uri);
    }
    assert!(n_pl > 0, "sin playlists en la biblioteca");
}

/// Tracklist: abre la primera playlist y lee canciones (sin reproducir).
#[ignore]
#[tokio::test]
async fn playlist_tracks_se_leen() {
    use crate::cdp::{ensure_spotify_tab, library_items, open_playlist, playlist_tracks};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
    if let Ok(u) = crate::cdp::spotify_tab_url().await {
        println!("TAB: {u}");
    }
    let items = library_items().await.expect("leer biblioteca del DOM");
    let pl = items.iter().find(|i| i.kind == "playlist").expect("sin playlists");
    println!("PLAYLIST: {} | {}", pl.name, pl.uri);
    open_playlist(&pl.uri).await.expect("abrir playlist");
    let tracks = playlist_tracks().await.expect("leer canciones");
    assert!(!tracks.is_empty(), "tracklist vacio");
    println!("canciones={}", tracks.len());
    for t in tracks.iter().take(5) {
        println!("TR: {} | {} - {} [{}]", t.n, t.title, t.artist, t.duration);
    }
}
