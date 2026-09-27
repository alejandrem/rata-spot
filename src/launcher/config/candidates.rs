//! Candidates: rutas conocidas con env vars + scoop + portable (sin `C:\`).

use std::path::PathBuf;

use super::exe_bases;

/// Candidatos armados con %ProgramFiles%, %LOCALAPPDATA%, etc. + portable.
pub(crate) fn candidates_from_env() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let bases = exe_bases();

    let apps: &[(&str, &str)] = &[
        (r"BraveSoftware\Brave-Browser\Application", "brave.exe"),
        (r"BraveSoftware\Brave-Browser-Beta\Application", "brave.exe"),
        (r"BraveSoftware\Brave-Browser-Nightly\Application", "brave.exe"),
        (r"Google\Chrome\Application", "chrome.exe"),
        (r"Google\Chrome Beta\Application", "chrome.exe"),
        (r"Microsoft\Edge\Application", "msedge.exe"),
        (r"Chromium\Application", "chromium.exe"),
    ];
    for base in &bases {
        for (sub, exe) in apps {
            out.push(base.join(sub).join(exe));
        }
    }

    // Scoop + portable junto al proyecto/exe (USB).
    if let Ok(home) = std::env::var("USERPROFILE") {
        let h = PathBuf::from(home);
        out.push(h.join(r"scoop\apps\brave\current\brave.exe"));
        out.push(h.join(r"scoop\apps\googlechrome\current\chrome.exe"));
        out.push(h.join(r"scoop\apps\microsoft-edge\current\msedge.exe"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        for exe in super::KNOWN_EXES {
            out.push(cwd.join(exe));
            out.push(cwd.join("browser").join(exe));
            out.push(cwd.join("brave").join(exe));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in super::KNOWN_EXES {
                out.push(dir.join(name));
                out.push(dir.join("browser").join(name));
            }
        }
    }
    out
}
