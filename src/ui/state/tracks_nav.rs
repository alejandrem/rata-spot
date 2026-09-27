//! Tracks_nav: mover/seleccionar canciones de la playlist abierta.

use super::view::AppState;
use crate::cdp::TrackItem;

pub fn tr_move(s: &mut AppState, delta: i32) {
    if s.tracks.is_empty() {
        return;
    }
    let n = s.tracks.len() as i32;
    s.tr_index = (s.tr_index as i32 + delta).rem_euclid(n) as usize;
    s.tr_state.select(Some(s.tr_index));
}

pub fn tr_selected(s: &AppState) -> Option<&TrackItem> {
    s.tracks.get(s.tr_index)
}
