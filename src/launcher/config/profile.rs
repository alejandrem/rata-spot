//! Perfil dedicado de rata-spot: casita propia con tus logins.
//!
//! Por qué existe (ver bitácora #29): con el perfil normal, un Brave ya
//! abierto secuestra el lanzamiento (delegación Chromium: el segundo
//! proceso muere y el vivo ignora `--remote-debugging-port` y los flags
//! de dieta). Con perfil propio cada `cargo run` es proceso propio:
//! flags + CDP siempre aplicados, y tu Brave personal ni se entera.
//!
//! Te logueas UNA vez (el perfil persiste en disco) y listo. Si borras
//! la carpeta, Spotify pide login de nuevo (es solo eso, no es un bug).

use std::path::{Path, PathBuf};

/// Nombre de la carpeta del perfil dentro del dir de datos.
const PROFILE_DIR_NAME: &str = "rata-spot-brave-profile";
/// Puerto CDP de la última instancia en frío (para reenganchar sin adivinar).
const SAVED_PORT_FILE: &str = "rata-spot-cdp-port.txt";

/// Override manual: `RATA_SPOT_PROFILE=D:\ruta\mi-perfil`.
fn override_dir() -> Option<PathBuf> {
    let raw = std::env::var("RATA_SPOT_PROFILE").ok()?;
    let raw = raw.trim().trim_matches('"');
    if raw.is_empty() {
        return None;
    }
    Some(PathBuf::from(raw))
}

/// Dir del perfil dedicado (persistente entre runs).
pub fn profile_dir() -> PathBuf {
    if let Some(p) = override_dir() {
        return p;
    }
    for key in ["LOCALAPPDATA", "USERPROFILE"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return PathBuf::from(v).join(PROFILE_DIR_NAME);
            }
        }
    }
    std::env::temp_dir().join(PROFILE_DIR_NAME)
}

/// true si el perfil aún no tiene login (sin Preferences de Chromium).
pub fn is_fresh() -> bool {
    is_fresh_in(&profile_dir())
}

pub(crate) fn is_fresh_in(dir: &Path) -> bool {
    // Si no existe el dir o no hay Preferences, es fresco.
    !dir.join("Default").join("Preferences").is_file() && !dir.join("Preferences").is_file()
}

/// ¿Hay una instancia nuestra viva con este perfil? (lock de Chromium).
/// Sirve para el status ("reenganchando instancia propia"); el lanzamiento
/// no se bloquea por esto: delegar a lo nuestro está bien (tiene CDP).
pub fn is_locked() -> bool {
    let dir = profile_dir();
    ["SingletonLock", "SingletonSocket", "lockfile", "SingletonCookie"]
        .iter()
        .any(|n| dir.join(n).exists())
}

fn saved_port_path_in(dir: &Path) -> PathBuf {
    dir.join(SAVED_PORT_FILE)
}

/// Puerto CDP guardado por el último arranque en frío (None si no hay).
pub fn read_saved_port() -> Option<u16> {
    read_saved_port_in(&profile_dir())
}

pub(crate) fn read_saved_port_in(dir: &Path) -> Option<u16> {
    let raw = std::fs::read_to_string(saved_port_path_in(dir)).ok()?;
    let p: u16 = raw.trim().parse().ok()?;
    if p == 0 {
        return None;
    }
    Some(p)
}

/// Guarda el puerto (SOLO arranque en frío: el delegado no tiene puerto).
/// Best-effort: si falla, el transporte usa el 9222 legacy.
pub fn write_saved_port(port: u16) {
    write_saved_port_in(&profile_dir(), port);
}

pub(crate) fn write_saved_port_in(dir: &Path, port: u16) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(saved_port_path_in(dir), port.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_case(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rata-spot-profile-test-{}-{}",
            std::process::id(),
            tag
        ))
    }

    #[test]
    fn dir_no_vacio_y_con_nombre_propio() {
        // Solo lee env real (sin mutarlo): debe terminar en la carpeta rata.
        let d = profile_dir();
        assert!(!d.as_os_str().is_empty());
        let name = d
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        // Con override o sin él, nunca debe ser un dir raíz/pelado.
        assert!(!name.is_empty(), "profile_dir sin nombre final: {d:?}");
    }

    #[test]
    fn fresco_si_no_hay_preferences() {
        let dir = tmp_case("fresco");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(is_fresh_in(&dir), "dir inexistente debe verse fresco");
        std::fs::create_dir_all(&dir).expect("mkdir tmp");
        assert!(is_fresh_in(&dir), "dir sin Preferences debe verse fresco");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deja_de_ser_fresco_con_preferences() {
        let dir = tmp_case("login");
        let pref = dir.join("Default").join("Preferences");
        std::fs::create_dir_all(pref.parent().expect("parent")).expect("mkdir tmp");
        std::fs::write(&pref, "{}").expect("write prefs");
        assert!(!is_fresh_in(&dir), "con Preferences ya no es fresco");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn puerto_guardado_roundtrip() {
        let dir = tmp_case("puerto");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_saved_port_in(&dir), None, "sin archivo no hay puerto");
        write_saved_port_in(&dir, 9333);
        assert_eq!(read_saved_port_in(&dir), Some(9333));
        // Basura o cero no valen.
        std::fs::write(saved_port_path_in(&dir), "hola").expect("write basura");
        assert_eq!(read_saved_port_in(&dir), None);
        std::fs::write(saved_port_path_in(&dir), "0").expect("write cero");
        assert_eq!(read_saved_port_in(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
