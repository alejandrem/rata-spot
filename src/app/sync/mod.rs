//! Sync GSMTC por tick: reintento de sesion + refresh del track.
//!
//! Antidotos de barra congelada (sin costo extra):
//! - sesion preferida = la que suena (no una vieja pausada)
//! - posicion que avanza => sonando (status mentiroso)
//! - duracion reutilizada si falta el timeline
//! - sesion muerta (12 fallos ~3s) => soltar y reconectar

mod state;
mod switch;
mod tick;

pub use state::SyncState;
pub use tick::tick_gsmtc;
#[allow(unused_imports)]
pub(crate) use switch::switch_if_changed;
