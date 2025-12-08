pub mod file_manager {
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::{Component, Path, PathBuf};

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
                    description: "Full absolute path to the file or folder. Must stay inside the workspace. IMPORTANT: Use absolute paths (e.g., /full/path/to/file.txt).".to_string(),
                    enum_values: None,
                },
            );

            let mut kind_items = HashMap::new();
            kind_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "kind".to_string(),
                Parameter {
                    items: kind_items,
                    description: "What to create. Use 'file' (default) or 'directory'/'folder'."
                        .to_string(),
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
                    description: "File contents when kind=file. Optional; defaults to empty. IMPORTANT: To modify only part of a file, you MUST provide startLine and endLine parameters. Without line numbers, the entire file will be replaced.".to_string(),
                    enum_values: None,
                },
            );

            let mut start_line_items = HashMap::new();
            start_line_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "startLine".to_string(),
                Parameter {
                    items: start_line_items,
                    description: "Starting line number (1-based) for partial file replacement. If provided with endLine, only lines from startLine to endLine (inclusive) will be replaced. You MUST know the exact line numbers to replace specific parts of a file. Use grep or read the file first to determine line numbers.".to_string(),
                    enum_values: None,
                },
            );

            let mut end_line_items = HashMap::new();
            end_line_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "endLine".to_string(),
                Parameter {
                    items: end_line_items,
                    description: "Ending line number (1-based) for partial file replacement. Must be provided with startLine. Lines from startLine to endLine (inclusive) will be replaced with content. You MUST know the exact line numbers to replace specific parts of a file.".to_string(),
                    enum_values: None,
                },
            );

            let mut overwrite_items = HashMap::new();
            overwrite_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "overwrite".to_string(),
                Parameter {
                    items: overwrite_items,
                    description: "If true, replace an existing file. Directories are not removed."
                        .to_string(),
                    enum_values: None,
                },
            );

            let mut parents_items = HashMap::new();
            parents_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "createParents".to_string(),
                Parameter {
                    items: parents_items,
                    description: "Create parent directories as needed (default true). Used for single file operations.".to_string(),
                    enum_values: None,
                },
            );

            // Add files parameter for batch operations
            let mut files_items = HashMap::new();
            files_items.insert("type".to_string(), "array".to_string());
            files_items.insert("item_type".to_string(), "object".to_string());
            parameters.insert(
                "files".to_string(),
                Parameter {
                    items: files_items,
                    description: "Array of file operations to perform. Each object should have: path (required, use full absolute path), kind (optional, default 'file'), content (optional), startLine/endLine (optional, for partial replacement - you MUST know line numbers), overwrite (optional, default false), createParents (optional, default true). Use this for batch operations to modify multiple files at once.".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "file_manager".to_string(),
                description: "Create files or folders inside the current workspace. Supports single file operations (use 'path') or batch operations (use 'files' array). IMPORTANT: To modify only part of an existing file, you MUST provide startLine and endLine parameters along with content. Without line numbers, the entire file will be replaced. Always use full absolute paths. Use grep or read the file first to determine the exact line numbers you need to modify.".to_string(),
                parameters,
                required: vec![],
            };

            let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            Self {
                tool,
                workspace_root,
            }
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
            let workspace_root = self.workspace_root.canonicalize().unwrap_or_else(|_| {
                // If canonicalize fails, use absolute path
                std::fs::canonicalize(".").unwrap_or_else(|_| self.workspace_root.clone())
            });

            let candidate = if Path::new(raw_path).is_absolute() {
                PathBuf::from(raw_path)
            } else {
                // Convert relative path to absolute
                workspace_root.join(raw_path)
            };

            let normalized = Self::clean_path(&candidate);

            // Normalize workspace_root for comparison
            let normalized_workspace = Self::clean_path(&workspace_root);

            // Check if normalized path is within workspace
            // Try canonicalizing both for proper comparison (handles symlinks)
            let normalized_canonical = normalized
                .canonicalize()
                .unwrap_or_else(|_| normalized.clone());
            let workspace_canonical = normalized_workspace
                .canonicalize()
                .unwrap_or_else(|_| normalized_workspace.clone());

            // Use string comparison as a fallback for cross-platform compatibility
            let normalized_str = normalized_canonical.to_string_lossy().to_string();
            let workspace_str = workspace_canonical.to_string_lossy().to_string();

            if !normalized_str.starts_with(&workspace_str)
                && !normalized_canonical.starts_with(&workspace_canonical)
            {
                return Err(format!(
                    "Path is outside the workspace. Requested: {}, workspace root: {}. Use full absolute paths within the workspace.",
                    normalized_canonical.display(),
                    workspace_canonical.display()
                )
                .into());
            }

            // Return canonicalized absolute path if possible, otherwise return normalized path
            Ok(normalized.canonicalize().unwrap_or_else(|_| normalized))
        }

        fn create_directory(
            &self,
            path: &Path,
            create_parents: bool,
        ) -> Result<String, Box<dyn Error>> {
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
            start_line: Option<usize>,
            end_line: Option<usize>,
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

            // If line numbers are provided, do partial replacement
            if let (Some(start), Some(end)) = (start_line, end_line) {
                if !path.exists() {
                    return Err(format!(
                        "Cannot replace lines in non-existent file: {} (file must exist for line-based replacement)",
                        path.display()
                    ).into());
                }
                if path.is_dir() {
                    return Err(
                        format!("Path is a directory, not a file: {}", path.display()).into(),
                    );
                }

                // Read existing file
                let existing_content = fs::read_to_string(path)?;
                let lines: Vec<&str> = existing_content.lines().collect();

                // Validate line numbers (1-based to 0-based conversion)
                if start < 1 || end < 1 {
                    return Err("Line numbers must be 1-based (start from 1)".into());
                }
                if start > end {
                    return Err(
                        format!("startLine ({}) must be <= endLine ({})", start, end).into(),
                    );
                }
                if start > lines.len() || end > lines.len() {
                    return Err(format!(
                        "Line numbers out of range: file has {} lines, but requested lines {}-{}",
                        lines.len(),
                        start,
                        end
                    )
                    .into());
                }

                // Build new content: lines before + new content + lines after
                let mut new_lines = Vec::new();

                // Lines before the replacement (0-based: start-1)
                new_lines.extend_from_slice(&lines[..(start - 1)]);

                // New content (split by lines)
                let new_content_lines: Vec<&str> = content.lines().collect();
                new_lines.extend(new_content_lines);

                // Lines after the replacement (0-based: end, which is exclusive, so we use end)
                if end < lines.len() {
                    new_lines.extend_from_slice(&lines[end..]);
                }

                // Reconstruct file with original line endings
                let new_content = if existing_content.contains("\r\n") {
                    new_lines.join("\r\n")
                } else if existing_content.contains('\r') {
                    new_lines.join("\r")
                } else {
                    new_lines.join("\n")
                };

                fs::write(path, new_content)?;
                return Ok(format!(
                    "Replaced lines {}-{} in {}",
                    start,
                    end,
                    path.display()
                ));
            }

            // Full file replacement (existing behavior)
            if path.exists() {
                if path.is_dir() {
                    return Err(
                        format!("Path is a directory, not a file: {}", path.display()).into(),
                    );
                }
                if !overwrite {
                    return Err(format!(
                        "File already exists: {} (set overwrite=true to replace it, or use startLine/endLine for partial replacement)",
                        path.display()
                    )
                    .into());
                }
            }

            fs::write(path, content)?;
            Ok(format!("File written at {}", path.display()))
        }

        fn process_single_file(
            &self,
            file_op: &serde_json::Value,
        ) -> Result<String, Box<dyn Error>> {
            let raw_path = file_op
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: path in file operation")?;

            let kind = file_op
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_lowercase();

            let overwrite = file_op
                .get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let create_parents = file_op
                .get("createParents")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            // Get line numbers for partial replacement
            let start_line = file_op
                .get("startLine")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let end_line = file_op
                .get("endLine")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            // Validate line numbers are provided together
            if start_line.is_some() != end_line.is_some() {
                return Err("Both startLine and endLine must be provided together for partial file replacement".into());
            }

            // Handle content - can be string, array, or object - convert to string
            let content = file_op
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
                self.write_file(
                    &target_path,
                    &content,
                    overwrite,
                    create_parents,
                    start_line,
                    end_line,
                )
            }
        }
    }

    impl ToolCall for FileManagerTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            // Check if batch operation (files array) is provided
            if let Some(files_array) = args.get("files").and_then(|v| v.as_array()) {
                // Batch operation: process multiple files
                let mut results = Vec::new();
                let mut errors = Vec::new();

                for (index, file_op) in files_array.iter().enumerate() {
                    match self.process_single_file(file_op) {
                        Ok(result) => {
                            results.push(format!("[{}] {}", index + 1, result));
                        }
                        Err(e) => {
                            let path = file_op
                                .get("path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            errors.push(format!("[{}] {}: {}", index + 1, path, e));
                        }
                    }
                }

                // Combine results
                let mut output = String::new();
                if !results.is_empty() {
                    output.push_str(&format!(
                        "Successfully processed {} file(s):\n",
                        results.len()
                    ));
                    output.push_str(&results.join("\n"));
                }
                if !errors.is_empty() {
                    if !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    output.push_str(&format!("Failed to process {} file(s):\n", errors.len()));
                    output.push_str(&errors.join("\n"));
                }

                // Handle empty array case
                if results.is_empty() && errors.is_empty() {
                    output.push_str("Successfully processed 0 file(s)");
                }

                if errors.is_empty() {
                    Ok(output)
                } else if results.is_empty() {
                    Err(output.into())
                } else {
                    // Partial success - return as error but with details
                    Err(format!("Partial success:\n{}", output).into())
                }
            } else if args.get("path").is_some() {
                // Single file operation (backward compatibility)
                self.process_single_file(&args)
            } else {
                Err("Missing required parameter: either 'path' (for single file) or 'files' (for batch operations) must be provided".into())
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
        assert!(
            result.is_ok(),
            "Should write file successfully: {:?}",
            result
        );

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
        assert!(
            result.is_ok(),
            "Should create parent directories and write file: {:?}",
            result
        );

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
        let (tool, _temp_dir) = create_test_tool();
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
        assert!(
            result.is_err(),
            "Should fail when file exists and overwrite=false"
        );
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
        assert!(
            dir_path.exists() && dir_path.is_dir(),
            "Directory should exist"
        );
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
        assert_eq!(
            fs::read_to_string(&file_path).unwrap(),
            "Test content from run"
        );
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
        assert!(
            content.contains("line1") || content.contains("line2"),
            "Should serialize array to string"
        );
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
        assert!(
            content.contains("key") || content.contains("value"),
            "Should serialize object to string"
        );
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

    #[test]
    fn test_batch_operations() {
        let (tool, temp_dir) = create_test_tool();
        let args = r#"{
            "files": [
                {
                    "path": "batch1.txt",
                    "kind": "file",
                    "content": "File 1 content",
                    "createParents": true
                },
                {
                    "path": "batch2.txt",
                    "kind": "file",
                    "content": "File 2 content",
                    "createParents": true
                },
                {
                    "path": "batch_dir",
                    "kind": "directory",
                    "createParents": true
                }
            ]
        }"#;

        let result = tool.run(args);
        assert!(
            result.is_ok(),
            "Should process batch operations successfully: {:?}",
            result
        );

        // Verify all files were created
        let file1 = temp_dir.path().join("batch1.txt");
        let file2 = temp_dir.path().join("batch2.txt");
        let dir = temp_dir.path().join("batch_dir");

        assert!(file1.exists(), "batch1.txt should exist");
        assert!(file2.exists(), "batch2.txt should exist");
        assert!(
            dir.exists() && dir.is_dir(),
            "batch_dir should exist as directory"
        );

        assert_eq!(fs::read_to_string(&file1).unwrap(), "File 1 content");
        assert_eq!(fs::read_to_string(&file2).unwrap(), "File 2 content");
    }

    #[test]
    fn test_batch_operations_partial_failure() {
        let (tool, temp_dir) = create_test_tool();

        // Create a file that will conflict
        let existing_file = temp_dir.path().join("existing.txt");
        fs::write(&existing_file, "existing").unwrap();

        let args = r#"{
            "files": [
                {
                    "path": "new_file.txt",
                    "kind": "file",
                    "content": "New content",
                    "createParents": true
                },
                {
                    "path": "existing.txt",
                    "kind": "file",
                    "content": "Should fail",
                    "createParents": true
                }
            ]
        }"#;

        let result = tool.run(args);
        // Should return error for partial failure
        assert!(result.is_err(), "Should return error for partial failure");

        // But first file should still be created
        let new_file = temp_dir.path().join("new_file.txt");
        assert!(new_file.exists(), "new_file.txt should be created");
    }

    #[test]
    fn test_batch_operations_empty_array() {
        let (tool, _temp_dir) = create_test_tool();
        let args = r#"{"files": []}"#;

        let result = tool.run(args);
        assert!(result.is_ok(), "Should handle empty array");
        assert!(result.unwrap().contains("Successfully processed 0 file(s)"));
    }

    #[test]
    fn test_line_based_replacement() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("test_lines.txt");

        // Create initial file with multiple lines
        let initial_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";
        fs::write(&file_path, initial_content).unwrap();

        // Replace lines 2-4
        let args = format!(
            r#"{{
                "path": "{}",
                "kind": "file",
                "content": "New Line 2\nNew Line 3\nNew Line 4",
                "startLine": 2,
                "endLine": 4,
                "createParents": true
            }}"#,
            file_path.display()
        );

        let result = tool.run(&args);
        assert!(
            result.is_ok(),
            "Should replace lines successfully: {:?}",
            result
        );

        // Verify replacement
        let content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 5, "Should have 5 lines total");
        assert_eq!(lines[0], "Line 1", "Line 1 should remain");
        assert_eq!(lines[1], "New Line 2", "Line 2 should be replaced");
        assert_eq!(lines[2], "New Line 3", "Line 3 should be replaced");
        assert_eq!(lines[3], "New Line 4", "Line 4 should be replaced");
        assert_eq!(lines[4], "Line 5", "Line 5 should remain");
    }

    #[test]
    fn test_line_based_replacement_single_line() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("test_single_line.txt");

        // Create initial file
        let initial_content = "Line 1\nLine 2\nLine 3\n";
        fs::write(&file_path, initial_content).unwrap();

        // Replace line 2 only
        let args = format!(
            r#"{{
                "path": "{}",
                "kind": "file",
                "content": "Replaced Line 2",
                "startLine": 2,
                "endLine": 2,
                "createParents": true
            }}"#,
            file_path.display()
        );

        let result = tool.run(&args);
        assert!(result.is_ok(), "Should replace single line successfully");

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Line 1"), "Line 1 should remain");
        assert!(
            content.contains("Replaced Line 2"),
            "Line 2 should be replaced"
        );
        assert!(content.contains("Line 3"), "Line 3 should remain");
    }

    #[test]
    fn test_line_based_replacement_error_missing_one_line_number() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("test_error.txt");
        fs::write(&file_path, "Line 1\nLine 2\n").unwrap();

        // Missing endLine
        let args = format!(
            r#"{{
                "path": "{}",
                "kind": "file",
                "content": "New content",
                "startLine": 1,
                "createParents": true
            }}"#,
            file_path.display()
        );

        let result = tool.run(&args);
        assert!(
            result.is_err(),
            "Should fail when only one line number is provided"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("startLine and endLine")
        );
    }

    #[test]
    fn test_line_based_replacement_error_out_of_range() {
        let (tool, temp_dir) = create_test_tool();
        let file_path = temp_dir.path().join("test_range.txt");
        fs::write(&file_path, "Line 1\nLine 2\n").unwrap();

        // Line numbers out of range
        let args = format!(
            r#"{{
                "path": "{}",
                "kind": "file",
                "content": "New content",
                "startLine": 10,
                "endLine": 15,
                "createParents": true
            }}"#,
            file_path.display()
        );

        let result = tool.run(&args);
        assert!(
            result.is_err(),
            "Should fail when line numbers are out of range"
        );
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }
}
