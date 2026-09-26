//! Historial: ultima cancion persistida en disco (UTF-8, emojis intactos).
//! La TUI la muestra al arrancar aunque aun diga "Conectando...".

use std::time::Duration;

use super::track::TrackInfo;

/// Ruta del historial (UTF-8, emojis incluidos sin problema).
fn last_track_path() -> std::path::PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(base).join("rata-spot-last-track.txt")
}

/// Guarda la ultima cancion en disco. Formato simple de 6 lineas.
/// Se llama solo cuando cambia de cancion (no cada tick).
pub fn save_last_track(track: &TrackInfo) {
    // Los titulos con salto de linea romperian el formato -> aplanar.
    let flat = |s: &str| s.replace('\n', " ").replace('\r', " ");
    let content = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n",
        flat(&track.title),
        flat(&track.artist),
        flat(&track.album),
        track.position.as_secs(),
        track.duration.as_secs(),
        if track.playing { "1" } else { "0" },
    );
    let _ = std::fs::write(last_track_path(), content);
}

/// Carga la ultima cancion guardada, si existe.
pub fn load_last_track() -> Option<TrackInfo> {
    let content = std::fs::read_to_string(last_track_path()).ok()?;
    let mut lines = content.lines();
    let title = lines.next()?.to_string();
    let artist = lines.next().unwrap_or("—").to_string();
    let album = lines.next().unwrap_or("").to_string();
    let position = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);
    let duration = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);
    if title.is_empty() || title == "—" {
        return None;
    }
    let progress = if duration.is_zero() {
        0.0
    } else {
        (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
    };
    Some(TrackInfo {
        title,
        artist,
        album,
        playing: false, // al arrancar siempre pausado (historial)
        position,
        duration,
        progress,
    })
}
