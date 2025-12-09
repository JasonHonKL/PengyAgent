pub mod read_file {
    //! Read file contents with optional line slicing. This is intended for quick
    //! inspection of files without modifying them.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    const DEFAULT_MAX_LINES: usize = 250;

    /// Simple file reader tool.
    pub struct ReadFileTool {
        tool: Tool,
    }

    impl ReadFileTool {
        /// Build the tool schema.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut target_items = HashMap::new();
            target_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "target_file".to_string(),
                Parameter {
                    items: target_items,
                    description: "Path to the file to read (absolute or relative).".to_string(),
                    enum_values: None,
                },
            );

            let mut read_entire_items = HashMap::new();
            read_entire_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "should_read_entire_file".to_string(),
                Parameter {
                    items: read_entire_items,
                    description: "If true, read the whole file (use carefully for large files)."
                        .to_string(),
                    enum_values: None,
                },
            );

            let mut start_items = HashMap::new();
            start_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "start_line_one_indexed".to_string(),
                Parameter {
                    items: start_items,
                    description: "1-based start line (inclusive) when reading a slice.".to_string(),
                    enum_values: None,
                },
            );

            let mut end_items = HashMap::new();
            end_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "end_line_one_indexed_inclusive".to_string(),
                Parameter {
                    items: end_items,
                    description: "1-based end line (inclusive) when reading a slice.".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "read_file".to_string(),
                description: "Read a file entirely or a line slice; if no range is given, the whole file is returned.".to_string(),
                parameters,
                required: vec!["target_file".to_string()],
            };

            Self { tool }
        }

        fn read_entire(path: &Path) -> Result<String, Box<dyn Error>> {
            Ok(fs::read_to_string(path)?)
        }

        fn read_slice(path: &Path, start: usize, end: usize) -> Result<String, Box<dyn Error>> {
            let content = fs::read_to_string(path)?;
            let lines: Vec<&str> = content.lines().collect();
            if lines.is_empty() {
                return Ok("File is empty.".to_string());
            }
            let start_idx = start.saturating_sub(1);
            let end_idx = end.min(lines.len());
            let slice = &lines[start_idx..end_idx];
            let mut output = Vec::new();
            for (offset, line) in slice.iter().enumerate() {
                let line_no = start_idx + offset + 1;
                output.push(format!("L{}:{}", line_no, line));
            }
            Ok(output.join("\n"))
        }
    }

    impl ToolCall for ReadFileTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let target = args
                .get("target_file")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: target_file")?;

            let path = Path::new(target);
            if !path.exists() {
                return Err(format!("File not found: {}", target).into());
            }
            if path.is_dir() {
                return Err(format!("Path is a directory, not a file: {}", target).into());
            }

            let read_entire = args
                .get("should_read_entire_file")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let start = args
                .get("start_line_one_indexed")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let end = args
                .get("end_line_one_indexed_inclusive")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            if read_entire || (start.is_none() && end.is_none()) {
                return Self::read_entire(path);
            }

            let start = start.unwrap_or(1);
            let mut end = end.unwrap_or(start + DEFAULT_MAX_LINES - 1);

            if end < start {
                end = start;
            }
            let end = (end - start + 1).min(DEFAULT_MAX_LINES) + start - 1;

            Self::read_slice(path, start, end)
        }

        fn name(&self) -> &str {
            "read_file"
        }
    }
}
