use crate::app::App;
use crate::editor::editor::{EditorMode, EditorState};
use crate::syntax::highlight_line_with_tree_sitter;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub fn render_editor(f: &mut Frame, app: &mut App) {
    let theme = app.current_theme();
    
    // Show welcome screen if no file is open
    if app.editor_state.show_welcome && app.editor_state.file_path.is_none() {
        render_welcome_screen(f, &mut app.editor_state, &theme);
        return;
    }
    
    // Main layout: file explorer (optional) + editor + status
    let main_chunks = if app.editor_state.file_explorer_open {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // File explorer
                Constraint::Min(1),     // Editor
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(f.area())
    };
    
    let editor_area = if app.editor_state.file_explorer_open {
        let editor_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1), // Status bar
            ])
            .split(main_chunks[1]);
        
        render_file_explorer(f, &mut app.editor_state, &theme, main_chunks[0]);
        editor_chunks[0]
    } else {
        let editor_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1), // Status bar
            ])
            .split(f.area());
        editor_chunks[0]
    };
    
    render_editor_area(f, &mut app.editor_state, &theme, editor_area);
    
    let status_area = if app.editor_state.file_explorer_open {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(main_chunks[1])[1]
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(f.area())[1]
    };
    
    render_status_bar(f, &app.editor_state, &theme, status_area);
}

fn render_file_explorer(f: &mut Frame, editor: &mut EditorState, theme: &crate::theme::Theme, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Files")
        .style(Style::default().fg(theme.text).bg(theme.bg));
    
    let items: Vec<ListItem> = editor.file_explorer_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let icon = if entry.is_dir { "📁 " } else { "📄 " };
            let name = format!("{}{}", icon, entry.name);
            let style = if idx == editor.file_explorer_selected {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(name).style(style)
        })
        .collect();
    
    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(theme.text).bg(theme.bg));
    
    f.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::default().with_selected(Some(editor.file_explorer_selected)));
}

fn render_editor_area(f: &mut Frame, editor: &mut EditorState, theme: &crate::theme::Theme, area: Rect) {
    let width = area.width as usize;
    let height = area.height as usize;
    
    // Ensure cursor is visible
    editor.ensure_cursor_visible(width, height);
    
    // Layout: line numbers + editor content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(editor.line_numbers_width as u16 + 1),
            Constraint::Min(1),
        ])
        .split(area);
    
    // Render line numbers
    render_line_numbers(f, editor, theme, chunks[0]);
    
    // Render editor content
    render_editor_content(f, editor, theme, chunks[1]);
}

fn render_line_numbers(f: &mut Frame, editor: &EditorState, theme: &crate::theme::Theme, area: Rect) {
    let height = area.height as usize;
    let visible_start = editor.scroll_offset;
    let visible_end = (visible_start + height).min(editor.lines.len());
    
    let mut lines = Vec::new();
    for i in visible_start..visible_end {
        let line_num = i + 1;
        let line_num_str = format!("{:width$}", line_num, width = editor.line_numbers_width);
        
        let style = if i == editor.cursor_row {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Rgb(100, 100, 100))
        };
        
        lines.push(Line::from(Span::styled(line_num_str, style)));
    }
    
    // Fill remaining space
    while lines.len() < height {
        lines.push(Line::from(""));
    }
    
    let block = Block::default()
        .borders(Borders::RIGHT)
        .style(Style::default().fg(Color::Rgb(60, 60, 60)).bg(theme.bg));
    
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Right);
    
    f.render_widget(paragraph, area);
}

fn render_editor_content(f: &mut Frame, editor: &mut EditorState, theme: &crate::theme::Theme, area: Rect) {
    let width = area.width as usize;
    let height = area.height as usize;
    let content_width = width.saturating_sub(1);
    
    let visible_start = editor.scroll_offset;
    let visible_end = (visible_start + height).min(editor.lines.len());
    
    // Detect language from file extension
    let lang = editor
        .file_path
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|ext| ext.to_str())
        .unwrap_or("text");
    
    let mut text_lines = Vec::new();
    
    for i in visible_start..visible_end {
        let line = &editor.lines[i];
        let is_cursor_line = i == editor.cursor_row;
        
        // Get syntax highlighted spans
        let mut spans = if !line.is_empty() {
            highlight_line_with_tree_sitter(line, lang, theme.accent, theme.bg)
        } else {
            vec![Span::raw("")]
        };
        
        // Apply horizontal scrolling
        if editor.horizontal_scroll > 0 && !line.is_empty() {
            let mut display_pos = 0;
            let mut new_spans: Vec<ratatui::text::Span> = Vec::new();
            
            for span in spans.iter() {
                let span_text = span.content.as_ref();
                let span_chars: Vec<char> = span_text.chars().collect();
                
                for ch in span_chars {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                    if display_pos >= editor.horizontal_scroll {
                        if new_spans.is_empty() || new_spans.last().unwrap().style != span.style {
                            new_spans.push(Span::styled(String::new(), span.style));
                        }
                        let last_idx = new_spans.len() - 1;
                        let mut content = new_spans[last_idx].content.to_string();
                        content.push(ch);
                        new_spans[last_idx] = Span::styled(content, span.style);
                    }
                    display_pos += ch_width;
                }
            }
            
            spans = new_spans;
        }
        
        // Truncate to fit width
        let mut final_spans = Vec::new();
        let mut current_width = 0;
        
        for span in spans.iter() {
            let span_text = span.content.as_ref();
            let span_width = span_text.width();
            
            if current_width + span_width <= content_width {
                final_spans.push(span.clone());
                current_width += span_width;
            } else {
                let remaining = content_width.saturating_sub(current_width);
                if remaining > 0 {
                    let truncated: String = span_text.chars().take(remaining).collect();
                    final_spans.push(Span::styled(truncated, span.style));
                }
                break;
            }
        }
        
        // Add cursor if on this line
        if is_cursor_line {
            // Calculate the display width position of the cursor
            // First, calculate how many characters are before the cursor in the original line
            let original_line = &editor.lines[i];
            let cursor_byte_pos = editor.cursor_col.min(original_line.len());
            
            // Count characters and their widths up to cursor position, accounting for horizontal scroll
            let mut display_cursor_width = 0;
            let mut char_idx = 0;
            
            for (byte_idx, ch) in original_line.char_indices() {
                if byte_idx >= cursor_byte_pos {
                    break;
                }
                // Only count characters that are visible (after horizontal scroll)
                if char_idx >= editor.horizontal_scroll {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                    display_cursor_width += ch_width;
                }
                char_idx += 1;
            }
            
            // Now find where to insert cursor in the displayed spans
            let mut accumulated_width = 0;
            let mut cursor_inserted = false;
            let mut new_spans = Vec::new();
            
            for span in final_spans.iter() {
                let span_text = span.content.as_ref();
                let span_width = span_text.width();
                
                if !cursor_inserted && accumulated_width + span_width >= display_cursor_width {
                    // Cursor should be inserted in this span
                    let chars: Vec<char> = span_text.chars().collect();
                    let mut before = String::new();
                    let mut at_cursor = String::new();
                    let mut after = String::new();
                    let mut pos = accumulated_width;
                    
                    for ch in chars {
                        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                        if pos < display_cursor_width {
                            before.push(ch);
                            pos += ch_width;
                        } else if pos == display_cursor_width || (pos < display_cursor_width + ch_width && at_cursor.is_empty()) {
                            // Cursor is at this character
                            at_cursor.push(ch);
                            pos += ch_width;
                        } else {
                            after.push(ch);
                            pos += ch_width;
                        }
                    }
                    
                    if !before.is_empty() {
                        new_spans.push(Span::styled(before, span.style));
                    }
                    
                    // Cursor - make it very visible, especially in normal mode
                    let cursor_char = if at_cursor.is_empty() { 
                        " " 
                    } else { 
                        &at_cursor 
                    };
                    
                    let cursor_style = match editor.mode {
                        EditorMode::Insert => {
                            // Insert mode: thin cursor bar with bright color
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(100, 150, 255))
                                .add_modifier(Modifier::BOLD)
                        }
                        _ => {
                            // Normal mode: block cursor (reversed) - very visible
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(255, 200, 100))
                                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
                        }
                    };
                    new_spans.push(Span::styled(cursor_char.to_string(), cursor_style));
                    
                    if !after.is_empty() {
                        new_spans.push(Span::styled(after, span.style));
                    }
                    
                    cursor_inserted = true;
                } else {
                    new_spans.push(span.clone());
                }
                
                accumulated_width += span_width;
            }
            
            if !cursor_inserted {
                // Cursor at end of line or beyond visible area
                let cursor_style = match editor.mode {
                    EditorMode::Insert => {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(100, 150, 255))
                            .add_modifier(Modifier::BOLD)
                    }
                    _ => {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(255, 200, 100))
                            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
                    }
                };
                new_spans.push(Span::styled(" ", cursor_style));
            }
            
            final_spans = new_spans;
        }
        
        // Pad to full width
        let line_width: usize = final_spans.iter().map(|s| s.content.width()).sum();
        if line_width < content_width {
            final_spans.push(Span::styled(
                " ".repeat(content_width - line_width),
                Style::default().bg(theme.bg),
            ));
        }
        
        text_lines.push(Line::from(final_spans));
    }
    
    // Fill remaining space
    while text_lines.len() < height {
        let empty_line = Line::from(vec![
            Span::styled(" ".repeat(content_width), Style::default().bg(theme.bg))
        ] as Vec<Span>);
        text_lines.push(empty_line);
    }
    
    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().fg(theme.text).bg(theme.bg));
    
    let paragraph = Paragraph::new(text_lines)
        .block(block);
    
    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, editor: &EditorState, theme: &crate::theme::Theme, area: Rect) {
    let mode_text = match editor.mode {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Command => "COMMAND",
        EditorMode::FileExplorer => "FILES",
    };
    
    let file_name = editor
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("[No Name]");
    
    let line_info = format!("Ln {}, Col {}", editor.cursor_row + 1, editor.cursor_col + 1);
    let total_lines = editor.lines.len();
    let file_info = format!("{} lines", total_lines);
    
    let left_text = if editor.show_command {
        format!(":{}", editor.command_buffer)
    } else {
        format!(" {} │ {} │ {} │ {}", mode_text, file_name, line_info, file_info)
    };
    
    let status_style = Style::default()
        .fg(Color::Black)
        .bg(match editor.mode {
            EditorMode::Normal => Color::Rgb(100, 150, 255),
            EditorMode::Insert => Color::Rgb(100, 200, 100),
            EditorMode::Command => Color::Rgb(255, 200, 100),
            EditorMode::FileExplorer => Color::Rgb(200, 150, 255),
        });
    
    let mut status_line = vec![Span::styled(left_text.clone(), status_style)];
    
    // Add status message if present
    if let Some(ref msg) = editor.status_message {
        let msg_span = Span::styled(
            format!(" │ {}", msg),
            status_style,
        );
        status_line.push(msg_span);
    }
    
    // Fill remaining space
    let used_width = left_text.width() + 
        editor.status_message.as_ref().map(|m| m.width() + 3).unwrap_or(0);
    let remaining = area.width.saturating_sub(used_width as u16);
    if remaining > 0 {
        status_line.push(Span::styled(
            " ".repeat(remaining as usize),
            status_style,
        ));
    }
    
    let paragraph = Paragraph::new(Line::from(status_line))
        .block(Block::default().borders(Borders::NONE));
    
    f.render_widget(paragraph, area);
}

fn render_welcome_screen(f: &mut Frame, editor: &mut EditorState, theme: &crate::theme::Theme) {
    let area = f.area();
    
    // Calculate layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Title area
            Constraint::Min(10),    // Menu area
            Constraint::Length(2),  // Status area
        ])
        .split(area);
    
    // Render title "PENGY EDITOR"
    render_welcome_title(f, theme, chunks[0]);
    
    // Render menu
    render_welcome_menu(f, editor, theme, chunks[1]);
    
    // Render status
    render_welcome_status(f, theme, chunks[2]);
}

fn render_welcome_title(f: &mut Frame, theme: &crate::theme::Theme, area: Rect) {
    let title = "PENGY EDITOR";
    let title_lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Modern text editor with AI agent integration",
            Style::default().fg(Color::Rgb(150, 150, 150)),
        )),
    ];
    
    let paragraph = Paragraph::new(title_lines)
        .block(Block::default().borders(Borders::NONE))
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(paragraph, area);
}

fn render_welcome_menu(f: &mut Frame, editor: &mut EditorState, theme: &crate::theme::Theme, area: Rect) {
    let menu_items = vec![
        ("Find File", "f", "Open file explorer to browse and open files"),
        ("New File", "n", "Create a new file"),
        ("Find Text", "g", "Search for text in files (grep)"),
        ("Recent Files", "r", "Open a recently edited file"),
        ("Config", "c", "Open editor configuration"),
        ("Quit", "q", "Exit editor mode"),
    ];
    
    let mut lines = Vec::new();
    
    for (idx, (label, key, desc)) in menu_items.iter().enumerate() {
        let is_selected = idx == editor.welcome_selected;
        
        let checkbox = if is_selected {
            Span::styled("▶ ", Style::default().fg(theme.accent))
        } else {
            Span::styled("  ", Style::default())
        };
        
        let label_span = Span::styled(
            format!("{}", label),
            if is_selected {
                Style::default()
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(180, 180, 180))
            },
        );
        
        let key_span = Span::styled(
            format!(" ({})", key),
            Style::default().fg(Color::Rgb(255, 150, 100)),
        );
        
        let desc_span = Span::styled(
            format!("  {}", desc),
            Style::default().fg(Color::Rgb(120, 120, 120)),
        );
        
        let line = Line::from(vec![checkbox, label_span, key_span, desc_span]);
        lines.push(line);
    }
    
    // Add recent files section if available
    if !editor.recent_files.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Recent Files:",
            Style::default()
                .fg(Color::Rgb(150, 150, 150))
                .add_modifier(Modifier::BOLD),
        )));
        
        for (_path, name) in editor.recent_files.iter().take(5) {
            let file_line = Line::from(Span::styled(
                format!("  • {}", name),
                Style::default().fg(Color::Rgb(180, 180, 180)),
            ));
            lines.push(file_line);
        }
    }
    
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .alignment(ratatui::layout::Alignment::Left);
    
    f.render_widget(paragraph, area);
}

fn render_welcome_status(f: &mut Frame, _theme: &crate::theme::Theme, area: Rect) {
    let status_text = "Press a key to select an option, or type :e <file> to open a file";
    
    let line = Line::from(Span::styled(
        status_text,
        Style::default().fg(Color::Rgb(150, 150, 150)),
    ));
    
    let paragraph = Paragraph::new(line)
        .block(Block::default().borders(Borders::NONE))
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(paragraph, area);
}
