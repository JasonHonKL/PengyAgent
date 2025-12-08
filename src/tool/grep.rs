pub mod grep {
    //! Search file contents via ripgrep with a grep fallback, returning matched
    //! lines with file paths and numbers for quick navigation.

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::process::Command;

    /// Executes regex searches across files using ripgrep when available.
    pub struct GrepTool {
        tool: Tool,
    }

    impl GrepTool {
        /// Build the tool schema for regex search parameters.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            // pattern parameter (required)
            let mut pattern_items = HashMap::new();
            pattern_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "pattern".to_string(),
                Parameter {
                    items: pattern_items,
                    description: "The regular expression pattern to search for in file contents."
                        .to_string(),
                    enum_values: None,
                },
            );

            // path parameter (optional)
            let mut path_items = HashMap::new();
            path_items.insert("type".to_string(), "string".to_string());
            parameters.insert("path".to_string(), Parameter {
                items: path_items,
                description: "The directory to search in. Defaults to current working directory if not provided.".to_string(),
                enum_values: None,
            });

            // include parameter (optional)
            let mut include_items = HashMap::new();
            include_items.insert("type".to_string(), "string".to_string());
            parameters.insert("include".to_string(), Parameter {
                items: include_items,
                description: "File pattern to include (e.g., '.js', '.{ts,tsx}'). If not provided, searches all files.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "grep".to_string(),
                description: "Search file contents using regular expressions with ripgrep integration. Searches for patterns in files and returns matching lines with file paths and line numbers.".to_string(),
                parameters,
                required: vec!["pattern".to_string()],
            };

            Self { tool }
        }

        /// Run ripgrep (or fall back to grep) with optional path and include
        /// filters, returning a normalized textual result.
        fn execute_grep(
            &self,
            pattern: &str,
            path: Option<&str>,
            include: Option<&str>,
        ) -> Result<String, Box<dyn Error>> {
            // Try to use ripgrep (rg) first, fall back to grep if not available
            let mut cmd = Command::new("rg");

            // Add pattern (ripgrep takes pattern as first positional argument)
            cmd.arg(pattern);

            // Add path if provided
            let search_path = path.unwrap_or(".");
            cmd.arg(search_path);

            // Add file pattern filter if provided
            if let Some(include_pattern) = include {
                // Convert include pattern to ripgrep's glob format
                // e.g., ".js" -> "*.js"
                // e.g., ".{ts,tsx}" -> "*.{ts,tsx}"
                let glob_pattern = if include_pattern.starts_with(".") {
                    format!("*{}", include_pattern)
                } else if include_pattern.starts_with("*") {
                    include_pattern.to_string()
                } else {
                    format!("*{}", include_pattern)
                };
                cmd.arg("-g");
                cmd.arg(&glob_pattern);
            }

            // Add useful flags
            cmd.arg("--line-number"); // Show line numbers
            cmd.arg("--no-heading"); // Don't group by file
            cmd.arg("--color=never"); // Disable color for cleaner output

            // Execute the command
            let output = cmd.output();

            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

                    // ripgrep exits with code 1 when no matches are found (this is normal)
                    if result.status.code() == Some(1) && stdout.is_empty() {
                        return Ok(format!(
                            "No matches found for pattern '{}' in {}",
                            pattern, search_path
                        ));
                    }

                    // If ripgrep failed for other reasons, try falling back to grep
                    if !result.status.success() && result.status.code() != Some(1) {
                        return self.fallback_to_grep(pattern, search_path, include);
                    }

                    // Combine stdout and stderr if stderr has content
                    let mut result_str = stdout;
                    if !stderr.trim().is_empty() {
                        if !result_str.is_empty() {
                            result_str.push_str("\n");
                        }
                        result_str.push_str(&stderr);
                    }

                    if result_str.trim().is_empty() {
                        Ok(format!(
                            "No matches found for pattern '{}' in {}",
                            pattern, search_path
                        ))
                    } else {
                        Ok(result_str.trim().to_string())
                    }
                }
                Err(_) => {
                    // ripgrep not found, try falling back to grep
                    self.fallback_to_grep(pattern, search_path, include)
                }
            }
        }

        /// Execute the legacy `grep` command when ripgrep is unavailable or
        /// fails unexpectedly.
        fn fallback_to_grep(
            &self,
            pattern: &str,
            path: &str,
            include: Option<&str>,
        ) -> Result<String, Box<dyn Error>> {
            let mut cmd = Command::new("grep");

            // Add recursive flag
            cmd.arg("-r");
            cmd.arg("-n"); // Show line numbers
            cmd.arg("--color=never"); // Disable color

            // Add file pattern if provided
            if let Some(include_pattern) = include {
                // Convert include pattern to grep's --include format
                // e.g., ".js" -> "--include=*.js"
                // e.g., ".{ts,tsx}" -> "--include=*.ts" "--include=*.tsx"
                if include_pattern.starts_with(".") {
                    let ext = &include_pattern[1..];
                    if ext.contains("{") && ext.contains("}") {
                        // Handle multiple extensions like ".{ts,tsx}"
                        let parts: Vec<&str> =
                            ext.trim_matches('{').trim_matches('}').split(',').collect();
                        for part in parts {
                            cmd.arg(format!("--include=*.{}", part.trim()));
                        }
                    } else {
                        cmd.arg(format!("--include=*{}", include_pattern));
                    }
                } else {
                    cmd.arg(format!("--include={}", include_pattern));
                }
            }

            cmd.arg("-E"); // Extended regex
            cmd.arg(pattern);
            cmd.arg(path);

            let output = cmd.output()?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // grep exits with code 1 when no matches are found (this is normal)
            if output.status.code() == Some(1) && stdout.is_empty() {
                return Ok(format!(
                    "No matches found for pattern '{}' in {}",
                    pattern, path
                ));
            }

            if !output.status.success() && output.status.code() != Some(1) {
                return Err(format!(
                    "grep command failed with exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr
                )
                .into());
            }

            let mut result_str = stdout;
            if !stderr.trim().is_empty() {
                if !result_str.is_empty() {
                    result_str.push_str("\n");
                }
                result_str.push_str(&stderr);
            }

            if result_str.trim().is_empty() {
                Ok(format!(
                    "No matches found for pattern '{}' in {}",
                    pattern, path
                ))
            } else {
                Ok(result_str.trim().to_string())
            }
        }
    }

    impl ToolCall for GrepTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Parse arguments and perform the search, defaulting to the current
        /// directory when no path is provided.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            // Get required pattern
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: pattern")?;

            // Get optional parameters
            let path = args.get("path").and_then(|v| v.as_str());

            let include = args.get("include").and_then(|v| v.as_str());

            // Execute the grep command
            match self.execute_grep(pattern, path, include) {
                Ok(output) => {
                    if output.is_empty() {
                        Ok("Search completed (no matches found)".to_string())
                    } else {
                        Ok(output)
                    }
                }
                Err(e) => Err(format!("Failed to execute grep: {}", e).into()),
            }
        }

        fn name(&self) -> &str {
            "grep"
        }
    }
}
