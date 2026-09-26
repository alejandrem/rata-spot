//! El loop: junta estado + dibujo + teclado + sync por tick.
//! Al salir (q o error) pausa por GSMTC y limpia el Brave nuestro.

use anyhow::Result;

use crate::{gsmtc, launcher, ui::AppState};

use super::{
    keys,
    library::{drain_library, spawn_library_loader},
    sync::{tick_gsmtc, SyncState},
    terminal::Tui,
};

pub async fn run_app(
    terminal: &mut Tui,
    was_already_running: bool,
    boot_status: String,
) -> Result<()> {
    let mut state = AppState::new();
    state.status = boot_status;
    // Historial: mostrar ultima cancion escuchada aunque aun no haya sesion.
    if let Some(last) = gsmtc::load_last_track() {
        state.smoother.restore_last_known(last);
    }
    let mut session: Option<gsmtc::Session> = None;
    let mut sync = SyncState::new(state.smoother.last_known().title);
    let mut pl_rx = spawn_library_loader();

    loop {
        drain_library(&mut state, &mut pl_rx);

        // a. Dibujar (si no conectado, el centro muestra "Conectando...").
        terminal.draw(|f| crate::ui::render(f, &mut state))?;

        // b/c. Teclado (poll 250ms adentro). true = salir.
        if let Some(key) = keys::poll_key()? {
            if keys::handle_key(terminal, &mut state, &session, key).await? {
                break;
            }
        }

        // d. Sync GSMTC del tick (reconexion + refresh).
        tick_gsmtc(&mut state, &mut session, &mut sync).await;
    }

    // Parar la musica al salir (q): pausa best-effort + cleanup.
    // Si Brave lo lanzamos nosotros, cleanup() ademas lo mata.
    // Si ya estaba abierto, lo dejamos vivo pero pausado.
    if let Some(ref s) = session {
        let _ = gsmtc::pause(s).await;
    }
    launcher::cleanup(was_already_running);
    Ok(())
}
