//! Switch: cambia a otra sesion Brave solo si flippeo a sonando.

use crate::{gsmtc, ui::AppState};

use super::state::SyncState;

/// Anti-oscilacion: con la actual sonando jamas se cambia; el mapa `seen`
/// recuerda playing por titulo entre escaneos (~1s).
pub(crate) async fn switch_if_changed(
    state: &mut AppState,
    session: &mut Option<gsmtc::Session>,
    sync: &mut SyncState,
    cur_title: &str,
) {
    let all = match gsmtc::brave_sessions().await {
        Ok(a) if a.len() > 1 => a,
        _ => return, // 0-1 sesiones: nada que elegir
    };
    let cur_key = cur_title.to_lowercase();
    let mut candidate: Option<gsmtc::Session> = None;
    for s in &all {
        let (playing, title) = match gsmtc::get_track(s).await {
            Ok(t) => (t.playing, t.title.to_lowercase()),
            Err(_) => continue,
        };
        let was = sync.seen.get(&title).copied();
        sync.seen.insert(title.clone(), playing);
        if playing && was != Some(true) && title != cur_key {
            candidate = Some(s.clone());
        }
    }
    if sync.seen.len() > 40 {
        sync.seen.clear();
    }
    if let Some(s) = candidate {
        *session = Some(s);
        state.connected = true;
    }
}
