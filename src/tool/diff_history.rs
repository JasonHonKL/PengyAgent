pub mod diff_history {
    //! Show recent changes using `git diff --stat`. This is a lightweight view
    //! of uncommitted work.
    use crate::tool::tool::tool::{Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::process::Command;

    /// Displays a git diff summary.
    pub struct DiffHistoryTool {
        tool: Tool,
    }

    impl DiffHistoryTool {
        pub fn new() -> Self {
            let parameters = HashMap::new();
            let tool = Tool {
                name: "diff_history".to_string(),
                description: "Show recent changes using `git diff --stat`.".to_string(),
                parameters,
                required: vec![],
            };
            Self { tool }
        }
    }

    impl ToolCall for DiffHistoryTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, _arguments: &str) -> Result<String, Box<dyn Error>> {
            let output = Command::new("git")
                .arg("diff")
                .arg("--stat")
                .output();

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    if !out.status.success() && !stderr.is_empty() {
                        return Err(format!("git diff failed: {}", stderr).into());
                    }
                    if stdout.trim().is_empty() {
                        Ok("No pending changes.".to_string())
                    } else {
                        Ok(stdout.trim().to_string())
                    }
                }
                Err(e) => Err(format!("Failed to run git: {}", e).into()),
            }
        }

        fn name(&self) -> &str {
            "diff_history"
        }
    }
}

