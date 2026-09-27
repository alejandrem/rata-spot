//! Estado de la TUI: smoother + conexion + biblioteca + canciones.

mod detail_nav;
mod library_nav;
mod search_nav;
mod tracks_nav;
mod view;

pub use view::{AppState, View};

use crate::cdp::{LibraryItem, TrackItem};
use view::AppState as State;

impl State {
    /// Mover seleccion de la biblioteca (j/k), con wrap.
    pub fn pl_move(&mut self, delta: i32) {
        library_nav::pl_move(self, delta)
    }
    pub fn pl_selected(&self) -> Option<&LibraryItem> {
        library_nav::pl_selected(self)
    }
    /// Mover seleccion de canciones (j/k en vista Tracks), con wrap.
    pub fn tr_move(&mut self, delta: i32) {
        tracks_nav::tr_move(self, delta)
    }
    pub fn tr_selected(&self) -> Option<&TrackItem> {
        tracks_nav::tr_selected(self)
    }
    /// Mover seleccion de resultados (j/k en vista Search), con wrap.
    pub fn sr_move(&mut self, delta: i32) {
        search_nav::sr_move(self, delta)
    }
    pub fn sr_selected(&self) -> Option<&LibraryItem> {
        search_nav::sr_selected(self)
    }
    /// Scrollear letra del dashboard (j/k en vista Detail), con tope.
    pub fn lyr_move(&mut self, delta: i32) {
        detail_nav::lyr_move(self, delta)
    }
}
