use crate::app::{AgentType, App, AppState, ChatMessage, ModelOption, ToolStatus};
use crate::constants::{DEFAULT_BASE_URL, VERSION};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
    },
};
use std::collections::HashSet;

fn wrap_to_width(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.len() >= max {
            lines.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn agent_accent(agent: AgentType) -> Color {
    match agent {
        AgentType::Coder => Color::Rgb(92, 136, 255),
        AgentType::CodeResearcher => Color::Rgb(80, 190, 200),
        AgentType::TestAgent => Color::Rgb(120, 200, 120),
        AgentType::PengyAgent => Color::Rgb(200, 120, 220),
        AgentType::ControlAgent => Color::Rgb(230, 200, 120),
        AgentType::IssueAgent => Color::Rgb(240, 120, 120),
    }
}

fn keyword_set(lang: &str) -> HashSet<&'static str> {
    let mut set = HashSet::new();
    let keywords = match lang.to_lowercase().as_str() {
        "rust" => vec![
            "fn", "let", "mut", "pub", "struct", "impl", "trait", "enum", "match", "use", "mod",
            "ref", "if", "else", "loop", "for", "while", "in", "move", "return", "async", "await",
        ],
        "python" => vec![
            "def", "class", "import", "from", "return", "if", "elif", "else", "for", "while", "in",
            "with", "as", "lambda", "yield", "async", "await",
        ],
        "typescript" | "javascript" | "ts" | "js" => vec![
            "function", "const", "let", "var", "import", "from", "export", "return", "if", "else",
            "for", "while", "async", "await", "class", "extends",
        ],
        "ocaml" | "ml" => vec![
            "let", "in", "rec", "type", "module", "match", "with", "open", "fun", "if", "then",
            "else",
        ],
        _ => vec![
            "let", "fn", "function", "const", "return", "if", "else", "for", "while", "match",
            "class", "struct", "impl", "module", "type",
        ],
    };
    for kw in keywords {
        set.insert(kw);
    }
    set
}

fn style_token(token: &str, lang: &str, accent: Color, bg: Color) -> Span<'static> {
    let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    let keywords = keyword_set(lang);
    let base_style = Style::default().bg(bg).fg(Color::White);

    if token.trim_start().starts_with("//") || token.trim_start().starts_with("#") {
        Span::styled(token.to_string(), base_style.fg(Color::Gray))
    } else if token.starts_with('"') || token.starts_with('\'') {
        Span::styled(token.to_string(), base_style.fg(Color::LightGreen))
    } else if keywords.contains(clean) {
        Span::styled(
            token.to_string(),
            base_style.fg(accent).add_modifier(Modifier::BOLD),
        )
    } else if token.chars().all(|c| c.is_ascii_digit()) {
        Span::styled(token.to_string(), base_style.fg(Color::Cyan))
    } else {
        Span::styled(token.to_string(), base_style.fg(Color::White))
    }
}

fn highlight_code_line(line: &str, lang: &str, accent: Color) -> Line<'static> {
    let bg = Color::Rgb(25, 25, 25);
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled("  ", Style::default().bg(bg)));

    let mut current = String::new();
    for ch in line.chars() {
        if ch.is_whitespace() {
            if !current.is_empty() {
                spans.push(style_token(&current, lang, accent, bg));
                current.clear();
            }
            spans.push(Span::styled(ch.to_string(), Style::default().bg(bg).fg(Color::Gray)));
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        spans.push(style_token(&current, lang, accent, bg));
    }

    Line::from(spans)
}

fn render_markdown_with_code(content: &str, accent: Color) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::raw("")));

    let mut in_code_block = false;
    let mut lang = String::new();

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    "  ",
                    Style::default()
                        .bg(Color::Rgb(25, 25, 25))
                        .fg(Color::Gray),
                )));
            }
            in_code_block = !in_code_block;
            lang = line.trim_start().trim_matches('`').to_string();
            continue;
        }

        if in_code_block {
            lines.push(highlight_code_line(line, &lang, accent));
        } else {
            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(accent)),
                Span::styled(line.to_string(), Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(Span::raw("")));
    lines
}

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(f.area());

    render_header(f, app, layout[0]);

    match app.state {
        AppState::Welcome => {
            render_welcome(f, app, layout[1]);
        }
        AppState::Chat => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(85), Constraint::Length(25)])
                .split(layout[1]);
            render_messages(f, app, main_chunks[0]);
            render_chat_sidebar(f, app, main_chunks[1]);
        }
        AppState::SessionSelector => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);
            render_messages(f, app, main_chunks[0]);
            render_session_selector(f, app, main_chunks[1]);
        }
        _ => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);

            render_messages(f, app, main_chunks[0]);

            match app.state {
                AppState::ModelSelector => render_model_selector(f, app, main_chunks[1]),
                AppState::AgentSelector => render_agent_selector(f, app, main_chunks[1]),
                AppState::Settings => render_settings(f, app, main_chunks[1]),
                AppState::Help => render_help(f, app, main_chunks[1]),
                AppState::CustomModel => render_custom_model(f, app, main_chunks[1]),
                AppState::BaseUrlSelector => render_baseurl_selector(f, app, main_chunks[1]),
                AppState::SessionSelector | AppState::Chat | AppState::Welcome => unreachable!(),
            }
        }
    }

    let input_area = match app.state {
        AppState::Welcome => centered_rect(80, 10, layout[1]),
        AppState::CustomModel => Rect::default(),
        _ => layout[2],
    };

    if (app.state == AppState::Welcome || app.state == AppState::Chat)
        && app.show_command_hints
        && app.chat_input.starts_with('/')
    {
        render_command_hints(f, app, input_area);
    }

    match app.state {
        AppState::Welcome => {
            render_input(f, app, input_area);
        }
        AppState::CustomModel => {}
        _ => {
            render_input(f, app, input_area);
        }
    }

    render_status_bar(f, app, layout[3]);
}

fn parse_tool_args(args: &str) -> Option<serde_json::Value> {
    serde_json::from_str(args).ok()
}

fn format_tool_args_display(name: &str, args: &str) -> Vec<Line<'static>> {
    let parsed = parse_tool_args(args);

    if let Some(json) = &parsed {
        let mut result = Vec::new();
        match name {
            "file_manager" => {
                let op = json
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let path = json
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content = json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                result.push(Line::from(vec![
                    Span::styled("  Op: ", Style::default().fg(Color::Gray)),
                    Span::styled(op.to_string(), Style::default().fg(Color::White)),
                ]));
                if !path.is_empty() {
                    result.push(Line::from(vec![
                        Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
                if !content.is_empty() {
                    result.push(Line::from(vec![
                        Span::styled("  Content:", Style::default().fg(Color::Gray)),
                    ]));
                    for line in content.lines().take(8) {
                        let preview = if line.len() > 160 {
                            format!("{}…", &line[..159])
                        } else {
                            line.to_string()
                        };
                        result.push(Line::from(vec![
                            Span::styled(
                                format!("    {}", preview),
                                Style::default()
                                    .fg(Color::White)
                                    .bg(Color::Rgb(20, 20, 20)),
                            ),
                        ]));
                    }
                }
            }
            "todo" => {
                if let Some(action_str) = json.get("action").and_then(|v| v.as_str()) {
                    let action = action_str.to_string();
                    let op = json
                        .get("operation")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let desc = json
                        .get("task_description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let id = json.get("task_id").and_then(|v| v.as_u64());

                    result.push(Line::from(vec![
                        Span::styled("  Action: ", Style::default().fg(Color::Gray)),
                        Span::styled(action, Style::default().fg(Color::White)),
                    ]));
                    if let Some(op) = op {
                        result.push(Line::from(vec![
                            Span::styled("  Operation: ", Style::default().fg(Color::Gray)),
                            Span::styled(op, Style::default().fg(Color::White)),
                        ]));
                    }
                    if let Some(desc) = desc {
                        let truncated = if desc.len() > 60 {
                            format!("{}...", &desc[..60])
                        } else {
                            desc
                        };
                        result.push(Line::from(vec![
                            Span::styled("  Task: ", Style::default().fg(Color::Gray)),
                            Span::styled(truncated, Style::default().fg(Color::White)),
                        ]));
                    }
                    if let Some(id) = id {
                        result.push(Line::from(vec![
                            Span::styled("  Task ID: ", Style::default().fg(Color::Gray)),
                            Span::styled(id.to_string(), Style::default().fg(Color::White)),
                        ]));
                    }
                }
            }
            "edit" => {
                if let (Some(file_path), Some(old_string), Some(new_string)) = (
                    json.get("filePath")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    json.get("oldString")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    json.get("newString")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ) {
                    result.push(Line::from(vec![
                        Span::styled("  File: ", Style::default().fg(Color::Gray)),
                        Span::styled(file_path.clone(), Style::default().fg(Color::White)),
                    ]));

                    let old_lines: Vec<String> =
                        old_string.lines().map(|s| s.to_string()).collect();
                    let new_lines: Vec<String> =
                        new_string.lines().map(|s| s.to_string()).collect();

                    let max_lines = old_lines.len().max(new_lines.len());
                    let max_display_lines = 25.min(max_lines);

                    let left_width: usize = 52;
                    let right_width: usize = 52;

                    let truncate_to = |s: &str, width: usize| -> String {
                        if s.len() > width {
                            format!("{}…", &s[..width.saturating_sub(1)])
                        } else {
                            s.to_string()
                        }
                    };

                    for idx in 0..max_display_lines {
                        let old_line_owned = old_lines
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| "".to_string());
                        let new_line_owned = new_lines
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| "".to_string());

                        let line_num = idx + 1;

                        let old_display =
                            truncate_to(&old_line_owned, left_width.saturating_sub(6));
                        let new_display =
                            truncate_to(&new_line_owned, right_width.saturating_sub(6));

                        let left_col = format!("{:>4} {}", line_num, old_display);
                        let right_col = format!("{:>4} {}", line_num, new_display);

                        let padded_left = format!("{:<width$}", left_col, width = left_width);
                        let padded_right = format!("{:<width$}", right_col, width = right_width);

                        if old_line_owned != new_line_owned {
                            result.push(Line::from(vec![
                                Span::styled(
                                    padded_left.clone(),
                                    Style::default().fg(Color::White).bg(Color::Rgb(40, 20, 20)),
                                ),
                                Span::styled(" │ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                                Span::styled(
                                    padded_right.clone(),
                                    Style::default()
                                        .fg(Color::White)
                                        .bg(Color::Rgb(20, 100, 20)),
                                ),
                            ]));
                        } else if !new_line_owned.is_empty() {
                            result.push(Line::from(vec![Span::styled(
                                format!(
                                    "{:<width$}",
                                    format!("{:>4} {}", line_num, new_display),
                                    width = left_width + 3 + right_width
                                ),
                                Style::default().fg(Color::White),
                            )]));
                        }
                    }

                    if max_lines > max_display_lines {
                        result.push(Line::from(Span::styled(
                            format!("  ... {} more lines", max_lines - max_display_lines),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
            }
            "file_manager" => {
                if let Some(path) = json
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    result.push(Line::from(vec![
                        Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.clone(), Style::default().fg(Color::White)),
                    ]));
                    if let Some(kind) = json
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                    {
                        result.push(Line::from(vec![
                            Span::styled("  Kind: ", Style::default().fg(Color::Gray)),
                            Span::styled(kind, Style::default().fg(Color::White)),
                        ]));
                    }
                    if let Some(start) = json.get("startLine").and_then(|v| v.as_u64()) {
                        let end = json
                            .get("endLine")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(start);
                        result.push(Line::from(vec![
                            Span::styled("  Range: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                format!("{}-{}", start, end),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                    }
                }
            }
            "read_file" => {
                if let Some(path) = json
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    result.push(Line::from(vec![
                        Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.clone(), Style::default().fg(Color::White)),
                    ]));
                }
            }
            "grep" => {
                if let Some(pattern) = json
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    result.push(Line::from(vec![
                        Span::styled("  Pattern: ", Style::default().fg(Color::Gray)),
                        Span::styled(pattern.clone(), Style::default().fg(Color::White)),
                    ]));
                }
                if let Some(path) = json
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    result.push(Line::from(vec![
                        Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.clone(), Style::default().fg(Color::White)),
                    ]));
                }
            }
            _ => {
                for seg in wrap_to_width(&format!("Args: {}", args), 80) {
                    result.push(Line::from(Span::styled(
                        seg,
                        Style::default().fg(Color::Gray),
                    )));
                }
            }
        }

        return result;
    }

    wrap_to_width(&format!("Args: {}", args), 80)
        .into_iter()
        .map(|seg| Line::from(Span::styled(seg, Style::default().fg(Color::Gray))))
        .collect()
}

fn render_tool_call_card(
    name: &str,
    args: &str,
    result: &Option<String>,
    status: &ToolStatus,
) -> ListItem<'static> {
    let mut lines = Vec::new();

    if let ToolStatus::Error = status {
        lines.push(Line::from(vec![
            Span::styled("[ERR]", Style::default().fg(Color::Red)),
            Span::styled(" Tool: ", Style::default().fg(Color::Gray)),
            Span::styled(name.to_string(), Style::default().fg(Color::White)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Tool: ", Style::default().fg(Color::Gray)),
            Span::styled(name.to_string(), Style::default().fg(Color::White)),
        ]));
    }

    if !args.is_empty() {
        let arg_lines = format_tool_args_display(name, args);
        lines.extend(arg_lines);
    }

    if let Some(res) = result {
        // Make bash outputs readable: turn escaped newlines into real newlines.
        let normalized = res.replace("\\r\\n", "\n").replace("\\n", "\n");

        // Wrap per line to keep columns tidy.
        let mut wrapped_lines: Vec<String> = Vec::new();
        for line in normalized.lines() {
            let segments = wrap_to_width(line, 80);
            wrapped_lines.extend(segments);
        }

        if wrapped_lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Result: ", Style::default().fg(Color::Gray)),
                Span::styled("", Style::default().fg(Color::Gray)),
            ]));
        } else {
            for (idx, seg) in wrapped_lines.iter().enumerate() {
                if idx == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("  Result: ", Style::default().fg(Color::Gray)),
                        Span::styled(seg.clone(), Style::default().fg(Color::White)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("           ", Style::default().fg(Color::Gray)),
                        Span::styled(seg.clone(), Style::default().fg(Color::White)),
                    ]));
                }
            }
        }
    }

    let card_bg = match status {
        ToolStatus::Running => Color::Rgb(32, 32, 32),
        ToolStatus::Success => Color::Rgb(28, 28, 28), // keep neutral, avoid big green blocks
        ToolStatus::Error => Color::Rgb(40, 24, 24),
    };

    ListItem::new(lines).style(Style::default().bg(card_bg))
}

fn render_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let accent = agent_accent(app.selected_agent);
    let messages: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| match msg {
            ChatMessage::User(content) => {
                let user_lines = vec![
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::Gray)),
                        Span::styled(content.clone(), Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![Span::raw("")]),
                ];
                ListItem::new(user_lines)
            }
            ChatMessage::Assistant(content) => {
                let mut assistant_lines = render_markdown_with_code(content, accent);
                assistant_lines.insert(0, Line::from(Span::raw("")));
                assistant_lines.push(Line::from(Span::raw("")));
                ListItem::new(assistant_lines)
            }
            ChatMessage::ToolCall {
                name,
                args,
                result,
                status,
                ..
            } => render_tool_call_card(name, args, result, status),
            ChatMessage::Thinking(content) => ListItem::new(Line::from(vec![
                Span::styled(
                    content.clone(),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ])),
            ChatMessage::Error(err) => ListItem::new(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err.clone(), Style::default().fg(Color::Red)),
            ])),
        })
        .collect();

    let messages_len = messages.len();

    if !app.user_scrolled && messages_len > 0 {
        app.list_state.select(Some(messages_len.saturating_sub(1)));
    } else if let Some(selected) = app.list_state.selected() {
        if selected >= messages_len && messages_len > 0 {
            app.list_state.select(Some(messages_len.saturating_sub(1)));
        }
    } else if messages_len > 0 {
        app.list_state.select(Some(messages_len.saturating_sub(1)));
    }

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)));

    f.render_stateful_widget(messages_list, area, &mut app.list_state);

    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scroll_state = app.scroll_state;
    let selected_idx = app.list_state.selected().unwrap_or(0);
    scroll_state = scroll_state
        .content_length(app.chat_messages.len())
        .position(selected_idx.saturating_sub(1));
    f.render_stateful_widget(scrollbar, area, &mut scroll_state);
    app.scroll_state = scroll_state;
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let state = format!("{:?}", app.state);
    let title = format!(" Pengy Agent {} │ State: {} ", VERSION, state);
    let accent = agent_accent(app.selected_agent);
    let header = Paragraph::new(vec![
        Line::from(title),
        Line::from(""),
    ])
    .style(
        Style::default()
            .fg(Color::White)
            .bg(accent)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, area);
}

fn render_input(f: &mut Frame, app: &mut App, area: Rect) {
    let input_bg_color = Color::Rgb(30, 30, 30);
    let accent = agent_accent(app.selected_agent);
    let gutter_width: u16 = 2; // "│ "

    let bg_block = Block::default().style(Style::default().bg(input_bg_color));
    f.render_widget(bg_block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let prompt = "> ";

    let available_width = inner_area
        .width
        .saturating_sub(gutter_width + prompt.len() as u16);

    let input_text = &app.chat_input;
    let mut wrapped_lines = Vec::new();
    let mut current_line = String::new();

    for ch in input_text.chars() {
        if current_line.len() as u16 >= available_width {
            wrapped_lines.push(current_line);
            current_line = ch.to_string();
        } else {
            current_line.push(ch);
        }
    }
    if !current_line.is_empty() || wrapped_lines.is_empty() {
        wrapped_lines.push(current_line);
    }

    let mut input_content = Vec::new();

    let content_lines = wrapped_lines.len();
    let available_height = inner_area.height as usize;

    let top_padding = available_height.saturating_sub(content_lines) / 2;
    for _ in 0..top_padding {
        input_content.push(Line::from(vec![Span::raw("")]));
    }

    for (i, line) in wrapped_lines.iter().enumerate() {
        let prefix = if i == 0 { prompt } else { "  " };
        input_content.push(Line::from(vec![
            Span::styled(
                "│ ",
                Style::default()
                    .fg(accent)
                    .bg(input_bg_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                prefix.to_string(),
                Style::default()
                    .fg(accent)
                    .bg(input_bg_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                line.clone(),
                Style::default()
                    .fg(Color::White)
                    .bg(input_bg_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        if i < wrapped_lines.len() - 1 && top_padding > 0 {}
    }

    let input_paragraph = Paragraph::new(input_content);
    f.render_widget(input_paragraph, inner_area);

    let mut char_count = 0;
    let mut cursor_line = 0;
    let mut cursor_col = 0;

    for (line_idx, line) in wrapped_lines.iter().enumerate() {
        if char_count + line.len() >= app.input_cursor {
            cursor_line = line_idx;
            cursor_col = app.input_cursor - char_count;
            break;
        }
        char_count += line.len();
    }

    if cursor_line >= wrapped_lines.len() && !wrapped_lines.is_empty() {
        cursor_line = wrapped_lines.len() - 1;
        cursor_col = wrapped_lines[cursor_line].len();
    }

    let prefix_len = if cursor_line == 0 { prompt.len() } else { 2 };
    let cursor_x = (inner_area.x + gutter_width + prefix_len as u16 + cursor_col as u16)
        .min(inner_area.x + inner_area.width.saturating_sub(1));
    let cursor_y = inner_area.y + top_padding as u16 + cursor_line as u16;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let model_name = app
        .selected_model
        .as_ref()
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "None".to_string());
    let agent_name = format!("{:?}", app.selected_agent);
    let loading = if app.loading {
        "● Running"
    } else {
        "● Idle"
    };
    let accent = agent_accent(app.selected_agent);
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} │ Model: {} │ ", cwd, model_name),
            Style::default().fg(Color::Rgb(170, 170, 170)),
        ),
        Span::styled(
            format!("Agent: {}", agent_name),
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" │ {}", loading),
            Style::default().fg(Color::Rgb(170, 170, 170)),
        ),
    ]);
    let status = Paragraph::new(vec![status_line, Line::from("")])
        .style(Style::default().bg(Color::Rgb(10, 10, 10)));
    f.render_widget(status, area);
}

fn render_chat_panel(f: &mut Frame, app: &App, area: Rect) {
    let commands = vec![
        "/models - Select Model",
        "/agents - Select Agent",
        "/settings - API key / model / base URL",
        "/help - Help",
        "/clear - Reset conversation",
    ];
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Shortcuts",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    for c in commands {
        lines.push(Line::from(Span::styled(
            c,
            Style::default().fg(Color::Gray),
        )));
    }

    if app.loading {
        lines.push(Line::from(Span::styled(
            "Status: running...",
            Style::default().fg(Color::Yellow),
        )));
    }

    let block = Block::default().borders(Borders::ALL).title("Panel");
    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

fn render_chat_sidebar(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let title = Paragraph::new("Modified Files").style(
        Style::default()
            .fg(Color::Rgb(200, 200, 200))
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(title, vertical[0]);

    if !app.modified_files.is_empty() {
        let mut file_items: Vec<Line> = Vec::new();
        for (file_path, (added, removed)) in app.modified_files.iter() {
            let file_name = file_path
                .rsplit('/')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_path.clone());
            file_items.push(Line::from(vec![
                Span::styled(file_name, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(
                    format!("+{}", added),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("-{}", removed),
                    Style::default().fg(Color::LightRed),
                ),
            ]));
        }

        let file_block = Block::default().borders(Borders::ALL).title("Files");
        let file_list = List::new(file_items).block(file_block);
        f.render_widget(file_list, vertical[1]);
    } else {
        let empty_block = Block::default().borders(Borders::ALL).title("Files");
        let empty = Paragraph::new("No edits yet").block(empty_block);
        f.render_widget(empty, vertical[1]);
    }

    let session_block = Block::default().borders(Borders::ALL).title("Context");
    let mut context_lines: Vec<Line> = Vec::new();
    context_lines.push(Line::from(vec![
        Span::styled("Session: ", Style::default().fg(Color::Gray)),
        Span::styled(
            app.sessions
                .get(app.current_session)
                .cloned()
                .unwrap_or_else(|| "New session - default".to_string()),
            Style::default().fg(Color::White),
        ),
    ]));
    context_lines.push(Line::from(vec![
        Span::styled("State: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:?}", app.state),
            Style::default().fg(Color::White),
        ),
    ]));

    let context_para = Paragraph::new(context_lines).block(session_block);
    f.render_widget(context_para, vertical[3]);
}

fn render_welcome(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let panel = centered_rect(80, 70, area);
    let block = Block::default().borders(Borders::NONE);
    let content_area = block.inner(panel);
    f.render_widget(block, panel);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(content_area);

    let logo_paragraph = Paragraph::new(app.logo.clone())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    f.render_widget(logo_paragraph, chunks[0]);

    let info = vec![Line::from("Type your prompt to start chatting.")];
    let info_block = Block::default()
        .borders(Borders::NONE)
        .title("Getting Started");
    let info_para = Paragraph::new(info)
        .block(info_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(info_para, chunks[1]);
}

fn render_command_hints(f: &mut Frame, app: &App, area: Rect) {
    if app.chat_input.is_empty() || !app.chat_input.starts_with('/') {
        return;
    }

    let hints = app.get_command_hints();
    let query = app.chat_input.to_lowercase();
    let filtered: Vec<(&str, &str)> = hints
        .into_iter()
        .filter(|(cmd, _)| cmd.to_lowercase().starts_with(&query))
        .collect();

    if filtered.is_empty() {
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(cmd, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    *cmd,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - "),
                Span::styled(*desc, Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Commands")
        .title_alignment(Alignment::Left)
        .border_type(ratatui::widgets::BorderType::Rounded);
    let list = List::new(items).block(block);

    let hint_area = Rect {
        x: area.x,
        y: area.y.saturating_sub(10),
        width: area.width,
        height: 10.min(area.y + area.height),
    };

    f.render_widget(Clear, hint_area);
    f.render_widget(list, hint_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let middle = popup_layout[1];

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(middle)[1]
}

fn render_model_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);

    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Select Model");
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.model_search_focused {
            "Search (active)"
        } else {
            "Search"
        })
        .title_style(if app.model_search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });

    let search_text = if app.search_query.is_empty() {
        "Type to search models...".to_string()
    } else {
        app.search_query.clone()
    };

    let search_style = if app.search_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_para = Paragraph::new(search_text)
        .block(search_block)
        .style(search_style);
    f.render_widget(search_para, layout[0]);

    let all_models = App::get_available_models();
    let filtered_models: Vec<&ModelOption> = if app.search_query.is_empty() {
        all_models.iter().collect()
    } else {
        let query_lower = app.search_query.to_lowercase();
        all_models
            .iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.provider.to_lowercase().contains(&query_lower)
                    || m.base_url.to_lowercase().contains(&query_lower)
            })
            .collect()
    };

    if let Some(selected) = app.model_list_state.selected() {
        if selected >= filtered_models.len() {
            app.model_list_state
                .select(Some(0.max(filtered_models.len().saturating_sub(1))));
        }
    } else if !filtered_models.is_empty() {
        if let Some(ref selected_model) = app.selected_model {
            if let Some(idx) = filtered_models.iter().position(|m| {
                m.name == selected_model.name && m.provider == selected_model.provider
            }) {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    }

    let items: Vec<ListItem> = filtered_models
        .iter()
        .map(|m| {
            let is_selected = app
                .selected_model
                .as_ref()
                .map(|sm| sm.name == m.name && sm.provider == m.provider)
                .unwrap_or(false);

            let caption = if m.provider == "Custom" {
                format!("{} (custom)", m.name)
            } else {
                format!("{} - {}", m.name, m.provider)
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(if is_selected { "✓ " } else { "  " }, style),
                Span::styled(caption, style),
            ]))
        })
        .collect();

    let model_block = Block::default()
        .borders(Borders::ALL)
        .title("Models (↑/↓ to select)");
    let model_list = List::new(items).block(model_block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Rgb(50, 50, 50))
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(model_list, layout[1], &mut app.model_list_state);

    let summary = {
        let current = app
            .selected_model
            .as_ref()
            .map(|m| format!("{} ({})", m.name, m.provider))
            .unwrap_or_else(|| "None selected".to_string());
        let base = truncate(
            &app.selected_model
                .as_ref()
                .map(|m| m.base_url.clone())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            48,
        );
        format!("Selected: {}  |  Base: {}", current, base)
    };
    let summary_para = Paragraph::new(summary)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(summary_para, layout[2]);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

fn render_agent_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Select Agent");
    let agents = App::get_available_agents();
    let items: Vec<ListItem> = agents
        .iter()
        .map(|(name, desc, agent_type)| {
            let is_selected = *agent_type == app.selected_agent;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(if is_selected { "✓ " } else { "  " }, style),
                Span::styled(name.to_string(), style),
                Span::styled(" - ", Style::default().fg(Color::Gray)),
                Span::styled(desc.to_string(), Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Rgb(50, 50, 50))
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, rect, &mut app.agent_list_state);
}

fn render_settings(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 70, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Settings");
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let api_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.settings_field == 0 {
            "API Key (active)"
        } else {
            "API Key"
        });
    let masked_key = if app.settings_api_key.is_empty() {
        "<not set>".to_string()
    } else {
        let visible_tail: String = app
            .settings_api_key
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let hidden_len = app
            .settings_api_key
            .len()
            .saturating_sub(visible_tail.len());
        format!("{}{}", "*".repeat(hidden_len), visible_tail)
    };
    let api_para = Paragraph::new(masked_key.clone())
        .block(api_block)
        .wrap(Wrap { trim: true });
    f.render_widget(api_para, layout[0]);

    let url_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.settings_field == 1 {
            "Base URL (active)"
        } else {
            "Base URL"
        });
    let base_url = if app.settings_base_url.is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        app.settings_base_url.clone()
    };
    let url_para = Paragraph::new(truncate(&base_url, 64))
        .block(url_block)
        .wrap(Wrap { trim: true });
    f.render_widget(url_para, layout[1]);

    let models = App::get_available_models();

    if let Some(selected) = app.model_list_state.selected() {
        if selected >= models.len() {
            app.model_list_state.select(Some(0));
        }
    } else if !models.is_empty() {
        if let Some(ref selected_model) = app.selected_model {
            if let Some(idx) = models.iter().position(|m| {
                m.name == selected_model.name && m.provider == selected_model.provider
            }) {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    }

    let items: Vec<ListItem> = models
        .iter()
        .map(|m| {
            let is_selected = app
                .selected_model
                .as_ref()
                .map(|sm| sm.name == m.name && sm.provider == m.provider)
                .unwrap_or(false);

            let caption = if m.provider == "Custom" {
                format!("{} (custom)", m.name)
            } else if m.name.starts_with("Provider:") {
                format!("{} → {}", m.name, m.base_url)
            } else {
                format!("{} - {}", m.name, m.provider)
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(if is_selected { "✓ " } else { "  " }, style),
                Span::styled(caption, style),
            ]))
        })
        .collect();

    let model_block = Block::default()
        .borders(Borders::ALL)
        .title("Models (↑/↓ to select)");
    let model_list = List::new(items).block(model_block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Rgb(50, 50, 50))
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(model_list, layout[2], &mut app.model_list_state);

    let summary = {
        let current = app
            .selected_model
            .as_ref()
            .map(|m| format!("{} ({})", m.name, m.provider))
            .unwrap_or_else(|| "None selected".to_string());
        let base = truncate(&base_url, 48);
        format!("Selected: {}  |  Base: {}", current, base)
    };
    let summary_para = Paragraph::new(summary)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(summary_para, layout[3]);

    let mut footer_lines: Vec<Line> = vec![Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw("/"),
        Span::styled("Shift+Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" move fields  "),
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::raw(" models  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" save  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" close"),
    ])];

    if let Some(err) = &app.error {
        footer_lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let footer = Paragraph::new(footer_lines).wrap(Wrap { trim: true });
    f.render_widget(footer, layout[4]);

    if app.settings_field == 0 {
        let cursor_x = (layout[0].x + 1 + masked_key.len() as u16)
            .min(layout[0].x + layout[0].width.saturating_sub(1));
        let cursor_y = layout[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    } else if app.settings_field == 1 {
        let cursor_x = (layout[1].x + 1 + base_url.len() as u16)
            .min(layout[1].x + layout[1].width.saturating_sub(1));
        let cursor_y = layout[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_session_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Sessions (hjkl/↑↓)");
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let marker = if i == app.current_session { "●" } else { " " };
            ListItem::new(format!("{} {}", marker, s))
        })
        .collect();
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, rect, &mut app.session_list_state);
}

fn render_help(f: &mut Frame, _app: &App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let text = "Available Commands:\n\n/models - Select Model\n/agents - Select Agent\n/settings - Configure API key / model / base URL\n/baseurl - Select provider base URL (Mistral, DeepSeek, OpenRouter, etc.)\n/help - Show this help screen\n/clear - Clear conversation and reset agent\n\nNavigation:\nUse Arrows to navigate lists.\nTab to switch between fields/agents.\nEnter to select.\nEsc to go back.\n\nTip: Type '/' in the input to see all available commands with autocomplete.";
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, rect);
}

fn render_baseurl_selector(f: &mut Frame, app: &mut App, area: Rect) {
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
        .title("Select Provider Base URL")
        .title_style(Style::default().fg(Color::White));

    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.model_search_focused {
            "Search (active)"
        } else {
            "Search"
        })
        .title_style(if app.model_search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });

    let search_text = if app.search_query.is_empty() {
        "Type to search providers...".to_string()
    } else {
        app.search_query.clone()
    };

    let search_style = if app.search_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_para = Paragraph::new(search_text)
        .block(search_block)
        .style(search_style);
    f.render_widget(search_para, layout[0]);

    let all_models = App::get_available_models();
    let provider_models: Vec<&ModelOption> = all_models
        .iter()
        .filter(|m| m.name.starts_with("Provider:"))
        .collect();

    let filtered_providers: Vec<&ModelOption> = if app.search_query.is_empty() {
        provider_models
    } else {
        let query_lower = app.search_query.to_lowercase();
        provider_models
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.provider.to_lowercase().contains(&query_lower)
                    || m.base_url.to_lowercase().contains(&query_lower)
            })
            .collect()
    };

    if let Some(selected) = app.model_list_state.selected() {
        if selected >= filtered_providers.len() {
            app.model_list_state
                .select(Some(0.max(filtered_providers.len().saturating_sub(1))));
        }
    } else if !filtered_providers.is_empty() {
        if let Some(ref selected_model) = app.selected_model {
            if let Some(idx) = filtered_providers
                .iter()
                .position(|m| m.base_url == selected_model.base_url)
            {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    }

    let items: Vec<ListItem> = filtered_providers
        .iter()
        .map(|m| {
            let display_name = format!("{} → {}", m.name, m.base_url);
            ListItem::new(display_name)
        })
        .collect();

    let provider_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Providers ({} found)", filtered_providers.len()));

    let list = List::new(items).block(provider_block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Rgb(50, 50, 50)),
    );

    f.render_stateful_widget(list, layout[1], &mut app.model_list_state);

    let hints = if app.model_search_focused {
        "Type to search | Tab: switch to list | Esc: back"
    } else {
        "Tab: focus search | ↑/↓: navigate | Enter: select | Esc: back"
    };
    let hints_para = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hints_para, layout[2]);
}

fn render_custom_model(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Custom Model");

    let err_msg = if let Some(ref e) = app.error {
        format!("\n\nError: {}", e)
    } else {
        String::new()
    };

    let name_label = if app.custom_model_field == 0 {
        "> Name: "
    } else {
        "  Name: "
    };
    let url_label = if app.custom_model_field == 1 {
        "> Base URL: "
    } else {
        "  Base URL: "
    };

    let text = format!(
        "{}{}\n{}{}\n(Tab to switch, Enter save){}",
        name_label, app.custom_model_name, url_label, app.custom_base_url, err_msg
    );

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, rect);

    let (active_label, active_value, line_offset) = if app.custom_model_field == 0 {
        (name_label, &app.custom_model_name, 1)
    } else {
        (url_label, &app.custom_base_url, 2)
    };

    let cursor_x = (rect.x + 1 + (active_label.len() + active_value.len()) as u16)
        .min(rect.x + rect.width.saturating_sub(1));
    let cursor_y = rect.y + line_offset;
    f.set_cursor_position((cursor_x, cursor_y));
}
