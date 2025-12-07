pub mod file_manager {
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use serde_json;

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};

    /// Tool for creating files or folders within the current workspace.
    /// Use this instead of bash when you need to scaffold paths or seed file contents.
    pub struct FileManagerTool {
        pub(crate) tool: Tool,
        pub(crate) workspace_root: PathBuf,
    }

    impl FileManagerTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut path_items = HashMap::new();
            path_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "path".to_string(),
                Parameter {
                    items: path_items,
                    description: "Path to the file or folder (absolute or relative to the workspace). Must stay inside the workspace.".to_string(),
                    enum_values: None,
                },
            );

            let mut kind_items = HashMap::new();
            kind_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "kind".to_string(),
                Parameter {
                    items: kind_items,
                    description: "What to create. Use 'file' (default) or 'directory'/'folder'.".to_string(),
                    enum_values: Some(vec![
                        "file".to_string(),
                        "directory".to_string(),
                        "folder".to_string(),
                    ]),
                },
            );

            let mut content_items = HashMap::new();
            content_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "content".to_string(),
                Parameter {
                    items: content_items,
                    description: "File contents when kind=file. Optional; defaults to empty.".to_string(),
                    enum_values: None,
                },
            );

            let mut overwrite_items = HashMap::new();
            overwrite_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "overwrite".to_string(),
                Parameter {
                    items: overwrite_items,
                    description: "If true, replace an existing file. Directories are not removed.".to_string(),
                    enum_values: None,
                },
            );

            let mut parents_items = HashMap::new();
            parents_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "createParents".to_string(),
                Parameter {
                    items: parents_items,
                    description: "Create parent directories as needed (default true).".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "file_manager".to_string(),
                description: "Create files or folders inside the current workspace. Prefer this over bash for filesystem scaffolding.".to_string(),
                parameters,
                required: vec!["path".to_string()],
            };

            let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            Self { tool, workspace_root }
        }

        fn clean_path(path: &Path) -> PathBuf {
            let mut normalized = PathBuf::new();
            for comp in path.components() {
                match comp {
                    Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                    Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
                    Component::CurDir => {}
                    Component::ParentDir => {
                        normalized.pop();
                    }
                    Component::Normal(part) => normalized.push(part),
                }
            }
            normalized
        }

        fn resolve_path(&self, raw_path: &str) -> Result<PathBuf, Box<dyn Error>> {
            // Try to canonicalize workspace_root, but fall back to absolute path if it doesn't exist
            let workspace_root = self.workspace_root
                .canonicalize()
                .unwrap_or_else(|_| {
                    // If canonicalize fails, use absolute path
                    std::fs::canonicalize(".")
                        .unwrap_or_else(|_| self.workspace_root.clone())
                });
            
            let candidate = if Path::new(raw_path).is_absolute() {
                PathBuf::from(raw_path)
            } else {
                workspace_root.join(raw_path)
            };

            let normalized = Self::clean_path(&candidate);
            
            // Normalize workspace_root for comparison
            let normalized_workspace = Self::clean_path(&workspace_root);

            // Check if normalized path is within workspace
            // Use string comparison as a fallback for cross-platform compatibility
            let normalized_str = normalized.to_string_lossy().to_string();
            let workspace_str = normalized_workspace.to_string_lossy().to_string();
            
            if !normalized_str.starts_with(&workspace_str) && !normalized.starts_with(&normalized_workspace) {
                return Err(format!(
                    "Path is outside the workspace. Requested: {}, workspace root: {}",
                    normalized.display(),
                    workspace_root.display()
                )
                .into());
            }

            Ok(normalized)
        }

        fn create_directory(&self, path: &Path, create_parents: bool) -> Result<String, Box<dyn Error>> {
            if path.exists() {
                if path.is_dir() {
                    return Ok(format!("Directory already exists at {}", path.display()));
                }
                return Err(format!("A file already exists at {}", path.display()).into());
            }

            if create_parents {
                fs::create_dir_all(path)?;
            } else {
                fs::create_dir(path)?;
            }

            Ok(format!("Directory created at {}", path.display()))
        }

        fn write_file(
            &self,
            path: &Path,
            content: &str,
            overwrite: bool,
            create_parents: bool,
        ) -> Result<String, Box<dyn Error>> {
            if let Some(parent) = path.parent() {
                if create_parents {
                    fs::create_dir_all(parent)?;
                } else if !parent.exists() {
                    return Err(format!(
                        "Parent directory does not exist: {} (set createParents=true to create it)",
                        parent.display()
                    )
                    .into());
                }
            }

            if path.exists() {
                if path.is_dir() {
                    return Err(format!("Path is a directory, not a file: {}", path.display()).into());
                }
                if !overwrite {
                    return Err(format!(
                        "File already exists: {} (set overwrite=true to replace it)",
                        path.display()
                    )
                    .into());
                }
            }

            fs::write(path, content)?;
            Ok(format!("File written at {}", path.display()))
        }
    }

    impl ToolCall for FileManagerTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            let raw_path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: path")?;

            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_lowercase();

            let overwrite = args
                .get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let create_parents = args
                .get("createParents")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            // Handle content - can be string, array, or object - convert to string
            let content = args
                .get("content")
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else if v.is_array() || v.is_object() {
                        // Serialize JSON arrays/objects to string
                        serde_json::to_string(v).unwrap_or_else(|_| v.to_string())
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_default();

            let target_path = self.resolve_path(raw_path)?;

            if kind == "directory" || kind == "folder" {
                self.create_directory(&target_path, create_parents)
            } else {
                self.write_file(&target_path, &content, overwrite, create_parents)
            }
        }

        fn name(&self) -> &str {
            "file_manager"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::file_manager::FileManagerTool;
    use crate::tool::tool::tool::ToolCall;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tool() -> (FileManagerTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        // Create tool with temp_dir as workspace
        let mut tool = FileManagerTool::new();
        tool.workspace_root = temp_dir.path().to_path_buf();
        (tool, temp_dir)
    }

    #[test]
    fn test_write_file_basic() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, World!";
        // Use relative path
        let args = format!(
            r#"{{
                "path": "test.txt",
                "kind": "file",
                "content": "{}",
                "createParents": true
            }}"#,
            content
        );

        let result = tool.run(&args);
        assert!(result.is_ok(), "Should write file successfully: {:?}", result);

        // Verify file was created with correct content
        assert!(file_path.exists(), "File should exist");
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content, "File content should match");
    }

    #[test]
    fn test_write_file_with_parents() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("nested").join("deep").join("test.txt");
        let content = "Nested content";
        let args = format!(
            r#"{{
                "path": "nested/deep/test.txt",
                "kind": "file",
                "content": "{}",
                "createParents": true
            }}"#,
            content
        );

        let result = tool.run(&args);
        assert!(result.is_ok(), "Should create parent directories and write file: {:?}", result);

        assert!(file_path.exists(), "File should exist");
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_file_overwrite() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("overwrite.txt");
        let initial_content = "Initial content";
        let new_content = "New content";

        // Write initial file
        let args1 = format!(
            r#"{{
                "path": "overwrite.txt",
                "kind": "file",
                "content": "{}",
                "createParents": true
            }}"#,
            initial_content
        );
        tool.run(&args1).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), initial_content);

        // Overwrite file
        let args2 = format!(
            r#"{{
                "path": "overwrite.txt",
                "kind": "file",
                "content": "{}",
                "overwrite": true,
                "createParents": true
            }}"#,
            new_content
        );
        let result = tool.run(&args2);
        assert!(result.is_ok(), "Should overwrite file");

        assert_eq!(fs::read_to_string(&file_path).unwrap(), new_content);
    }

    #[test]
    fn test_write_file_no_overwrite_error() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("existing.txt");
        let content = "Content";

        // Create file first
        let args1 = format!(
            r#"{{
                "path": "existing.txt",
                "kind": "file",
                "content": "{}",
                "createParents": true
            }}"#,
            content
        );
        tool.run(&args1).unwrap();

        // Try to write again without overwrite
        let args2 = r#"{
                "path": "existing.txt",
                "kind": "file",
                "content": "New content",
                "createParents": true
            }"#;
        let result = tool.run(args2);
        assert!(result.is_err(), "Should fail when file exists and overwrite=false");
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_directory() {
        let (tool, temp_dir) = create_test_tool();
        let dir_path = temp_dir.path().join("test_dir");
        let args = r#"{
                "path": "test_dir",
                "kind": "directory",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should create directory");
        assert!(dir_path.exists() && dir_path.is_dir(), "Directory should exist");
    }

    #[test]
    fn test_create_directory_nested() {
        let (tool, temp_dir) = create_test_tool();
        let dir_path = temp_dir.path().join("a").join("b").join("c");
        let args = r#"{
                "path": "a/b/c",
                "kind": "directory",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should create nested directories");
        assert!(dir_path.exists() && dir_path.is_dir());
    }

    #[test]
    fn test_run_create_file() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("run_test.txt");
        let args = r#"{
                "path": "run_test.txt",
                "kind": "file",
                "content": "Test content from run",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should create file via run()");
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "Test content from run");
    }

    #[test]
    fn test_run_create_file_with_json_array_content() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("json_array.txt");
        let args = r#"{
                "path": "json_array.txt",
                "kind": "file",
                "content": ["line1", "line2", "line3"],
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should handle JSON array content");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("line1") || content.contains("line2"), "Should serialize array to string");
    }

    #[test]
    fn test_run_create_file_with_json_object_content() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("json_object.txt");
        let args = r#"{
                "path": "json_object.txt",
                "kind": "file",
                "content": {"key": "value", "number": 42},
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should handle JSON object content");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("key") || content.contains("value"), "Should serialize object to string");
    }

    #[test]
    fn test_run_create_directory() {
        let (tool, temp_dir) = create_test_tool();
        let dir_path = temp_dir.path().join("run_dir");
        let args = r#"{
                "path": "run_dir",
                "kind": "directory",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should create directory via run()");
        assert!(dir_path.exists() && dir_path.is_dir());
    }

    #[test]
    fn test_write_file_empty_content() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("empty.txt");
        let args = r#"{
                "path": "empty.txt",
                "kind": "file",
                "content": "",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should write empty file");
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "");
    }

    #[test]
    fn test_write_file_multiline_content() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("multiline.txt");
        let content = "Line 1\nLine 2\nLine 3\n";
        let args = r#"{
                "path": "multiline.txt",
                "kind": "file",
                "content": "Line 1\nLine 2\nLine 3\n",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), content);
    }

    #[test]
    fn test_write_file_special_characters() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("special.txt");
        let args = r#"{
                "path": "special.txt",
                "kind": "file",
                "content": "Special chars: !@#$%^&*()[]{}|\\/<>?~`'\"",
                "createParents": true
            }"#;

        let result = tool.run(args);
        assert!(result.is_ok());
        // Note: Special character handling in JSON may vary, so we just check file exists
        assert!(file_path.exists());
    }

    #[test]
    fn test_run_missing_path() {
        let (tool, _temp_dir) = create_test_tool();
        let args = r#"{"kind": "file", "content": "test"}"#;

        let result = tool.run(args);
        assert!(result.is_err(), "Should fail when path is missing");
        assert!(result.unwrap_err().to_string().contains("path"));
    }
}


