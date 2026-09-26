//! Estado de la TUI: smoother + conexion + biblioteca + canciones.

use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::cdp::{LibraryItem, TrackItem};
use crate::gsmtc::{ProgressSmoother, TrackInfo};

/// Pantalla del panel central: biblioteca o canciones de una playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    Tracks,
}

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
    /// Vista central: biblioteca (estado) o canciones de playlist.
    pub view: View,
    /// Canciones de la playlist abierta (→ / Enter).
    pub tracks: Vec<TrackItem>,
    pub tr_index: usize,
    pub tr_state: ListState,
    /// Nombre de la playlist abierta / "cargando..." / error.
    pub tr_msg: String,
    pub tr_playlist: String,
    /// Throttle de flechas: ultimo movimiento aceptado.
    last_nav: Instant,
}

/// Delay minimo entre movimientos de flecha (120ms): dejarla
/// presionada no debe volar la lista.
const NAV_DELAY: Duration = Duration::from_millis(120);

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let mut pl_state = ListState::default();
        pl_state.select(Some(0));
        let mut tr_state = ListState::default();
        tr_state.select(Some(0));
        Self {
            smoother: ProgressSmoother::new(),
            connected: false,
            status: String::new(),
            library: Vec::new(),
            pl_index: 0,
            pl_state,
            pl_msg: "cargando biblioteca...".to_string(),
            view: View::Library,
            tracks: Vec::new(),
            tr_index: 0,
            tr_state,
            tr_msg: String::new(),
            tr_playlist: String::new(),
            last_nav: Instant::now() - NAV_DELAY,
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

    /// Throttle: true si ya paso el delay desde el ultimo movimiento.
    /// Asi la flecha sostenida avanza a ritmo legible, no volando.
    pub fn nav_ok(&mut self) -> bool {
        if self.last_nav.elapsed() >= NAV_DELAY {
            self.last_nav = Instant::now();
            true
        } else {
            false
        }
    }

    /// Mover seleccion de canciones (j/k en vista Tracks), con wrap.
    pub fn tr_move(&mut self, delta: i32) {
        if self.tracks.is_empty() {
            return;
        }
        let n = self.tracks.len() as i32;
        self.tr_index = (self.tr_index as i32 + delta).rem_euclid(n) as usize;
        self.tr_state.select(Some(self.tr_index));
    }

    pub fn tr_selected(&self) -> Option<&TrackItem> {
        self.tracks.get(self.tr_index)
    }

    /// Volver a la vista biblioteca (← / Esc).
    pub fn back_to_library(&mut self) {
        self.view = View::Library;
    }
}
