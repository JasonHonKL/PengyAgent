use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Debug)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
    FileExplorer,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub name: String,
}

pub struct EditorState {
    pub mode: EditorMode,
    pub file_path: Option<PathBuf>,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
    pub command_buffer: String,
    pub show_command: bool,
    pub status_message: Option<String>,
    pub status_message_timer: u32,
    pub file_explorer_open: bool,
    pub file_explorer_path: PathBuf,
    pub file_explorer_entries: Vec<FileEntry>,
    pub file_explorer_selected: usize,
    pub file_explorer_scroll: usize,
    pub line_numbers_width: usize,
    pub definition_cache: HashMap<String, Vec<(PathBuf, usize, String)>>, // symbol -> (file, line, context)
    pub show_welcome: bool,
    pub welcome_selected: usize,
    pub recent_files: Vec<(PathBuf, String)>, // (path, display_name)
}

impl EditorState {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        
        Self {
            mode: EditorMode::Normal,
            file_path: None,
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            horizontal_scroll: 0,
            command_buffer: String::new(),
            show_command: false,
            status_message: None,
            status_message_timer: 0,
            file_explorer_open: false,
            file_explorer_path: current_dir.clone(),
            file_explorer_entries: Vec::new(),
            file_explorer_selected: 0,
            file_explorer_scroll: 0,
            line_numbers_width: 4,
            definition_cache: HashMap::new(),
            show_welcome: true,
            welcome_selected: 0,
            recent_files: Self::load_recent_files(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> Result<(), String> {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.file_path = Some(path.clone());
                self.lines = if content.is_empty() {
                    vec![String::new()]
                } else {
                    content.lines().map(|s| s.to_string()).collect()
                };
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.scroll_offset = 0;
                self.horizontal_scroll = 0;
                self.update_line_numbers_width();
                self.show_welcome = false;
                
                // Add to recent files
                self.add_to_recent_files(path);
                
                Ok(())
            }
            Err(e) => Err(format!("Failed to open file: {}", e)),
        }
    }
    
    fn load_recent_files() -> Vec<(PathBuf, String)> {
        // Load from a simple file or return empty for now
        // Could be enhanced to persist to disk
        Vec::new()
    }
    
    fn add_to_recent_files(&mut self, path: PathBuf) {
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.display().to_string());
        
        // Remove if already exists
        self.recent_files.retain(|(p, _)| *p != path);
        
        // Add to front
        self.recent_files.insert(0, (path, display_name));
        
        // Keep only last 10
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
    }

    pub fn save_file(&self) -> Result<(), String> {
        if let Some(ref path) = self.file_path {
            let content = self.lines.join("\n");
            std::fs::write(path, content)
                .map_err(|e| format!("Failed to save file: {}", e))?;
            Ok(())
        } else {
            Err("No file path set".to_string())
        }
    }

    pub fn update_line_numbers_width(&mut self) {
        let max_line = self.lines.len();
        self.line_numbers_width = if max_line == 0 {
            4
        } else {
            (max_line as f64).log10().floor() as usize + 2
        }.max(4);
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_row];
        let col = self.cursor_col.min(line.len());
        line.insert(col, c);
        self.cursor_col += 1;
        self.update_line_numbers_width();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            if self.cursor_col <= line.len() {
                line.remove(self.cursor_col - 1);
                self.cursor_col -= 1;
            }
        } else if self.cursor_col == 0 && self.cursor_row > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            let prev_line_len = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.cursor_col = prev_line_len;
        }
        self.update_line_numbers_width();
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }
        if self.cursor_col < self.lines[self.cursor_row].len() {
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_col == self.lines[self.cursor_row].len() && self.cursor_row < self.lines.len() - 1 {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
        }
        self.update_line_numbers_width();
    }

    pub fn insert_newline(&mut self) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_row];
        let col = self.cursor_col.min(line.len());
        let remainder = line[col..].to_string();
        line.truncate(col);
        self.lines.insert(self.cursor_row + 1, remainder);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.update_line_numbers_width();
    }

    pub fn move_left(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }
        
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            // Find the previous character boundary
            let mut pos = self.cursor_col;
            while pos > 0 && !line.is_char_boundary(pos) {
                pos -= 1;
            }
            if pos > 0 {
                // Find start of previous character
                let mut prev = pos - 1;
                while prev > 0 && !line.is_char_boundary(prev) {
                    prev -= 1;
                }
                self.cursor_col = prev;
            } else {
                self.cursor_col = 0;
            }
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let prev_line = &self.lines[self.cursor_row];
            self.cursor_col = prev_line.len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }
        let line = &self.lines[self.cursor_row];
        let line_len = line.chars().count();
        let cursor_char_pos = self.cursor_col.min(line_len);
        
        if cursor_char_pos < line_len {
            // Move to next character
            let mut char_count = 0;
            for (idx, _) in line.char_indices() {
                if char_count == cursor_char_pos {
                    // Find the byte position of the next character
                    let next_char_start = if idx + 1 < line.len() {
                        // Find start of next character
                        let mut next = idx + 1;
                        while next < line.len() && !line.is_char_boundary(next) {
                            next += 1;
                        }
                        next
                    } else {
                        line.len()
                    };
                    self.cursor_col = next_char_start;
                    break;
                }
                char_count += 1;
            }
        } else if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let line = &self.lines[self.cursor_row];
            // Preserve character position, not byte position
            let target_char_pos = {
                let current_line = if self.cursor_row + 1 < self.lines.len() {
                    &self.lines[self.cursor_row + 1]
                } else {
                    return;
                };
                current_line[..self.cursor_col.min(current_line.len())].chars().count()
            };
            
            // Convert character position to byte position in new line
            let mut char_count = 0;
            let mut byte_pos = 0;
            for (idx, _) in line.char_indices() {
                if char_count >= target_char_pos {
                    byte_pos = idx;
                    break;
                }
                char_count += 1;
            }
            if char_count < target_char_pos {
                byte_pos = line.len();
            }
            self.cursor_col = byte_pos;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row < self.lines.len().saturating_sub(1) {
            // Save current character position
            let current_line = &self.lines[self.cursor_row];
            let target_char_pos = current_line[..self.cursor_col.min(current_line.len())].chars().count();
            
            self.cursor_row += 1;
            if self.cursor_row < self.lines.len() {
                let line = &self.lines[self.cursor_row];
                // Convert character position to byte position
                let mut char_count = 0;
                let mut byte_pos = 0;
                for (idx, _) in line.char_indices() {
                    if char_count >= target_char_pos {
                        byte_pos = idx;
                        break;
                    }
                    char_count += 1;
                }
                if char_count < target_char_pos {
                    byte_pos = line.len();
                }
                self.cursor_col = byte_pos;
            }
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
        self.horizontal_scroll = 0;
    }

    pub fn move_to_line_end(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = line.len();
        }
    }

    pub fn move_to_file_start(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.horizontal_scroll = 0;
    }

    pub fn move_to_file_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_row = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    pub fn ensure_cursor_visible(&mut self, width: usize, height: usize) {
        // Vertical scrolling
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + height {
            self.scroll_offset = self.cursor_row.saturating_sub(height.saturating_sub(1));
        }
        
        // Horizontal scrolling - need to calculate based on character positions
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            let available_width = width.saturating_sub(self.line_numbers_width + 2);
            
            // Calculate character position of cursor
            let cursor_char_pos = line[..self.cursor_col.min(line.len())].chars().count();
            
            // Calculate width up to cursor (for reference, but we use char_pos for scrolling)
            
            // Adjust horizontal scroll
            if cursor_char_pos < self.horizontal_scroll {
                self.horizontal_scroll = cursor_char_pos.saturating_sub(5);
            } else {
                // Calculate how many characters fit in available width
                let mut display_width = 0;
                let mut char_count = self.horizontal_scroll;
                for ch in line.chars().skip(self.horizontal_scroll) {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                    if display_width + ch_width > available_width {
                        break;
                    }
                    display_width += ch_width;
                    char_count += 1;
                }
                
                if cursor_char_pos >= char_count {
                    // Cursor is beyond visible area, scroll right
                    self.horizontal_scroll = cursor_char_pos.saturating_sub(available_width.saturating_sub(10));
                }
            }
        }
    }

    pub fn get_current_line(&self) -> &str {
        if self.cursor_row < self.lines.len() {
            &self.lines[self.cursor_row]
        } else {
            ""
        }
    }

    pub fn get_word_under_cursor(&self) -> Option<String> {
        if self.cursor_row >= self.lines.len() {
            return None;
        }
        let line = &self.lines[self.cursor_row];
        if self.cursor_col >= line.len() {
            return None;
        }
        
        let chars: Vec<char> = line.chars().collect();
        let mut start = self.cursor_col;
        let mut end = self.cursor_col;
        
        // Find start of word
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        
        // Find end of word
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    pub fn refresh_file_explorer(&mut self) {
        let mut entries = Vec::new();
        
        // Add parent directory
        if let Some(parent) = self.file_explorer_path.parent() {
            entries.push(FileEntry {
                path: parent.to_path_buf(),
                is_dir: true,
                name: "..".to_string(),
            });
        }
        
        // Read directory entries
        if let Ok(read_dir) = std::fs::read_dir(&self.file_explorer_path) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            
            for entry in read_dir.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string();
                    
                    if metadata.is_dir() {
                        dirs.push(FileEntry {
                            path: path.clone(),
                            is_dir: true,
                            name,
                        });
                    } else {
                        files.push(FileEntry {
                            path,
                            is_dir: false,
                            name,
                        });
                    }
                }
            }
            
            dirs.sort_by(|a, b| a.name.cmp(&b.name));
            files.sort_by(|a, b| a.name.cmp(&b.name));
            
            entries.extend(dirs);
            entries.extend(files);
        }
        
        self.file_explorer_entries = entries;
        if self.file_explorer_selected >= self.file_explorer_entries.len() {
            self.file_explorer_selected = 0;
        }
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
