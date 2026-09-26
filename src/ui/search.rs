//! Buscador del centro: barra SIEMPRE visible + resultados.
//! `/` enfoca para escribir (no revela nada: la barra ya esta ahi),
//! Enter busca, j/k navegan, Enter toca, Esc sale.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::state::AppState;

/// Barra de busqueda (siempre visible, con hint si esta vacia).
pub fn render_search_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let caret = if state.search_active { "▌" } else { "" };
    let input_line = if state.search_query.is_empty() && !state.search_active {
        Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(Color::Green)),
            Span::styled(
                "presiona / para buscar",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(Color::Green)),
            Span::styled(state.search_query.clone(), Style::default().fg(Color::White)),
            Span::styled(caret, Style::default().fg(Color::Green)),
        ])
    };
    let input = Paragraph::new(input_line).block(
        Block::default().borders(Borders::ALL).title(
            if state.search_active {
                " buscar (Enter = ir, Esc = salir) "
            } else {
                " buscar (/ = escribir) "
            },
        ),
    );
    frame.render_widget(input, area);
}

/// Lista de resultados en `area` (stateful por cursor).
pub fn render_results(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let title = if state.results.is_empty() {
        if state.search_query.is_empty() {
            " resultados ".to_string()
        } else {
            format!(" 🔍 \"{}\" ", state.search_query)
        }
    } else if state.search_query.is_empty() {
        format!(" resultados ({}) ", state.results.len())
    } else {
        format!(" 🔍 \"{}\" ({}) ", state.search_query, state.results.len())
    };
    let block = Block::default().borders(Borders::ALL).title(title);

    if state.results.is_empty() {
        let msg = if state.sr_msg.is_empty() {
            "escribe y dale Enter owo".to_string()
        } else {
            state.sr_msg.clone()
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = state
        .results
        .iter()
        .map(|it| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", super::sidebar::kind_icon(&it.kind)),
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
    frame.render_stateful_widget(list, area, &mut state.sr_state);
}
