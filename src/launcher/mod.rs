//! Lanzador de Brave ultra-optimizado (Fase 2).
//!
//! - Encuentra brave.exe en rutas comunes
//! - Lo lanza con flags de bajo consumo + ventana nueva del mismo perfil
//! - Rastrea la ventana por HWND para cerrarla al salir (ver window.rs)
//!
//! Tareas (1 carpeta = 1 dominio):
//! - config/: find/lookup/candidates/running (URL, flags, navegador)
//! - ports: puerto CDP efimero + origins acotados (P0/P1)
//! - process: spawn del hijo + lanzamiento + cleanup al salir
//! - window: rastreo HWND (EnumWindows) + cierre quirurgico con WM_CLOSE

pub mod config;
pub mod ports;
pub mod process;
pub mod window;

pub use config::SPOTIFY_URL;
#[allow(unused_imports)]
pub use config::{find_brave_exe, find_browser_exe, is_brave_running, is_browser_running};
pub use process::{child_pid, cleanup, launch_brave_spotify};
