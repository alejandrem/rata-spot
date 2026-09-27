//! Search_nav: mover/seleccionar resultados del buscador.

use super::view::AppState;
use crate::cdp::LibraryItem;

pub fn sr_move(s: &mut AppState, delta: i32) {
    if s.results.is_empty() {
        return;
    }
    let n = s.results.len() as i32;
    s.sr_index = (s.sr_index as i32 + delta).rem_euclid(n) as usize;
    s.sr_state.select(Some(s.sr_index));
}

pub fn sr_selected(s: &AppState) -> Option<&LibraryItem> {
    s.results.get(s.sr_index)
}
