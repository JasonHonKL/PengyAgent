pub mod file_search {
    //! Fuzzy-ish search for files by name substring across the workspace.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    const DEFAULT_MAX_RESULTS: usize = 50;

    /// Searches for files whose paths contain the given query substring
    /// (case-insensitive).
    pub struct FileSearchTool {
        tool: Tool,
    }

    impl FileSearchTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut query_items = HashMap::new();
            query_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "query".to_string(),
                Parameter {
                    items: query_items,
                    description: "Substring to look for in file paths (case-insensitive)."
                        .to_string(),
                    enum_values: None,
                },
            );

            let mut root_items = HashMap::new();
            root_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "root".to_string(),
                Parameter {
                    items: root_items,
                    description:
                        "Optional root directory to search (default: current working directory)."
                            .to_string(),
                    enum_values: None,
                },
            );

            let mut max_results_items = HashMap::new();
            max_results_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "maxResults".to_string(),
                Parameter {
                    items: max_results_items,
                    description: "Maximum number of results (default 50).".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "file_search".to_string(),
                description: "Find files whose paths contain a given substring (case-insensitive)."
                    .to_string(),
                parameters,
                required: vec!["query".to_string()],
            };

            Self { tool }
        }

        fn is_ignored(path: &Path) -> bool {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                matches!(
                    name,
                    ".git"
                        | "target"
                        | "node_modules"
                        | ".idea"
                        | ".vscode"
                        | ".svn"
                        | ".hg"
                        | "dist"
                        | "build"
                        | "__pycache__"
                )
            } else {
                false
            }
        }

        fn walk(root: PathBuf, query_lower: &str, max_results: usize) -> Vec<String> {
            let mut results = Vec::new();
            let mut stack = vec![root];
            while let Some(path) = stack.pop() {
                if results.len() >= max_results {
                    break;
                }
                if Self::is_ignored(&path) {
                    continue;
                }

                if let Ok(meta) = fs::metadata(&path) {
                    if meta.is_dir() {
                        if let Ok(entries) = fs::read_dir(&path) {
                            for entry in entries.flatten() {
                                stack.push(entry.path());
                            }
                        }
                    } else if meta.is_file() {
                        if let Some(path_str) = path.to_str() {
                            if path_str.to_lowercase().contains(query_lower) {
                                results.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
            results
        }
    }

    impl ToolCall for FileSearchTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: query")?;
            let root = args.get("root").and_then(|v| v.as_str()).unwrap_or(".");
            let max_results = args
                .get("maxResults")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(DEFAULT_MAX_RESULTS)
                .max(1);

            let root_path = PathBuf::from(root);
            if !root_path.exists() {
                return Err(format!("Root path does not exist: {}", root).into());
            }

            let results = Self::walk(root_path, &query.to_lowercase(), max_results);
            if results.is_empty() {
                Ok(format!(
                    "No files matched '{}' (limit {}).",
                    query, max_results
                ))
            } else {
                Ok(results.join("\n"))
            }
        }

        fn name(&self) -> &str {
            "file_search"
        }
    }
}
