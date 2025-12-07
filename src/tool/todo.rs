pub mod todo {
    use std::collections::HashMap;
    use std::sync::{Mutex, Arc};
    use serde_json;
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    const TODO_FILE: &str = ".pengy_todo.json";

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TodoTask {
        description: String,
        completed: bool,
    }

    pub struct TodoTool {
        tool: Tool,
        state: Arc<Mutex<Vec<TodoTask>>>,
        file_path: PathBuf,
    }

    impl TodoTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // action parameter
            let mut action_items = HashMap::new();
            action_items.insert("type".to_string(), "string".to_string());
            parameters.insert("action".to_string(), Parameter {
                items: action_items,
                description: "The action to perform: 'read' to read the current todo list, or 'modify' to tick/insert/delete tasks.".to_string(),
                enum_values: Some(vec!["read".to_string(), "modify".to_string()]),
            });

            // operation parameter (for modify action)
            let mut operation_items = HashMap::new();
            operation_items.insert("type".to_string(), "string".to_string());
            parameters.insert("operation".to_string(), Parameter {
                items: operation_items,
                description: "The operation to perform when action is 'modify': 'tick' to mark a task as completed, 'insert' to add a new task, or 'delete' to remove a task. Required when not using the 'operations' array.".to_string(),
                enum_values: Some(vec!["tick".to_string(), "insert".to_string(), "delete".to_string()]),
            });

            // task_id parameter (for tick/delete operations)
            let mut task_id_items = HashMap::new();
            task_id_items.insert("type".to_string(), "number".to_string());
            parameters.insert("task_id".to_string(), Parameter {
                items: task_id_items,
                description: "The index (0-based) of the task to tick or delete. Required for 'tick' and 'delete' operations.".to_string(),
                enum_values: None,
            });

            // task_description parameter (for insert operation)
            let mut task_desc_items = HashMap::new();
            task_desc_items.insert("type".to_string(), "string".to_string());
            parameters.insert("task_description".to_string(), Parameter {
                items: task_desc_items,
                description: "The description of the task to insert. Required for 'insert' operation.".to_string(),
                enum_values: None,
            });

            // position parameter (optional, for insert operation)
            let mut position_items = HashMap::new();
            position_items.insert("type".to_string(), "number".to_string());
            parameters.insert("position".to_string(), Parameter {
                items: position_items,
                description: "Optional position (0-based index) where to insert the new task. If not provided, the task will be appended to the end.".to_string(),
                enum_values: None,
            });

            // operations parameter (optional batch support)
            let mut operations_items = HashMap::new();
            operations_items.insert("type".to_string(), "array".to_string());
            operations_items.insert("item_type".to_string(), "object".to_string());
            parameters.insert("operations".to_string(), Parameter {
                items: operations_items,
                description: "Optional array of operations to apply sequentially when action is 'modify'. Each item should include 'operation' plus the fields required for that operation (task_id/task_description/position). Use this to tick/insert/delete multiple tasks in one call.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "todo".to_string(),
                description: "Manage a todo list. Use 'read' action to view all tasks, or 'modify' action with 'tick', 'insert', or 'delete' operations to update the list. Supports batch updates via the 'operations' array.".to_string(),
                parameters,
                required: vec!["action".to_string()],
            };

            // Determine file path for persistence (cwd)
            let file_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(TODO_FILE);

            let initial_state = Self::load_state(&file_path).unwrap_or_default();

            Self {
                tool,
                state: Arc::new(Mutex::new(initial_state)),
                file_path,
            }
        }

        fn load_state(path: &Path) -> Result<Vec<TodoTask>, Box<dyn Error>> {
            if path.exists() {
                let content = fs::read_to_string(path)?;
                if content.trim().is_empty() {
                    return Ok(Vec::new());
                }
                let tasks: Vec<TodoTask> = serde_json::from_str(&content).unwrap_or_default();
                Ok(tasks)
            } else {
                Ok(Vec::new())
            }
        }

        fn save_state(&self, tasks: &[TodoTask]) -> Result<(), Box<dyn Error>> {
            let json = serde_json::to_string_pretty(tasks)?;
            fs::write(&self.file_path, json)?;
            Ok(())
        }

        fn read_tasks(&self) -> Result<String, Box<dyn Error>> {
            // Refresh from disk in case another session updated it
            {
                let mut state_guard = self.state.lock().unwrap();
                if let Ok(disk_state) = Self::load_state(&self.file_path) {
                    *state_guard = disk_state;
                }
            }

            let state_guard = self.state.lock().unwrap();
            
            if state_guard.is_empty() {
                return Ok("Todo list is empty. Use 'modify' action with 'insert' operation to add tasks.".to_string());
            }

            let mut result = String::from("Current todo list:\n");
            for (idx, task) in state_guard.iter().enumerate() {
                let status = if task.completed { "✓" } else { " " };
                result.push_str(&format!("{}. [{}] {}\n", idx, status, task.description));
            }
            
            // Add summary to help agent understand state
            let completed_count = state_guard.iter().filter(|t| t.completed).count();
            let total_count = state_guard.len();
            result.push_str(&format!("\nSummary: {} of {} tasks completed.", completed_count, total_count));

            Ok(result.trim().to_string())
        }

        fn apply_operation(tasks: &mut Vec<TodoTask>, args: &serde_json::Value) -> Result<String, Box<dyn Error>> {
            let operation = args.get("operation")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: operation (required when action is 'modify')")?;

            match operation {
                "tick" => {
                    let task_id = args.get("task_id")
                        .and_then(|v| v.as_u64())
                        .ok_or("Missing required parameter: task_id (required for 'tick' operation)")? as usize;

                    if task_id >= tasks.len() {
                        return Err(format!("Task index {} is out of range. There are {} tasks.", task_id, tasks.len()).into());
                    }

                    tasks[task_id].completed = !tasks[task_id].completed;
                    let status = if tasks[task_id].completed { "completed" } else { "uncompleted" };
                    Ok(format!("Task {} marked as {}.", task_id, status))
                }
                "insert" => {
                    let task_description = args.get("task_description")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: task_description (required for 'insert' operation)")?;

                    let new_task = TodoTask {
                        description: task_description.to_string(),
                        completed: false,
                    };

                    // Check if position is provided
                    let result_msg = if let Some(position) = args.get("position").and_then(|v| v.as_u64()) {
                        let pos = position as usize;
                        if pos > tasks.len() {
                            return Err(format!("Position {} is out of range. There are {} tasks. Use position <= {} to insert.", pos, tasks.len(), tasks.len()).into());
                        }
                        tasks.insert(pos, new_task);
                        format!("Task inserted at position {}. Continue with the next step.", pos)
                    } else {
                        let pos = tasks.len();
                        tasks.push(new_task);
                        format!("Task added to the end of the list (position {}). Continue with the next step.", pos)
                    };
                    
                    Ok(result_msg)
                }
                "delete" => {
                    let task_id = args.get("task_id")
                        .and_then(|v| v.as_u64())
                        .ok_or("Missing required parameter: task_id (required for 'delete' operation)")? as usize;

                    if task_id >= tasks.len() {
                        return Err(format!("Task index {} is out of range. There are {} tasks.", task_id, tasks.len()).into());
                    }

                    let removed_task = tasks.remove(task_id);
                    Ok(format!("Task {} deleted: '{}'", task_id, removed_task.description))
                }
                _ => Err(format!("Unknown operation: {}. Must be 'tick', 'insert', or 'delete'.", operation).into())
            }
        }

        fn modify_task(&self, args: &serde_json::Value) -> Result<String, Box<dyn Error>> {
            // Batch operations path
            if args.get("operations").is_some() {
                let ops = args.get("operations")
                    .and_then(|v| v.as_array())
                    .ok_or("The 'operations' parameter must be an array of objects.")?;

                if ops.is_empty() {
                    return Err("The 'operations' array is empty. Provide at least one operation.".into());
                }

                let mut state_guard = self.state.lock().unwrap();
                let mut updated_tasks = state_guard.clone();
                let mut messages = Vec::new();

                for (idx, op_args) in ops.iter().enumerate() {
                    let msg = Self::apply_operation(&mut updated_tasks, op_args)
                        .map_err(|e| format!("Operation {} failed: {}", idx, e))?;
                    messages.push(msg);
                }

                *state_guard = updated_tasks;
                self.save_state(&state_guard)?;

                let summary = format!("Applied {} operations:\n{}", messages.len(), messages.join("\n"));
                return Ok(summary);
            }

            // Single operation path (backward compatible)
            let mut state_guard = self.state.lock().unwrap();
            let mut updated_tasks = state_guard.clone();

            let message = Self::apply_operation(&mut updated_tasks, args)?;

            *state_guard = updated_tasks;
            self.save_state(&state_guard)?;

            Ok(message)
        }
    }

    impl ToolCall for TodoTool {
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
                "read" => self.read_tasks(),
                "modify" => self.modify_task(&args),
                _ => Err(format!("Unknown action: {}. Must be 'read' or 'modify'.", action).into())
            }
        }

        fn name(&self) -> &str {
            "todo"
        }
    }
}

