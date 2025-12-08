pub mod end {
    //! Tool that signals the agent loop to finish early and return control to
    //! the caller with an optional user-facing reason.

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    /// Tool that allows the agent to end the current run early.
    /// Returns a sentinel string that the agent loop interprets as a final response.
    pub struct EndTool {
        tool: Tool,
    }

    impl EndTool {
        /// Create the sentinel `end` tool definition with an optional reason
        /// parameter.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            let mut reason_items = HashMap::new();
            reason_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "reason".to_string(),
                Parameter {
                    items: reason_items,
                    description:
                        "Optional short reason for ending early. This will be echoed to the user."
                            .to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "end".to_string(),
                description: "End the current agent run immediately. Provide an optional 'reason' to include in the final message.".to_string(),
                parameters,
                required: Vec::new(),
            };

            Self { tool }
        }
    }

    impl ToolCall for EndTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Parse the optional reason and return the end-of-conversation marker.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON (may be empty or contain an optional reason)
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            let reason = args
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let marker = if let Some(reason) = reason {
                format!("END_CONVERSATION: {}", reason)
            } else {
                "END_CONVERSATION".to_string()
            };

            Ok(marker)
        }

        fn name(&self) -> &str {
            "end"
        }
    }
}

