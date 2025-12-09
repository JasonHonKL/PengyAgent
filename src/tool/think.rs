pub mod think {
    //! Internal thinking tool that logs a free-form thought without performing
    //! any external actions or mutations. Helpful for complex reasoning or
    //! capturing intermediate ideas.

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    /// Tool that appends an internal thought to the conversation log.
    pub struct ThinkTool {
        tool: Tool,
    }

    impl ThinkTool {
        /// Create the thinking tool definition with a required `thought`
        /// parameter.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut thought_items = HashMap::new();
            thought_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "thought".to_string(),
                Parameter {
                    items: thought_items,
                    description:
                        "The internal thought to record. No external actions are performed."
                            .to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "think".to_string(),
                description: "Use the tool to think about something. It will not obtain new information or change the database, but just append the thought to the log. Use it when complex reasoning or some cache memory is needed.".to_string(),
                parameters,
                required: vec!["thought".to_string()],
            };

            Self { tool }
        }
    }

    impl ToolCall for ThinkTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Return the supplied thought so it can be logged by the agent.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments and extract the required thought text
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            let thought = args
                .get("thought")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Missing 'thought' parameter",
                    )
                })?;

            Ok(format!("THOUGHT_LOG: {}", thought))
        }

        fn name(&self) -> &str {
            "think"
        }
    }
}
