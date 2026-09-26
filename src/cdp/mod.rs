//! Control de la pagina Spotify via Chrome DevTools Protocol (sin API Spotify).
//!
//! Problema que resuelve: GSMTC solo controla lo que YA esta sonando.
//! En una pagina fresca (recien abierta, sin cola) `TryPlayAsync` no hace
//! nada y [space] parece muerto. Con CDP la TUI hace click al Play de la
//! pagina y la musica arranca sin tocar el mouse.
//!
//! Requiere Brave lanzado con `--remote-debugging-port` (ver launcher).
//! Si el puerto no existe (Brave abierto a mano sin el flag), regresa error
//! y la TUI muestra "dale play una vez en Brave".
//!
//! Tareas (1 archivo = 1 tarea):
//! - transport: HTTP crudo a /json/* (sin reqwest para no engordar deps)
//! - client: llamada WS generica {id, method, params}
//! - tabs: localizar/crear la pestana Spotify
//! - selectors: TODOS los selectores del DOM + health-check (un solo lugar)
//! - api: capa JSON/red (token web + search estable, fallback al DOM)
//! - playback: click al Play + flujo del primer play
//! - library: leer "Tu biblioteca"
//! - search: buscador (API primero, DOM como fallback)
//! - track_page: dashboard individual (header + letra)
//! - tests: pruebas vivas (primer play + listado)

mod api;
mod client;
mod library;
mod playback;
mod search;
mod selectors;
mod tabs;
mod track_page;
mod tracks;
mod transport;

#[cfg(test)]
mod tests;

/// Solo tests/diagnostico (en binario no-test quedan sin uso).
#[allow(unused_imports)]
pub use api::{api_diag, search_via_api};
pub use library::{LibraryItem, library_items};
/// Solo tests/diagnostico (en binario no-test quedan sin uso).
#[allow(unused_imports)]
pub use library::library_diag;
pub use playback::play_from_scratch;
pub use search::search;
pub use selectors::health_summary;
#[allow(unused_imports)]
pub use playback::play_spotify;
pub use tabs::ensure_spotify_tab;
/// Solo tests/diagnostico.
#[allow(unused_imports)]
pub use tabs::spotify_tab_url;
pub use tracks::{open_playlist, play_track, play_uri, playlist_tracks, TrackItem};
pub use track_page::{track_detail, TrackDetail};
