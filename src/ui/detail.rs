//! Dashboard individual de la rola en el centro: header con meta +
//! letra con scroll (j/k). Llega de `section[data-testid="track-page"]`.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::state::AppState;

/// Dibuja el dashboard en `area`. Sin detalle = mensaje de carga/error.
pub fn render_detail(frame: &mut Frame, state: &AppState, area: Rect) {
    let Some(d) = state.detail.as_ref() else {
        let msg = if state.detail_msg.is_empty() {
            "cargando dashboard...".to_string()
        } else {
            state.detail_msg.clone()
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 🎵 dashboard "),
            );
        frame.render_widget(p, area);
        return;
    };

    let title = format!(" 🎵 {} ", d.title);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("▶ ", Style::default().fg(Color::Green)),
            Span::styled(
                d.title.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!("🎤 {}", d.artist),
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "💿 {} · {} · {} · 👁 {}",
                none_si_vacio(&d.album),
                none_si_vacio(&d.year),
                none_si_vacio(&d.duration),
                none_si_vacio(&d.playcount),
            ),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
    ];
    if d.lyrics.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(sin letra en Spotify)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "── letra ──",
            Style::default().fg(Color::DarkGray),
        )]));
        for l in &d.lyrics {
            lines.push(Line::from(vec![Span::styled(
                l.clone(),
                Style::default().fg(Color::Gray),
            )]));
        }
    }

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((state.lyr_scroll, 0));
    frame.render_widget(p, area);
}

fn none_si_vacio(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}
