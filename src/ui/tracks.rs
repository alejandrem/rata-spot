//! Vista central de canciones: el tracklist de la playlist abierta
//! (→ o Enter desde la sidebar). Solo dibuja las canciones, nada mas.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::state::AppState;

/// Dibuja la lista de canciones en `area` (stateful por el cursor).
pub fn render_tracks(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let title = if state.tracks.is_empty() {
        format!(" 🎵 {} ", state.tr_playlist)
    } else {
        format!(" 🎵 {} ({}) ", state.tr_playlist, state.tracks.len())
    };
    let block = Block::default().borders(Borders::ALL).title(title);

    if state.tracks.is_empty() {
        let msg = if state.tr_msg.is_empty() {
            "cargando canciones...".to_string()
        } else {
            state.tr_msg.clone()
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = state
        .tracks
        .iter()
        .map(|t| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>3}. ", t.n),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(t.title.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!(" — {}", t.artist.clone()),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!(" [{}]", t.duration.clone()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut state.tr_state);
}
