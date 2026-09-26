//! Lanzador de Brave ultra-optimizado (Fase 2).
//!
//! - Encuentra brave.exe en rutas comunes
//! - Lo lanza con flags de bajo consumo + ventana nueva del mismo perfil
//! - Rastrea la ventana por HWND para cerrarla al salir (ver window.rs)
//!
//! Tareas (1 archivo = 1 tarea):
//! - config: URL, flags, rutas de brave.exe y perfil
//! - process: spawn del hijo + lanzamiento + cleanup al salir
//! - window: rastreo HWND (EnumWindows) + cierre quirurgico con WM_CLOSE

pub mod config;
pub mod process;
pub mod window;

pub use config::SPOTIFY_URL;
#[allow(unused_imports)]
pub use config::{find_brave_exe, is_brave_running, rata_profile_dir};
#[allow(unused_imports)]
pub use config::is_fresh_profile;
pub use process::{child_pid, cleanup, launch_brave_spotify};
