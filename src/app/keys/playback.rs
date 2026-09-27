//! Playback: space/n/p (solo Press, Repeat se ignora).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{cdp, gsmtc, ui::AppState};

use super::super::terminal::Tui;

pub async fn handle_playback(
    terminal: &mut Tui,
    state: &mut AppState,
    session: &Option<gsmtc::Session>,
    key: KeyEvent,
    is_repeat: bool,
) -> Result<bool> {
    match key.code {
        KeyCode::Char(' ') => {
            if is_repeat {
                return Ok(false);
            }
            if let Some(ref s) = session {
                let _ = gsmtc::toggle(s).await;
            } else {
                // Pagina fresca sin sesion: click al Play via CDP.
                state.status = "▶ iniciando Spotify...".to_string();
                let _ = terminal.draw(|f| crate::ui::render(f, state));
                match cdp::play_from_scratch().await {
                    Ok(_) => state.status = "▶ play enviado, conectando...".to_string(),
                    Err(e) => {
                        let msg = e.to_string();
                        state.status = if msg.contains("sin puerto CDP") {
                            "cierra Brave por completo y haz cargo run".to_string()
                        } else if msg.contains("no hay pestana") {
                            "abre la pestana Spotify en Brave".to_string()
                        } else {
                            "dale play una vez en Brave".to_string()
                        };
                    }
                }
            }
        }
        KeyCode::Char('n') => {
            if is_repeat {
                return Ok(false);
            }
            if let Some(ref s) = session {
                let _ = gsmtc::next(s).await;
            }
        }
        KeyCode::Char('p') => {
            if is_repeat {
                return Ok(false);
            }
            if let Some(ref s) = session {
                let _ = gsmtc::prev(s).await;
            }
        }
        _ => {}
    }
    Ok(false)
}
