//! Running: ¿sigue vivo Brave? (solo brave.exe para el boot).
//! `is_browser_running` es aparte para diagnostico (Chrome/Edge no bloquean).

use std::process::Command;

/// Solo brave.exe: el boot solo se bloquea si BRAVE esta sordo.
/// Chrome/Edge abiertos NO deben activar la sala de espera.
pub fn is_brave_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq brave.exe", "/NH"])
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .to_lowercase()
            .contains("brave.exe"),
        Err(_) => false,
    }
}

/// Brave/Chrome/Edge/Chromium (diagnostico; NO bloquea el boot).
#[allow(dead_code)]
pub fn is_browser_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            stdout.contains("brave.exe")
                || stdout.contains("chrome.exe")
                || stdout.contains("msedge.exe")
                || stdout.contains("chromium.exe")
        }
        Err(_) => false,
    }
}
