use crate::app::App;
use crate::theme::THEMES;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn render_theme_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);

    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Themes")
        .title_style(Style::default().fg(Color::White));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search
            Constraint::Min(10),    // List
            Constraint::Length(1),  // Hint
        ])
        .split(inner);

    // Search
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.theme_search_focused {
            "Search (active)"
        } else {
            "Search"
        })
        .title_style(if app.theme_search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });

    let search_text = if app.theme_search_query.is_empty() {
        "Type to search themes...".to_string()
    } else {
        app.theme_search_query.clone()
    };

    let search_para = Paragraph::new(search_text)
        .block(search_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(search_para, layout[0]);

    // Filtered list
    let filtered: Vec<(usize, &str)> = THEMES
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            if app.theme_search_query.is_empty() {
                true
            } else {
                t.name
                    .to_lowercase()
                    .contains(&app.theme_search_query.to_lowercase())
            }
        })
        .map(|(i, t)| (i, t.name))
        .collect();

    if filtered.is_empty() {
        let empty = Paragraph::new("No themes found").style(Style::default().fg(Color::Gray));
        f.render_widget(empty, layout[1]);
    } else {
        if let Some(sel) = app.theme_list_state.selected() {
            if sel >= filtered.len() {
                app.theme_list_state.select(Some(filtered.len().saturating_sub(1)));
            }
        } else {
            app.theme_list_state.select(Some(0));
        }

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(_, name)| {
                ListItem::new(Span::styled(
                    *name,
                    Style::default().fg(Color::White),
                ))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Themes"))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list, layout[1], &mut app.theme_list_state);
    }

    let hint = Paragraph::new("Enter: apply  •  Tab: search  •  Esc: back")
        .style(Style::default().fg(Color::Gray));
    f.render_widget(hint, layout[2]);
}


