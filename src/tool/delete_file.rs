pub mod delete_file {
    //! Delete a file (or directory) within the current workspace, with safety
    //! checks to avoid accidentally removing paths outside the project.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::PathBuf;

    /// Deletes files or directories after validating they are inside the
    /// current workspace.
    pub struct DeleteFileTool {
        tool: Tool,
        workspace_root: PathBuf,
    }

    impl DeleteFileTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut path_items = HashMap::new();
            path_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "target_file".to_string(),
                Parameter {
                    items: path_items,
                    description:
                        "Absolute or relative path to delete. Must be inside the workspace."
                            .to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "delete_file".to_string(),
                description: "Delete a file or directory inside the workspace after validation."
                    .to_string(),
                parameters,
                required: vec!["target_file".to_string()],
            };

            let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            Self {
                tool,
                workspace_root,
            }
        }

        fn resolve_path(&self, raw: &str) -> Result<PathBuf, Box<dyn Error>> {
            let mut candidate = PathBuf::from(raw);
            if !candidate.is_absolute() {
                candidate = self.workspace_root.join(candidate);
            }
            let candidate = candidate
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(raw));
            let workspace = self
                .workspace_root
                .canonicalize()
                .unwrap_or_else(|_| self.workspace_root.clone());

            if !candidate.starts_with(&workspace) {
                return Err(format!(
                    "Refusing to delete outside workspace. Path: {}, workspace: {}",
                    candidate.display(),
                    workspace.display()
                )
                .into());
            }
            Ok(candidate)
        }
    }

    impl ToolCall for DeleteFileTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let target = args
                .get("target_file")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: target_file")?;

            let path = self.resolve_path(target)?;
            if !path.exists() {
                return Err(format!("Path does not exist: {}", path.display()).into());
            }

            if path.is_dir() {
                fs::remove_dir_all(&path)?;
                Ok(format!("Directory deleted: {}", path.display()))
            } else {
                fs::remove_file(&path)?;
                Ok(format!("File deleted: {}", path.display()))
            }
        }

        fn name(&self) -> &str {
            "delete_file"
        }
    }
}
