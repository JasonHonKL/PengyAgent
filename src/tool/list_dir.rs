pub mod list_dir {
    //! List directory contents with optional hidden filtering and entry limits.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    const DEFAULT_MAX_ENTRIES: usize = 200;

    /// Lists entries in a directory.
    pub struct ListDirTool {
        tool: Tool,
    }

    impl ListDirTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut path_items = HashMap::new();
            path_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "target_directory".to_string(),
                Parameter {
                    items: path_items,
                    description: "Directory to list (absolute or relative).".to_string(),
                    enum_values: None,
                },
            );

            let mut include_hidden_items = HashMap::new();
            include_hidden_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "include_hidden".to_string(),
                Parameter {
                    items: include_hidden_items,
                    description: "Include entries starting with '.' (default false).".to_string(),
                    enum_values: None,
                },
            );

            let mut max_items = HashMap::new();
            max_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "max_entries".to_string(),
                Parameter {
                    items: max_items,
                    description: "Maximum entries to display (default 200).".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "list_dir".to_string(),
                description: "List directory contents with optional hidden filtering and limits.".to_string(),
                parameters,
                required: vec!["target_directory".to_string()],
            };

            Self { tool }
        }
    }

    impl ToolCall for ListDirTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let target = args
                .get("target_directory")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: target_directory")?;

            let include_hidden = args
                .get("include_hidden")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let max_entries = args
                .get("max_entries")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(DEFAULT_MAX_ENTRIES);

            let path = Path::new(target);
            if !path.exists() {
                return Err(format!("Directory not found: {}", target).into());
            }
            if !path.is_dir() {
                return Err(format!("Path is not a directory: {}", target).into());
            }

            let mut entries = Vec::new();
            for entry in fs::read_dir(path)? {
                if entries.len() >= max_entries {
                    break;
                }
                if let Ok(entry) = entry {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !include_hidden && name_str.starts_with('.') {
                        continue;
                    }
                    let meta = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let suffix = if meta.is_dir() { "/" } else { "" };
                    entries.push(format!("{}{}", name_str, suffix));
                }
            }

            entries.sort();

            Ok(entries.join("\n"))
        }

        fn name(&self) -> &str {
            "list_dir"
        }
    }
}

