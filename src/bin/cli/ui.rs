use crate::app::{AgentType, App, AppState, ChatMessage, ModelOption, ToolStatus};
use crate::constants::{DEFAULT_BASE_URL, MAX_TOKENS, VERSION};
// Theme definitions are accessed via app.current_theme()
use crate::syntax::highlight_line_with_tree_sitter;
use crate::theme_select::render_theme_selector;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
    },
};
use serde_json;
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

fn wrap_to_width(text: &str, max: usize) -> Vec<String> {
    if text.width() <= max {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    
    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + ch_width > max && !current.is_empty() {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn wrap_preserve_lines(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        out.extend(wrap_to_width(line, max));
    }
    out
}

fn agent_accent(agent: AgentType) -> Color {
    match agent {
        AgentType::Coder => Color::Rgb(92, 136, 255),
        AgentType::CodeResearcher => Color::Rgb(80, 190, 200),
        AgentType::TestAgent => Color::Rgb(120, 200, 120),
        AgentType::PengyAgent => Color::Rgb(200, 120, 220),
        AgentType::ControlAgent => Color::Rgb(230, 200, 120),
        AgentType::IssueAgent => Color::Rgb(240, 120, 120),
        AgentType::ChatAgent => Color::Rgb(180, 180, 255),
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenType {
    Comment,
    String,
    Number,
    Keyword,
    Operator,
    Punctuation,
    Identifier,
    Whitespace,
}

fn tokenize_line(line: &str) -> Vec<(TokenType, String)> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = '\0';
    let mut in_comment = false;
    let mut comment_type = ""; // "//" or "#" or "/*"

    while let Some(ch) = chars.next() {
        if in_comment {
            current.push(ch);
            if comment_type == "//" || comment_type == "#" {
                // Single-line comment - continue until end of line
                continue;
            } else if comment_type == "/*" {
                // Multi-line comment - check for closing
                if current.ends_with("*/") {
                    tokens.push((TokenType::Comment, current.clone()));
                    current.clear();
                    in_comment = false;
                }
                continue;
            }
        } else if in_string {
            current.push(ch);
            if ch == '\\' {
                // Escape sequence - consume next char
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if ch == string_char {
                // End of string
                tokens.push((TokenType::String, current.clone()));
                current.clear();
                in_string = false;
            }
        } else if ch == '/' && chars.peek() == Some(&'/') {
            // Single-line comment
            if !current.is_empty() {
                tokens.push((TokenType::Identifier, current.clone()));
                current.clear();
            }
            current.push(ch);
            chars.next(); // consume '/'
            current.push('/');
            in_comment = true;
            comment_type = "//";
        } else if ch == '#' && current.is_empty() {
            // Python/Shell comment
            current.push(ch);
            in_comment = true;
            comment_type = "#";
        } else if ch == '/' && chars.peek() == Some(&'*') {
            // Multi-line comment start
            if !current.is_empty() {
                tokens.push((TokenType::Identifier, current.clone()));
                current.clear();
            }
            current.push(ch);
            chars.next(); // consume '*'
            current.push('*');
            in_comment = true;
            comment_type = "/*";
        } else if ch == '"' || ch == '\'' {
            // String literal
            if !current.is_empty() {
                tokens.push((TokenType::Identifier, current.clone()));
                current.clear();
            }
            current.push(ch);
            in_string = true;
            string_char = ch;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                // Determine token type for current
                let token_str = current.clone();
                if token_str.chars().all(|c| c.is_ascii_digit() || c == '.' || c == 'x' || c == 'X' || c.is_ascii_hexdigit()) {
                    tokens.push((TokenType::Number, token_str));
                } else {
                    tokens.push((TokenType::Identifier, token_str));
                }
                current.clear();
            }
            tokens.push((TokenType::Whitespace, ch.to_string()));
        } else if is_operator(ch) {
            if !current.is_empty() {
                tokens.push((TokenType::Identifier, current.clone()));
                current.clear();
            }
            // Check for multi-character operators
            let mut op = ch.to_string();
            if let Some(&next) = chars.peek() {
                let two_char = format!("{}{}", ch, next);
                if is_two_char_operator(&two_char) {
                    op = two_char.clone();
                    chars.next();
                }
            }
            tokens.push((TokenType::Operator, op));
        } else if is_punctuation(ch) {
            if !current.is_empty() {
                tokens.push((TokenType::Identifier, current.clone()));
                current.clear();
            }
            tokens.push((TokenType::Punctuation, ch.to_string()));
        } else {
            current.push(ch);
        }
    }

    // Handle remaining token
    if !current.is_empty() {
        if in_comment {
            tokens.push((TokenType::Comment, current));
        } else if in_string {
            tokens.push((TokenType::String, current));
        } else {
            let token_str = current;
            if token_str.chars().all(|c| c.is_ascii_digit() || c == '.' || c == 'x' || c == 'X' || c.is_ascii_hexdigit()) {
                tokens.push((TokenType::Number, token_str));
            } else {
                tokens.push((TokenType::Identifier, token_str));
            }
        }
    }

    tokens
}

fn is_operator(ch: char) -> bool {
    matches!(ch, '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '^' | '~' | '?')
}

fn is_two_char_operator(op: &str) -> bool {
    matches!(op, "==" | "!=" | "<=" | ">=" | "++" | "--" | "+=" | "-=" | "*=" | "/=" | "%=" | "&&" | "||" | "<<" | ">>" | "::" | "->" | "=>")
}

fn is_punctuation(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '.' | '?')
}

fn style_token(token_type: TokenType, token: &str, lang: &str, accent: Color, bg: Color) -> Span<'static> {
    let base_style = Style::default().bg(bg);
    
    match token_type {
        TokenType::Comment => {
            Span::styled(token.to_string(), base_style.fg(Color::Rgb(100, 100, 120)))
        }
        TokenType::String => {
            Span::styled(token.to_string(), base_style.fg(Color::Rgb(150, 200, 150)))
        }
        TokenType::Number => {
            Span::styled(token.to_string(), base_style.fg(Color::Rgb(180, 200, 255)))
        }
        TokenType::Keyword => {
        Span::styled(
            token.to_string(),
            base_style.fg(accent).add_modifier(Modifier::BOLD),
        )
        }
        TokenType::Operator => {
            Span::styled(token.to_string(), base_style.fg(Color::Rgb(200, 150, 200)))
        }
        TokenType::Punctuation => {
            Span::styled(token.to_string(), base_style.fg(Color::Rgb(150, 150, 150)))
        }
        TokenType::Identifier => {
            // Check if it's a keyword
            let keywords = keyword_set(lang);
            let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if keywords.contains(clean) {
                Span::styled(
                    token.to_string(),
                    base_style.fg(accent).add_modifier(Modifier::BOLD),
                )
    } else {
        Span::styled(token.to_string(), base_style.fg(Color::White))
            }
        }
        TokenType::Whitespace => {
            Span::styled(token.to_string(), base_style.fg(Color::Gray))
        }
    }
}

fn highlight_code_line(line: &str, lang: &str, accent: Color, bg: Color) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled("  ", Style::default().bg(bg)));

    // Use tree-sitter for syntax highlighting
    let highlighted_spans = highlight_line_with_tree_sitter(line, lang, accent, bg);
    spans.extend(highlighted_spans);

    Line::from(spans)
}

fn render_markdown_with_code(content: &str, accent: Color, code_bg: Color) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::raw("")));

    let mut in_code_block = false;
    let mut lang = String::new();

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    "  ",
                    Style::default().bg(code_bg).fg(Color::Gray),
                )));
            }
            in_code_block = !in_code_block;
            lang = line.trim_start().trim_matches('`').to_string();
            continue;
        }

        if in_code_block {
            lines.push(highlight_code_line(line, &lang, accent, code_bg));
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
    // Base background fill based on theme
    let theme = app.current_theme();
    let base = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(base, f.area());

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
        AppState::Editor => {
            // Editor disabled for performance reasons - code kept for future use
            // crate::editor::editor_ui::render_editor(f, app);
            // return; // Editor handles its own layout, so return early
            // Show a message instead
            let message = "Editor mode is currently disabled for performance optimization.";
            let paragraph = Paragraph::new(message)
                .block(Block::default().borders(Borders::ALL).title("Editor Disabled"))
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(paragraph, layout[1]);
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
                AppState::ThemeSelector => render_theme_selector(f, app, main_chunks[1]),
                AppState::SessionSelector | AppState::Chat | AppState::Welcome | AppState::Editor => unreachable!(),
            }
        }
    }

    let input_area = match app.state {
        AppState::Welcome => centered_rect(80, 10, layout[1]),
        AppState::CustomModel => Rect::default(),
        AppState::Editor => Rect::default(), // Editor doesn't use input area
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
        AppState::CustomModel | AppState::Editor => {}
        _ => {
            render_input(f, app, input_area);
        }
    }

    render_status_bar(f, app, layout[3]);
}

fn render_tool_call_card(
    id: &str,
    name: &str,
    args: &str,
    result: &Option<String>,
    status: &ToolStatus,
    is_light_theme: bool,
    available_width: usize,
    accent: Color,
) -> ListItem<'static> {
    let mut lines = Vec::new();
    let parsed_args: Option<serde_json::Value> = serde_json::from_str(args).ok();

    // Professional status indicators with theme-aware backgrounds
    let (status_icon, status_fg, card_bg, code_bg) = match status {
        ToolStatus::Error => {
            let card = if is_light_theme {
                Color::Rgb(252, 240, 240)
            } else {
                Color::Rgb(26, 26, 28)
            };
            let code = if is_light_theme {
                Color::Rgb(245, 245, 250)
            } else {
                Color::Rgb(25, 25, 25)
            };
            ("✗", Color::Rgb(220, 80, 80), card, code)
        }
        ToolStatus::Success => {
            let card = if is_light_theme {
                Color::Rgb(240, 248, 242)
            } else {
                Color::Rgb(26, 28, 26)
            };
            let code = if is_light_theme {
                Color::Rgb(245, 245, 250)
            } else {
                Color::Rgb(25, 25, 25)
            };
            ("✓", Color::Rgb(100, 180, 120), card, code)
        }
        ToolStatus::Running => {
            let card = if is_light_theme {
                Color::Rgb(252, 248, 240)
            } else {
                Color::Rgb(28, 27, 25)
            };
            let code = if is_light_theme {
                Color::Rgb(245, 245, 250)
            } else {
                Color::Rgb(25, 25, 25)
            };
            ("●", Color::Rgb(200, 160, 80), card, code)
        }
    };

    // Subtle top border - use available width minus padding
    let border_width = available_width.saturating_sub(2).max(10);
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("─".repeat(border_width), Style::default().fg(Color::Rgb(50, 50, 55))),
    ]));

    // Clean header without emojis
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            format!(" {} ", status_icon),
            Style::default().fg(status_fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            name.to_string(),
            Style::default()
                .fg(Color::Rgb(200, 200, 220))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" (call #{})", id.replace("tool_", "")),
            Style::default().fg(Color::Rgb(100, 100, 120)),
        ),
    ]));

    // Running indicator
    if matches!(status, ToolStatus::Running) && result.is_none() {
        lines.push(Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("→ ", Style::default().fg(status_fg)),
            Span::styled(
                "Executing...",
                Style::default()
                    .fg(Color::Rgb(140, 140, 160))
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Tool-specific summary - professional display
    if name == "edit" || name == "edit_file" {
        if let Some(json) = parsed_args.as_ref() {
            let path = json
                .get("filePath")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !path.is_empty() {
                let max_path_len = available_width.saturating_sub(10).max(20); // Account for "File: " prefix
                let display_path = if path.chars().count() > max_path_len {
                    format!("{}…", path.chars().take(max_path_len.saturating_sub(1)).collect::<String>())
                } else {
                    path.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        "File: ".to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    Span::styled(display_path, Style::default().fg(Color::Rgb(160, 180, 220))),
                ]));

                let old_snip = json.get("oldString").and_then(|v| v.as_str()).unwrap_or("");
                let new_snip = json.get("newString").and_then(|v| v.as_str()).unwrap_or("");

                let old_lines = old_snip.lines().count();
                let new_lines = new_snip.lines().count();

                if old_lines > 0 || new_lines > 0 {
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default()),
                        Span::styled(
                            "Changes: ".to_string(),
                            Style::default().fg(Color::Rgb(110, 110, 130)),
                        ),
                        Span::styled(
                            format!("-{} ", old_lines),
                            Style::default().fg(Color::Rgb(200, 100, 100)),
                        ),
                        Span::styled(
                            format!("+{} ", new_lines),
                            Style::default().fg(Color::Rgb(100, 180, 120)),
                        ),
                        Span::styled(
                            "lines".to_string(),
                            Style::default().fg(Color::Rgb(110, 110, 130)),
                        ),
                    ]));
                }
            }
        }
    } else if name == "bash" || name == "run_terminal_cmd" {
        if let Some(json) = parsed_args.as_ref() {
            if let Some(cmd) = json.get("cmd").and_then(|v| v.as_str()) {
                let max_cmd_len = available_width.saturating_sub(15).max(20); // Account for "Command: " prefix
                let preview = if cmd.len() > max_cmd_len {
                    format!("{}…", &cmd[..max_cmd_len.saturating_sub(1)])
                } else {
                    cmd.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        "Command: ".to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    Span::styled(preview, Style::default().fg(Color::Rgb(180, 180, 200))),
                ]));
            }
        }
    } else if name == "read_file" {
        if let Some(json) = parsed_args.as_ref() {
            if let Some(path) = json
                .get("target_file")
                .or_else(|| json.get("path"))
                .and_then(|v| v.as_str())
            {
                let max_path_len = available_width.saturating_sub(12).max(20); // Account for "Reading: " prefix
                let display_path = if path.chars().count() > max_path_len {
                    format!("{}…", path.chars().take(max_path_len.saturating_sub(1)).collect::<String>())
                } else {
                    path.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        "Reading: ".to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    Span::styled(
                        display_path,
                        Style::default().fg(Color::Rgb(160, 180, 220)),
                    ),
                ]));
            }
        }
    } else if name == "grep" || name == "grep_search" {
        if let Some(json) = parsed_args.as_ref() {
            if let Some(pattern) = json.get("pattern").and_then(|v| v.as_str()) {
                let max_pattern_len = available_width.saturating_sub(20).max(20); // Account for "Pattern: " prefix and quotes
                let preview = if pattern.len() > max_pattern_len {
                    format!("{}…", &pattern[..max_pattern_len.saturating_sub(1)])
                } else {
                    pattern.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        "Pattern: ".to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    Span::styled(
                        format!("\"{}\"", preview),
                        Style::default().fg(Color::Rgb(200, 160, 100)),
                    ),
                ]));
            }
        }
    } else if name == "list_dir" {
        if let Some(json) = parsed_args.as_ref() {
            if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                let max_path_len = available_width.saturating_sub(15).max(20); // Account for "Directory: " prefix
                let display_path = if path.chars().count() > max_path_len {
                    format!("{}…", path.chars().take(max_path_len.saturating_sub(1)).collect::<String>())
                } else {
                    path.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        "Directory: ".to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    Span::styled(
                        display_path,
                        Style::default().fg(Color::Rgb(160, 180, 220)),
                    ),
                ]));
            }
        }
    } else if name == "file_manager" {
        if let Some(json) = parsed_args.as_ref() {
            // Check if it's a batch operation
            if let Some(files_array) = json.get("files").and_then(|v| v.as_array()) {
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        format!("Batch operation: {} file(s)", files_array.len()),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                ]));
                
                // Show summary of each file operation
                for (idx, file_op) in files_array.iter().take(3).enumerate() {
                    if let Some(path) = file_op.get("path").and_then(|v| v.as_str()) {
                        let kind = file_op.get("kind").and_then(|v| v.as_str()).unwrap_or("file");
                        let type_str = if kind == "directory" || kind == "folder" { "dir" } else { "file" };
                        lines.push(Line::from(vec![
                            Span::styled("       ", Style::default()),
                            Span::styled(
                                format!("{}. ", idx + 1),
                                Style::default().fg(Color::Rgb(110, 110, 130)),
                            ),
                            Span::styled(
                                format!("[{}] ", type_str),
                                Style::default().fg(Color::Rgb(140, 140, 160)),
                            ),
                            {
                                let max_path_len = available_width.saturating_sub(15).max(20); // Account for "       " + "1. " + "[file] " prefix
                                let display_path = if path.chars().count() > max_path_len {
                                    format!("{}…", path.chars().take(max_path_len.saturating_sub(1)).collect::<String>())
                                } else {
                                    path.to_string()
                                };
                            Span::styled(
                                    display_path,
                                Style::default().fg(Color::Rgb(160, 180, 220)),
                                )
                            },
                        ]));
                    }
                }
                if files_array.len() > 3 {
                    lines.push(Line::from(vec![
                        Span::styled("       ", Style::default()),
                        Span::styled(
                            format!("... and {} more", files_array.len() - 3),
                            Style::default()
                                .fg(Color::Rgb(90, 90, 110))
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            } else if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                // Single file operation
                let kind = json.get("kind").and_then(|v| v.as_str()).unwrap_or("file");
                
                lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(
                        if kind == "directory" || kind == "folder" {
                            "Creating directory: "
                        } else {
                            "File: "
                        }.to_string(),
                        Style::default().fg(Color::Rgb(110, 110, 130)),
                    ),
                    {
                        let max_path_len = available_width.saturating_sub(25).max(20); // Account for "Creating directory: " or "File: " prefix
                        let display_path = if path.chars().count() > max_path_len {
                            format!("{}…", path.chars().take(max_path_len.saturating_sub(1)).collect::<String>())
                        } else {
                            path.to_string()
                        };
                    Span::styled(
                            display_path,
                        Style::default().fg(Color::Rgb(160, 180, 220)),
                        )
                    },
                ]));

                // Show line numbers for partial replacement
                let start_line = json.get("startLine").and_then(|v| v.as_u64());
                let end_line = json.get("endLine").and_then(|v| v.as_u64());
                if let (Some(start), Some(end)) = (start_line, end_line) {
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default()),
                        Span::styled(
                            format!("Replacing lines {}-{}", start, end),
                            Style::default().fg(Color::Rgb(200, 160, 100)),
                        ),
                    ]));
                }

                // Show content preview with syntax highlighting if it's a file
                if kind == "file" {
                    if let Some(content) = json.get("content").and_then(|v| {
                        if v.is_string() {
                            v.as_str()
                        } else {
                            None
                        }
                    }) {
                        if !content.is_empty() {
                            lines.push(Line::from(Span::raw("")));
                            
                            // Detect language from file extension first (for display)
                            let display_lang = if let Some(ext) = path.rsplit('.').next() {
                                match ext {
                                    "rs" => "rust",
                                    "py" => "python",
                                    "js" => "javascript",
                                    "ts" | "tsx" => "typescript",
                                    "jsx" => "javascript",
                                    "go" => "go",
                                    "java" => "java",
                                    "c" | "h" => "c",
                                    "cpp" | "cc" | "cxx" | "hpp" => "c++",
                                    "ml" | "mli" => "ocaml",
                                    "json" => "json",
                                    "md" => "markdown",
                                    "toml" => "toml",
                                    "yaml" | "yml" => "yaml",
                                    _ => ext,
                                }
                            } else {
                                "text"
                            };
                            
                            let separator_width = available_width.saturating_sub(5).max(10);
                            lines.push(Line::from(vec![
                                Span::styled("     ", Style::default()),
                                Span::styled(
                                    "─".repeat(separator_width),
                                    Style::default().fg(Color::Rgb(60, 60, 80)),
                                ),
                            ]));
                            lines.push(Line::from(vec![
                                Span::styled("     ", Style::default()),
                                Span::styled(
                                    "Content Preview",
                                    Style::default()
                                        .fg(Color::Rgb(120, 120, 140))
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!(" ({})", display_lang),
                                    Style::default().fg(Color::Rgb(100, 100, 120)),
                                ),
                            ]));

                            // Detect language from file extension - improved detection
                            let lang = if let Some(ext) = path.rsplit('.').next() {
                                match ext.to_lowercase().as_str() {
                                    "rs" => "rust",
                                    "py" => "python",
                                    "pyw" => "python",
                                    "js" => "javascript",
                                    "mjs" => "javascript",
                                    "ts" => "typescript",
                                    "tsx" => "typescript",
                                    "jsx" => "javascript",
                                    "go" => "go",
                                    "java" => "java",
                                    "c" => "c",
                                    "h" => "c",
                                    "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
                                    "ml" | "mli" => "ocaml",
                                    "json" => "json",
                                    "toml" => "toml",
                                    "yaml" | "yml" => "yaml",
                                    "md" => "markdown",
                                    "sh" | "bash" => "bash",
                                    "zsh" => "bash",
                                    "fish" => "bash",
                                    "html" | "htm" => "html",
                                    "css" => "css",
                                    "xml" => "xml",
                                    "sql" => "sql",
                                    "rb" => "ruby",
                                    "php" => "php",
                                    "swift" => "swift",
                                    "kt" | "kts" => "kotlin",
                                    "scala" => "scala",
                                    "clj" | "cljs" => "clojure",
                                    "hs" => "haskell",
                                    "elm" => "elm",
                                    "ex" | "exs" => "elixir",
                                    "erl" | "hrl" => "erlang",
                                    "lua" => "lua",
                                    "vim" => "vim",
                                    "r" => "r",
                                    "m" => "matlab",
                                    _ => "",
                                }
                            } else {
                                ""
                            };

                            // Show content with syntax highlighting (limited to 15 lines)
                            let content_lines: Vec<&str> = content.lines().collect();
                            let display_limit = 15;
                            let total_lines = content_lines.len();
                            
                            for (idx, line) in content_lines.iter().take(display_limit).enumerate() {
                                let mut line_spans = vec![
                                    Span::styled("       ", Style::default()),
                                    Span::styled(
                                        format!("{:>3} │ ", idx + 1),
                                        Style::default().fg(Color::Rgb(70, 70, 90)),
                                    ),
                                ];
                                
                                // Use tree-sitter for syntax highlighting with accent color
                                let highlighted_spans = highlight_line_with_tree_sitter(line, lang, accent, code_bg);
                                line_spans.extend(highlighted_spans);
                                
                                lines.push(Line::from(line_spans));
                            }

                            if total_lines > display_limit {
                                lines.push(Line::from(vec![
                                    Span::styled("           ", Style::default()),
                                    Span::styled(
                                        format!("└─ {} more lines", total_lines - display_limit),
                                        Style::default()
                                            .fg(Color::Rgb(90, 90, 110))
                                            .add_modifier(Modifier::ITALIC),
                                    ),
                                ]));
                            }
                            
                            // Closing separator
                            let separator_width = available_width.saturating_sub(5).max(10);
                            lines.push(Line::from(vec![
                                Span::styled("     ", Style::default()),
                                Span::styled(
                                    "─".repeat(separator_width),
                                    Style::default().fg(Color::Rgb(60, 60, 80)),
                                ),
                            ]));
                        }
                    }
                }
            }
        }
    }

    // Detect language for result highlighting when we have a file path - improved detection
    let lang_hint = parsed_args.as_ref().and_then(|json| {
        json.get("target_file")
            .or_else(|| json.get("path"))
            .and_then(|v| v.as_str())
            .and_then(|path| {
                let ext = path.rsplit('.').next().unwrap_or("");
                match ext.to_lowercase().as_str() {
                    "rs" => Some("rust"),
                    "py" | "pyw" => Some("python"),
                    "js" | "mjs" => Some("javascript"),
                    "ts" => Some("typescript"),
                    "tsx" => Some("typescript"),
                    "jsx" => Some("javascript"),
                    "go" => Some("go"),
                    "java" => Some("java"),
                    "c" => Some("c"),
                    "h" => Some("c"),
                    "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp"),
                    "ml" | "mli" => Some("ocaml"),
                    "json" => Some("json"),
                    "toml" => Some("toml"),
                    "yaml" | "yml" => Some("yaml"),
                    "md" => Some("markdown"),
                    "sh" | "bash" | "zsh" | "fish" => Some("bash"),
                    "html" | "htm" => Some("html"),
                    "css" => Some("css"),
                    "xml" => Some("xml"),
                    "sql" => Some("sql"),
                    "rb" => Some("ruby"),
                    "php" => Some("php"),
                    "swift" => Some("swift"),
                    "kt" | "kts" => Some("kotlin"),
                    "scala" => Some("scala"),
                    "clj" | "cljs" => Some("clojure"),
                    "hs" => Some("haskell"),
                    "elm" => Some("elm"),
                    "ex" | "exs" => Some("elixir"),
                    "erl" | "hrl" => Some("erlang"),
                    "lua" => Some("lua"),
                    "vim" => Some("vim"),
                    "r" => Some("r"),
                    "m" => Some("matlab"),
                    _ => None,
                }
            })
    });

    // Result section with professional styling
    if let Some(res) = result {
        let normalized = res.replace("\\r\\n", "\n").replace("\\n", "\n");

        if !normalized.is_empty() {
            lines.push(Line::from(Span::raw("")));

            // Result header
            lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(
                    "Output:",
                    Style::default()
                        .fg(Color::Rgb(120, 120, 140))
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Smart result truncation with line count and wrapping
            let result_lines: Vec<&str> = normalized.lines().collect();
            let total_lines = result_lines.len();
            let display_limit = 12;
            // Account for "       " (7 chars) + "   │ " (5 chars) = 12 chars padding
            let max_line_width = available_width.saturating_sub(12).max(20);

            // Apply syntax highlighting for read_file - always try to detect language
            if name == "read_file" {
                // Try to get language from lang_hint first, then fallback to detecting from args
                let detected_lang = lang_hint
                    .or_else(|| {
                        parsed_args.as_ref()
                            .and_then(|json| {
                                json.get("target_file")
                                    .or_else(|| json.get("path"))
                                    .and_then(|v| v.as_str())
                                    .and_then(|path| {
                                        let ext = path.rsplit('.').next().unwrap_or("");
                                        match ext.to_lowercase().as_str() {
                                            "rs" => Some("rust"),
                                            "py" | "pyw" => Some("python"),
                                            "js" | "mjs" => Some("javascript"),
                                            "ts" => Some("typescript"),
                                            "tsx" => Some("typescript"),
                                            "jsx" => Some("javascript"),
                                            "go" => Some("go"),
                                            "java" => Some("java"),
                                            "c" => Some("c"),
                                            "h" => Some("c"),
                                            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp"),
                                            "ml" | "mli" => Some("ocaml"),
                                            "json" => Some("json"),
                                            "toml" => Some("toml"),
                                            "yaml" | "yml" => Some("yaml"),
                                            "md" => Some("markdown"),
                                            "sh" | "bash" | "zsh" | "fish" => Some("bash"),
                                            "html" | "htm" => Some("html"),
                                            "css" => Some("css"),
                                            "xml" => Some("xml"),
                                            "sql" => Some("sql"),
                                            "rb" => Some("ruby"),
                                            "php" => Some("php"),
                                            "swift" => Some("swift"),
                                            "kt" | "kts" => Some("kotlin"),
                                            "scala" => Some("scala"),
                                            "clj" | "cljs" => Some("clojure"),
                                            "hs" => Some("haskell"),
                                            "elm" => Some("elm"),
                                            "ex" | "exs" => Some("elixir"),
                                            "erl" | "hrl" => Some("erlang"),
                                            "lua" => Some("lua"),
                                            "vim" => Some("vim"),
                                            "r" => Some("r"),
                                            "m" => Some("matlab"),
                                            _ => None,
                                        }
                                    })
                            })
                    })
                    .unwrap_or("");

                let mut displayed = 0;
                for (idx, line) in result_lines.iter().take(display_limit).enumerate() {
                    // Handle both formats: "L{num}:{content}" and plain lines
                    let (line_no, code_body) = if let Some(rest) = line.strip_prefix('L') {
                        // Format: L{num}:{content}
                        let mut parts = rest.splitn(2, ':');
                        if let (Some(num), Some(body)) = (parts.next(), parts.next()) {
                            (num.parse::<usize>().unwrap_or(idx + 1), body)
                        } else {
                            (idx + 1, *line)
                        }
                    } else {
                        // Plain line format (entire file read)
                        (idx + 1, *line)
                    };

                    let mut line_spans = vec![
                        Span::styled("       ", Style::default()),
                        Span::styled(
                            format!("{:>3} │ ", line_no),
                            Style::default().fg(Color::Rgb(70, 70, 90)),
                        ),
                    ];
                    
                    // Always try syntax highlighting - tree-sitter will fallback to basic highlighting if needed
                    let highlighted_spans = highlight_line_with_tree_sitter(code_body, detected_lang, accent, code_bg);
                    line_spans.extend(highlighted_spans);
                    
                    lines.push(Line::from(line_spans));
                    displayed += 1;
                }

                if total_lines > display_limit {
                    lines.push(Line::from(vec![
                        Span::styled("           ", Style::default()),
                        Span::styled(
                            format!("└─ {} more lines omitted", total_lines - displayed),
                            Style::default()
                                .fg(Color::Rgb(90, 90, 110))
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            } else {
                let mut displayed_line_count = 0;

                for (original_idx, line) in result_lines.iter().enumerate() {
                    if displayed_line_count >= display_limit {
                        break;
                    }

                    // Wrap long lines - need to handle character boundaries properly
                    if line.chars().count() > max_line_width {
                        let mut remaining = *line;
                        let mut is_first = true;

                        while !remaining.is_empty() && displayed_line_count < display_limit {
                            // Find the next chunk respecting character boundaries
                            let mut chunk_end = remaining.len();
                            let mut current_char_count = 0;
                            for (idx, _) in remaining.char_indices() {
                                if current_char_count >= max_line_width {
                                    chunk_end = idx;
                                    break;
                                }
                                current_char_count += 1;
                            }
                            let chunk = &remaining[..chunk_end];

                            if !chunk.trim().is_empty() {
                                lines.push(Line::from(vec![
                                    Span::styled("       ", Style::default()),
                                    Span::styled(
                                        if is_first {
                                            format!("{:>3} │ ", original_idx + 1)
                                        } else {
                                            "    │ ".to_string()
                                        },
                                        Style::default().fg(Color::Rgb(70, 70, 90)),
                                    ),
                                    Span::styled(chunk.to_string(), Style::default().fg(Color::Rgb(180, 180, 200))),
                                ]));
                            } else if is_first {
                                lines.push(Line::from(Span::styled(
                                    "           │",
                                    Style::default().fg(Color::Rgb(70, 70, 90)),
                                )));
                            }

                            is_first = false;
                            remaining = &remaining[chunk_end..];
                            displayed_line_count += 1;
                        }
                    } else {
                        // Short line - display as is
                        if !line.trim().is_empty() {
                            lines.push(Line::from(vec![
                                Span::styled("       ", Style::default()),
                                Span::styled(
                                    format!("{:>3} │ ", original_idx + 1),
                                    Style::default().fg(Color::Rgb(70, 70, 90)),
                                ),
                                Span::styled(line.to_string(), Style::default().fg(Color::Rgb(180, 180, 200))),
                            ]));
                        } else {
                            lines.push(Line::from(Span::styled(
                                "           │",
                                Style::default().fg(Color::Rgb(70, 70, 90)),
                            )));
                        }
                        displayed_line_count += 1;
                    }
                }

                if displayed_line_count >= display_limit && total_lines > display_limit {
                    lines.push(Line::from(vec![
                        Span::styled("           ", Style::default()),
                        Span::styled(
                            format!("└─ {} more lines omitted", total_lines - displayed_line_count),
                            Style::default()
                                .fg(Color::Rgb(90, 90, 110))
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
        }
    }

    // Bottom border
    lines.push(Line::from(Span::raw("")));

    ListItem::new(lines).style(Style::default().bg(card_bg))
}

#[allow(unused_variables)]
fn render_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.current_theme();
    let accent = agent_accent(app.selected_agent);
    
    // Calculate available width for text wrapping (account for scrollbar and padding)
    let available_width = area.width.saturating_sub(2).max(20) as usize; // Subtract 2 for scrollbar, min 20

    // Theme-aware backgrounds
    let (user_bg, assistant_bg, thinking_bg, code_bg) = if theme.name == "Light" {
        (
            Color::Rgb(240, 242, 245),
            Color::Rgb(248, 250, 252),
            Color::Rgb(245, 247, 250),
            Color::Rgb(235, 240, 245), // Light code background
        )
    } else {
        (
            Color::Rgb(28, 28, 32),
            Color::Rgb(20, 24, 28),
            Color::Rgb(24, 24, 30),
            Color::Rgb(25, 25, 25), // Dark code background
        )
    };

    let messages: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| match msg {
            ChatMessage::User(content) => {
                let mut user_lines = Vec::new();

                // Top spacing
                user_lines.push(Line::from(Span::styled(" ", Style::default().bg(user_bg))));

                // Professional user header
                user_lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(user_bg)),
                    Span::styled(
                        "USER",
                        Style::default()
                            .fg(Color::Rgb(120, 160, 200))
                            .bg(user_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                let separator_width = available_width.saturating_sub(2).max(10);
                user_lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(user_bg)),
                    Span::styled(
                        "─".repeat(separator_width),
                        Style::default().fg(Color::Rgb(50, 50, 60)).bg(user_bg),
                    ),
                ]));

                // User content with proper wrapping
                let user_text_width = available_width.saturating_sub(2).max(20);
                for line in content.lines() {
                    for wrapped_line in wrap_to_width(line, user_text_width) {
                    user_lines.push(Line::from(vec![
                        Span::styled("  ", Style::default().bg(user_bg)),
                        Span::styled(
                                wrapped_line,
                            Style::default().fg(Color::Rgb(220, 220, 240)).bg(user_bg),
                        ),
                    ]));
                    }
                }

                // Bottom spacing
                user_lines.push(Line::from(Span::styled(" ", Style::default().bg(user_bg))));

                ListItem::new(user_lines)
            }
            ChatMessage::Assistant(content) => {
                let mut assistant_lines = Vec::new();

                // Top spacing
                assistant_lines.push(Line::from(Span::styled(
                    " ",
                    Style::default().bg(assistant_bg),
                )));

                // Professional assistant header
                assistant_lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(assistant_bg)),
                    Span::styled(
                        "ASSISTANT",
                        Style::default()
                            .fg(accent)
                            .bg(assistant_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                let separator_width = available_width.saturating_sub(2).max(10);
                assistant_lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(assistant_bg)),
                    Span::styled(
                        "─".repeat(separator_width),
                        Style::default().fg(Color::Rgb(45, 48, 52)).bg(assistant_bg),
                    ),
                ]));

                // Content with markdown and code highlighting
                let content_lines = render_markdown_with_code(content, accent, code_bg)
                    .into_iter()
                    .map(|ln| ln.patch_style(Style::default().bg(assistant_bg)))
                    .collect::<Vec<_>>();

                assistant_lines.extend(content_lines);

                // Bottom spacing
                assistant_lines.push(Line::from(Span::styled(
                    " ",
                    Style::default().bg(assistant_bg),
                )));

                ListItem::new(assistant_lines)
            }
            ChatMessage::ToolCall {
                id,
                name,
                args,
                result,
                status,
                ..
            } => render_tool_call_card(id, name, args, result, status, theme.name == "Light", available_width, accent),
            ChatMessage::Thinking(content) => {
                let mut lines = Vec::new();

                // Professional thinking header
                lines.push(Line::from(Span::styled(
                    " ",
                    Style::default().bg(thinking_bg),
                )));

                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(thinking_bg)),
                    Span::styled(
                        "REASONING",
                        Style::default()
                            .fg(Color::Rgb(140, 160, 200))
                            .bg(thinking_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                let separator_width = available_width.saturating_sub(2).max(10);
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(thinking_bg)),
                    Span::styled(
                        "─".repeat(separator_width),
                        Style::default().fg(Color::Rgb(50, 52, 58)).bg(thinking_bg),
                    ),
                ]));

                // Thinking content with professional styling
                let thinking_text_width = available_width.saturating_sub(2).max(20);
                for seg in wrap_preserve_lines(content, thinking_text_width).into_iter() {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default().bg(thinking_bg)),
                        Span::styled(
                            seg,
                            Style::default()
                                .fg(Color::Rgb(150, 160, 180))
                                .bg(thinking_bg)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }

                lines.push(Line::from(Span::styled(
                    " ",
                    Style::default().bg(thinking_bg),
                )));

                ListItem::new(lines)
            }
            ChatMessage::Error(err) => {
                let error_bg = Color::Rgb(32, 26, 26);
                let mut lines = Vec::new();

                // Top spacing
                lines.push(Line::from(Span::styled(" ", Style::default().bg(error_bg))));

                // Professional error header
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(error_bg)),
                    Span::styled(
                        "ERROR",
                        Style::default()
                            .fg(Color::Rgb(220, 80, 80))
                            .bg(error_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                let separator_width = available_width.saturating_sub(2).max(10);
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(error_bg)),
                    Span::styled(
                        "─".repeat(separator_width),
                        Style::default().fg(Color::Rgb(60, 45, 45)).bg(error_bg),
                    ),
                ]));

                // Error message with proper wrapping
                let error_text_width = available_width.saturating_sub(2).max(20);
                for line in err.lines() {
                    for wrapped_line in wrap_to_width(line, error_text_width) {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default().bg(error_bg)),
                        Span::styled(
                                wrapped_line,
                            Style::default().fg(Color::Rgb(220, 140, 140)).bg(error_bg),
                        ),
                    ]));
                    }
                }

                // Bottom spacing
                lines.push(Line::from(Span::styled(" ", Style::default().bg(error_bg))));

                ListItem::new(lines)
            }
        })
        .collect();

    let messages_len = messages.len();

    if messages_len == 0 {
        app.list_state.select(None);
    } else {
        let selected = app
            .list_state
            .selected()
            .unwrap_or_else(|| messages_len.saturating_sub(1))
            .min(messages_len.saturating_sub(1));

        if app.user_scrolled {
            app.list_state.select(Some(selected));
        } else {
            app.list_state.select(Some(messages_len.saturating_sub(1)));
        }
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
        .content_length(messages_len)
        .position(selected_idx);
    f.render_stateful_widget(scrollbar, area, &mut scroll_state);
    app.scroll_state = scroll_state;
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.current_theme();
    let state = format!("{:?}", app.state);
    let title = format!(" Pengy Agent {} │ State: {} ", VERSION, state);
    let header = Paragraph::new(vec![Line::from(title), Line::from("")]).style(
        Style::default()
            .fg(theme.text)
            .bg(theme.header_bg)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, area);
}

fn render_input(f: &mut Frame, app: &mut App, area: Rect) {
    use unicode_width::UnicodeWidthChar;

    let theme = app.current_theme();
    let input_bg_color = theme.input_bg;
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

    let prompt = "> ";
    let wrap_width = (available_width as usize).max(1);

    // Wrap text manually so trailing spaces are preserved and cursor math stays correct.
    let mut lines: Vec<String> = vec![String::new()];
    let mut line_widths: Vec<usize> = vec![0];
    let mut cursor_line: usize = 0;
    let mut cursor_col_width: usize = 0;
    let total_chars = app.chat_input.chars().count();

    if app.input_cursor == 0 {
        cursor_line = 0;
        cursor_col_width = 0;
    }

    for (idx, ch) in app.chat_input.chars().enumerate() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);

        if ch == '\n' {
            lines.push(String::new());
            line_widths.push(0);
            if idx + 1 == app.input_cursor {
                cursor_line = lines.len().saturating_sub(1);
                cursor_col_width = 0;
            }
            continue;
        }

        let current_width = *line_widths.last().unwrap();
        if current_width + ch_width > wrap_width {
            lines.push(String::new());
            line_widths.push(0);
        }

        lines.last_mut().unwrap().push(ch);
        *line_widths.last_mut().unwrap() += ch_width;

        if idx + 1 == app.input_cursor {
            cursor_line = lines.len().saturating_sub(1);
            cursor_col_width = *line_widths.last().unwrap();
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
        line_widths.push(0);
    }

    if app.input_cursor >= total_chars {
        cursor_line = lines.len().saturating_sub(1);
        cursor_col_width = *line_widths.last().unwrap_or(&0);
    }

    // Only show the tail that fits in the available height
    let available_height = inner_area.height as usize;
    let start_idx = lines
        .len()
        .saturating_sub(available_height)
        .min(lines.len());
    let visible = &lines[start_idx..];

    let mut input_content = Vec::new();

    for (i, line) in visible.iter().enumerate() {
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
                line.to_string(),
                Style::default()
                    .fg(Color::White)
                    .bg(input_bg_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    let input_paragraph = Paragraph::new(input_content);
    f.render_widget(input_paragraph, inner_area);

    // Adjust cursor_line relative to visible slice
    if cursor_line < start_idx {
        cursor_line = start_idx;
        cursor_col_width = 0;
    }
    let visible_line = cursor_line.saturating_sub(start_idx);

    let prefix_len = if visible_line == 0 { prompt.len() } else { 2 };
    let cursor_x = (inner_area.x + gutter_width + prefix_len as u16 + cursor_col_width as u16)
        .min(inner_area.x + inner_area.width.saturating_sub(1));
    let cursor_y = inner_area.y + visible_line as u16;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.current_theme();
    let model_name = app
        .selected_model
        .as_ref()
        .map(|m| {
            // Truncate long model names
            let name = m.name.clone();
            if name.len() > 30 {
                format!("{}…", &name[..29])
            } else {
                name
            }
        })
        .unwrap_or_else(|| "None".to_string());

    let agent_name = format!("{:?}", app.selected_agent);

    let loading = if app.loading { "Running" } else { "Idle" };

    let loading_color = if app.loading {
        Color::Rgb(200, 160, 80)
    } else {
        Color::Rgb(100, 180, 120)
    };

    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string();

    let status_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(cwd, Style::default().fg(Color::Rgb(140, 140, 160))),
        Span::styled(" │ ", Style::default().fg(Color::Rgb(80, 80, 100))),
        Span::styled("Model: ", Style::default().fg(Color::Rgb(120, 120, 140))),
        Span::styled(model_name, Style::default().fg(Color::Rgb(180, 180, 200))),
        Span::styled(" │ ", Style::default().fg(Color::Rgb(80, 80, 100))),
        Span::styled("Agent: ", Style::default().fg(Color::Rgb(120, 120, 140))),
        Span::styled(agent_name, Style::default().fg(Color::Rgb(180, 180, 200))),
        Span::styled(" │ ", Style::default().fg(Color::Rgb(80, 80, 100))),
        Span::styled(
            loading,
            Style::default()
                .fg(loading_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let status = Paragraph::new(vec![status_line, Line::from("")])
        .style(Style::default().bg(theme.status_bg));
    f.render_widget(status, area);
}

fn render_chat_sidebar(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(5),
            Constraint::Length(6),
        ])
        .split(area);

    // Token usage panel
    let token_block = Block::default().borders(Borders::ALL).title("Token Usage");

    let mut token_lines: Vec<Line> = Vec::new();
    if let Some((prompt, completion, total)) = app.last_token_usage {
        let pct = if MAX_TOKENS > 0 {
            ((total as f64) / (MAX_TOKENS as f64) * 100.0).min(999.9)
        } else {
            0.0
        };

        token_lines.push(Line::from(vec![
            Span::styled(
                "Prompt:     ",
                Style::default().fg(Color::Rgb(120, 120, 140)),
            ),
            Span::styled(
                format!("{}", prompt),
                Style::default().fg(Color::Rgb(180, 180, 200)),
            ),
        ]));
        token_lines.push(Line::from(vec![
            Span::styled(
                "Completion: ",
                Style::default().fg(Color::Rgb(120, 120, 140)),
            ),
            Span::styled(
                format!("{}", completion),
                Style::default().fg(Color::Rgb(180, 180, 200)),
            ),
        ]));
        token_lines.push(Line::from(vec![
            Span::styled(
                "Total:      ",
                Style::default().fg(Color::Rgb(120, 120, 140)),
            ),
            Span::styled(
                format!("{} ({:.1}%)", total, pct),
                Style::default()
                    .fg(Color::Rgb(200, 200, 220))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        token_lines.push(Line::from(Span::styled(
            "No usage data yet",
            Style::default().fg(Color::Rgb(100, 100, 120)),
        )));
    }

    let token_para = Paragraph::new(token_lines).block(token_block);
    f.render_widget(token_para, vertical[0]);

    // Modified files panel
    if !app.modified_files.is_empty() {
        let mut file_items: Vec<Line> = Vec::new();
        for (file_path, (_added, _removed)) in app.modified_files.iter() {
            let file_name = file_path
                .rsplit('/')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_path.clone());

            // Truncate long filenames
            let display_name = if file_name.len() > 18 {
                format!("{}…", &file_name[..17])
            } else {
                file_name
            };

            file_items.push(Line::from(vec![Span::styled(
                display_name,
                Style::default().fg(Color::Rgb(180, 180, 200)),
            )]));
        }

        let file_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Modified ({} files)", app.modified_files.len()));
        let file_list = List::new(file_items).block(file_block);
        f.render_widget(file_list, vertical[1]);
    } else {
        let empty_block = Block::default()
            .borders(Borders::ALL)
            .title("Modified Files");
        let empty =
            Paragraph::new("No changes yet").style(Style::default().fg(Color::Rgb(100, 100, 120)));
        f.render_widget(empty.block(empty_block), vertical[1]);
    }

    // Session info panel
    let session_block = Block::default().borders(Borders::ALL).title("Session");
    let mut context_lines: Vec<Line> = Vec::new();

    let session_name = app
        .sessions
        .get(app.current_session)
        .cloned()
        .unwrap_or_else(|| "New".to_string());

    let truncated_session = if session_name.len() > 18 {
        format!("{}…", &session_name[..17])
    } else {
        session_name
    };

    context_lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Rgb(120, 120, 140))),
        Span::styled(
            truncated_session,
            Style::default().fg(Color::Rgb(180, 180, 200)),
        ),
    ]));
    context_lines.push(Line::from(vec![
        Span::styled("Messages: ", Style::default().fg(Color::Rgb(120, 120, 140))),
        Span::styled(
            format!("{}", app.chat_messages.len()),
            Style::default().fg(Color::Rgb(180, 180, 200)),
        ),
    ]));

    let context_para = Paragraph::new(context_lines).block(session_block);
    f.render_widget(context_para, vertical[2]);
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
    let text = "Available Commands:\n\n/models - Select Model\n/agents - Select Agent\n/settings - Configure API key / model / base URL\n/baseurl - Select provider base URL (Mistral, DeepSeek, OpenRouter, etc.)\n/help - Show this help screen\n/clear - Clear conversation and reset agent\n/sandbox - Enable sandbox mode (auto-commit every run; merge with /save)\n/save - Merge sandbox branch back to the base branch and switch back\n\nNavigation:\nUse Arrows to navigate lists.\nTab to switch between fields/agents.\nEnter to select.\nEsc to go back.\n\nTip: Type '/' in the input to see all available commands with autocomplete.";
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
            ListItem::new(Line::from(vec![
                Span::styled(
                    m.name.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("({})", m.base_url),
                    Style::default().fg(Color::Gray),
                ),
            ]))
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
        "Tab: focus search | ↑/↓: preview | Enter: select | Esc: back"
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

    // Only the name is edited here; base URL follows the current settings/base URL.
    app.custom_model_field = 0;

    let err_msg = if let Some(ref e) = app.error {
        format!("\n\nError: {}", e)
    } else {
        String::new()
    };

    let name_label = "> Name: ";

    let text = format!(
        "{}{}\n(Enter to save, Esc back){}",
        name_label, app.custom_model_name, err_msg
    );

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, rect);

    let cursor_x = (rect.x + 1 + (name_label.len() + app.custom_model_name.len()) as u16)
        .min(rect.x + rect.width.saturating_sub(1));
    let cursor_y = rect.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}
