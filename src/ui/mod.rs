//! Interfaz TUI con Ratatui (Fase 4 + biblioteca).
//!
//! Layout: sidebar IZQUIERDA (Tu biblioteca) + columna principal con
//! header, progreso, centro y footer.
//!
//! Tareas (1 archivo = 1 tarea):
//! - state: AppState + seleccion de biblioteca
//! - player: header, barra de progreso, centro y footer
//! - sidebar: panel izquierdo con Tu biblioteca (j/k + Enter)

pub mod detail;
pub mod player;
pub mod search;
pub mod sidebar;
pub mod state;
pub mod tracks;

pub use state::{AppState, View};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

/// Render raiz: split horizontal sidebar | reproductor.
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32), // sidebar biblioteca
            Constraint::Min(1),         // reproductor
        ])
        .split(frame.size());

    sidebar::render_sidebar(frame, state, outer[0]);
    player::render_player(frame, state, outer[1]);
}
