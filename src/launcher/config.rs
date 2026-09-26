//! Config del lanzador: URL, flags de bajo consumo y rutas de Brave.

use std::path::PathBuf;
use std::process::Command;

/// URL objetivo al lanzar Brave.
pub const SPOTIFY_URL: &str = "https://open.spotify.com/intl-es/";

/// Flags base en orden de impacto en RAM (ver plan Fase 2).
/// `--disable-gpu` queda FUERA por defecto: ver Fase 5.4 (Widevine DRM).
/// `--single-process` DESACTIVADO: rompe Widevine/sandbox y Spotify
/// cierra la ventana al instante (el plan lo permite).
/// `--remote-debugging-port`: CDP para iniciar musica con [space] sin mouse.
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
    // DEBUG: `--single-process` DESACTIVADO temporalmente. Rompe Widevine/sandbox
    // y Spotify cierra la ventana al instante (por eso "se cerro" sin ver nada).
    // El plan lo permite: si preocupa la seguridad, usar solo --process-per-site.
    // "--single-process",
    "--window-size=1280,720",
    "--autoplay-policy=no-user-gesture-required",
    "--no-first-run",
    // CDP para que la TUI inicie musica con [space] sin tocar el mouse.
    // Sin esto no hay forma de darle play a una pagina fresca (GSMTC solo
    // controla lo que YA suena). Si Brave ya estaba abierto sin este flag,
    // el puerto no existe y space degradada a GSMTC (dale play una vez).
    "--remote-debugging-port=9222",
    "--remote-allow-origins=*",
    // OPCIONAL (Fase 5.4): "--disable-gpu",
];

pub(crate) const WINDOW_HIDDEN: &str = "--window-position=-32000,-32000";
pub(crate) const WINDOW_VISIBLE: &str = "--window-position=100,100";

pub(crate) fn brave_flags_base() -> &'static [&'static str] {
    BRAVE_FLAGS_BASE
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

/// Checa si brave.exe ya esta corriendo (solo diagnostico).
/// Usa `tasklist` para no agregar crates solo para esto.
#[allow(dead_code)]
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

/// Perfil aislado de rata-spot (reservado para el modo oculto final).
/// Ahora lanzamos con el perfil normal para reutilizar tu login.
#[allow(dead_code)]
pub fn rata_profile_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("rata-spot-brave-profile")
}

/// true si el perfil aislado es nuevo (aun sin login de Spotify).
#[allow(dead_code)]
pub fn is_fresh_profile() -> bool {
    let dir = rata_profile_dir();
    // Si no existe el dir o no hay Preferences, es fresco.
    !dir.join("Default").join("Preferences").is_file() && !dir.join("Preferences").is_file()
}
