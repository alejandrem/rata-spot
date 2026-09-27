//! Sesion GSMTC de Brave: busqueda, lectura y controles de transporte.
//!
//! Nota windows 0.58: IAsyncOperation NO es Future -> se usa `.get()`
//! bloqueante (llamadas COM locales de ms, aceptable en loops 250ms/1s).

mod control;
mod find;
mod pick;
mod read;

pub use control::{next, pause, play, prev, toggle};
pub use find::{brave_sessions, get_brave_session};
pub use pick::pick_session;
pub use read::get_track;
