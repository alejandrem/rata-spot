//! Orquestacion de la app (loop TUI) por tareas, 1 archivo = 1 tarea:
//!
//! - terminal: setup/restore del terminal (raw + pantalla alternativa)
//! - boot: arranque (reutilizar sesion o lanzar Brave nuevo)
//! - keys: teclado (Press/Repeat/Release + acciones por tecla)
//! - sync: reintento de sesion + refresh del track por tick
//! - library: carga de biblioteca en fondo (DOM tarda segundos)
//! - run: el loop que une todo + pausa/cleanup al salir

pub mod boot;
pub mod keys;
pub mod library;
pub mod run;
pub mod sync;
pub mod terminal;
