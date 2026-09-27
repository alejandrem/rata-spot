//! Names: nombres canonicos (si Spotify cambia uno, se cambia AQUI).

/// Barra del player: el boton play/pause real.
#[allow(dead_code)]
pub const PLAY_TESTIDS: &[&str] = &["control-button-playpause", "play-button"];

/// Substrings de aria-label para el fallback ES/EN.
#[allow(dead_code)]
pub const PLAY_ARIA_NEEDLES: &[&str] = &["reproducir", "play"];

/// Sidebar izquierda de Brave/Spotify.
#[allow(dead_code)]
pub const SIDEBAR_ID: &str = "#Desktop_LeftSidebar_Id";

/// Prefijo de los ids de titulos en la biblioteca virtualizada.
#[allow(dead_code)]
pub const LIBRARY_ROW_PREFIX: &str = "listrow-title-spotify:";

/// Contenedor del tracklist: playlists `playlist-tracklist`, albumes `track-list`.
#[allow(dead_code)]
pub const TRACKLIST_TESTIDS: &[&str] = &["playlist-tracklist", "track-list"];

/// Anchor del titulo de cada rola (estable, sin clases hash).
#[allow(dead_code)]
pub const TRACK_LINK_TESTID: &str = "internal-track-link";

/// Seccion del dashboard individual de rola.
#[allow(dead_code)]
pub const TRACK_PAGE_TESTID: &str = "track-page";
