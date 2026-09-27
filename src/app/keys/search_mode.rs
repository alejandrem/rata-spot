//! Search_mode: cuando el buscador esta enfocado, todo va al query.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{cdp, ui::{AppState, View}};

use super::super::terminal::Tui;

pub async fn handle_search_input(
    terminal: &mut Tui,
    state: &mut AppState,
    key: KeyEvent,
) -> Result<bool> {
    match key.code {
        KeyCode::Esc => state.back_to_library(),
        KeyCode::Enter => {
            state.search_active = false;
            state.view = View::Search;
            let q = state.search_query.clone();
            state.results.clear();
            state.sr_index = 0;
            state.sr_state.select(Some(0));
            state.sr_msg = "buscando...".to_string();
            let _ = terminal.draw(|f| crate::ui::render(f, state));
            match cdp::search(&q).await {
                Ok(items) => {
                    state.results = items;
                    state.sr_index = 0;
                    state.sr_state.select(Some(0));
                    state.sr_msg.clear();
                }
                Err(e) => state.sr_msg = format!("busqueda: {e:.80}"),
            }
        }
        KeyCode::Backspace => {
            state.search_query.pop();
        }
        KeyCode::Char(c) => {
            // Escribiendo SI se acepta Repeat (tecla pegada).
            state.search_query.push(c);
        }
        _ => {}
    }
    Ok(false)
}
