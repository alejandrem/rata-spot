//! Detail_nav: scroll de la letra del dashboard.

use super::view::AppState;

pub fn lyr_move(s: &mut AppState, delta: i32) {
    let max = s
        .detail
        .as_ref()
        .map(|d| d.lyrics.len().saturating_sub(1))
        .unwrap_or(0) as i32;
    s.lyr_scroll = (s.lyr_scroll as i32 + delta).clamp(0, max.max(0)) as u16;
}
