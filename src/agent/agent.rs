pub mod agent {
    use crate::model::model::model::{Model, Message, Role};
    use crate::tool::tool::tool::ToolCall;
    use serde_json;

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
                                        // Try to parse as JSON first (new format) with robust error handling
                                        let (tool_name, args, parse_success) = match serde_json::from_str::<serde_json::Value>(tool_info.trim()) {
                                            Ok(json) => {
                                                // New JSON format: {"name":"...","arguments":"..."}
                                                let name = json.get("name")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();
                                                
                                                // Handle arguments - it's stored as a JSON string, so extract it as a string
                                                // The arguments field contains a JSON string that may be escaped
                                                let args = json.get("arguments")
                                                    .and_then(|v| {
                                                        // If it's a string, use it directly (it's already a JSON string)
                                                        if let Some(s) = v.as_str() {
                                                            Some(s.to_string())
                                                        } else {
                                                            // If it's an object/array, serialize it back to JSON string
                                                            serde_json::to_string(v).ok()
                                                        }
                                                    })
                                                    .unwrap_or_default();
                                                
                                                (name, args, true)
                                            }
                                            Err(_e) => {
                                                // JSON parsing failed - try fallback methods
                                                
                                                // Method 1: Try old format
                                        if let Some((name, args)) = tool_info.split_once(" with arguments: ") {
                                                    (name.to_string(), args.to_string(), false)
                                                } else {
                                                    // Method 2: Try to extract name and arguments using string manipulation
                                                    // Look for "name":"value" and "arguments":"value" patterns
                                                    let mut extracted_name = String::new();
                                                    let mut extracted_args = String::new();
                                                    
                                                    // Extract name
                                                    if let Some(name_idx) = tool_info.find("\"name\"") {
                                                        let after_name = &tool_info[name_idx + 6..];
                                                        if let Some(colon_idx) = after_name.find(':') {
                                                            let value_part = &after_name[colon_idx + 1..].trim_start();
                                                            if value_part.starts_with('"') {
                                                                // Find the closing quote, handling escaped quotes
                                                                let chars = value_part[1..].char_indices();
                                                                let mut end_idx = 0;
                                                                let mut escaped = false;
                                                                
                                                                for (i, ch) in chars {
                                                                    if escaped {
                                                                        escaped = false;
                                                                        continue;
                                                                    }
                                                                    if ch == '\\' {
                                                                        escaped = true;
                                                                        continue;
                                                                    }
                                                                    if ch == '"' {
                                                                        end_idx = i;
                                                                        break;
                                                                    }
                                                                }
                                                                
                                                                if end_idx > 0 {
                                                                    extracted_name = value_part[1..end_idx + 1].to_string();
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Extract arguments (similar logic)
                                                    if let Some(args_idx) = tool_info.find("\"arguments\"") {
                                                        let after_args = &tool_info[args_idx + 11..];
                                                        if let Some(colon_idx) = after_args.find(':') {
                                                            let value_part = &after_args[colon_idx + 1..].trim_start();
                                                            if value_part.starts_with('"') {
                                                                // Find the closing quote, handling escaped quotes and nested JSON
                                                                let chars = value_part[1..].char_indices();
                                                                let mut end_idx = 0;
                                                                let mut escaped = false;
                                                                let mut depth = 0;
                                                                
                                                                for (i, ch) in chars {
                                                                    if escaped {
                                                                        escaped = false;
                                                                        continue;
                                                                    }
                                                                    if ch == '\\' {
                                                                        escaped = true;
                                                                        continue;
                                                                    }
                                                                    if ch == '{' || ch == '[' {
                                                                        depth += 1;
                                                                    } else if ch == '}' || ch == ']' {
                                                                        depth -= 1;
                                                                    } else if ch == '"' && depth == 0 {
                                                                        end_idx = i;
                                                                        break;
                                                                    }
                                                                }
                                                                
                                                                if end_idx > 0 {
                                                                    extracted_args = value_part[1..end_idx + 1].to_string();
                                                                } else {
                                                                    // If we didn't find a closing quote, try to extract until the next comma or }
                                                                    if let Some(comma_idx) = value_part[1..].find(',') {
                                                                        extracted_args = value_part[1..comma_idx + 1].to_string();
                                                                    } else if let Some(brace_idx) = value_part[1..].find('}') {
                                                                        extracted_args = value_part[1..brace_idx + 1].to_string();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    if !extracted_name.is_empty() {
                                                        (extracted_name, extracted_args, false)
                                                    } else {
                                                        // Last resort: try to find any tool name pattern
                                                        // Look for common tool names in the string
                                                        let known_tools = ["bash", "file_manager", "web", "todo", "end", "grep", "vector_search"];
                                                        let found_tool = known_tools.iter()
                                                            .find(|tool| tool_info.contains(*tool))
                                                            .map(|s| s.to_string());
                                                        
                                                        if let Some(tool) = found_tool {
                                                            (tool, String::new(), false)
                                        } else {
                                                            // Really can't parse - return unknown but log error
                                                            eprintln!("Critical: Could not parse tool call: {}", tool_info);
                                                            ("unknown".to_string(), String::new(), false)
                                                        }
                                                    }
                                                }
                                            }
                                        };
                                        
                                        // If parsing failed, log warning for debugging
                                        if !parse_success {
                                            eprintln!("Tool call parsing warning - tool: {}, args: {}", tool_name, args);
                                        }
                                        
                                        callback(AgentEvent::ToolCall { 
                                            tool_name, 
                                            args 
                                        });
                                    }
                                    found_tool_call = true;
                                    break;
                                }
                            }
                            
                            // Find the tool result message to show the result
                            // Also check if this tool call was actually executed (has a tool result)
                            if found_tool_call {
                                let mut tool_name: Option<String> = None;
                                let mut tool_result: Option<String> = None;
                                let mut tool_was_executed = false;
                                
                                // Find the tool call and result
                                for msg in messages.iter().rev() {
                                    if matches!(msg.role, Role::User) && msg.content.starts_with("Tool result: ") {
                                        let result = msg.content.strip_prefix("Tool result: ").unwrap_or(&msg.content);
                                        tool_result = Some(result.to_string());
                                        tool_was_executed = true;
                                        callback(AgentEvent::ToolResult { result: result.to_string() });
                                    }
                                    if matches!(msg.role, Role::Assistant) && msg.content.starts_with("Tool call: ") {
                                        if let Some(tool_info) = msg.content.strip_prefix("Tool call: ") {
                                            // Try to parse as JSON first (new format) with robust error handling
                                            tool_name = match serde_json::from_str::<serde_json::Value>(tool_info.trim()) {
                                                Ok(json) => {
                                                    // New JSON format
                                                    json.get("name")
                                                        .and_then(|v| v.as_str())
                                                        .map(|s| s.to_string())
                                                }
                                                Err(_) => {
                                                    // Fallback to old format
                                            if let Some((name, _)) = tool_info.split_once(" with arguments: ") {
                                                        Some(name.to_string())
                                                    } else {
                                                        // Try to extract name from malformed JSON
                                                        if let Some(name_idx) = tool_info.find("\"name\"") {
                                                            let after_name = &tool_info[name_idx + 6..];
                                                            if let Some(colon_idx) = after_name.find(':') {
                                                                let value_part = &after_name[colon_idx + 1..].trim_start();
                                                                if value_part.starts_with('"') {
                                                                    if let Some(end_idx) = value_part[1..].find('"') {
                                                                        Some(value_part[1..end_idx + 1].to_string())
                                                                    } else {
                                                                        Some("unknown".to_string())
                                                                    }
                                                                } else {
                                                                    Some("unknown".to_string())
                                                                }
                                                            } else {
                                                                Some("unknown".to_string())
                                                            }
                                            } else {
                                                            Some(tool_info.to_string())
                                                        }
                                                    }
                                            }
                                            };
                                        }
                                        break;
                                    }
                                }
                                
                                // Check if end tool was called to terminate early
                                if tool_name.as_deref() == Some("end") || tool_name.as_deref() == Some("endtool") ||
                                    tool_result.as_deref().map(|r| r.starts_with("END_CONVERSATION")).unwrap_or(false) {
                                    let content = if let Some(result) = tool_result {
                                        if let Some(reason) = result.strip_prefix("END_CONVERSATION: ") {
                                            format!("Ending conversation early: {}", reason)
                                        } else if result == "END_CONVERSATION" {
                                            "Ending conversation early as requested.".to_string()
                                        } else {
                                            format!("Ending conversation early: {}", result)
                                        }
                                    } else {
                                        "Ending conversation early as requested.".to_string()
                                    };

                                    let mut updated_messages = messages.clone();
                                    updated_messages.push(Message::new(Role::Assistant, content.clone()));
                                    self.messages = updated_messages;
                                    callback(AgentEvent::FinalResponse { content });
                                    return;
                                }

                                // Check if summarizer tool was called
                                if tool_name.as_deref() == Some("summarizer") && tool_result.as_deref() == Some("SUMMARIZE_CONVERSATION") {
                                    // Handle summarization
                                    match self.summarize_conversation().await {
                                        Ok(summarized_messages) => {
                                            self.messages = summarized_messages;
                                            callback(AgentEvent::ToolResult { result: "Conversation summarized successfully".to_string() });
                                        }
                                        Err(e) => {
                                            callback(AgentEvent::Error { error: format!("Failed to summarize conversation: {}", e) });
                                            // Continue with original messages
                                            self.messages = messages.clone();
                                        }
                                    }
                                } else if tool_was_executed {
                                    // Tool was executed, just update messages
                                    self.messages = messages.clone();
                                }
                                
                                // If tool was already executed (has tool result), we're done with this iteration
                                if tool_was_executed {
                                    break Ok(());
                                }
                                
                                // If tool was NOT executed (no tool result), fall through to text-based execution below
                                eprintln!("DEBUG: Tool call found but not executed, will execute as text-based tool call");
                            }
                            
                            // Check if the model returned a text-based tool call that wasn't executed
                            // (Sometimes models return "Tool call: {JSON}" as text instead of using the API's tool_calls feature)
                            if let Some(last_msg) = messages.last() {
                                let content_trimmed = last_msg.content.trim();
                                if matches!(last_msg.role, Role::Assistant) && content_trimmed.starts_with("Tool call:") {
                                    eprintln!("DEBUG: Detected text-based tool call in assistant message");
                                    eprintln!("DEBUG: Full content (first 200 chars): {}", &last_msg.content[..last_msg.content.len().min(200)]);
                                    // This is a text-based tool call that needs to be executed manually
                                    if let Some(tool_info) = content_trimmed.strip_prefix("Tool call: ") {
                                        eprintln!("DEBUG: Tool info: {}", &tool_info[..tool_info.len().min(100)]);
                                        // Try to parse and execute the tool call
                                        match serde_json::from_str::<serde_json::Value>(tool_info.trim()) {
                                            Ok(json) => {
                                                eprintln!("DEBUG: Successfully parsed tool call JSON");
                                                let tool_name = json.get("name")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("unknown");
                                                let arguments = json.get("arguments")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("");
                                                
                                                eprintln!("DEBUG: Tool name: {}, arguments length: {}", tool_name, arguments.len());
                                                
                                                // Find and execute the tool
                                                if let Some(tool) = tools_slice.and_then(|tools| tools.iter().find(|t| t.name() == tool_name)) {
                                                    eprintln!("DEBUG: Found tool {}, executing...", tool_name);
                                                    callback(AgentEvent::ToolCall { 
                                                        tool_name: tool_name.to_string(), 
                                                        args: arguments.to_string() 
                                                    });
                                                    
                                                    match tool.run(arguments) {
                                                        Ok(result) => {
                                                            eprintln!("DEBUG: Tool execution succeeded");
                                                            callback(AgentEvent::ToolResult { result: result.clone() });
                                                            // Add the tool result message and update self.messages
                                                            let mut updated_messages = messages.clone();
                                                            updated_messages.push(Message::new(Role::User, format!("Tool result: {}", result)));
                                                            self.messages = updated_messages;
                                                            break Ok(());
                                                        }
                                                        Err(e) => {
                                                            eprintln!("DEBUG: Tool execution failed: {}", e);
                                                            let error_result = format!("Tool error: {}", e);
                                                            callback(AgentEvent::ToolResult { result: error_result.clone() });
                                                            let mut updated_messages = messages.clone();
                                                            updated_messages.push(Message::new(Role::User, format!("Tool result: {}", error_result)));
                                                            self.messages = updated_messages;
                                                            break Ok(());
                                                        }
                                                    }
                                                } else {
                                                    eprintln!("DEBUG: Tool '{}' not found in tools list", tool_name);
                                                    // Tool not found
                                                    let error_result = format!("Tool error: Tool '{}' not found", tool_name);
                                                    callback(AgentEvent::ToolResult { result: error_result.clone() });
                                                    let mut updated_messages = messages.clone();
                                                    updated_messages.push(Message::new(Role::User, format!("Tool result: {}", error_result)));
                                                    self.messages = updated_messages;
                                                    break Ok(());
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("DEBUG: Failed to parse tool call JSON: {}", e);
                                                // Failed to parse - treat as malformed tool call
                                                let error_result = format!("Tool error: Failed to parse tool call JSON: {}", e);
                                                callback(AgentEvent::ToolResult { result: error_result.clone() });
                                                let mut updated_messages = messages.clone();
                                                updated_messages.push(Message::new(Role::User, format!("Tool result: {}", error_result)));
                                                self.messages = updated_messages;
                                break Ok(());
                                            }
                                        }
                                    } else {
                                        eprintln!("DEBUG: Could not strip 'Tool call: ' prefix");
                                    }
                                } else {
                                    eprintln!("DEBUG: Last message is not a tool call. Role: {:?}, starts_with Tool call: {}", 
                                        last_msg.role, 
                                        last_msg.content.starts_with("Tool call:"));
                                }
                            }
                            
                            // Check for final response
                            if let Some(last_msg) = messages.last() {
                                let content_trimmed = last_msg.content.trim();
                                if matches!(last_msg.role, Role::Assistant) && !content_trimmed.starts_with("Tool call:") {
                                    // Final response
                                    self.messages = messages.clone();
                                    callback(AgentEvent::FinalResponse { content: last_msg.content.clone() });
                                    return;
                                } else if matches!(last_msg.role, Role::Assistant) && content_trimmed.starts_with("Tool call:") {
                                    eprintln!("DEBUG: Final response check detected tool call, should not show as final response");
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
                // After a tool call execution, messages will be: Assistant "Tool call: ..." followed by User "Tool result: ..."
                // We need to continue the loop to get the model's response to the tool result
                if let Some(last_msg) = self.messages.last() {
                    // If last message is a tool result, a tool was just executed - continue to get model's response
                    if matches!(last_msg.role, Role::User) && last_msg.content.starts_with("Tool result: ") {
                        continue;
                    }
                    
                    // If last message is Assistant with tool call, tool was called but result not added yet - continue
                    if matches!(last_msg.role, Role::Assistant) && last_msg.content.starts_with("Tool call:") {
                            continue;
                    }
                    
                    // If last message is Assistant without tool call, it's a final response - we're done
                    if matches!(last_msg.role, Role::Assistant) && !last_msg.content.starts_with("Tool call:") {
                            callback(AgentEvent::FinalResponse { content: last_msg.content.clone() });
                            return;
                    }
                }
                
                // Default: continue the loop (might be waiting for model response or tool execution)
                continue;
            }

            if step >= self.max_step {
                callback(AgentEvent::Error { error: format!("Reached maximum steps ({})", self.max_step) });
            }
        }

        pub fn get_messages(&self) -> &Vec<Message> {
            &self.messages
        }

        async fn summarize_conversation(&self) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
            // Find the last user message (excluding tool results)
            let mut last_user_message: Option<String> = None;
            
            // Iterate backwards to find the last non-tool-result user message
            for msg in self.messages.iter().rev() {
                if matches!(msg.role, Role::User) && !msg.content.starts_with("Tool result: ") {
                    last_user_message = Some(msg.content.clone());
                    break;
                }
            }
            
            // Get all messages except system prompt and the last user message for summarization
            let mut messages_to_summarize: Vec<Message> = Vec::new();
            let mut found_last_user = false;
            
            // Iterate through messages and collect those to summarize
            for msg in self.messages.iter() {
                match &msg.role {
                    Role::System => {
                        // Skip system prompt - we'll add it back later
                        continue;
                    }
                    Role::User => {
                        // Skip tool result messages
                        if msg.content.starts_with("Tool result: ") {
                            continue;
                        }
                        // Check if this is the last user message - skip it as we'll add it back
                        if !found_last_user && last_user_message.as_ref() == Some(&msg.content) {
                            found_last_user = true;
                            continue;
                        }
                        messages_to_summarize.push(msg.clone());
                    }
                    Role::Assistant => {
                        // Include assistant messages in summary
                        messages_to_summarize.push(msg.clone());
                    }
                }
            }
            
            // If we have messages to summarize, create a summary
            let summary = if !messages_to_summarize.is_empty() {
                // Create summary prompt
                let conversation_text: String = messages_to_summarize.iter()
                    .map(|msg| {
                        let role_str = match msg.role {
                            Role::User => "User",
                            Role::Assistant => "Assistant",
                            Role::System => "System",
                        };
                        format!("{}: {}\n", role_str, msg.content)
                    })
                    .collect();
                
                let summary_prompt = format!(
                    "Please provide a concise summary of the following conversation. Focus on key decisions, actions taken, and important context. Keep it brief but informative:\n\n{}",
                    conversation_text
                );
                
                // Call the model to generate summary
                let summary_messages = vec![
                    Message::new(Role::System, "You are a helpful assistant that summarizes conversations concisely.".to_string()),
                    Message::new(Role::User, summary_prompt),
                ];
                
                let summary_response = self.model.complete(summary_messages, None).await?;
                
                // Extract summary from response
                summary_response.iter()
                    .find(|msg| matches!(msg.role, Role::Assistant))
                    .map(|msg| msg.content.clone())
                    .unwrap_or_else(|| "Summary: Previous conversation context".to_string())
            } else {
                "Summary: Previous conversation context".to_string()
            };
            
            // Build new messages: system prompt, {assistant, summarize}, {user, last message}
            let mut new_messages = Vec::new();
            new_messages.push(Message::new(Role::System, self.system_prompt.clone()));
            // Assistant message with the summary
            new_messages.push(Message::new(Role::Assistant, summary));
            
            // User message with the last message
            if let Some(last_msg) = last_user_message {
                new_messages.push(Message::new(Role::User, last_msg));
            }
            
            Ok(new_messages)
        }
    }
}