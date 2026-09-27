//! Find: localiza un Chromium compatible en cualquier PC Windows.
//! Orden: env override -> `where` -> registro -> rutas conocidas -> portable.

use std::path::PathBuf;

use super::candidates::candidates_from_env;
use super::lookup::{reg_app_path, where_lookup};
use super::KNOWN_EXES;

/// Alias historico: Brave era el unico soportado.
pub fn find_brave_exe() -> Option<PathBuf> {
    find_browser_exe()
}

/// Cualquier Chromium sirve para CDP+GSMTC. Prefiere Brave (adblock gratis).
pub fn find_browser_exe() -> Option<PathBuf> {
    if let Some(p) = env_override() {
        return Some(p);
    }
    for name in [
        "brave.exe",
        "chrome.exe",
        "msedge.exe",
        "chromium.exe",
        "brave",
        "chrome",
        "msedge",
        "chromium",
    ] {
        if let Some(p) = where_lookup(name) {
            return Some(p);
        }
    }
    for exe in KNOWN_EXES {
        if let Some(p) = reg_app_path(exe) {
            return Some(p);
        }
    }
    candidates_from_env().into_iter().find(|p| p.is_file())
}

/// Override manual: RATA_SPOT_BRAVE=/ruta/a/brave.exe o carpeta contenedora.
fn env_override() -> Option<PathBuf> {
    for key in ["RATA_SPOT_BRAVE", "RATA_SPOT_BROWSER", "RATA_SPOT_CHROME"] {
        let raw = std::env::var(key).ok()?;
        let raw = raw.trim().trim_matches('"');
        if raw.is_empty() {
            continue;
        }
        let p = PathBuf::from(raw);
        if p.is_file() {
            return Some(p);
        }
        for exe in KNOWN_EXES {
            let cand = p.join(exe);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}
