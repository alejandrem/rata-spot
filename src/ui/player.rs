//! Columna principal: header, barra de progreso, centro y footer.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use super::state::{AppState, View};
use super::tracks::render_tracks;

/// Dibuja header + progreso + centro + footer en `area`.
/// El centro muestra canciones si se abrio una playlist (→/Enter),
/// o el estado de conexion en vista biblioteca.
pub fn render_player(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // progreso
            Constraint::Min(1),    // centro flexible
            Constraint::Length(1), // footer
        ])
        .split(area);

    let track = state.display();

    // ---- HEADER ----
    let icon = if track.playing { "▶" } else { "⏸" };
    let header_block = Block::default()
        .borders(Borders::ALL)
        .title(" 🐀 rata-spot ");
    let header_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", icon),
            Style::default().fg(if track.playing {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(
            track.title.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" — ", Style::default().fg(Color::DarkGray)),
        Span::styled(track.artist.clone(), Style::default().fg(Color::Gray)),
    ]))
    .block(header_block);
    frame.render_widget(header_text, chunks[0]);

    // ---- BARRA DE PROGRESO ----
    let gauge_block = Block::default().borders(Borders::ALL).title(" progreso ");
    let gauge = Gauge::default()
        .block(gauge_block)
        .gauge_style(Style::default().fg(if track.playing {
            Color::Green
        } else {
            Color::DarkGray
        }))
        .ratio(track.progress.clamp(0.0, 1.0))
        .label(track.format_time());
    frame.render_widget(gauge, chunks[1]);

    // ---- CENTRO: canciones de la playlist o estado ----
    if state.view == View::Tracks {
        render_tracks(frame, state, chunks[2]);
    } else {
        let center_text = if !state.connected {
            if state.status.is_empty() {
                "Conectando con Spotify...".to_string()
            } else {
                format!("Conectando con Spotify...\n{}", state.status)
            }
        } else if track.album.is_empty() {
            String::new()
        } else {
            format!("💿 {}", track.album)
        };
        let center =
            Paragraph::new(center_text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(center, chunks[2]);
    }

    // ---- FOOTER ----
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        "[spc] play [n] sig [p] ant [j/k] mover [→] canciones [←] volver [q] salir",
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(footer, chunks[3]);
}
