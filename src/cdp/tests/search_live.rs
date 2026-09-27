//! Search_live: buscador + regresion de no-corte de audio.
//! Vivos y RUIDOSOS: ignorados por default.

/// Buscador: navega a /search y lee resultados (no reproduce al inicio).
#[ignore]
#[tokio::test]
async fn busqueda_funciona() {
    use crate::cdp::{ensure_spotify_tab, play_uri, search};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
    let items = search("fuerza regida").await.expect("buscar");
    assert!(!items.is_empty(), "sin resultados");
    let n_tr = items.iter().filter(|i| i.kind == "track").count();
    println!("resultados={} tracks={}", items.len(), n_tr);
    for i in items.iter().take(10) {
        println!("{}: {} | {} | {}", i.kind, i.name, i.detail, i.uri);
    }
    assert!(n_tr > 0, "sin tracks en resultados");
    let first = items.iter().find(|i| i.kind == "track").expect("sin track");
    println!("TOCANDO: {} - {} [{}]", first.name, first.detail, first.uri);
    let out = play_uri(&first.uri, &first.name).await.expect("tocar track");
    println!("CLICK: {out}");
    let mut found = None;
    for _ in 0..20 {
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

/// Regresion: buscar NO debe cortar la musica (Page.navigate mataba audio).
#[ignore]
#[tokio::test]
async fn busqueda_no_corta_musica() {
    use std::time::Duration as StdDur;
    use crate::cdp::{ensure_spotify_tab, play_uri, search};
    if let Err(e) = ensure_spotify_tab().await {
        println!("sin pestana ({e}); salto");
        return;
    }
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
    play_uri("spotify:track:5LHPcY9yd0hWVFIW4yfOCJ", "Pika Pika")
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
    let items = search("blackpink").await.expect("buscar");
    assert!(!items.is_empty(), "busqueda vacia");
    tokio::time::sleep(StdDur::from_secs(2)).await;
    let despues = sonando_pika().await.expect("se corto la musica al buscar!");
    println!("SUENA: {despues}");
    let a = antes.to_lowercase();
    let d = despues.to_lowercase();
    assert!(a.contains(&d) || d.contains(&a), "cambio la rola: '{antes}' -> '{despues}'");
}
