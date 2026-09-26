//! Interfaz TUI con Ratatui (Fase 4).
//!
//! Layout de 4 zonas verticales:
//!   HEADER   3 lineas — titulo/artista
//!   PROGRESO 3 lineas — Gauge 1:23 / 3:45
//!   CENTRO   flexible — estado / album
//!   FOOTER   1 linea  — keybindings

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::gsmtc::{ProgressSmoother, TrackInfo};

/// Estado que dibuja la TUI. El smoother ya guarda
/// last_position + last_timestamp para el progreso suave (Fase 3.1).
pub struct AppState {
    pub smoother: ProgressSmoother,
    /// false mientras GSMTC aun no registra sesion Brave.
    pub connected: bool,
    /// Diagnostico visible en la TUI (qué Brave lanzamos / PID / URL).
    /// Los println! de antes quedaban ocultos bajo la pantalla alternativa.
    pub status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            smoother: ProgressSmoother::new(),
            connected: false,
            status: String::new(),
        }
    }

    /// Track a mostrar (con posicion/progreso interpolados).
    pub fn display(&self) -> TrackInfo {
        self.smoother.display_track()
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // progreso
            Constraint::Min(1),    // centro flexible
            Constraint::Length(1), // footer
        ])
        .split(frame.size());

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

    // ---- CENTRO ----
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
    let center = Paragraph::new(center_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(center, chunks[2]);

    // ---- FOOTER ----
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        "[space] play/pause  [n] next  [p] prev  [q] quit",
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(footer, chunks[3]);
}
