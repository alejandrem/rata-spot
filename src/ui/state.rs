//! Estado de la TUI: smoother + conexion + biblioteca.

use ratatui::widgets::ListState;

use crate::cdp::LibraryItem;
use crate::gsmtc::{ProgressSmoother, TrackInfo};

/// Estado que dibuja la TUI. El smoother ya guarda
/// last_position + last_timestamp para el progreso suave (Fase 3.1).
pub struct AppState {
    pub smoother: ProgressSmoother,
    /// false mientras GSMTC aun no registra sesion Brave.
    pub connected: bool,
    /// Diagnostico visible en la TUI (qué Brave lanzamos / PID / URL).
    /// Los println! de antes quedaban ocultos bajo la pantalla alternativa.
    pub status: String,
    /// Biblioteca leida del DOM (carga en fondo al arrancar).
    pub library: Vec<LibraryItem>,
    pub pl_index: usize,
    pub pl_state: ListState,
    /// "cargando..." / error / "" cuando ya cargo.
    pub pl_msg: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let mut pl_state = ListState::default();
        pl_state.select(Some(0));
        Self {
            smoother: ProgressSmoother::new(),
            connected: false,
            status: String::new(),
            library: Vec::new(),
            pl_index: 0,
            pl_state,
            pl_msg: "cargando biblioteca...".to_string(),
        }
    }

    /// Track a mostrar (con posicion/progreso interpolados).
    pub fn display(&self) -> TrackInfo {
        self.smoother.display_track()
    }

    /// Mover seleccion de la biblioteca (j/k), con wrap.
    pub fn pl_move(&mut self, delta: i32) {
        if self.library.is_empty() {
            return;
        }
        let n = self.library.len() as i32;
        self.pl_index = (self.pl_index as i32 + delta).rem_euclid(n) as usize;
        self.pl_state.select(Some(self.pl_index));
    }

    pub fn pl_selected(&self) -> Option<&LibraryItem> {
        self.library.get(self.pl_index)
    }
}
