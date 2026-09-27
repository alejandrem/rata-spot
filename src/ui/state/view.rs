//! View: struct AppState + vista central + throttle de flechas.

use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::cdp::{LibraryItem, TrackItem};
use crate::gsmtc::{ProgressSmoother, TrackInfo};

/// Pantalla del panel central: biblioteca, canciones, busqueda o dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    Tracks,
    Search,
    Detail,
}

/// Estado que dibuja la TUI. El smoother ya guarda
/// last_position + last_timestamp para el progreso suave (Fase 3.1).
pub struct AppState {
    pub smoother: ProgressSmoother,
    /// false mientras GSMTC aun no registra sesion Brave.
    pub connected: bool,
    /// Diagnostico visible en la TUI (qué navegador lanzamos / PID / URL).
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
    /// Buscador: query en edicion, resultados y su cursor.
    pub search_query: String,
    pub search_active: bool,
    pub results: Vec<LibraryItem>,
    pub sr_index: usize,
    pub sr_state: ListState,
    pub sr_msg: String,
    /// Dashboard de la rola en reproduccion (vista Detail).
    pub detail: Option<crate::cdp::TrackDetail>,
    pub detail_msg: String,
    pub lyr_scroll: u16,
    /// Throttle de flechas: ultimo movimiento aceptado.
    pub(crate) last_nav: Instant,
}

/// Delay minimo entre movimientos de flecha (120ms).
pub(crate) const NAV_DELAY: Duration = Duration::from_millis(120);

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
        let mut sr_state = ListState::default();
        sr_state.select(Some(0));
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
            search_query: String::new(),
            search_active: false,
            results: Vec::new(),
            sr_index: 0,
            sr_state,
            sr_msg: String::new(),
            detail: None,
            detail_msg: String::new(),
            lyr_scroll: 0,
            last_nav: Instant::now() - NAV_DELAY,
        }
    }

    /// Track a mostrar (con posicion/progreso interpolados).
    pub fn display(&self) -> TrackInfo {
        self.smoother.display_track()
    }

    /// Throttle: true si ya paso el delay desde el ultimo movimiento.
    pub fn nav_ok(&mut self) -> bool {
        if self.last_nav.elapsed() >= NAV_DELAY {
            self.last_nav = Instant::now();
            true
        } else {
            false
        }
    }

    /// Volver a la vista biblioteca (← / Esc). Apaga la escritura.
    pub fn back_to_library(&mut self) {
        self.view = View::Library;
        self.search_active = false;
    }

    /// Volver a resultados (← / Esc desde Detail, conserva la busqueda).
    pub fn back_to_search(&mut self) {
        self.view = View::Search;
        self.search_active = false;
    }
}
