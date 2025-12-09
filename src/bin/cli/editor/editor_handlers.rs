use crate::editor::editor::EditorMode;
use crossterm::event::KeyCode;
use std::error::Error;
use std::process::Command;

pub fn handle_editor_key(
    app: &mut crate::app::App,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    // Handle welcome screen
    if app.editor_state.show_welcome && app.editor_state.file_path.is_none() {
        return handle_welcome_screen(&mut app.editor_state, key);
    }
    
    // Extract values we need before mutable borrows
    let mode = app.editor_state.mode.clone();
    let cmd_buffer = if mode == EditorMode::Command && key == KeyCode::Enter {
        app.editor_state.command_buffer.clone()
    } else {
        String::new()
    };
    let should_switch_to_agent = cmd_buffer.trim() == "agent";
    
    // Now handle the key with mutable borrow
    let result = match mode {
        EditorMode::Normal => {
            handle_normal_mode(&mut app.editor_state, key)
        }
        EditorMode::Insert => {
            handle_insert_mode(&mut app.editor_state, key)
        }
        EditorMode::Command => {
            handle_command_mode(&mut app.editor_state, key)
        }
        EditorMode::FileExplorer => {
            handle_file_explorer_mode(&mut app.editor_state, key)
        }
    }?;
    
    // Switch to agent mode if needed (after all borrows are done)
    if should_switch_to_agent {
        app.state = crate::app::AppState::Chat;
    }
    
    // Update status message timer
    if app.editor_state.status_message.is_some() {
        app.editor_state.status_message_timer += 1;
        if app.editor_state.status_message_timer > 100 {
            app.editor_state.status_message = None;
            app.editor_state.status_message_timer = 0;
        }
    }
    
    Ok(result)
}

fn handle_welcome_screen(
    editor: &mut crate::editor::editor::EditorState,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            return Err("quit".into());
        }
        KeyCode::Char('f') => {
            // Open file explorer
            editor.file_explorer_open = true;
            editor.mode = EditorMode::FileExplorer;
            editor.show_welcome = false;
            editor.refresh_file_explorer();
        }
        KeyCode::Char('n') => {
            // Create new file - prompt for filename
            editor.mode = EditorMode::Command;
            editor.command_buffer = "e ".to_string();
            editor.show_command = true;
            editor.show_welcome = false;
        }
        KeyCode::Char('g') => {
            // Find text - go to command mode with grep
            editor.mode = EditorMode::Command;
            editor.command_buffer = "grep ".to_string();
            editor.show_command = true;
            editor.show_welcome = false;
        }
        KeyCode::Char('r') => {
            // Recent files - open file explorer at current dir
            if !editor.recent_files.is_empty() {
                editor.file_explorer_open = true;
                editor.mode = EditorMode::FileExplorer;
                editor.show_welcome = false;
                editor.refresh_file_explorer();
            }
        }
        KeyCode::Char('c') => {
            // Config - open config file if exists
            let config_path = std::path::PathBuf::from(".pengy_editor_config");
            if config_path.exists() {
                let _ = editor.open_file(config_path);
            } else {
                // Create empty config
                editor.mode = EditorMode::Command;
                editor.command_buffer = format!("e {}", config_path.display());
                editor.show_command = true;
                editor.show_welcome = false;
            }
        }
        KeyCode::Char(':') => {
            // Command mode
            editor.mode = EditorMode::Command;
            editor.command_buffer.clear();
            editor.show_command = true;
            editor.show_welcome = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if editor.welcome_selected > 0 {
                editor.welcome_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if editor.welcome_selected < 5 {
                editor.welcome_selected += 1;
            }
        }
        KeyCode::Enter => {
            // Execute selected menu item
            match editor.welcome_selected {
                0 => {
                    // Find File
                    editor.file_explorer_open = true;
                    editor.mode = EditorMode::FileExplorer;
                    editor.show_welcome = false;
                    editor.refresh_file_explorer();
                }
                1 => {
                    // New File
                    editor.mode = EditorMode::Command;
                    editor.command_buffer = "e ".to_string();
                    editor.show_command = true;
                    editor.show_welcome = false;
                }
                2 => {
                    // Find Text
                    editor.mode = EditorMode::Command;
                    editor.command_buffer = "grep ".to_string();
                    editor.show_command = true;
                    editor.show_welcome = false;
                }
                3 => {
                    // Recent Files
                    if !editor.recent_files.is_empty() {
                        editor.file_explorer_open = true;
                        editor.mode = EditorMode::FileExplorer;
                        editor.show_welcome = false;
                        editor.refresh_file_explorer();
                    }
                }
                4 => {
                    // Config
                    let config_path = std::path::PathBuf::from(".pengy_editor_config");
                    if config_path.exists() {
                        let _ = editor.open_file(config_path);
                    } else {
                        editor.mode = EditorMode::Command;
                        editor.command_buffer = format!("e {}", config_path.display());
                        editor.show_command = true;
                        editor.show_welcome = false;
                    }
                }
                5 => {
                    // Quit
                    return Err("quit".into());
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_normal_mode(
    editor: &mut crate::editor::editor::EditorState,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    match key {
        KeyCode::Esc => return Err("quit".into()),
        KeyCode::Char('i') => {
            editor.mode = EditorMode::Insert;
        }
        KeyCode::Char('a') => {
            editor.mode = EditorMode::Insert;
            editor.move_right();
        }
        KeyCode::Char('A') => {
            editor.mode = EditorMode::Insert;
            editor.move_to_line_end();
        }
        KeyCode::Char('o') => {
            if editor.cursor_row < editor.lines.len() {
                editor.lines.insert(editor.cursor_row + 1, String::new());
            } else {
                editor.lines.push(String::new());
            }
            editor.cursor_row += 1;
            editor.cursor_col = 0;
            editor.mode = EditorMode::Insert;
        }
        KeyCode::Char('O') => {
            editor.lines.insert(editor.cursor_row, String::new());
            editor.cursor_col = 0;
            editor.mode = EditorMode::Insert;
        }
        KeyCode::Char(':') => {
            editor.mode = EditorMode::Command;
            editor.command_buffer.clear();
            editor.show_command = true;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            editor.move_left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            editor.move_right();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            editor.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            editor.move_up();
        }
        KeyCode::Char('0') => {
            editor.move_to_line_start();
        }
        KeyCode::Char('$') => {
            editor.move_to_line_end();
        }
        KeyCode::Char('g') => {
            // 'gg' to go to top - simplified for now
            editor.move_to_file_start();
        }
        KeyCode::Char('G') => {
            editor.move_to_file_end();
        }
        KeyCode::Char('x') => {
            editor.delete_char_forward();
        }
        KeyCode::Char('d') => {
            // 'dd' to delete line
            if editor.cursor_row < editor.lines.len() {
                editor.lines.remove(editor.cursor_row);
                if editor.lines.is_empty() {
                    editor.lines.push(String::new());
                }
                if editor.cursor_row >= editor.lines.len() {
                    editor.cursor_row = editor.lines.len().saturating_sub(1);
                }
                editor.cursor_col = editor.cursor_col.min(editor.lines[editor.cursor_row].len());
            }
        }
        KeyCode::Char('w') => {
            // Save file
            match editor.save_file() {
                Ok(_) => {
                    editor.status_message = Some("✓ Saved".to_string());
                    editor.status_message_timer = 0;
                }
                Err(e) => {
                    editor.status_message = Some(format!("✗ Error: {}", e));
                    editor.status_message_timer = 0;
                }
            }
        }
        KeyCode::Char('e') => {
            // Toggle file explorer
            editor.file_explorer_open = !editor.file_explorer_open;
            if editor.file_explorer_open {
                editor.mode = EditorMode::FileExplorer;
                editor.refresh_file_explorer();
            }
        }
        KeyCode::F(12) => {
            // F12 - go to definition
            if let Some(word) = editor.get_word_under_cursor() {
                go_to_definition(editor, &word);
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_insert_mode(
    editor: &mut crate::editor::editor::EditorState,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    match key {
        KeyCode::Esc => {
            editor.mode = EditorMode::Normal;
        }
        KeyCode::Enter => {
            editor.insert_newline();
        }
        KeyCode::Backspace => {
            editor.delete_char();
        }
        KeyCode::Delete => {
            editor.delete_char_forward();
        }
        KeyCode::Left => {
            editor.move_left();
        }
        KeyCode::Right => {
            editor.move_right();
        }
        KeyCode::Up => {
            editor.move_up();
        }
        KeyCode::Down => {
            editor.move_down();
        }
        KeyCode::Home => {
            editor.move_to_line_start();
        }
        KeyCode::End => {
            editor.move_to_line_end();
        }
        KeyCode::Char(c) => {
            editor.insert_char(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_command_mode(
    editor: &mut crate::editor::editor::EditorState,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    match key {
        KeyCode::Esc => {
            editor.mode = EditorMode::Normal;
            editor.command_buffer.clear();
            editor.show_command = false;
        }
        KeyCode::Enter => {
            let cmd = editor.command_buffer.clone();
            let cmd_trimmed = cmd.trim();
            editor.command_buffer.clear();
            editor.show_command = false;
            
            if cmd_trimmed == "agent" {
                // State change will be handled by caller
                editor.mode = EditorMode::Normal;
                return Ok(false);
            } else if cmd_trimmed.starts_with("w") || cmd_trimmed == "write" {
                match editor.save_file() {
                    Ok(_) => {
                        editor.status_message = Some("✓ Saved".to_string());
                        editor.status_message_timer = 0;
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("✗ Error: {}", e));
                        editor.status_message_timer = 0;
                    }
                }
                editor.mode = EditorMode::Normal;
            } else if cmd_trimmed.starts_with("q") || cmd_trimmed == "quit" {
                return Err("quit".into());
            } else if cmd_trimmed.starts_with("wq") {
                match editor.save_file() {
                    Ok(_) => {
                        return Err("quit".into());
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("✗ Error: {}", e));
                        editor.status_message_timer = 0;
                        editor.mode = EditorMode::Normal;
                    }
                }
            } else if cmd_trimmed.starts_with("e ") {
                // Open file: e <filename>
                let path = cmd_trimmed[2..].trim();
                let path_buf = std::path::PathBuf::from(path);
                match editor.open_file(path_buf) {
                    Ok(_) => {
                        editor.status_message = Some(format!("✓ Opened: {}", path));
                        editor.status_message_timer = 0;
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("✗ Error: {}", e));
                        editor.status_message_timer = 0;
                    }
                }
                editor.mode = EditorMode::Normal;
            } else if cmd_trimmed.starts_with("gd ") {
                // Go to definition: gd <symbol>
                let symbol = cmd_trimmed[3..].trim();
                go_to_definition(editor, symbol);
                editor.mode = EditorMode::Normal;
            } else if !cmd_trimmed.is_empty() {
                editor.status_message = Some(format!("Unknown command: {}", cmd_trimmed));
                editor.status_message_timer = 0;
                editor.mode = EditorMode::Normal;
            } else {
                editor.mode = EditorMode::Normal;
            }
        }
        KeyCode::Backspace => {
            editor.command_buffer.pop();
        }
        KeyCode::Char(c) => {
            editor.command_buffer.push(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_file_explorer_mode(
    editor: &mut crate::editor::editor::EditorState,
    key: KeyCode,
) -> Result<bool, Box<dyn Error>> {
    match key {
        KeyCode::Esc | KeyCode::Char('e') => {
            editor.file_explorer_open = false;
            editor.mode = EditorMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if editor.file_explorer_selected < editor.file_explorer_entries.len().saturating_sub(1) {
                editor.file_explorer_selected += 1;
                // Scroll if needed
                let visible_height = 20; // Approximate
                if editor.file_explorer_selected >= editor.file_explorer_scroll + visible_height {
                    editor.file_explorer_scroll = editor.file_explorer_selected.saturating_sub(visible_height - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if editor.file_explorer_selected > 0 {
                editor.file_explorer_selected -= 1;
                if editor.file_explorer_selected < editor.file_explorer_scroll {
                    editor.file_explorer_scroll = editor.file_explorer_selected;
                }
            }
        }
        KeyCode::Enter => {
            let selected_idx = editor.file_explorer_selected;
            if selected_idx < editor.file_explorer_entries.len() {
                let entry = editor.file_explorer_entries[selected_idx].clone();
                let path = entry.path.clone();
                let is_dir = entry.is_dir;
                let name = entry.name.clone();
                
                if is_dir {
                    editor.file_explorer_path = path;
                    editor.file_explorer_selected = 0;
                    editor.file_explorer_scroll = 0;
                    editor.refresh_file_explorer();
                } else {
                    // Open file
                    match editor.open_file(path) {
                        Ok(_) => {
                            editor.file_explorer_open = false;
                            editor.mode = EditorMode::Normal;
                            editor.status_message = Some(format!("✓ Opened: {}", name));
                            editor.status_message_timer = 0;
                        }
                        Err(e) => {
                            editor.status_message = Some(format!("✗ Error: {}", e));
                            editor.status_message_timer = 0;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

fn go_to_definition(editor: &mut crate::editor::editor::EditorState, symbol: &str) {
    // Check cache first - extract values to avoid borrow conflicts
    let cached_result = editor.definition_cache.get(symbol)
        .and_then(|defs| defs.first())
        .map(|(file, line, _)| (file.clone(), *line));
    
    if let Some((file, line)) = cached_result {
        if editor.open_file(file.clone()).is_ok() {
            editor.cursor_row = line.saturating_sub(1);
            editor.cursor_col = 0;
            editor.status_message = Some(format!("→ Found: {}:{}", file.display(), line));
            editor.status_message_timer = 0;
            return;
        }
    }
    
    // Use grep to find definition
    let current_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    
    // Try to find function/struct/class definitions
    let patterns = vec![
        format!(r"fn\s+{}", symbol),
        format!(r"struct\s+{}", symbol),
        format!(r"class\s+{}", symbol),
        format!(r"impl\s+{}", symbol),
        format!(r"trait\s+{}", symbol),
        format!(r"enum\s+{}", symbol),
        format!(r"const\s+{}", symbol),
        format!(r"let\s+{}", symbol),
    ];
    
    for pattern in patterns {
        if let Ok(output) = Command::new("rg")
            .arg("--line-number")
            .arg("--no-heading")
            .arg(&pattern)
            .current_dir(&current_dir)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = stdout.lines().next() {
                    // Parse: file:line:content
                    let parts: Vec<&str> = first_line.splitn(3, ':').collect();
                    if parts.len() >= 2 {
                        if let Ok(line_num) = parts[1].parse::<usize>() {
                            let file_path = current_dir.join(parts[0]);
                            if let Ok(_) = editor.open_file(file_path.clone()) {
                                editor.cursor_row = line_num.saturating_sub(1);
                                editor.cursor_col = 0;
                                editor.status_message = Some(format!("→ Found: {}:{}", parts[0], line_num));
                                editor.status_message_timer = 0;
                                
                                // Cache the result
                                let cache_entry = (file_path.clone(), line_num, first_line.to_string());
                                editor.definition_cache.insert(
                                    symbol.to_string(),
                                    vec![cache_entry],
                                );
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: simple grep
    if let Ok(output) = Command::new("rg")
        .arg("--line-number")
        .arg("--no-heading")
        .arg(symbol)
        .current_dir(&current_dir)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let parts: Vec<&str> = first_line.splitn(3, ':').collect();
                if parts.len() >= 2 {
                    if let Ok(line_num) = parts[1].parse::<usize>() {
                        let file_path = current_dir.join(parts[0]);
                        if let Ok(_) = editor.open_file(file_path.clone()) {
                            editor.cursor_row = line_num.saturating_sub(1);
                            editor.cursor_col = 0;
                            editor.status_message = Some(format!("→ Found: {}:{}", parts[0], line_num));
                            editor.status_message_timer = 0;
                            return;
                        }
                    }
                }
            }
        }
    }
    
    editor.status_message = Some(format!("✗ Not found: {}", symbol));
    editor.status_message_timer = 0;
}
