pub mod codebase_search {
    //! Lightweight semantic-like search across the workspace. This scans text
    //! files for a query substring (case-insensitive) and returns matching lines
    //! with file paths and line numbers. It is intentionally simple to avoid
    //! heavy dependencies.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};

    const DEFAULT_MAX_RESULTS: usize = 20;
    const MAX_FILE_SIZE_BYTES: u64 = 512 * 1024; // skip very large files

    /// Performs a substring search across the workspace.
    pub struct CodebaseSearchTool {
        tool: Tool,
    }

    impl CodebaseSearchTool {
        /// Build the tool schema.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut query_items = HashMap::new();
            query_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "query".to_string(),
                Parameter {
                    items: query_items,
                    description: "Search query string (case-insensitive substring match)."
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
                    description: "Maximum number of matches to return (default 20).".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "codebase_search".to_string(),
                description: "Search text files for a query (case-insensitive substring) and return matching lines with file paths and numbers.".to_string(),
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
                        | ".svn"
                        | ".hg"
                        | "target"
                        | "node_modules"
                        | "dist"
                        | "build"
                        | "__pycache__"
                        | ".idea"
                        | ".vscode"
                )
            } else {
                false
            }
        }

        fn search_file(
            path: &Path,
            query_lower: &str,
            max_results: usize,
            results: &mut Vec<String>,
        ) -> Result<(), Box<dyn Error>> {
            // Skip huge files to avoid expensive reads
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() > MAX_FILE_SIZE_BYTES || !meta.is_file() {
                    return Ok(());
                }
            }

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return Ok(()), // skip unreadable/binary files
            };

            for (idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(query_lower) {
                    let display = format!("{}:{}: {}", path.display(), idx + 1, line.trim_end());
                    results.push(display);
                    if results.len() >= max_results {
                        break;
                    }
                }
            }

            Ok(())
        }

        fn walk(
            root: PathBuf,
            query_lower: &str,
            max_results: usize,
            results: &mut Vec<String>,
        ) -> Result<(), Box<dyn Error>> {
            let mut stack = vec![root];
            while let Some(path) = stack.pop() {
                if results.len() >= max_results {
                    break;
                }
                if Self::is_ignored(&path) {
                    continue;
                }

                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if meta.is_dir() {
                    if let Ok(entries) = fs::read_dir(&path) {
                        for entry in entries.flatten() {
                            stack.push(entry.path());
                        }
                    }
                } else if meta.is_file() {
                    Self::search_file(&path, query_lower, max_results, results)?;
                }
            }
            Ok(())
        }
    }

    impl ToolCall for CodebaseSearchTool {
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

            let mut results = Vec::new();
            Self::walk(root_path, &query.to_lowercase(), max_results, &mut results)?;

            if results.is_empty() {
                Ok(format!(
                    "No matches found for '{}'. Searched up to {} results.",
                    query, max_results
                ))
            } else {
                Ok(results.join("\n"))
            }
        }

        fn name(&self) -> &str {
            "codebase_search"
        }
    }
}
