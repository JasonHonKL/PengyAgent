pub mod summarizer {
    use crate::tool::tool::tool::{Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    pub struct SummarizerTool {
        tool: Tool,
    }

    impl SummarizerTool {
        pub fn new() -> Self {
            let tool = Tool {
                name: "summarizer".to_string(),
                description: "Summarize the previous conversation to avoid context explosion. This tool takes no parameters and should be called when the conversation becomes too long. It will summarize all previous messages while keeping the last user message intact. After summarization, the conversation will continue with the summary and the last message.".to_string(),
                parameters: HashMap::new(),
                required: Vec::new(),
            };

            Self { tool }
        }
    }

    impl ToolCall for SummarizerTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments - should be empty or {}
            let _args: serde_json::Value = serde_json::from_str(arguments)?;

            // This tool returns a special marker that indicates summarization should happen
            // The actual summarization logic will be handled in the agent
            Ok("SUMMARIZE_CONVERSATION".to_string())
        }

        fn name(&self) -> &str {
            "summarizer"
        }
    }
}
