//! Navigate: j/k/flechas (con throttle) + l (recargar biblioteca).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{cdp, ui::{AppState, View}};

use super::super::terminal::Tui;

pub async fn handle_navigate(
    terminal: &mut Tui,
    state: &mut AppState,
    key: KeyEvent,
) -> Result<bool> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if state.nav_ok() {
                match state.view {
                    View::Library => state.pl_move(1),
                    View::Tracks => state.tr_move(1),
                    View::Detail => state.lyr_move(1),
                    View::Search => state.sr_move(1),
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.nav_ok() {
                match state.view {
                    View::Library => state.pl_move(-1),
                    View::Tracks => state.tr_move(-1),
                    View::Detail => state.lyr_move(-1),
                    View::Search => state.sr_move(-1),
                }
            }
        }
        KeyCode::Char('l') => {
            // Recargar biblioteca a mano (por si se abrio tarde).
            state.pl_msg = "cargando biblioteca...".to_string();
            let _ = terminal.draw(|f| crate::ui::render(f, state));
            match cdp::library_items().await {
                Ok(items) => {
                    state.library = items;
                    state.pl_index = 0;
                    state.pl_state.select(Some(0));
                    state.pl_msg.clear();
                }
                Err(e) => state.pl_msg = format!("biblioteca: {e}"),
            }
        }
        _ => {}
    }
    Ok(false)
}
