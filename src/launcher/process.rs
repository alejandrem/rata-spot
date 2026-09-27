//! Proceso Brave: spawn del hijo, lanzamiento y limpieza al salir.

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result};

use super::config::{
    brave_flags_base, find_brave_exe,
    profile::{is_fresh, profile_dir, read_saved_port, write_saved_port},
    SPOTIFY_URL, WINDOW_HIDDEN, WINDOW_VISIBLE,
};
use super::ports::{debug_flags, set_cdp_port, wanted_cdp_port};
use super::window::{
    close_our_windows, snapshot_brave_windows, window_pid, window_slot, window_title,
};

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

/// Lanza Spotify en su PERFIL DEDICADO (casita propia, ver config/profile).
/// Retorna `false` (la matamos al salir si la lanzamos).
///
/// - CON `--user-data-dir` (perfil rata-spot, logueado una vez): cada
///   arranque es proceso propio aunque tu Brave personal siga abierto.
///   Sin esto Chromium delegaba al vivo e ignoraba CDP + flags (bugs #25/#29).
/// - CON `--new-window`: si ya hay instancia NUESTRA viva, abre ventana
///   nueva en ella (delegar a lo nuestro sí está bien: tiene CDP).
///   UNA sola URL: duplicarla abria 2 pestanas de Spotify.
/// - MODO DEBUG (temporal): ventana VISIBLE siempre, salvo `RATA_SPOT_HIDDEN=1`
///   (en perfil fresco se fuerza visible: hay que loguearse a mano).
pub async fn launch_brave_spotify() -> Result<bool> {
    let brave_exe = find_brave_exe().context(
        "no se encontro ningun Chromium (brave/chrome/edge/chromium). \
         Instala Brave o Edge, o fija la ruta con RATA_SPOT_BRAVE=\"C:\\ruta\\brave.exe\"",
    )?;

    let hidden = std::env::var("RATA_SPOT_HIDDEN").map(|v| v == "1").unwrap_or(false);
    // Perfil fresco = hay que loguearse a mano: visible a fuerza.
    let fresh = is_fresh();
    let visible = !hidden || fresh;
    if fresh && hidden {
        eprintln!("rata-spot: perfil nuevo, fuerzo ventana visible para el login único.");
    }

    // Puerto CDP de ESTE arranque (efimero; RATA_SPOT_PORT lo fija).
    // Se publica antes del spawn para que el transporte lo use.
    let cdp_port = wanted_cdp_port()?;
    set_cdp_port(cdp_port);

    // Foto de ventanas antes: lo nuevo que aparezca es NUESTRA ventana.
    // (Ojo: la clase es de todo Chromium; el PID filtra abajo.)
    let baseline = snapshot_brave_windows();

    let profile = profile_dir();
    let mut cmd = Command::new(&brave_exe);
    // Perfil dedicado PRIMERO (convención Chromium) + resto de flags.
    cmd.arg(format!("--user-data-dir={}", profile.display()));
    // Opt-out de dieta GPU: `--disable-gpu` va por defecto (~15-25MB menos);
    // si Widevine corta el audio, `RATA_SPOT_NOGPU=0` lo quita.
    // (`RATA_SPOT_NOGPU=1` legacy también lo deja puesto: mismo resultado.)
    let gpu_off = std::env::var("RATA_SPOT_NOGPU")
        .map(|v| v != "0")
        .unwrap_or(true);
    for f in brave_flags_base() {
        if *f == "--disable-gpu" && !gpu_off {
            continue;
        }
        cmd.arg(f);
    }
    for flag in debug_flags(cdp_port) {
        cmd.arg(flag);
    }
    // Posición de la ventana (oculta fuera de pantalla por defecto).
    cmd.arg(if visible {
        WINDOW_VISIBLE
    } else {
        WINDOW_HIDDEN
    });
    // Ventana nueva: en frío es la nuestra; si ya hay instancia NUESTRA
    // viva, delega a ella (bien: tiene nuestro CDP y perfil).
    cmd.arg("--new-window");
    cmd.arg(SPOTIFY_URL);
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    let child = cmd
        .spawn()
        .with_context(|| format!("no se pudo lanzar {:?}", brave_exe))?;
    let our_pid = child.id();

    *child_slot()
        .lock()
        .expect("mutex del handle de Brave envenenado") = Some(child);

    // Esperar a que la ventana exista para rastrear su HWND al salir.
    // (No es el delay de GSMTC: la TUI igual reintenta en paralelo.)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // ¿Delegó? Hijo muerto = abrió ventana en instancia nuestra ya viva.
    // (Con perfil dedicado NUNCA delega a tu Brave personal.)
    let delegated = child_slot()
        .lock()
        .expect("mutex del handle de Brave envenenado")
        .as_mut()
        .and_then(|c| c.try_wait().ok())
        .is_some_and(|st| st.is_some());
    if delegated {
        // El puerto nuevo se ignoró: reenganchar el guardado (instancia viva).
        if let Some(saved) = read_saved_port() {
            set_cdp_port(saved);
        }
    } else {
        write_saved_port(cdp_port);
    }

    // Verificación ruidosa: lo nuevo debe ser NUESTRO (mismo PID en frío;
    // con delegación el hijo murió: filtrar por título Spotify).
    // La clase sola no basta (Chrome/Edge comparten clase).
    let fresh_all: Vec<isize> = snapshot_brave_windows()
        .into_iter()
        .filter(|h| !baseline.contains(h))
        .collect();
    let ours: Vec<isize> = if delegated {
        fresh_all
            .into_iter()
            .filter(|h| window_title(*h).to_lowercase().contains("spotify"))
            .collect()
    } else {
        let by_pid: Vec<isize> = fresh_all
            .iter()
            .copied()
            .filter(|h| window_pid(*h) == Some(our_pid))
            .collect();
        if by_pid.is_empty() && !fresh_all.is_empty() {
            // Raro (reparenting): aceptar el diff pero avisar.
            eprintln!(
                "rata-spot [aviso]: ventana nueva sin PID nuestro (¿reparenting?). \
                 HWNDs: {fresh_all:?}, PID nuestro: {our_pid}"
            );
            fresh_all
        } else {
            by_pid
        }
    };
    if ours.is_empty() {
        eprintln!(
            "rata-spot [aviso]: no se detectó ventana nueva (delegada={delegated}). \
             Si Spotify no aparece, revisa la ventana de Brave."
        );
    }
    if let Ok(mut g) = window_slot().lock() {
        *g = ours;
    }

    if visible {
        eprintln!(
            "rata-spot [debug]: ventana NUEVA de Brave (perfil rata-spot) con {}.\n\
             Dale play una vez y controla desde la TUI.\n\
             (RATA_SPOT_HIDDEN=1 para volver a oculto)",
            SPOTIFY_URL
        );
    }

    // Sin delay fijo para GSMTC: regresar ya; la TUI mostrara "Conectando...".
    Ok(false)
}

/// Limpieza al salir con `q`: cierra lo que NOSOTROS abrimos
/// (perfil dedicado: tu Brave personal ni se toca).
/// Si `was_already_running == true`, no toca nada (reutilizamos sesion).
///
/// Dos casos:
/// - Arranque en frio (hijo vivo): el proceso es nuestro -> kill.
/// - Delegado a instancia nuestra viva (hijo muerto): kill no sirve ->
///   cerrar solo NUESTRA ventana por HWND (verificado por PID/título).
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
