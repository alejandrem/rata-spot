//! Selectores del DOM de Spotify Web en UN SOLO LUGAR.
//!
//! El DOM cambia cada semanas; con los JS regados cada cambio era caceria.
//! Cadena de fallback (estable -> menos estable):
//! 1. `data-testid` (contrato interno, cambia poco)
//! 2. `role` + `href` con IDs (`/track/{id}` es contrato de URL)
//! 3. `aria-label` ES/EN (cambia con idioma/rediseno)
//! 4. clases CSS con hash: PROHIBIDAS.

mod dashboard_js;
mod health;
mod library_js;
mod names;
mod page;
mod player;
mod search_js;
#[cfg(test)]
mod tests;
mod tracklist;

pub use health::health_summary;
#[allow(unused_imports)]
pub use names::*;
#[allow(unused_imports)]
pub(crate) use dashboard_js::DASHBOARD_JS;
#[allow(unused_imports)]
pub(crate) use health::{summarize_health, HEALTH_JS};
#[allow(unused_imports)]
pub(crate) use library_js::{LIBRARY_DIAG_JS, LIBRARY_JS};
#[allow(unused_imports)]
pub(crate) use page::PAGE_STATE_JS;
#[allow(unused_imports)]
pub(crate) use player::{track_row_click_js, PLAYER_ARIA_JS, PLAY_CLICK_JS};
#[allow(unused_imports)]
pub(crate) use search_js::SEARCH_JS;
#[allow(unused_imports)]
pub(crate) use tracklist::{
    TRACKLIST_EXISTS_JS, TRACKLIST_SCROLL_JS, TRACKLIST_SCROLL_TOP_JS, TRACKS_JS,
};
