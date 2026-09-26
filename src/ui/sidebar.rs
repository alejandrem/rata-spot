//! Sidebar IZQUIERDA: Tu biblioteca tal cual del DOM (nombres con
//! emojis/UTF-8 intactos). `j/k` mueven, `Enter` reproduce.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::state::AppState;

/// Dibuja la lista de biblioteca en `area` (stateful por el cursor).
pub fn render_sidebar(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let title = if state.library.is_empty() {
        " 📚 biblioteca ".to_string()
    } else {
        format!(" 📚 biblioteca ({}) ", state.library.len())
    };
    let block = Block::default().borders(Borders::ALL).title(title);

    if state.library.is_empty() {
        let msg = if state.pl_msg.is_empty() {
            "cargando...".to_string()
        } else {
            state.pl_msg.clone()
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = state
        .library
        .iter()
        .map(|it| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", kind_icon(&it.kind)),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(it.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!(" · {}", it.detail.clone()),
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
    frame.render_stateful_widget(list, area, &mut state.pl_state);
}

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "playlist" => "🎵",
        "artist" => "🎤",
        "album" => "💿",
        "show" => "🎙",
        _ => "•",
    }
}
