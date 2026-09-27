//! Orquestacion de la app (loop TUI) por tareas, 1 carpeta = 1 dominio:
//!
//! - terminal: setup/restore del terminal (raw + pantalla alternativa)
//! - boot: arranque (reutilizar sesion o lanzar navegador nuevo)
//! - keys/: search_mode/playback/navigate/open/view_keys + dispatcher
//! - sync/: state/tick/switch (reintento + refresh por tick)
//! - library: carga de biblioteca en fondo (DOM tarda segundos)
//! - run: el loop que une todo + pausa/cleanup al salir

pub mod boot;
pub mod keys;
pub mod library;
pub mod run;
pub mod sync;
pub mod terminal;
