//! Lanzador de Brave ultra-optimizado (Fase 2).
//!
//! - Encuentra brave.exe en rutas comunes
//! - Lo lanza con flags de bajo consumo si no estaba abierto
//! - Mueve la ventana fuera de pantalla (no headless, pero invisible)
//! - Guarda el handle para matarlo al salir (solo si nosotros lo lanzamos)
//!
//! NOTA: no usa delay fijo. Lanza y regresa inmediatamente;
//! el loop reactivo de la Fase 3 se encarga de esperar la sesion GSMTC.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};

/// URL objetivo al lanzar Brave.
pub const SPOTIFY_URL: &str = "https://open.spotify.com";

/// Flags en orden de impacto en RAM (ver plan Fase 2).
/// `--disable-gpu` queda FUERA por defecto: ver Fase 5.4 (Widevine DRM).
const BRAVE_FLAGS: &[&str] = &[
    "--disable-extensions",
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-sync",
    "--disable-translate",
    "--disable-plugins",
    "--disable-default-apps",
    "--process-per-site",
    // TODO en un solo proceso (~60MB extra). Deshabilita sandboxing.
    // Para rata-spot (solo abre una URL) es aceptable. Si te preocupa
    // la seguridad, comenta esta linea y usa solo --process-per-site.
    "--single-process",
    "--window-position=-32000,-32000",
    "--window-size=1280,720",
    "--autoplay-policy=no-user-gesture-required",
    "--no-first-run",
    // OPCIONAL (Fase 5.4): "--disable-gpu",
];

/// Handle global del proceso Brave que nosotros lanzamos (si aplica).
static BRAVE_CHILD: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();

fn child_slot() -> &'static Arc<Mutex<Option<Child>>> {
    BRAVE_CHILD.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Busca brave.exe en las rutas comunes de Windows.
pub fn find_brave_exe() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
        PathBuf::from(
            r"C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe",
        ),
    ];

    // %LOCALAPPDATA%\BraveSoftware\Brave-Browser\Application\brave.exe
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_app_data)
                .join("BraveSoftware")
                .join("Brave-Browser")
                .join("Application")
                .join("brave.exe"),
        );
    }

    candidates.into_iter().find(|p| p.is_file())
}

/// Checa si brave.exe ya esta corriendo (sin dependencias extra).
/// Usa `tasklist` para no agregar crates solo para esto.
pub fn is_brave_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq brave.exe", "/NH"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            stdout.contains("brave.exe")
        }
        Err(_) => false,
    }
}

/// Lanza Brave con los flags optimizados si no estaba abierto.
///
/// Retorna `true` si Brave YA estaba abierto (no debemos matarlo al salir),
/// `false` si nosotros lo lanzamos (debemos matarlo en `cleanup()`).
pub fn ensure_brave_running() -> Result<bool> {
    if is_brave_running() {
        return Ok(true); // ya estaba abierto -> no tocar al salir
    }

    let brave_exe = find_brave_exe().context(
        "no se encontro brave.exe en rutas comunes. \
         Instala Brave o ajusta find_brave_exe()",
    )?;

    let child = Command::new(&brave_exe)
        .args(BRAVE_FLAGS)
        .arg(SPOTIFY_URL)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("no se pudo lanzar {:?}", brave_exe))?;

    // Guardar handle para cleanup(). Sin delay fijo: regresar ya;
    // la TUI mostrara "Conectando..." mientras GSMTC reintenta (Fase 3.2).
    *child_slot()
        .lock()
        .expect("mutex del handle de Brave envenenado") = Some(child);

    Ok(false)
}

/// Mata el Brave que lanzamos, solo si fuimos nosotros.
/// Si `was_already_running == true`, no hace nada.
pub fn cleanup(was_already_running: bool) {
    if was_already_running {
        return;
    }

    if let Ok(mut guard) = child_slot().lock() {
        if let Some(mut child) = guard.take() {
            // Intento graceful primero; si falla, kill.
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
