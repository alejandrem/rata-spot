//! Canciones de una colección (playlist o álbum): leer el tracklist del
//! DOM y tocar por track.
//!
//! Filas `[role="row"]`: numero + `a[internal-track-link](/track/{id})`
//! + artistas + duracion `m:ss` + boton `Reproducir` para tocar en contexto.

mod open;
mod play_track;
mod play_uri;
mod read;

pub use open::open_playlist;
pub use play_track::play_track;
pub use play_uri::play_uri;
pub use read::playlist_tracks;
#[allow(unused_imports)]
pub use open::open_page;

/// Una rola del tracklist: numero, titulo, artistas, duracion y track id.
#[derive(Debug, Clone)]
pub struct TrackItem {
    pub n: String,
    pub title: String,
    pub artist: String,
    pub duration: String,
    pub id: String,
}
