pub mod find_replace {
    //! Simple find-and-replace tool that performs exact string substitutions
    //! across an entire file.
    //!
    //! This is a lightweight alternative to the edit tool when you already know
    //! the exact text to replace. It replaces all occurrences of the provided
    //! search string within the target file.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    /// Tool for performing exact find-and-replace operations on files.
    pub struct FindReplaceTool {
        tool: Tool,
    }

    impl FindReplaceTool {
        /// Build the tool definition and schema metadata.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            // filePath parameter (required)
            let mut file_path_items = HashMap::new();
            file_path_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "filePath".to_string(),
                Parameter {
                    items: file_path_items,
                    description: "Absolute path to the file to modify.".to_string(),
                    enum_values: None,
                },
            );

            // searchContent parameter (required)
            let mut search_items = HashMap::new();
            search_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "searchContent".to_string(),
                Parameter {
                    items: search_items,
                    description: "Exact text to search for within the file.".to_string(),
                    enum_values: None,
                },
            );

            // replaceContent parameter (required)
            let mut replace_items = HashMap::new();
            replace_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "replaceContent".to_string(),
                Parameter {
                    items: replace_items,
                    description: "Replacement text to insert for every match.".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "find_replace".to_string(),
                description:
                    "Finds and replaces all occurrences of a string within a file using exact matching."
                        .to_string(),
                parameters,
                required: vec![
                    "filePath".to_string(),
                    "searchContent".to_string(),
                    "replaceContent".to_string(),
                ],
            };

            Self { tool }
        }

        /// Execute an exact find-and-replace against the provided file.
        fn execute(
            &self,
            file_path: &str,
            search_content: &str,
            replace_content: &str,
        ) -> Result<String, Box<dyn Error>> {
            if search_content.is_empty() {
                return Err("searchContent cannot be empty".into());
            }
            if search_content == replace_content {
                return Err("searchContent and replaceContent must differ".into());
            }

            let path = Path::new(file_path);
            if !path.exists() {
                return Err(format!("File does not exist: {}", file_path).into());
            }
            if !path.is_file() {
                return Err(format!("Path is not a file: {}", file_path).into());
            }

            let content = fs::read_to_string(path)?;
            let occurrences: Vec<_> = content.match_indices(search_content).collect();
            if occurrences.is_empty() {
                return Err(
                    format!("No matches for searchContent were found in {}", file_path).into(),
                );
            }

            let replaced = content.replace(search_content, replace_content);
            fs::write(path, replaced)?;

            Ok(format!(
                "Replaced {} occurrence(s) of searchContent in {}",
                occurrences.len(),
                file_path
            ))
        }
    }

    impl ToolCall for FindReplaceTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Parse arguments and run the find/replace operation.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            let file_path = args
                .get("filePath")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: filePath")?;

            let search_content = args
                .get("searchContent")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: searchContent")?;

            let replace_content = args
                .get("replaceContent")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: replaceContent")?;

            self.execute(file_path, search_content, replace_content)
        }

        fn name(&self) -> &str {
            "find_replace"
        }
    }

    #[cfg(test)]
    mod tests {
        use super::FindReplaceTool;
        use crate::tool::tool::tool::ToolCall;
        use std::fs;
        use tempfile::NamedTempFile;

        #[test]
        fn test_find_replace_success() {
            let mut tmp = NamedTempFile::new().unwrap();
            fs::write(tmp.path(), "alpha beta alpha").unwrap();

            let tool = FindReplaceTool::new();
            let args = format!(
                r#"{{
                "filePath": "{}",
                "searchContent": "alpha",
                "replaceContent": "gamma"
            }}"#,
                tmp.path().display()
            );

            let result = tool.run(&args);
            assert!(result.is_ok(), "Tool should run successfully: {:?}", result);

            let content = fs::read_to_string(tmp.path()).unwrap();
            assert_eq!(content, "gamma beta gamma");
        }

        #[test]
        fn test_find_replace_no_matches() {
            let mut tmp = NamedTempFile::new().unwrap();
            fs::write(tmp.path(), "no targets here").unwrap();

            let tool = FindReplaceTool::new();
            let args = format!(
                r#"{{
                "filePath": "{}",
                "searchContent": "missing",
                "replaceContent": "found"
            }}"#,
                tmp.path().display()
            );

            let result = tool.run(&args);
            assert!(result.is_err(), "Expected error when nothing matches");
        }
    }
}
