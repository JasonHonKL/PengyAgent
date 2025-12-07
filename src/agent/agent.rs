pub mod agent {
    use crate::model::model::model::{Model, Message, Role};
    use crate::tool::tool::tool::ToolCall;
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    pub enum AgentEvent {
        Step { step: u32, max_steps: u32 },
        ToolCall { tool_name: String, args: String },
        ToolResult { result: String },
        Thinking { content: String },
        FinalResponse { content: String },
        Error { error: String },
        VisionAnalysis { status: String },
    }

    pub struct Agent {
        pub model: Model,
        pub tools: Vec<Box<dyn ToolCall>>,
        pub system_prompt: String,
        pub messages: Vec<Message>,
        max_retry: u32,
        max_step: u32,
    }

    impl Agent {
        pub fn new(
            model: Model,
            tools: Vec<Box<dyn ToolCall>>,
            system_prompt: String,
            max_retry: Option<u32>,
            max_step: Option<u32>,
        ) -> Self {
            let mut messages = Vec::new();
            messages.push(Message::new(Role::System, system_prompt.clone()));

            Self {
                model,
                tools,
                system_prompt,
                max_retry: max_retry.unwrap_or(3),
                max_step: max_step.unwrap_or(10),
                messages,
            }
        }

        pub async fn run<F>(&mut self, user_message: String, callback: F)
        where
            F: Fn(AgentEvent) + Send + Sync + 'static,
        {
            // Add user message
            self.messages.push(Message::new(Role::User, user_message));

            let mut step = 0;
            while step < self.max_step {
                step += 1;
                callback(AgentEvent::Step { step, max_steps: self.max_step });

                // Check if previous tool call was vision_judge
                let previous_was_vision_judge = self.messages.iter().rev()
                    .find(|msg| matches!(msg.role, Role::Assistant) && msg.content.starts_with("Tool call: vision_judge"))
                    .is_some();

                // If previous tool was vision_judge, first summarize the image
                if previous_was_vision_judge {
                    // Find the tool result message from vision_judge
                    let mut image_data_url: Option<String> = None;
                    
                    // Look backwards for the vision_judge tool result
                    for msg in self.messages.iter().rev() {
                        if matches!(msg.role, Role::User) && msg.content.starts_with("Tool result: ") {
                            let result = msg.content.strip_prefix("Tool result: ").unwrap_or(&msg.content);
                            // Check if this result is from vision_judge (it should be a data URL)
                            if result.starts_with("data:image/") {
                                image_data_url = Some(result.to_string());
                                break;
                            }
                        }
                        // Stop if we hit the vision_judge tool call
                        if matches!(msg.role, Role::Assistant) && msg.content.starts_with("Tool call: vision_judge") {
                            break;
                        }
                    }

                    if let Some(image_url) = image_data_url {
                        callback(AgentEvent::VisionAnalysis { status: "Analyzing image...".to_string() });
                        
                        // Check if the model base URL is OpenRouter
                        if !self.model.base_url.contains("openrouter.ai") {
                            callback(AgentEvent::Error { error: "Vision not supported for non-OpenRouter models".to_string() });
                            // Add a message indicating vision is not available
                            self.messages.push(Message::new(Role::User, 
                                "Oh no, we can't see the image right now. The current model doesn't support vision capabilities. Please use an OpenRouter model to analyze images.".to_string()));
                        } else {
                            // Create messages for vision completion (just the system prompt and a user message)
                            let vision_messages = vec![
                                Message::new(Role::System, self.system_prompt.clone()),
                                Message::new(Role::User, "here's the summary of this image".to_string()),
                            ];
                            
                            // Call vision completion to summarize the image
                            match self.model.open_router_vision_completion(image_url, vision_messages).await {
                                Ok(summary) => {
                                    callback(AgentEvent::VisionAnalysis { status: "Image analyzed".to_string() });
                                    // Add the summary as a user message
                                    self.messages.push(Message::new(Role::User, format!("Image summary: {}", summary)));
                                }
                                Err(e) => {
                                    callback(AgentEvent::Error { error: format!("Failed to summarize image: {}", e) });
                                    // Continue anyway, but add an error message
                                    self.messages.push(Message::new(Role::User, format!("Failed to summarize image: {}", e)));
                                }
                            }
                        }
                    }
                }

                // Prepare tools slice for the API call
                let tools_slice: Option<&[Box<dyn ToolCall>]> = if !self.tools.is_empty() {
                    Some(&self.tools)
                } else {
                    None
                };

                // Try to complete with retries
                let mut retry_count = 0;
                let result = loop {
                    match self.model.complete(
                        self.messages.clone(),
                        tools_slice,
                    ).await {
                        Ok(messages) => {
                            // Check if we got tool calls or final response
                            // Look for tool call messages (they come in pairs: Assistant with "Tool call:" then User with "Tool result:")
                            let mut found_tool_call = false;
                            for msg in messages.iter().rev() {
                                if matches!(msg.role, Role::Assistant) && msg.content.starts_with("Tool call:") {
                                    // Tool was called, extract and display tool info
                                    if let Some(tool_info) = msg.content.strip_prefix("Tool call: ") {
                                        // Try to parse tool name and args
                                        if let Some((name, args)) = tool_info.split_once(" with arguments: ") {
                                            callback(AgentEvent::ToolCall { 
                                                tool_name: name.to_string(), 
                                                args: args.to_string() 
                                            });
                                        } else {
                                             callback(AgentEvent::ToolCall { 
                                                tool_name: tool_info.to_string(), 
                                                args: "".to_string() 
                                            });
                                        }
                                    }
                                    found_tool_call = true;
                                    break;
                                }
                            }
                            
                            // Find the tool result message to show the result
                            if found_tool_call {
                                for msg in messages.iter().rev() {
                                    if matches!(msg.role, Role::User) && msg.content.starts_with("Tool result: ") {
                                        let result = msg.content.strip_prefix("Tool result: ").unwrap_or(&msg.content);
                                        callback(AgentEvent::ToolResult { result: result.to_string() });
                                        break;
                                    }
                                }
                                self.messages = messages;
                                break Ok(());
                            }
                            
                            // Check for final response
                            if let Some(last_msg) = messages.last() {
                                if matches!(last_msg.role, Role::Assistant) && !last_msg.content.starts_with("Tool call:") {
                                    // Final response
                                    self.messages = messages.clone();
                                    callback(AgentEvent::FinalResponse { content: last_msg.content.clone() });
                                    return;
                                }
                            }
                            
                            self.messages = messages;
                            break Ok(());
                        }
                        Err(e) => {
                            retry_count += 1;
                            if retry_count >= self.max_retry {
                                callback(AgentEvent::Error { error: format!("Error after {} retries: {}", self.max_retry, e) });
                                break Err(e);
                            }
                            callback(AgentEvent::Error { error: format!("Error (retry {}/{}): {}", retry_count, self.max_retry, e) });
                            tokio::time::sleep(tokio::time::Duration::from_millis(100 * retry_count as u64)).await;
                        }
                    }
                };

                if result.is_err() {
                    callback(AgentEvent::Error { error: "Failed to complete after retries".to_string() });
                    return;
                }

                // Check if we need to continue (tool was called) or we're done
                if let Some(last_msg) = self.messages.last() {
                    if matches!(last_msg.role, Role::Assistant) {
                        if last_msg.content.starts_with("Tool call:") {
                            // Tool was called, continue to next step
                            continue;
                        } else {
                            // Final response, we're done
                            callback(AgentEvent::FinalResponse { content: last_msg.content.clone() });
                            return;
                        }
                    }
                }
            }

            if step >= self.max_step {
                callback(AgentEvent::Error { error: format!("Reached maximum steps ({})", self.max_step) });
            }
        }

        pub fn get_messages(&self) -> &Vec<Message> {
            &self.messages
        }
    }
}