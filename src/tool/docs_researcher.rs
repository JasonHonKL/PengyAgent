pub mod docs_researcher {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use serde_json;
    use std::error::Error;
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    pub struct DocsResearcherTool {
        tool: Tool,
        docs_dir: PathBuf,
    }

    impl DocsResearcherTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // action parameter (required)
            let mut action_items = HashMap::new();
            action_items.insert("type".to_string(), "string".to_string());
            parameters.insert("action".to_string(), Parameter {
                items: action_items,
                description: "The action to perform: 'create' to create a new document, 'read' to read an entire document, or 'search' to search for content in a document.".to_string(),
                enum_values: Some(vec!["create".to_string(), "read".to_string(), "search".to_string()]),
            });

            // file_name parameter (for create action)
            let mut file_name_items = HashMap::new();
            file_name_items.insert("type".to_string(), "string".to_string());
            parameters.insert("file_name".to_string(), Parameter {
                items: file_name_items,
                description: "The name of the file to create. Required for 'create' action. File will be created in 'pengy_docs' folder.".to_string(),
                enum_values: None,
            });

            // content parameter (for create action)
            let mut content_items = HashMap::new();
            content_items.insert("type".to_string(), "string".to_string());
            parameters.insert("content".to_string(), Parameter {
                items: content_items,
                description: "The content to write to the file. Required for 'create' action. For 'search' action, this is the text or words to search for.".to_string(),
                enum_values: None,
            });

            // file parameter (for read and search actions)
            let mut file_items = HashMap::new();
            file_items.insert("type".to_string(), "string".to_string());
            parameters.insert("file".to_string(), Parameter {
                items: file_items,
                description: "The name of the file to read or search in. Required for 'read' and 'search' actions. File should be in 'pengy_docs' folder.".to_string(),
                enum_values: None,
            });

            // context_lines parameter (for search action, optional)
            let mut context_lines_items = HashMap::new();
            context_lines_items.insert("type".to_string(), "number".to_string());
            parameters.insert("context_lines".to_string(), Parameter {
                items: context_lines_items,
                description: "Number of lines above and below each match to return. Defaults to 10. Only used for 'search' action.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "docs_researcher".to_string(),
                description: "Manage documents in the 'pengy_docs' folder. Use 'create' to create a new document, 'read' to read an entire document, or 'search' to search for content in a document with context lines. The tool will automatically create the 'pengy_docs' folder if it doesn't exist.".to_string(),
                parameters,
                required: vec!["action".to_string()],
            };

            // Get the docs directory path
            let current_dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let docs_dir = current_dir.join("pengy_docs");

            Self {
                tool,
                docs_dir,
            }
        }

        fn ensure_docs_dir(&self) -> Result<(), Box<dyn Error>> {
            if !self.docs_dir.exists() {
                fs::create_dir_all(&self.docs_dir)?;
            }
            Ok(())
        }

        fn get_file_path(&self, file_name: &str) -> PathBuf {
            self.docs_dir.join(file_name)
        }

        fn create_document(&self, file_name: &str, content: &str) -> Result<String, Box<dyn Error>> {
            // Ensure docs directory exists
            self.ensure_docs_dir()?;

            // Get full file path
            let file_path = self.get_file_path(file_name);

            // Write content to file
            fs::write(&file_path, content)?;

            Ok(format!("Document '{}' created successfully in pengy_docs folder.", file_name))
        }

        fn read_document(&self, file: &str) -> Result<String, Box<dyn Error>> {
            let file_path = self.get_file_path(file);

            if !file_path.exists() {
                return Err(format!("File '{}' not found in pengy_docs folder.", file).into());
            }

            let content = fs::read_to_string(&file_path)?;
            
            Ok(content)
        }

        fn search_document(&self, file: &str, search_term: &str, context_lines: usize) -> Result<String, Box<dyn Error>> {
            let file_path = self.get_file_path(file);

            if !file_path.exists() {
                return Err(format!("File '{}' not found in pengy_docs folder.", file).into());
            }

            let content = fs::read_to_string(&file_path)?;
            let lines: Vec<&str> = content.lines().collect();

            let mut results = Vec::new();
            let mut found_any = false;

            // Search for the term (case-insensitive)
            let search_lower = search_term.to_lowercase();

            for (line_num, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&search_lower) {
                    found_any = true;
                    
                    // Calculate start and end line indices
                    let start_line = line_num.saturating_sub(context_lines);
                    let end_line = (line_num + context_lines + 1).min(lines.len());

                    // Add a separator if this isn't the first match
                    if !results.is_empty() {
                        results.push("---".to_string());
                    }

                    // Add context lines
                    for i in start_line..end_line {
                        let prefix = if i == line_num {
                            ">>> "  // Mark the matching line
                        } else {
                            "    "
                        };
                        results.push(format!("{}{}: {}", prefix, i + 1, lines[i]));
                    }
                }
            }

            if !found_any {
                return Ok(format!("No matches found for '{}' in file '{}'.", search_term, file));
            }

            Ok(results.join("\n"))
        }
    }

    impl ToolCall for DocsResearcherTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // Get the action
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: action")?;

            match action {
                "create" => {
                    let file_name = args.get("file_name")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: file_name (required for 'create' action)")?;

                    let content = args.get("content")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: content (required for 'create' action)")?;

                    self.create_document(file_name, content)
                }
                "read" => {
                    let file = args.get("file")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: file (required for 'read' action)")?;

                    self.read_document(file)
                }
                "search" => {
                    let file = args.get("file")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: file (required for 'search' action)")?;

                    let content = args.get("content")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: content (required for 'search' action - this is the search term)")?;

                    let context_lines = args.get("context_lines")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize)
                        .unwrap_or(10);

                    self.search_document(file, content, context_lines)
                }
                _ => Err(format!("Unknown action: {}. Must be 'create', 'read', or 'search'.", action).into())
            }
        }

        fn name(&self) -> &str {
            "docs_researcher"
        }
    }
}


