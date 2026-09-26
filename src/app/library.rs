//! Biblioteca en fondo: leer el DOM tarda segundos (scroll virtualizado);
//! no bloquear el arranque. La sidebar muestra "cargando..." mientras.
//! Tarea de una sola entrega: al terminar muere sola (sin abort).

use std::time::Duration;

use tokio::sync::mpsc;

use crate::{
    cdp::{self, LibraryItem},
    ui::AppState,
};

pub fn spawn_library_loader() -> mpsc::Receiver<Result<Vec<LibraryItem>, String>> {
    let (pl_tx, pl_rx) = mpsc::channel(1);
    tokio::spawn(async move {
        for _ in 0..20 {
            if cdp::ensure_spotify_tab().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let res = cdp::library_items().await.map_err(|e| e.to_string());
        let _ = pl_tx.send(res).await;
    });
    pl_rx
}

/// Recoger el resultado cuando el fondo termine (sin bloquear).
/// Despues del primer mensaje el canal muere: try_recv da Disconnected
/// y se ignora (costo cero).
pub fn drain_library(
    state: &mut AppState,
    pl_rx: &mut mpsc::Receiver<Result<Vec<LibraryItem>, String>>,
) {
    if let Ok(res) = pl_rx.try_recv() {
        match res {
            Ok(items) => {
                state.library = items;
                state.pl_index = 0;
                state.pl_state.select(Some(0));
                state.pl_msg.clear();
            }
            Err(e) => state.pl_msg = format!("biblioteca: {e}"),
        }
    }
}
