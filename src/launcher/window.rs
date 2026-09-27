//! Rastreo de ventanas Brave por HWND + cierre quirurgico.
//!
//! Si Brave ya estaba abierto, nuestro hijo delega `--new-window` y muere
//! al instante: el `kill` no cierra nada. Por eso se toma foto de ventanas
//! (clase Chrome_WidgetWin_1) antes/despues de lanzar; lo nuevo es NUESTRA
//! ventana y al salir se le manda WM_CLOSE (las demas ni se tocan).

use std::sync::{Mutex, OnceLock};

use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible, PostMessageW, WM_CLOSE,
    },
};

/// HWNDs (como isize) de la ventana que NOSOTROS abrimos.
static BRAVE_WINDOWS: OnceLock<Mutex<Vec<isize>>> = OnceLock::new();

pub(crate) fn window_slot() -> &'static Mutex<Vec<isize>> {
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
pub(crate) fn snapshot_brave_windows() -> Vec<isize> {
    let mut out: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_brave_cb),
            LPARAM(&mut out as *mut Vec<isize> as isize),
        );
    }
    out
}

/// Título actual de la ventana ("": cerrada o sin título).
/// Se usa para NO cerrar ventanas ajenas y para verificar la nuestra.
pub(crate) fn window_title(raw: isize) -> String {
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

/// PID dueño de la ventana (None si ya murió).
/// La clase `Chrome_WidgetWin_1` es de TODO Chromium (Brave/Chrome/Edge),
/// así que el snapshot solo no basta: el PID distingue nuestra instancia.
pub(crate) fn window_pid(raw: isize) -> Option<u32> {
    unsafe {
        let hwnd = HWND(raw as *mut core::ffi::c_void);
        if !IsWindow(hwnd).as_bool() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        Some(pid)
    }
}

/// Cierra solo NUESTRA ventana (verificada por titulo Spotify).
/// No toca tus otras ventanas de Brave. Sin prompt: WM_CLOSE graceful.
pub(crate) fn close_our_windows() {
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
