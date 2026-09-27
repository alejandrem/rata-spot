//! View_keys: `/` enfoca buscador, ←/Esc vuelve, `q` sale.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::ui::{AppState, View};

pub fn handle_view(state: &mut AppState, key: KeyEvent) -> Result<bool> {
    let is_repeat = key.kind == KeyEventKind::Repeat;
    match key.code {
        // `/`: enfocar el buscador (la barra ya esta visible).
        KeyCode::Char('/') => {
            if is_repeat {
                return Ok(false);
            }
            state.search_active = true;
        }
        // ← o Esc: desde Detail vuelve a resultados; si no, biblioteca.
        KeyCode::Left | KeyCode::Esc => {
            if state.view == View::Detail {
                state.back_to_search();
            } else {
                state.back_to_library();
            }
        }
        KeyCode::Char('q') => return Ok(true),
        _ => {}
    }
    Ok(false)
}
