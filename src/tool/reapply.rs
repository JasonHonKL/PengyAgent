pub mod reapply {
    //! Placeholder tool for compatibility. The system does not track prior edits,
    //! so this tool returns an informative message instead of performing an
    //! action.
    use crate::tool::tool::tool::{Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    pub struct ReapplyTool {
        tool: Tool,
    }

    impl ReapplyTool {
        pub fn new() -> Self {
            let parameters = HashMap::new();
            let tool = Tool {
                name: "reapply".to_string(),
                description: "Reapply the last edit (not supported in this runtime; returns a message).".to_string(),
                parameters,
                required: vec![],
            };
            Self { tool }
        }
    }

    impl ToolCall for ReapplyTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, _arguments: &str) -> Result<String, Box<dyn Error>> {
            Ok("Reapply is not supported because prior edit context is unavailable.".to_string())
        }

        fn name(&self) -> &str {
            "reapply"
        }
    }
}

