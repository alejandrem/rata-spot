//! Dashboard_live: abre pagina del track y lee header + letra (solo lee).

/// Vivo (navega): ignorado por default.
#[ignore]
#[tokio::test]
async fn track_detail_se_lee() {
    use crate::cdp::{ensure_spotify_tab, track_detail, tracks::open_page};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
    open_page("spotify:track:5LHPcY9yd0hWVFIW4yfOCJ")
        .await
        .expect("abrir track");
    let d = track_detail().await.expect("leer dashboard");
    assert!(!d.title.is_empty(), "sin titulo en dashboard");
    println!(
        "DASH: {} - {} [{}] album={} plays={} letra={}",
        d.title, d.artist, d.duration, d.album, d.playcount, d.lyrics.len()
    );
    for l in d.lyrics.iter().take(3) {
        println!("  LY: {l}");
    }
}
