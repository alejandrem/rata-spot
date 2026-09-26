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
use std::time::Duration;

use anyhow::{Context, Result};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW, IsWindow,
        IsWindowVisible, PostMessageW, WM_CLOSE,
    },
};

/// URL objetivo al lanzar Brave.
pub const SPOTIFY_URL: &str = "https://open.spotify.com/intl-es/";

/// Flags base en orden de impacto en RAM (ver plan Fase 2).
/// `--disable-gpu` queda FUERA por defecto: ver Fase 5.4 (Widevine DRM).
/// `--user-data-dir` y `--window-position` se agregan dinamicamente
/// en `launch_isolated_brave()` (perfil propio para coexistir con tu Brave normal).
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

const WINDOW_HIDDEN: &str = "--window-position=-32000,-32000";
const WINDOW_VISIBLE: &str = "--window-position=100,100";

/// Handle global del proceso Brave que nosotros lanzamos (si aplica).
static BRAVE_CHILD: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();

fn child_slot() -> &'static Arc<Mutex<Option<Child>>> {
    BRAVE_CHILD.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// HWNDs (como isize) de la ventana que NOSOTROS abrimos.
/// Si Brave ya estaba abierto, nuestro hijo delega y muere al instante,
/// asi que el `kill` no cierra nada: hay que cerrar la ventana por HWND.
static BRAVE_WINDOWS: OnceLock<Mutex<Vec<isize>>> = OnceLock::new();

fn window_slot() -> &'static Mutex<Vec<isize>> {
    BRAVE_WINDOWS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Clase de ventana top-level de Chromium (Brave la usa).
const BRAVE_WINDOW_CLASS: &str = "Chrome_WidgetWin_1";

unsafe extern "system" fn enum_brave_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<isize>);
    if IsWindowVisible(hwnd).as_bool() {
        let mut cls = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut cls);
        if n > 0 && String::from_utf16_lossy(&cls[..n as usize]) == BRAVE_WINDOW_CLASS {
            out.push(hwnd.0 as isize);
        }
    }
    BOOL(1)
}

/// Todas las ventanas visibles de Brave ahora mismo.
fn snapshot_brave_windows() -> Vec<isize> {
    let mut out: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_brave_cb),
            LPARAM(&mut out as *mut Vec<isize> as isize),
        );
    }
    out
}

fn window_title(raw: isize) -> String {
    unsafe {
        let hwnd = HWND(raw as *mut core::ffi::c_void);
        if !IsWindow(hwnd).as_bool() {
            return String::new();
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

/// Cierra solo NUESTRA ventana (verificada por titulo Spotify).
/// No toca tus otras ventanas de Brave. Sin prompt: WM_CLOSE graceful.
fn close_our_windows() {
    let raws: Vec<isize> = window_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    for raw in raws {
        unsafe {
            let hwnd = HWND(raw as *mut core::ffi::c_void);
            if !IsWindow(hwnd).as_bool() {
                continue; // el usuario ya la cerro
            }
            // Seguridad: si navegaste esa ventana a otro sitio, no cerrarla.
            if !window_title(raw).to_lowercase().contains("spotify") {
                continue;
            }
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
    if let Ok(mut g) = window_slot().lock() {
        g.clear();
    }
}

/// PID del Brave aislado que lanzamos (None si aun no lanzamos ninguno).
/// Sirve para verificar en la TUI / Task Manager que es NUESTRO proceso.
pub fn child_pid() -> Option<u32> {
    child_slot()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.id()))
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

/// Lanza Spotify en VENTANA NUEVA del Brave predeterminado (tu perfil,
/// ya logueado). Retorna `false` (la matamos al salir si la lanzamos).
///
/// - SIN `--user-data-dir`: usa tu Brave de siempre, con tus cuentas.
/// - CON `--new-window`: aunque Brave ya este abierto, abre una ventana
///   nueva e independiente, no una pestaña en tu ventana actual.
/// - NOTA honesta: si Brave ya estaba abierto, Chromium ignora los flags
///   de ahorro en esta ventana (ira normal de RAM). El modo optimizado
///   total solo aplica cuando Brave estaba cerrado.
/// - MODO DEBUG (temporal): ventana VISIBLE siempre, salvo `RATA_SPOT_HIDDEN=1`.
pub async fn launch_brave_spotify() -> Result<bool> {
    let brave_exe = find_brave_exe().context(
        "no se encontro brave.exe en rutas comunes. \
         Instala Brave o ajusta find_brave_exe()",
    )?;

    let hidden = std::env::var("RATA_SPOT_HIDDEN").map(|v| v == "1").unwrap_or(false);
    let visible = !hidden;

    // Foto de ventanas antes: lo nuevo que aparezca es NUESTRA ventana.
    let baseline = snapshot_brave_windows();

    let mut cmd = Command::new(&brave_exe);
    cmd.args(BRAVE_FLAGS_BASE);
    // Tu perfil: sin --user-data-dir para conservar tus logins.
    cmd.arg(if visible {
        WINDOW_VISIBLE
    } else {
        WINDOW_HIDDEN
    });
    // Ventana nueva e independiente del mismo navegador (no pestaña).
    // UNA sola URL: duplicarla abria 2 pestanas de Spotify.
    cmd.arg("--new-window");
    cmd.arg(SPOTIFY_URL);
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    let child = cmd
        .spawn()
        .with_context(|| format!("no se pudo lanzar {:?}", brave_exe))?;

    *child_slot()
        .lock()
        .expect("mutex del handle de Brave envenenado") = Some(child);

    // Esperar a que la ventana exista para rastrear su HWND al salir.
    // (No es el delay de GSMTC: la TUI igual reintenta en paralelo.)
    tokio::time::sleep(Duration::from_secs(3)).await;
    let ours: Vec<isize> = snapshot_brave_windows()
        .into_iter()
        .filter(|h| !baseline.contains(h))
        .collect();
    if let Ok(mut g) = window_slot().lock() {
        *g = ours;
    }

    if visible {
        eprintln!(
            "rata-spot [debug]: ventana NUEVA de Brave con {}.\n\
             Ya logueado (tu perfil), dale play una vez y controla desde la TUI.\n\
             (RATA_SPOT_HIDDEN=1 para volver a oculto)",
            SPOTIFY_URL
        );
    }

    // Sin delay fijo para GSMTC: regresar ya; la TUI mostrara "Conectando...".
    Ok(false)
}

/// Limpieza al salir con `q`: cierra lo que NOSOTROS abrimos.
/// Si `was_already_running == true`, no toca nada (reutilizamos sesion).
///
/// Dos casos:
/// - Arranque en frio (hijo vivo): el proceso es nuestro -> kill.
/// - Brave ya abierto (hijo muerto, delego): kill no sirve -> cerrar
///   solo NUESTRA ventana por HWND (tus otras ventanas ni se tocan).
/// En ambos, la musica ya se pauso por GSMTC antes de llegar aqui.
pub fn cleanup(was_already_running: bool) {
    if was_already_running {
        return;
    }

    if let Ok(mut guard) = child_slot().lock() {
        if let Some(mut child) = guard.take() {
            match child.try_wait() {
                Ok(None) => {
                    // Sigue vivo = arranque en frio, proceso nuestro.
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
                // Ya murio = delego al Brave abierto. Cerrar ventana abajo.
                _ => {
                    let _ = child.wait();
                }
            }
        }
    }
    close_our_windows();
}
