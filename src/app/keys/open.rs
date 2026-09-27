//! Open: →/Enter abre playlist, toca rola o toca resultado + dashboard.

use anyhow::Result;
use crossterm::event::KeyEvent;

use crate::{cdp, ui::{AppState, View}};

use super::super::terminal::Tui;

pub async fn handle_open(
    terminal: &mut Tui,
    state: &mut AppState,
    _key: KeyEvent,
    is_repeat: bool,
) -> Result<bool> {
    if is_repeat {
        return Ok(false);
    }
    match state.view {
        View::Library => open_library(terminal, state).await,
        View::Tracks => play_selected_track(terminal, state).await,
        View::Search => play_selected_result(terminal, state).await,
        // En Detail, →/Enter no hace nada (j/k scrollean letra).
        View::Detail => Ok(false),
    }
}

async fn open_library(terminal: &mut Tui, state: &mut AppState) -> Result<bool> {
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
                Err(e) => state.tr_msg = format!("canciones: {e:.80}"),
            },
            Err(e) => state.tr_msg = format!("no abre: {e:.80}"),
        }
    }
    Ok(false)
}

async fn play_selected_track(terminal: &mut Tui, state: &mut AppState) -> Result<bool> {
    if let Some(t) = state.tr_selected().cloned() {
        state.status = format!("▶ tocando {}...", t.title);
        let _ = terminal.draw(|f| crate::ui::render(f, state));
        match cdp::play_track(&t.id).await {
            Ok(_) => state.status = "▶ reproduciendo, conectando...".to_string(),
            Err(e) => state.status = format!("no sono: {e:.60}"),
        }
    }
    Ok(false)
}

async fn play_selected_result(terminal: &mut Tui, state: &mut AppState) -> Result<bool> {
    if let Some(it) = state.sr_selected().cloned() {
        state.status = format!("▶ tocando {}...", it.name);
        let _ = terminal.draw(|f| crate::ui::render(f, state));
        match cdp::play_uri(&it.uri, &it.name).await {
            Ok(_) => {
                state.status = "▶ reproduciendo...".to_string();
                state.detail = None;
                state.detail_msg = "cargando dashboard...".to_string();
                state.lyr_scroll = 0;
                state.view = View::Detail;
                let _ = terminal.draw(|f| crate::ui::render(f, state));
                match cdp::track_detail().await {
                    Ok(d) => {
                        state.detail = Some(d);
                        state.detail_msg.clear();
                    }
                    Err(e) => state.detail_msg = format!("sin dashboard: {e:.60}"),
                }
            }
            Err(e) => state.status = format!("no sono: {e:.60}"),
        }
    }
    Ok(false)
}
