pub mod run_terminal_cmd {
    //! Execute a shell command. This is a minimal wrapper over `bash -c` and is
    //! intended for short, non-interactive commands. For persistent state, use
    //! the `bash` tool.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::process::{Command, Stdio};

    /// Runs arbitrary commands via bash.
    pub struct RunTerminalCmdTool {
        tool: Tool,
    }

    impl RunTerminalCmdTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut cmd_items = HashMap::new();
            cmd_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "command".to_string(),
                Parameter {
                    items: cmd_items,
                    description: "Command to execute (string)".to_string(),
                    enum_values: None,
                },
            );

            let mut background_items = HashMap::new();
            background_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "is_background".to_string(),
                Parameter {
                    items: background_items,
                    description: "If true, run without waiting for completion.".to_string(),
                    enum_values: None,
                },
            );

            let mut approval_items = HashMap::new();
            approval_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert(
                "require_user_approval".to_string(),
                Parameter {
                    items: approval_items,
                    description: "Ignored in this implementation; included for compatibility."
                        .to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "run_terminal_cmd".to_string(),
                description: "Execute a shell command via `bash -c`. For persistent sessions, prefer the `bash` tool.".to_string(),
                parameters,
                required: vec!["command".to_string()],
            };

            Self { tool }
        }
    }

    impl ToolCall for RunTerminalCmdTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: command")?;
            let is_background = args
                .get("is_background")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_background {
                let child = Command::new("bash")
                    .arg("-c")
                    .arg(command)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
                Ok(format!(
                    "Command started in background with PID {}",
                    child.id()
                ))
            } else {
                let output = Command::new("bash").arg("-c").arg(command).output()?;
                let mut result = String::new();
                result.push_str(&String::from_utf8_lossy(&output.stdout));
                if !output.stderr.is_empty() {
                    if !result.is_empty() {
                        result.push_str("\nSTDERR:\n");
                    }
                    result.push_str(&String::from_utf8_lossy(&output.stderr));
                }

                if !output.status.success() {
                    return Err(format!(
                        "Command failed with exit code {}: {}",
                        output.status.code().unwrap_or(-1),
                        result
                    )
                    .into());
                }

                if result.trim().is_empty() {
                    Ok("Command executed successfully (no output)".to_string())
                } else {
                    Ok(result.trim().to_string())
                }
            }
        }

        fn name(&self) -> &str {
            "run_terminal_cmd"
        }
    }
}
