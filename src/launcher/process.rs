//! Proceso Brave: spawn del hijo, lanzamiento y limpieza al salir.

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result};

use super::config::{
    brave_flags_base, find_brave_exe, SPOTIFY_URL, WINDOW_HIDDEN, WINDOW_VISIBLE,
};
use super::window::{close_our_windows, snapshot_brave_windows, window_slot};

/// Handle global del proceso Brave que nosotros lanzamos (si aplica).
static BRAVE_CHILD: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();

fn child_slot() -> &'static Arc<Mutex<Option<Child>>> {
    BRAVE_CHILD.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// PID del Brave que lanzamos (None si aun no lanzamos ninguno).
/// Sirve para verificar en la TUI / Task Manager que es NUESTRO proceso.
pub fn child_pid() -> Option<u32> {
    child_slot()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.id()))
}

/// Lanza Spotify en VENTANA NUEVA del Brave predeterminado (tu perfil,
/// ya logueado). Retorna `false` (la matamos al salir si la lanzamos).
///
/// - SIN `--user-data-dir`: usa tu Brave de siempre, con tus cuentas.
/// - CON `--new-window`: aunque Brave ya este abierto, abre una ventana
///   nueva e independiente, no una pestaña en tu ventana actual.
///   UNA sola URL: duplicarla abria 2 pestanas de Spotify.
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
    cmd.args(brave_flags_base());
    // Tu perfil: sin --user-data-dir para conservar tus logins.
    cmd.arg(if visible {
        WINDOW_VISIBLE
    } else {
        WINDOW_HIDDEN
    });
    // Ventana nueva e independiente del mismo navegador (no pestaña).
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
