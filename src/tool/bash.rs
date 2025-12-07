pub mod bash {
    use std::collections::HashMap;
    use std::process::Command;
    use std::sync::{Mutex, Arc};
    use std::path::PathBuf;
    use serde_json;
    use std::error::Error;
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    pub struct BashTool {
        tool: Tool,
        state: Arc<Mutex<BashState>>,
    }

    struct BashState {
        working_dir: Option<PathBuf>,
        env_vars: HashMap<String, String>,
    }

    impl BashTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // restart parameter
            let mut restart_items = HashMap::new();
            restart_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert("restart".to_string(), Parameter {
                items: restart_items,
                description: "If true, restart the bash session before executing the command. This clears any previous state, environment variables, and working directory changes.".to_string(),
                enum_values: None,
            });

            // cmd parameter
            let mut cmd_items = HashMap::new();
            cmd_items.insert("type".to_string(), "string".to_string());
            parameters.insert("cmd".to_string(), Parameter {
                items: cmd_items,
                description: "The bash command to execute. SECURITY: Never write to /tmp/ or system directories. Always use relative paths like './file.txt' or 'file.txt' in the current working directory. This is a security requirement.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "bash".to_string(),
                description: "Execute bash commands in a persistent shell session. SECURITY: Never write to /tmp/ or other system directories. Always write files in the current working directory using relative paths like './file.txt' or 'file.txt'. Use the restart parameter to clear the session state if needed.".to_string(),
                parameters,
                required: vec!["cmd".to_string()],
            };

            Self {
                tool,
                state: Arc::new(Mutex::new(BashState {
                    working_dir: None,
                    env_vars: HashMap::new(),
                })),
            }
        }

        fn restart_session(&self) -> Result<(), Box<dyn Error>> {
            let mut state_guard = self.state.lock().unwrap();
            state_guard.working_dir = None;
            state_guard.env_vars.clear();
            Ok(())
        }

        fn execute_command(&self, cmd: &str) -> Result<String, Box<dyn Error>> {
            let state_guard = self.state.lock().unwrap();
            
            // Build the command with state preservation
            let mut full_cmd = String::new();
            
            // Set environment variables if any
            for (key, value) in &state_guard.env_vars {
                full_cmd.push_str(&format!("export {}='{}'; ", key, value.replace('\'', "'\\''")));
            }
            
            // Change to working directory if set
            if let Some(ref wd) = state_guard.working_dir {
                full_cmd.push_str(&format!("cd '{}' && ", wd.display()));
            }
            
            // Add the actual command
            full_cmd.push_str(cmd);
            
            // Capture the new working directory after command execution
            full_cmd.push_str(" && pwd > /tmp/bash_tool_pwd_$$");
            
            // Execute the command
            let output = Command::new("bash")
                .arg("-c")
                .arg(&full_cmd)
                .output()?;

            // Update state with new working directory
            if let Ok(pwd_content) = std::fs::read_to_string(format!("/tmp/bash_tool_pwd_{}", std::process::id())) {
                let pwd_path = PathBuf::from(pwd_content.trim());
                drop(state_guard);
                let mut state_guard = self.state.lock().unwrap();
                state_guard.working_dir = Some(pwd_path);
                let _ = std::fs::remove_file(format!("/tmp/bash_tool_pwd_{}", std::process::id()));
            }

            // Combine stdout and stderr
            let mut result = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if !stderr.trim().is_empty() {
                if !result.is_empty() {
                    result.push_str("\nSTDERR:\n");
                }
                result.push_str(&stderr);
            }

            // Check exit status
            if !output.status.success() {
                return Err(format!("Command failed with exit code {}: {}", 
                    output.status.code().unwrap_or(-1), result).into());
            }

            Ok(result.trim().to_string())
        }
    }

    impl ToolCall for BashTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // Check if restart is requested
            let restart = args.get("restart")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Get the command
            let cmd = args.get("cmd")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: cmd")?;

            // Restart session if requested
            if restart {
                self.restart_session()?;
            }

            // Execute the command
            match self.execute_command(cmd) {
                Ok(output) => {
                    if output.is_empty() {
                        Ok("Command executed successfully (no output)".to_string())
                    } else {
                        Ok(output)
                    }
                }
                Err(e) => Err(format!("Failed to execute command: {}", e).into())
            }
        }

        fn name(&self) -> &str {
            "bash"
        }
    }
}

