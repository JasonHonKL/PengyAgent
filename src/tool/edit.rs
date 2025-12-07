pub mod edit {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use serde_json;
    use std::error::Error;
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    pub struct EditTool {
        tool: Tool,
    }

    impl EditTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // filePath parameter (required)
            let mut file_path_items = HashMap::new();
            file_path_items.insert("type".to_string(), "string".to_string());
            parameters.insert("filePath".to_string(), Parameter {
                items: file_path_items,
                description: "Absolute path to the file to modify.".to_string(),
                enum_values: None,
            });

            // oldString parameter (required)
            let mut old_string_items = HashMap::new();
            old_string_items.insert("type".to_string(), "string".to_string());
            parameters.insert("oldString".to_string(), Parameter {
                items: old_string_items,
                description: "The text to replace.".to_string(),
                enum_values: None,
            });

            // newString parameter (required)
            let mut new_string_items = HashMap::new();
            new_string_items.insert("type".to_string(), "string".to_string());
            parameters.insert("newString".to_string(), Parameter {
                items: new_string_items,
                description: "The replacement text (must differ from oldString).".to_string(),
                enum_values: None,
            });

            // replaceAll parameter (optional)
            let mut replace_all_items = HashMap::new();
            replace_all_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert("replaceAll".to_string(), Parameter {
                items: replace_all_items,
                description: "Replace all occurrences of oldString (default false).".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "edit".to_string(),
                description: "Modifies existing files using exact string replacements with 9 fallback strategies for robust matching.".to_string(),
                parameters,
                required: vec!["filePath".to_string(), "oldString".to_string(), "newString".to_string()],
            };

            Self { tool }
        }

        fn normalize_line_endings(text: &str) -> String {
            text.replace("\r\n", "\n").replace("\r", "\n")
        }

        fn normalize_indentation(text: &str) -> String {
            // Convert tabs to 4 spaces
            text.replace("\t", "    ")
        }

        fn case_insensitive_match(content: &str, old_string: &str) -> Option<usize> {
            let content_lower = content.to_lowercase();
            let old_lower = old_string.to_lowercase();
            content_lower.find(&old_lower)
        }

        fn fuzzy_match(content: &str, old_string: &str, tolerance: usize) -> Option<usize> {
            if old_string.is_empty() {
                return None;
            }

            let old_chars: Vec<char> = old_string.chars().collect();
            let content_chars: Vec<char> = content.chars().collect();

            for i in 0..content_chars.len().saturating_sub(old_chars.len()) {
                let mut differences = 0;
                let mut matched = true;

                for j in 0..old_chars.len() {
                    if i + j >= content_chars.len() {
                        matched = false;
                        break;
                    }
                    if content_chars[i + j] != old_chars[j] {
                        differences += 1;
                        if differences > tolerance {
                            matched = false;
                            break;
                        }
                    }
                }

                if matched && differences <= tolerance {
                    return Some(i);
                }
            }

            None
        }

        fn find_match_with_fallbacks(content: &str, old_string: &str) -> Option<(usize, usize)> {
            // Strategy 1: Exact match
            if let Some(pos) = content.find(old_string) {
                return Some((pos, pos + old_string.len()));
            }

            // Strategy 2: Normalize line endings (CRLF/CR -> LF)
            let normalized_content = Self::normalize_line_endings(content);
            let normalized_old = Self::normalize_line_endings(old_string);
            if let Some(norm_pos) = normalized_content.find(&normalized_old) {
                // Map back to original: count characters up to position
                let original_pos = Self::map_normalized_to_original(content, &normalized_content, norm_pos);
                return Some((original_pos, original_pos + old_string.len()));
            }

            // Strategy 3: Case-insensitive match
            if let Some(pos) = Self::case_insensitive_match(content, old_string) {
                return Some((pos, pos + old_string.len()));
            }

            // Strategy 4: Normalize indentation (tabs -> spaces)
            let normalized_content = Self::normalize_indentation(content);
            let normalized_old = Self::normalize_indentation(old_string);
            if let Some(norm_pos) = normalized_content.find(&normalized_old) {
                let original_pos = Self::map_normalized_to_original(content, &normalized_content, norm_pos);
                return Some((original_pos, original_pos + old_string.len()));
            }

            // Strategy 5: Try matching with normalized whitespace (collapse multiple spaces)
            if let Some(pos) = Self::find_with_normalized_whitespace(content, old_string) {
                return Some(pos);
            }

            // Strategy 6: Try matching with trimmed lines
            if let Some(pos) = Self::find_with_trimmed_lines(content, old_string) {
                return Some(pos);
            }

            // Strategy 7: Try with different line ending styles
            for line_ending in &["\n", "\r\n", "\r"] {
                let content_with_le = content.replace("\r\n", "\n").replace("\r", "\n").replace("\n", line_ending);
                let old_with_le = old_string.replace("\r\n", "\n").replace("\r", "\n").replace("\n", line_ending);
                if let Some(pos) = content_with_le.find(&old_with_le) {
                    let original_pos = Self::map_normalized_to_original(content, &content_with_le, pos);
                    return Some((original_pos, original_pos + old_string.len()));
                }
            }

            // Strategy 8: Remove all whitespace and match
            if let Some(pos) = Self::find_ignoring_whitespace(content, old_string) {
                return Some(pos);
            }

            // Strategy 9: Fuzzy match (allow up to 5% character differences)
            let tolerance = (old_string.len() as f64 * 0.05).ceil() as usize;
            if let Some(pos) = Self::fuzzy_match(content, old_string, tolerance.max(1)) {
                return Some((pos, pos + old_string.len()));
            }

            None
        }

        fn map_normalized_to_original(original: &str, normalized: &str, norm_pos: usize) -> usize {
            // For simple normalizations (line endings, tabs), map character by character
            let mut orig_chars = 0;
            
            let orig_chars_vec: Vec<char> = original.chars().collect();
            let norm_chars_vec: Vec<char> = normalized.chars().collect();
            
            for (i, &norm_ch) in norm_chars_vec.iter().enumerate() {
                if i >= norm_pos {
                    break;
                }
                
                // Find corresponding character in original
                if orig_chars < orig_chars_vec.len() {
                    let orig_ch = orig_chars_vec[orig_chars];
                    // Handle line ending normalization
                    if norm_ch == '\n' && orig_ch == '\r' {
                        // Skip the \r, we'll count it on next iteration
                        continue;
                    }
                    // Handle tab normalization
                    if norm_ch == ' ' && orig_ch == '\t' {
                        // For tabs expanded to 4 spaces, we need to advance by 4 in normalized
                        // but only 1 in original. This is handled by the loop.
                        if i + 3 < norm_pos {
                            orig_chars += 1;
                            continue;
                        }
                    }
                    orig_chars += 1;
                }
            }
            
            // Convert character position to byte position
            let mut byte_pos = 0;
            for (i, (b, _)) in original.char_indices().enumerate() {
                if i >= orig_chars {
                    return byte_pos;
                }
                byte_pos = b;
            }
            original.len()
        }

        fn find_with_normalized_whitespace(content: &str, old_string: &str) -> Option<(usize, usize)> {
            // Try to find match by normalizing whitespace (collapse multiple spaces/tabs to single space)
            let content_lines: Vec<&str> = content.lines().collect();
            let old_lines: Vec<&str> = old_string.lines().collect();
            
            if old_lines.is_empty() {
                return None;
            }
            
            for (line_idx, _line) in content_lines.iter().enumerate() {
                if line_idx + old_lines.len() > content_lines.len() {
                    break;
                }
                
                // Check if lines match with normalized whitespace
                let mut matches = true;
                for (i, &old_line) in old_lines.iter().enumerate() {
                    let content_line = if line_idx + i < content_lines.len() {
                        content_lines[line_idx + i]
                    } else {
                        matches = false;
                        break;
                    };
                    
                    let norm_content: String = content_line.split_whitespace().collect::<Vec<&str>>().join(" ");
                    let norm_old: String = old_line.split_whitespace().collect::<Vec<&str>>().join(" ");
                    
                    if norm_content != norm_old {
                        matches = false;
                        break;
                    }
                }
                
                if matches {
                    // Find byte position of the start of the matching lines
                    let mut byte_pos = 0;
                    let mut line_count = 0;
                    for (pos, _) in content.match_indices('\n') {
                        if line_count == line_idx {
                            byte_pos = pos + 1;
                            break;
                        }
                        line_count += 1;
                    }
                    // Find the exact match within the line
                    if let Some(line_pos) = content[byte_pos..].find(old_string.lines().next().unwrap_or("")) {
                        return Some((byte_pos + line_pos, byte_pos + line_pos + old_string.len()));
                    }
                }
            }
            
            None
        }

        fn find_with_trimmed_lines(content: &str, old_string: &str) -> Option<(usize, usize)> {
            // Try to find match by trimming leading/trailing whitespace from each line
            let content_lines: Vec<&str> = content.lines().collect();
            let old_lines: Vec<&str> = old_string.lines().collect();
            
            if old_lines.is_empty() {
                return None;
            }
            
            for (line_idx, _line) in content_lines.iter().enumerate() {
                if line_idx + old_lines.len() > content_lines.len() {
                    break;
                }
                
                let mut matches = true;
                for (i, &old_line) in old_lines.iter().enumerate() {
                    let content_line = if line_idx + i < content_lines.len() {
                        content_lines[line_idx + i]
                    } else {
                        matches = false;
                        break;
                    };
                    
                    if content_line.trim() != old_line.trim() {
                        matches = false;
                        break;
                    }
                }
                
                if matches {
                    // Find byte position
                    let mut byte_pos = 0;
                    let mut line_count = 0;
                    for (pos, _) in content.match_indices('\n') {
                        if line_count == line_idx {
                            byte_pos = pos + 1;
                            break;
                        }
                        line_count += 1;
                    }
                    if let Some(line_pos) = content[byte_pos..].find(old_string.lines().next().unwrap_or("")) {
                        return Some((byte_pos + line_pos, byte_pos + line_pos + old_string.len()));
                    }
                }
            }
            
            None
        }

        fn find_ignoring_whitespace(content: &str, old_string: &str) -> Option<(usize, usize)> {
            // Remove all whitespace and try to match
            let content_no_ws: Vec<char> = content.chars().filter(|c| !c.is_whitespace()).collect();
            let old_no_ws: Vec<char> = old_string.chars().filter(|c| !c.is_whitespace()).collect();
            
            if old_no_ws.is_empty() || content_no_ws.len() < old_no_ws.len() {
                return None;
            }
            
            // Try to find the pattern
            for i in 0..=content_no_ws.len() - old_no_ws.len() {
                if content_no_ws[i..i + old_no_ws.len()] == old_no_ws[..] {
                    // Map back to original positions
                    let start_pos = Self::find_nth_non_whitespace_char(content, i);
                    let end_pos = Self::find_nth_non_whitespace_char(content, i + old_no_ws.len());
                    return Some((start_pos, end_pos));
                }
            }
            
            None
        }

        fn find_nth_non_whitespace_char(text: &str, n: usize) -> usize {
            let mut count = 0;
            for (byte_pos, ch) in text.char_indices() {
                if !ch.is_whitespace() {
                    if count == n {
                        return byte_pos;
                    }
                    count += 1;
                }
            }
            text.len()
        }

        fn execute_edit(&self, file_path: &str, old_string: &str, new_string: &str, replace_all: bool) -> Result<String, Box<dyn Error>> {
            // Validate that oldString and newString are different
            if old_string == new_string {
                return Err("oldString and newString must be different".into());
            }

            // Check if file exists
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(format!("File does not exist: {}", file_path).into());
            }

            // Read file content
            let content = fs::read_to_string(file_path)?;

            if replace_all {
                // Replace all occurrences
                let mut modified_content = content.clone();
                let mut replacements = 0;
                let mut search_pos = 0;

                loop {
                    let remaining = &modified_content[search_pos..];
                    if let Some((start, end)) = Self::find_match_with_fallbacks(remaining, old_string) {
                        let actual_start = search_pos + start;
                        let actual_end = search_pos + end;
                        modified_content.replace_range(actual_start..actual_end, new_string);
                        replacements += 1;
                        search_pos = actual_start + new_string.len();
                    } else {
                        break;
                    }
                }

                if replacements == 0 {
                    return Err(format!("No matches found for oldString in file: {}", file_path).into());
                }

                fs::write(file_path, &modified_content)?;
                Ok(format!("Successfully replaced {} occurrence(s) in {}", replacements, file_path))
            } else {
                // Replace first occurrence only
                if let Some((start, end)) = Self::find_match_with_fallbacks(&content, old_string) {
                    let mut modified_content = content;
                    modified_content.replace_range(start..end, new_string);
                    fs::write(file_path, &modified_content)?;
                    Ok(format!("Successfully replaced first occurrence in {}", file_path))
                } else {
                    Err(format!("No match found for oldString in file: {}", file_path).into())
                }
            }
        }
    }

    impl ToolCall for EditTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // Get required parameters
            let file_path = args.get("filePath")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: filePath")?;

            let old_string = args.get("oldString")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: oldString")?;

            let new_string = args.get("newString")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: newString")?;

            // Get optional parameter
            let replace_all = args.get("replaceAll")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Execute the edit
            self.execute_edit(file_path, old_string, new_string, replace_all)
        }

        fn name(&self) -> &str {
            "edit"
        }
    }
}

