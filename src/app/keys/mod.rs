//! Teclado: traduce eventos crossterm a acciones de la app.
//!
//! Windows manda Press + Repeat + Release por tecla. Reglas:
//! - Release se ignora en todo.
//! - Repeat solo en flechas/j/k (con throttle 120ms) y escribiendo.
//! - Acciones (space/n/p/enter/l) solo responden a Press.

mod navigate;
mod open;
mod playback;
mod search_mode;
mod view_keys;

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::{gsmtc, ui::AppState};

use super::terminal::Tui;

pub async fn handle_key(
    terminal: &mut Tui,
    state: &mut AppState,
    session: &Option<gsmtc::Session>,
    key: KeyEvent,
) -> Result<bool> {
    if key.kind == KeyEventKind::Release {
        return Ok(false);
    }
    let is_repeat = key.kind == KeyEventKind::Repeat;
    // Modo escritura: todo va al query (si no, space pausaria escribiendo).
    if state.search_active {
        return search_mode::handle_search_input(terminal, state, key).await;
    }
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('n') | KeyCode::Char('p') => {
            playback::handle_playback(terminal, state, session, key, is_repeat).await
        }
        KeyCode::Char('j') | KeyCode::Down
        | KeyCode::Char('k') | KeyCode::Up
        | KeyCode::Char('l') => navigate::handle_navigate(terminal, state, key).await,
        KeyCode::Right | KeyCode::Enter => {
            open::handle_open(terminal, state, key, is_repeat).await
        }
        KeyCode::Char('/') | KeyCode::Left | KeyCode::Esc | KeyCode::Char('q') => {
            view_keys::handle_view(state, key)
        }
        _ => Ok(false),
    }
}

/// Poll no-bloqueante largo: None si no hubo tecla en 250ms.
pub fn poll_key() -> Result<Option<KeyEvent>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(key));
        }
    }
    Ok(None)
}
