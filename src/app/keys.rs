//! Teclado: traduce eventos crossterm a acciones de la app.
//!
//! Windows manda Press + Repeat (sostenida) + Release por tecla. Sin
//! filtrar, 1 tap = Press+Release = 2 toggles (pausa fantasma que se
//! deshace sola) y sostener = flicker aleatorio. Reglas:
//! - Release se ignora en todo.
//! - Repeat solo en flechas/j/k (con throttle 120ms de nav_ok).
//! - Acciones (space/n/p/enter/l) solo responden a Press.
//!
//! Regresa Ok(true) si hay que salir del loop (q).

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::{
    cdp,
    gsmtc,
    ui::{AppState, View},
};

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
    match key.code {
        KeyCode::Char(' ') => {
            if is_repeat {
                return Ok(false);
            }
            if let Some(ref s) = session {
                // Ya suena o pausado con contexto -> GSMTC basta.
                let _ = gsmtc::toggle(s).await;
            } else {
                // Pagina fresca sin sesion: GSMTC no puede;
                // click al Play via CDP para arrancar sin mouse.
                state.status = "▶ iniciando Spotify...".to_string();
                // Redibujar ANTES del await (el click tarda segundos
                // y el loop se queda esperando aqui).
                let _ = terminal.draw(|f| crate::ui::render(f, state));
                match cdp::play_from_scratch().await {
                    Ok(_) => {
                        state.status = "▶ play enviado, conectando...".to_string()
                    }
                    Err(e) => {
                        // Mensaje especifico segun la causa:
                        // el generico "dale play" no decia nada.
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
        // Biblioteca (sidebar) y canciones (centro).
        // Flechas con throttle: sostenida avanza legible.
        KeyCode::Char('j') | KeyCode::Down => {
            if state.nav_ok() {
                match state.view {
                    View::Library => state.pl_move(1),
                    View::Tracks => state.tr_move(1),
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.nav_ok() {
                match state.view {
                    View::Library => state.pl_move(-1),
                    View::Tracks => state.tr_move(-1),
                }
            }
        }
        KeyCode::Char('l') => {
            if is_repeat {
                return Ok(false);
            }
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
        // → o Enter: en biblioteca abre las canciones de la
        // playlist; en canciones toca la rola elegida.
        KeyCode::Right | KeyCode::Enter => {
            if is_repeat {
                return Ok(false);
            }
            match state.view {
                View::Library => {
                    if let Some(item) = state.pl_selected().cloned() {
                        state.tr_playlist = item.name.clone();
                        state.tracks.clear();
                        state.tr_index = 0;
                        state.tr_state.select(Some(0));
                        state.tr_msg = "abriendo playlist...".to_string();
                        state.view = View::Tracks;
                        let _ = terminal.draw(|f| crate::ui::render(f, state));
                        match cdp::open_playlist(&item.uri).await {
                            Ok(()) => match cdp::playlist_tracks().await {
                                Ok(tracks) => {
                                    state.tracks = tracks;
                                    state.tr_index = 0;
                                    state.tr_state.select(Some(0));
                                    state.tr_msg.clear();
                                }
                                Err(e) => {
                                    state.tr_msg = format!("canciones: {e:.80}")
                                }
                            },
                            Err(e) => {
                                state.tr_msg = format!("no abre: {e:.80}")
                            }
                        }
                    }
                }
                View::Tracks => {
                    if let Some(t) = state.tr_selected().cloned() {
                        state.status = format!("▶ tocando {}...", t.title);
                        let _ = terminal.draw(|f| crate::ui::render(f, state));
                        match cdp::play_track(&t.id).await {
                            Ok(_) => {
                                state.status =
                                    "▶ reproduciendo, conectando...".to_string()
                            }
                            Err(e) => {
                                state.status = format!("no sono: {e:.60}");
                            }
                        }
                    }
                }
            }
        }
        // ← o Esc: volver a la biblioteca.
        KeyCode::Left | KeyCode::Esc => state.back_to_library(),
        KeyCode::Char('q') => return Ok(true),
        _ => {}
    }
    Ok(false)
}

/// Poll no-bloqueante largo: None si no hubo tecla en 250ms.
/// (Balance responsividad/CPU: no <100ms ni >1000ms.)
pub fn poll_key() -> Result<Option<KeyEvent>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(key));
        }
    }
    Ok(None)
}
