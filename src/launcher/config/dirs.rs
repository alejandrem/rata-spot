//! Dirs: perfil aislado + dir de datos portable (sin `C:\` quemado).

use std::path::PathBuf;

/// Perfil aislado reservado (pediría login nuevo; usamos el normal).
#[allow(dead_code)]
pub fn rata_profile_dir() -> PathBuf {
    data_dir().join("rata-spot-brave-profile")
}

/// Dir de datos: LOCALAPPDATA -> USERPROFILE -> temp.
pub fn data_dir() -> PathBuf {
    for key in ["LOCALAPPDATA", "USERPROFILE"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return PathBuf::from(v);
            }
        }
    }
    std::env::temp_dir()
}

/// true si el perfil aislado es nuevo (aun sin login de Spotify).
#[allow(dead_code)]
pub fn is_fresh_profile() -> bool {
    let dir = rata_profile_dir();
    !dir.join("Default").join("Preferences").is_file() && !dir.join("Preferences").is_file()
}
