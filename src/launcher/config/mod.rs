//! Config del lanzador: URL, flags de bajo consumo y rutas del navegador.

mod candidates;
mod find;
mod lookup;
mod running;

pub use find::{find_brave_exe, find_browser_exe};
pub use running::{is_brave_running, is_browser_running};
#[allow(unused_imports)]
pub(crate) use candidates::candidates_from_env;
#[allow(unused_imports)]
pub(crate) use lookup::{reg_app_path, where_lookup};

use std::path::PathBuf;

/// URL objetivo al lanzar el navegador.
/// Sin locale fijo (`intl-es`) para que funcione en cualquier dispositivo;
/// Spotify redirige solo al locale del usuario.
pub const SPOTIFY_URL: &str = "https://open.spotify.com/";

/// Flags base en orden de impacto en RAM (ver plan Fase 2).
/// `--disable-gpu` queda FUERA por defecto: ver Fase 5.4 (Widevine DRM).
/// `--single-process` DESACTIVADO: rompe Widevine/sandbox y Spotify
/// cierra la ventana al instante (el plan lo permite).
/// Los flags de depuracion (puerto + origins) NO van aqui: se construyen
/// por arranque en `ports::debug_flags` (puerto efimero + origins
/// acotados a 127.0.0.1, jamas `*`: ver ports.rs P0/P1).
const BRAVE_FLAGS_BASE: &[&str] = &[
    "--disable-extensions",
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-sync",
    "--disable-translate",
    "--disable-plugins",
    "--disable-default-apps",
    "--process-per-site",
    "--window-size=1280,720",
    "--autoplay-policy=no-user-gesture-required",
    "--no-first-run",
];

pub(crate) const WINDOW_HIDDEN: &str = "--window-position=-32000,-32000";
pub(crate) const WINDOW_VISIBLE: &str = "--window-position=100,100";

pub(crate) fn brave_flags_base() -> &'static [&'static str] {
    BRAVE_FLAGS_BASE
}

/// Exes Chromium conocidos (Brave primero: Shields gratis para free).
pub(crate) const KNOWN_EXES: &[&str] =
    &["brave.exe", "chrome.exe", "msedge.exe", "chromium.exe"];

pub(crate) fn exe_bases() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    for key in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                bases.push(PathBuf::from(v));
            }
        }
    }
    bases.sort();
    bases.dedup();
    bases
}
