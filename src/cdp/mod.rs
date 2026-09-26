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
//! - playback: click al Play + flujo del primer play
//! - library: leer "Tu biblioteca" + reproducir por URI
//! - tests: pruebas vivas (primer play + listado)

mod client;
mod library;
mod playback;
mod tabs;
mod transport;

#[cfg(test)]
mod tests;

pub use library::{play_library_uri, LibraryItem, library_items};
pub use playback::play_from_scratch;
#[allow(unused_imports)]
pub use playback::play_spotify;
pub use tabs::ensure_spotify_tab;
