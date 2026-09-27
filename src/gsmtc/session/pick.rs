//! Pick: elige la sesion correcta cuando hay varias (video + Spotify).

use anyhow::Result;

use super::find::{brave_sessions, is_playing};
use super::read::get_track;
use super::super::track::Session;

/// Elige sesion: hint del historial > primera SONANDO > primera.
/// Evita engancharse a un video de Facebook cuando Spotify tambien suena.
pub async fn pick_session(hint_title: Option<&str>) -> Result<Session> {
    let all = brave_sessions().await?;
    if all.len() == 1 {
        return Ok(all.into_iter().next().expect("uno"));
    }
    // 1) Hint del historial rata-spot: la de Spotify casi seguro.
    if let Some(hint) = hint_title.filter(|h| !h.is_empty() && *h != "—") {
        let want = hint.to_lowercase();
        for s in &all {
            let title = get_track(s)
                .await
                .map(|t| t.title.to_lowercase())
                .unwrap_or_default();
            if !title.is_empty() && (title.contains(&want) || want.contains(&title)) {
                return Ok(s.clone());
            }
        }
    }
    // 2) Primera sonando. 3) Primera a secas.
    for s in &all {
        if is_playing(s) {
            return Ok(s.clone());
        }
    }
    Ok(all.into_iter().next().expect("no vacio"))
}
