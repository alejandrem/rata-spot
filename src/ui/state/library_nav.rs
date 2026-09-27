//! Library_nav: mover/seleccionar en la biblioteca (sidebar).

use super::view::AppState;
use crate::cdp::LibraryItem;

pub fn pl_move(s: &mut AppState, delta: i32) {
    if s.library.is_empty() {
        return;
    }
    let n = s.library.len() as i32;
    s.pl_index = (s.pl_index as i32 + delta).rem_euclid(n) as usize;
    s.pl_state.select(Some(s.pl_index));
}

pub fn pl_selected(s: &AppState) -> Option<&LibraryItem> {
    s.library.get(s.pl_index)
}
