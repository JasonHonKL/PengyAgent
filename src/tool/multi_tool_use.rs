pub mod multi_tool_use {
    //! Compatibility wrapper for a hypothetical parallel executor. This runtime
    //! executes tools sequentially, so the tool returns a message describing the
    //! limitation.
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    pub struct ParallelTool {
        tool: Tool,
    }

    impl ParallelTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            // Accept an arbitrary payload to stay compatible, but ignore it.
            let mut tool_uses_items = HashMap::new();
            tool_uses_items.insert("type".to_string(), "array".to_string());
            tool_uses_items.insert("item_type".to_string(), "object".to_string());
            parameters.insert(
                "tool_uses".to_string(),
                Parameter {
                    items: tool_uses_items,
                    description: "List of tool calls to execute (ignored in this implementation; tools run sequentially).".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "multi_tool_use.parallel".to_string(),
                description:
                    "Placeholder: parallel tool execution is not supported; tools run sequentially."
                        .to_string(),
                parameters,
                required: vec![],
            };
            Self { tool }
        }
    }

    impl ToolCall for ParallelTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, _arguments: &str) -> Result<String, Box<dyn Error>> {
            Ok("Parallel execution is not supported; please run tools sequentially.".to_string())
        }

        fn name(&self) -> &str {
            "multi_tool_use.parallel"
        }
    }
}
