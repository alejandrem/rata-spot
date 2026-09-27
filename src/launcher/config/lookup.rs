//! Lookup: `where` en PATH + registro App Paths (sin crates nuevos).

use std::path::PathBuf;
use std::process::Command;

/// `where <exe>`: primera linea que sea archivo existente.
pub(crate) fn where_lookup(name: &str) -> Option<PathBuf> {
    let out = Command::new("where").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().trim_matches('"'))
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

/// Lee `HKLM/HKCU\...\App Paths\<exe>` con `reg query` (sin crates nuevos).
pub(crate) fn reg_app_path(exe: &str) -> Option<PathBuf> {
    for hive in [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
    ] {
        let key = format!("{hive}\\{exe}");
        let out = Command::new("reg")
            .args(["query", &key, "/ve"])
            .output()
            .ok()?;
        if !out.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("REG_SZ") {
                if let Some(idx) = line.find("REG_SZ") {
                    let cand = line[idx + "REG_SZ".len()..].trim().trim_matches('"');
                    let p = PathBuf::from(cand);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}
