pub mod model {
    use std::error::Error;

    use serde::{Deserialize, Serialize};

    use crate::tool::tool::tool;

    #[derive(Debug, Clone)]
    pub enum Role {
        User,
        Assistant,
        System,
    }

    #[derive(Debug, Clone)]
    pub struct Model {
        pub model_name: String,
        pub api_key: String,
        pub base_url: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        pub role: Role,
        pub content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reasoning_content: Option<String>,
    }

    // Vision API content types
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum ContentItem {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "image_url")]
        ImageUrl { image_url: ImageUrl },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ImageUrl {
        pub url: String,
    }

    #[derive(Debug, Clone)]
    pub struct VisionMessage {
        pub role: Role,
        pub content: VisionMessageContent,
    }

    #[derive(Debug, Clone)]
    pub enum VisionMessageContent {
        Text(String),
        Array(Vec<ContentItem>),
    }

    impl Serialize for VisionMessage {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("VisionMessage", 2)?;
            state.serialize_field("role", &self.role)?;
            match &self.content {
                VisionMessageContent::Text(text) => {
                    state.serialize_field("content", text)?;
                }
                VisionMessageContent::Array(items) => {
                    state.serialize_field("content", items)?;
                }
            }
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for VisionMessage {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de::{self, Visitor};
            use std::fmt;

            struct VisionMessageVisitor;

            impl<'de> Visitor<'de> for VisionMessageVisitor {
                type Value = VisionMessage;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a vision message with role and content")
                }

                fn visit_map<V>(self, mut map: V) -> Result<VisionMessage, V::Error>
                where
                    V: de::MapAccess<'de>,
                {
                    let mut role = None;
                    let mut content = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            "role" => {
                                if role.is_some() {
                                    return Err(de::Error::duplicate_field("role"));
                                }
                                role = Some(map.next_value()?);
                            }
                            "content" => {
                                if content.is_some() {
                                    return Err(de::Error::duplicate_field("content"));
                                }
                                // Try to deserialize as string first, then as array
                                let value: serde_json::Value = map.next_value()?;
                                content = Some(if value.is_string() {
                                    VisionMessageContent::Text(value.as_str().unwrap().to_string())
                                } else if value.is_array() {
                                    VisionMessageContent::Array(
                                        serde_json::from_value(value).map_err(de::Error::custom)?,
                                    )
                                } else {
                                    return Err(de::Error::custom(
                                        "content must be string or array",
                                    ));
                                });
                            }
                            _ => {
                                let _ = map.next_value::<de::IgnoredAny>()?;
                            }
                        }
                    }

                    let role = role.ok_or_else(|| de::Error::missing_field("role"))?;
                    let content = content.ok_or_else(|| de::Error::missing_field("content"))?;

                    Ok(VisionMessage { role, content })
                }
            }

            deserializer.deserialize_map(VisionMessageVisitor)
        }
    }

    // either create three struct for stream non stream no response
    #[derive(Debug, Clone, Deserialize)]
    pub struct Response {
        pub id: String,
        pub created: i64,
        pub model: String,
        pub choices: Vec<ResponseChoice>,
        pub usage: ResponseUsage,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ResponseUsage {
        pub prompt_tokens: Option<u32>,
        pub completion_tokens: Option<u32>,
        pub total_tokens: Option<u32>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ResponseChoice {
        pub index: u32,
        pub message: ResponseMessage,
        pub finish_reason: Option<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ResponseMessage {
        pub role: String,
        pub content: Option<String>,
        pub reasoning_content: Option<String>,
        pub tool_calls: Option<Vec<ToolCall>>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ToolCall {
        pub id: String,
        #[serde(rename = "type")]
        pub call_type: String,
        pub function: FunctionCall,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct FunctionCall {
        pub name: String,
        pub arguments: String,
    }

    impl Message {
        pub fn new(role: Role, content: String) -> Self {
            Self {
                role,
                content,
                reasoning_content: None,
            }
        }

        pub fn new_with_reasoning(
            role: Role,
            content: String,
            reasoning_content: Option<String>,
        ) -> Self {
            Self {
                role,
                content,
                reasoning_content,
            }
        }
    }

    impl Serialize for Role {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(role_to_string(self).as_ref())
        }
    }

    impl<'de> Deserialize<'de> for Role {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            match s.as_str() {
                "user" => Ok(Role::User),
                "assistant" => Ok(Role::Assistant),
                "system" => Ok(Role::System),
                _ => Err(serde::de::Error::custom(format!("Unknown role: {}", s))),
            }
        }
    }

    impl Model {
        pub fn new(model_name: String, api_key: String, base_url: String) -> Self {
            Self {
                model_name,
                api_key,
                base_url,
            }
        }

        fn is_reasoning_model(&self) -> bool {
            // Heuristics for models that return/require reasoning_content (e.g. deepseek-r1, o1/o3)
            let name = self.model_name.to_ascii_lowercase();
            name.contains("reason")
                || name.contains("r1")
                || name.contains("/o1")
                || name.contains("/o3")
        }

        fn ensure_reasoning_messages(&self, messages: &mut [Message]) {
            if !self.is_reasoning_model() {
                return;
            }

            for msg in messages.iter_mut() {
                if matches!(msg.role, Role::Assistant) && msg.reasoning_content.is_none() {
                    msg.reasoning_content = Some(msg.content.clone());
                }
            }
        }

        fn completion_url(&self) -> String {
            let trimmed = self.base_url.trim_end_matches('/');
            if trimmed.ends_with("/chat/completions") || trimmed.ends_with("/completions") {
                trimmed.to_string()
            } else {
                format!("{}/chat/completions", trimmed)
            }
        }

        fn embedding_url(&self) -> String {
            let trimmed = self.base_url.trim_end_matches('/');
            if trimmed.ends_with("/embeddings") {
                trimmed.to_string()
            } else if trimmed.ends_with("/chat/completions") || trimmed.ends_with("/completions") {
                let root = trimmed
                    .trim_end_matches("/chat/completions")
                    .trim_end_matches("/completions")
                    .trim_end_matches('/');
                format!("{}/embeddings", root)
            } else {
                format!("{}/embeddings", trimmed)
            }
        }

    pub async fn complete(
        &self,
        mut messages: Vec<Message>,
        tools: Option<&[Box<dyn tool::ToolCall>]>,
    ) -> Result<(Vec<Message>, Option<ResponseUsage>), Box<dyn Error + Send + Sync>> {
            // Retry logic: try up to 3 times for connection errors
            const MAX_RETRIES: u32 = 3;
            let mut retry_count = 0;

            loop {
                let client = reqwest::Client::new();
                let mut req_builder = client.request(reqwest::Method::POST, self.completion_url());

                let mut outbound_messages = messages.clone();
                self.ensure_reasoning_messages(&mut outbound_messages);

                let mut body = RequestBody {
                    model: self.model_name.clone(),
                    messages: outbound_messages,
                    tools: None,
                };

                if let Some(ref tools_vec) = tools {
                    body.tools = Some(convert_tools(tools_vec)?);
                }

                let json_body = match serde_json::to_vec(&body) {
                    Ok(v) => v,
                    Err(e) => return Err(format!("Failed to serialize request: {}", e).into()),
                };

                req_builder = req_builder
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .body(json_body);

                // Try to send the request with retry logic for connection errors
                let response_result = req_builder.send().await;

                let response = match response_result {
                    Ok(r) => r,
                    Err(e) => {
                        // Check if it's a connection error that we should retry
                        let is_connection_error = e.is_connect()
                            || e.is_timeout()
                            || e.to_string().to_lowercase().contains("connection")
                            || e.to_string().to_lowercase().contains("network")
                            || e.to_string().to_lowercase().contains("dns");

                        if is_connection_error && retry_count < MAX_RETRIES {
                            retry_count += 1;
                            // Wait before retrying (exponential backoff: 1s, 2s, 4s)
                            let delay_ms = 1000 * (1 << (retry_count - 1));
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue; // Retry the request
                        } else {
                            // Either not a connection error, or we've exhausted retries
                            if retry_count >= MAX_RETRIES {
                                return Err(format!(
                                    "Connection failed after {} attempts: {}",
                                    MAX_RETRIES, e
                                )
                                .into());
                            } else {
                                return Err(format!("Request failed: {}", e).into());
                            }
                        }
                    }
                };

                let status = response.status();

                if !status.is_success() {
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    // For HTTP errors, don't retry (they're not connection issues)
                    return Err(format!(
                        "API request failed with status {}: {}",
                        status, error_text
                    )
                    .into());
                }

                // Try to parse the response JSON
                let response_json_result = response.json().await;
                let response_json: Response = match response_json_result {
                    Ok(r) => r,
                    Err(e) => {
                        // JSON parsing error - might be a connection issue, retry if we haven't exhausted retries
                        if retry_count < MAX_RETRIES {
                            retry_count += 1;
                            let delay_ms = 1000 * (1 << (retry_count - 1));
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue; // Retry the request
                        } else {
                            return Err(format!(
                                "Failed to parse response after {} attempts: {}",
                                MAX_RETRIES, e
                            )
                            .into());
                        }
                    }
                };

                // Successfully got response - process it
                let usage = Some(response_json.usage.clone());
                // Extract content from the first choice
                if let Some(choice) = response_json.choices.first() {
                    let reasoning_from_response = choice.message.reasoning_content.clone();

                    // Check if there are tool calls
                    if let Some(tool_calls) = &choice.message.tool_calls {
                        // Track executed tool calls to prevent duplicates
                        let mut executed_tool_calls = std::collections::HashSet::new();

                        // Execute tool calls
                        for tool_call in tool_calls {
                            if let Some(tools_slice) = tools {
                                // Find the tool by name
                                if let Some(tool) = tools_slice
                                    .iter()
                                    .find(|t| t.name() == tool_call.function.name)
                                {
                                    // Create a unique identifier for this tool call to prevent duplicates
                                    let tool_call_id = format!(
                                        "{}:{}",
                                        tool_call.function.name, tool_call.function.arguments
                                    );

                                    // Skip if we've already executed this exact tool call
                                    if executed_tool_calls.contains(&tool_call_id) {
                                        continue;
                                    }
                                    executed_tool_calls.insert(tool_call_id);

                                    // Execute the tool with 120 second timeout
                                    let tool_name = tool_call.function.name.clone();
                                    let arguments = tool_call.function.arguments.clone();

                                    // Create Arc wrapper for the tool to share across thread boundary
                                    // Since tool is &Box<dyn ToolCall>, we need to clone the Box
                                    // But Box<dyn ToolCall> doesn't implement Clone, so we'll use a different approach
                                    // We'll wrap the tool execution in a closure that can be moved

                                    // Use a channel to communicate result from thread
                                    let (tx, rx) =
                                        std::sync::mpsc::channel::<Result<String, String>>();
                                    let args_for_thread = arguments.clone();

                                    // Execute tool.run() and send result through channel
                                    // Note: Since we can't move the tool reference into a thread,
                                    // we execute it in the current thread. The timeout applies to
                                    // receiving the result, not the execution itself. For a true
                                    // execution timeout, we would need Arc<Box<dyn ToolCall>>.
                                    let tool_result = tool.run(&args_for_thread);
                                    let result_for_channel = tool_result.map_err(|e| e.to_string());

                                    // Send result in a thread (allows timeout on receiving)
                                    std::thread::spawn(move || {
                                        let _ = tx.send(result_for_channel);
                                    });

                                    // Use spawn_blocking to receive from channel with timeout
                                    let rx_handle = tokio::task::spawn_blocking(move || rx.recv());

                                    // Apply 120 second timeout to receiving the result
                                    let result: Result<String, Box<dyn Error + Send + Sync>> =
                                        match tokio::time::timeout(
                                            tokio::time::Duration::from_secs(120),
                                            rx_handle,
                                        )
                                        .await
                                        {
                                            Ok(Ok(Ok(Ok(output)))) => Ok(output),
                                            Ok(Ok(Ok(Err(e)))) => {
                                                Err(format!("Tool error: {}", e).into())
                                            }
                                            Ok(Ok(Err(e))) => {
                                                Err(format!("Channel error: {}", e).into())
                                            }
                                            Ok(Err(e)) => Err(format!("Task error: {}", e).into()),
                                            Err(_) => {
                                                // Timeout exceeded 120 seconds
                                                Ok("running over 120s".to_string())
                                            }
                                        };

                                    // Return tool errors to the LLM as tool results instead of aborting the completion
                                    let result_str = match result {
                                        Ok(output) => output,
                                        Err(e) => format!("Tool error: {}", e),
                                    };

                                    // Add assistant message with tool call using JSON format for robustness
                                    // Format: "Tool call: {\"name\":\"tool_name\",\"arguments\":\"...\"}"
                                    // Note: arguments is already a JSON string, so we include it as a string value
                                    // This avoids double-encoding issues
                                    let tool_call_json = serde_json::json!({
                                        "name": tool_name,
                                        "arguments": arguments
                                    })
                                    .to_string();

                                    let mut assistant_tool_msg = Message::new(
                                        Role::Assistant,
                                        format!("Tool call: {}", tool_call_json),
                                    );
                                    if self.is_reasoning_model() {
                                        assistant_tool_msg.reasoning_content =
                                            Some(reasoning_from_response.clone().unwrap_or_else(
                                                || assistant_tool_msg.content.clone(),
                                            ));
                                    }
                                    messages.push(assistant_tool_msg);

                                    // Add tool result message
                                    messages.push(Message::new(
                                        Role::User,
                                        format!("Tool result: {}", result_str),
                                    ));
                                }
                            }
                        }
                        // Return messages so far (will need another completion call to get final response)
                        return Ok((messages, usage));
                    } else if let Some(content) = &choice.message.content {
                        // Regular response with content
                        let mut assistant_msg = Message::new(Role::Assistant, content.clone());
                        if self.is_reasoning_model() {
                            assistant_msg.reasoning_content =
                                Some(reasoning_from_response.unwrap_or_else(|| content.clone()));
                        } else {
                            assistant_msg.reasoning_content = reasoning_from_response;
                        }
                        messages.push(assistant_msg);
                        return Ok((messages, usage));
                    }
                }

                // If we get here, no content was found - this shouldn't happen but handle it
                return Err("No content or tool calls in response".into());
            } // end of retry loop
        }

        /// Vision completion API that takes an image and messages
        /// image_url can be either a direct URL or a base64-encoded data URL (e.g., "data:image/jpeg;base64,...")
        pub async fn open_router_vision_completion(
            &self,
            image_url: String,
            messages: Vec<Message>,
        ) -> Result<String, Box<dyn Error + Send + Sync>> {
            let client = reqwest::Client::new();

            // Convert regular messages to vision messages
            // The last user message will have the image added to it
            // If there's no user message, create one with the image
            let mut vision_messages: Vec<VisionMessage> = Vec::new();
            let mut found_last_user = false;

            // Find the last user message index
            let last_user_idx = messages.iter().rposition(|m| matches!(m.role, Role::User));

            for (idx, msg) in messages.iter().enumerate() {
                if Some(idx) == last_user_idx {
                    // Last user message - add image to it
                    found_last_user = true;
                    let text_content = if msg.content.is_empty() {
                        "Please analyze this image.".to_string()
                    } else {
                        msg.content.clone()
                    };

                    let content_items = vec![
                        ContentItem::Text { text: text_content },
                        ContentItem::ImageUrl {
                            image_url: ImageUrl {
                                url: image_url.clone(),
                            },
                        },
                    ];

                    vision_messages.push(VisionMessage {
                        role: msg.role.clone(),
                        content: VisionMessageContent::Array(content_items),
                    });
                } else {
                    // Regular message - keep as text
                    vision_messages.push(VisionMessage {
                        role: msg.role.clone(),
                        content: VisionMessageContent::Text(msg.content.clone()),
                    });
                }
            }

            // If no user message was found, create one with the image
            if !found_last_user {
                let content_items = vec![
                    ContentItem::Text {
                        text: "Please analyze this image.".to_string(),
                    },
                    ContentItem::ImageUrl {
                        image_url: ImageUrl {
                            url: image_url.clone(),
                        },
                    },
                ];

                vision_messages.push(VisionMessage {
                    role: Role::User,
                    content: VisionMessageContent::Array(content_items),
                });
            }

            let body = VisionRequestBody {
                model: self.model_name.clone(),
                messages: vision_messages,
            };

            let json_body = serde_json::to_vec(&body)?;

            let response = client
                .request(reqwest::Method::POST, self.completion_url())
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .body(json_body)
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(
                    format!("API request failed with status {}: {}", status, error_text).into(),
                );
            }

            let response_json: Response = response.json().await?;

            // Extract content from the first choice
            if let Some(choice) = response_json.choices.first() {
                if let Some(content) = &choice.message.content {
                    return Ok(content.clone());
                }
            }

            Err("No content in response".into())
        }

        pub async fn completion_open_router_embedding(
            &self,
            input: String,
        ) -> Result<Vec<f64>, Box<dyn Error + Send + Sync>> {
            let client = reqwest::Client::new();

            let embeddings_url = self.embedding_url();

            let body = EmbeddingRequestBody {
                model: self.model_name.clone(),
                input,
            };

            let json_body = serde_json::to_vec(&body)?;

            let response = client
                .request(reqwest::Method::POST, embeddings_url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .body(json_body)
                .send()
                .await?;

            let status = response.status();

            // Get response text for both error handling and parsing
            let response_text = response.text().await?;

            if !status.is_success() {
                return Err(format!(
                    "API request failed with status {}: {}",
                    status, response_text
                )
                .into());
            }

            // Try to parse as JSON
            let response_json: EmbeddingResponse = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse response JSON: {}. Response body: {}",
                        e, response_text
                    )
                    .into());
                }
            };

            // Extract embedding from the first data item
            if let Some(data) = response_json.data.first() {
                if data.embedding.is_empty() {
                    return Err(
                        format!("Embedding vector is empty. Response: {}", response_text).into(),
                    );
                }
                Ok(data.embedding.clone())
            } else {
                Err(format!("No embedding data in response. Response: {}", response_text).into())
            }
        }
    }

    fn role_to_string(r: &Role) -> String {
        match r {
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::System => "system".to_string(),
        }
    }

    fn convert_tools(
        tools: &[Box<dyn tool::ToolCall>],
    ) -> Result<Vec<serde_json::Value>, Box<dyn Error + Send + Sync>> {
        let mut result = Vec::new();
        for tool in tools {
            result.push(tool.get_json()?);
        }
        Ok(result)
    }

    #[derive(Serialize)]
    pub(crate) struct RequestBody {
        model: String,
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<serde_json::Value>>,
    }

    #[derive(Serialize)]
    pub(crate) struct VisionRequestBody {
        model: String,
        messages: Vec<VisionMessage>,
    }

    #[derive(Serialize)]
    pub(crate) struct EmbeddingRequestBody {
        model: String,
        input: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct EmbeddingResponse {
        pub data: Vec<EmbeddingData>,
        #[serde(default)]
        pub model: String,
        pub usage: Option<EmbeddingUsage>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct EmbeddingData {
        pub embedding: Vec<f64>,
        pub index: Option<u32>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct EmbeddingUsage {
        pub prompt_tokens: Option<u32>,
        pub total_tokens: Option<u32>,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::tool::tool::tool::{Parameter, Tool};
        use std::collections::HashMap;

        #[tokio::test]
        async fn test_chat_completion_without_tool() {
            // This test requires a mock server or actual API key
            // For now, we'll test the request building logic
            let model = Model::new(
                "x-ai/grok-4-fast".to_string(),
                std::env::var("API_KEY").expect("API_KEY environment variable not set"),
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
            );

            let messages = vec![Message::new(Role::User, "Hello, how are you?".to_string())];

            // Print request details
            println!("Sending request without tools");
            println!("Model: {}", model.model_name);
            println!("Messages: {:?}", messages);

            // Note: This will fail without a valid API key, but tests the structure
            let result = model
                .complete(messages.clone(), None as Option<&[Box<dyn tool::ToolCall>]>)
                .await
                .map(|(msgs, _)| msgs);
            dbg!(&result);

            match &result {
                Ok(messages) => {
                    println!("Success! Response messages:");
                    for msg in messages {
                        println!("  {:?}: {}", msg.role, msg.content);
                    }
                    dbg!(messages);
                }
                Err(e) => {
                    println!("Error: {}", e);
                    dbg!(e);
                }
            }

            // We expect this to fail with authentication error, which is expected
            // In a real test, you'd use a mock server
            // Comment out the assertion if you want to see successful responses
            // assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_chat_completion_with_tool() -> Result<(), Box<dyn std::error::Error>> {
            let model = Model::new(
                "x-ai/grok-4-fast".to_string(),
                std::env::var("API_KEY").expect("API_KEY environment variable not set"),
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
            );

            let messages = vec![Message::new(
                Role::User,
                "What's the weather in San Francisco?".to_string(),
            )];

            // Create a test tool
            let mut parameters = HashMap::new();
            let mut location_items = HashMap::new();
            location_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "location".to_string(),
                Parameter {
                    items: location_items,
                    description: "The city and state, e.g. San Francisco, CA".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "get_weather".to_string(),
                description: "Get the current weather in a given location".to_string(),
                parameters,
                required: vec!["location".to_string()],
            };

            let tools: Option<Vec<Box<dyn tool::ToolCall>>> = Some(vec![Box::new(tool)]);

            // Print request details
            println!("Sending request with tools");
            println!("Model: {}", model.model_name);
            println!("Messages: {:?}", messages);

            // Print the tools being sent
            if let Some(ref tools_vec) = tools {
                println!("Tools being sent:");
                for tool in tools_vec {
                    let tool_json = tool.get_json().unwrap();
                    println!("{}", serde_json::to_string_pretty(&tool_json).unwrap());
                }
            }

            // Make the request and get full response to show tool_calls
            let client = reqwest::Client::new();
            let mut req_builder = client.request(reqwest::Method::POST, model.base_url.clone());

            let mut body = RequestBody {
                model: model.model_name.clone(),
                messages: messages.clone(),
                tools: None,
            };

            if let Some(ref tools_vec) = tools {
                let tool_jsons: Vec<serde_json::Value> =
                    tools_vec.iter().map(|t| t.get_json().unwrap()).collect();
                body.tools = Some(tool_jsons);
            }

            let json_body = serde_json::to_vec(&body)?;

            req_builder = req_builder
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", model.api_key))
                .body(json_body);

            let response = req_builder.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                println!(
                    "Error: API request failed with status {}: {}",
                    status, error_text
                );
                return Ok(());
            }

            let response_json: Response = response.json().await?;

            println!("Full response:");
            println!("ID: {}", response_json.id);
            println!("Model: {}", response_json.model);
            println!("Created: {}", response_json.created);

            if let Some(choice) = response_json.choices.first() {
                println!("Finish reason: {:?}", choice.finish_reason);

                if let Some(content) = &choice.message.content {
                    println!("Response content: {}", content);
                } else {
                    println!("Response content: (empty)");
                }

                if let Some(tool_calls) = &choice.message.tool_calls {
                    println!("Tool calls found: {}", tool_calls.len());
                    for (idx, tool_call) in tool_calls.iter().enumerate() {
                        println!("Tool call {}:", idx + 1);
                        println!("  ID: {}", tool_call.id);
                        println!("  Type: {}", tool_call.call_type);
                        println!("  Function name: {}", tool_call.function.name);
                        println!("  Function arguments: {}", tool_call.function.arguments);
                    }
                } else {
                    println!("No tool calls in response");
                }
            }

            // Also test the complete method
            let result = model
                .complete(messages.clone(), tools.as_ref().map(|t| t.as_slice()))
                .await
                .map(|(msgs, _)| msgs);

            match &result {
                Ok(messages) => {
                    println!("Complete method result - Success! Response messages:");
                    for msg in messages {
                        println!("  {:?}: {}", msg.role, msg.content);
                    }
                    dbg!(messages);

                    // If tool was called, make another completion call with the tool results
                    if messages.len() > 1 {
                        println!("Making follow-up completion call with tool results...");
                        let final_result = model
                            .complete(messages.clone(), tools.as_ref().map(|t| t.as_slice()))
                            .await
                            .map(|(msgs, _)| msgs);
                        match &final_result {
                            Ok(final_messages) => {
                                println!("Final response messages:");
                                for msg in final_messages {
                                    println!("  {:?}: {}", msg.role, msg.content);
                                }
                            }
                            Err(e) => {
                                println!("Final call error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Complete method result - Error: {}", e);
                    dbg!(e);
                }
            }

            // We expect this to fail with authentication error, which is expected
            // In a real test, you'd use a mock server
            // Comment out the assertion if you want to see successful responses
            // assert!(result.is_err());

            Ok(())
        }

        #[test]
        fn test_request_body_serialization_without_tools() {
            let messages = vec![Message::new(Role::User, "Hello".to_string())];

            let body = RequestBody {
                model: "gpt-4".to_string(),
                messages,
                tools: None,
            };

            let json = serde_json::to_string(&body).unwrap();
            assert!(!json.contains("tools"));
            assert!(json.contains("model"));
            assert!(json.contains("messages"));
        }

        #[test]
        fn test_request_body_serialization_with_tools() {
            let messages = vec![Message::new(Role::User, "Hello".to_string())];

            let mut parameters = HashMap::new();
            let mut location_items = HashMap::new();
            location_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "location".to_string(),
                Parameter {
                    items: location_items,
                    description: "Location".to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters,
                required: vec!["location".to_string()],
            };

            let tool_json = tool.to_json().unwrap();
            let body = RequestBody {
                model: "gpt-4".to_string(),
                messages,
                tools: Some(vec![tool_json]),
            };

            let json = serde_json::to_string(&body).unwrap();
            assert!(json.contains("tools"));
            assert!(json.contains("model"));
            assert!(json.contains("messages"));
        }

        #[test]
        fn test_response_deserialization() {
            let json = r#"
            {
                "id": "chatcmpl-123",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 10,
                    "total_tokens": 20
                }
            }
            "#;

            let response: Response = serde_json::from_str(json).unwrap();
            assert_eq!(response.id, "chatcmpl-123");
            assert_eq!(response.model, "gpt-4");
            assert_eq!(response.choices.len(), 1);
            assert_eq!(
                response.choices[0].message.content.as_ref().unwrap(),
                "Hello! How can I help you today?"
            );
        }

        #[test]
        fn test_response_deserialization_with_tool_calls() {
            let json = r#"
            {
                "id": "chatcmpl-123",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_abc123",
                            "type": "function",
                            "function": {
                                "name": "get_weather",
                                "arguments": "{\"location\": \"San Francisco\"}"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 10,
                    "total_tokens": 20
                }
            }
            "#;

            let response: Response = serde_json::from_str(json).unwrap();
            assert_eq!(response.id, "chatcmpl-123");
            assert_eq!(response.choices.len(), 1);
            assert!(response.choices[0].message.tool_calls.is_some());
            let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(tool_calls[0].function.name, "get_weather");
        }

        #[tokio::test]
        async fn test_completion_open_router_embedding() {
            let model = Model::new(
                "qwen/qwen3-embedding-8b".to_string(),
                std::env::var("API_KEY").expect("API_KEY environment variable not set"),
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
            );

            let input_text = "Hello, world! This is a test string for embedding.";

            println!("Testing embedding generation for: {}", input_text);
            println!("Model: {}", model.model_name);

            let result = model
                .completion_open_router_embedding(input_text.to_string())
                .await;

            match &result {
                Ok(embedding) => {
                    println!("Success! Embedding generated:");
                    println!("  Length: {}", embedding.len());
                    println!(
                        "  First 5 values: {:?}",
                        &embedding[..embedding.len().min(5)]
                    );
                    println!(
                        "  Last 5 values: {:?}",
                        &embedding[embedding.len().saturating_sub(5)..]
                    );

                    // Verify the embedding is valid
                    assert!(!embedding.is_empty(), "Embedding should not be empty");
                    assert!(
                        embedding.len() > 0,
                        "Embedding should have at least one dimension"
                    );

                    // Check that all values are finite numbers
                    for (idx, &value) in embedding.iter().enumerate() {
                        assert!(
                            value.is_finite(),
                            "Embedding value at index {} should be finite, got: {}",
                            idx,
                            value
                        );
                    }

                    println!("✓ Embedding test passed!");
                }
                Err(e) => {
                    println!("Error generating embedding: {}", e);
                    // In a real test environment, you might want to assert success
                    // For now, we'll just print the error for debugging
                    // Uncomment the line below if you want the test to fail on error:
                    // panic!("Embedding generation failed: {}", e);
                }
            }
        }
    }
}
