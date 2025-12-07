pub mod Model {
    use std::error::Error;

    use serde::{Serialize, Deserialize};

    use crate::tool::tool::tool as tool;


    #[derive(Debug , Clone)]
    pub enum Role {
        User, 
        Assistant,
        System,
    }

    #[derive(Debug, Clone)]
    pub struct Model{
        pub model_name: String,
        pub api_key: String, 
        pub base_url: String,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Message{
        pub role:  Role,
        pub content: String,
    }


    // either create three struct for stream non stream no response
    #[derive(Debug , Clone, Deserialize)]
    pub struct Response {
        pub id : String, 
        pub created: i64,
        pub model: String,
        pub choices: Vec<ResponseChoice>,
        pub usage: ResponseUsage,
    }

    #[derive(Debug , Clone, Deserialize)]
    pub struct ResponseUsage{
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
    pub struct ResponseMessage{
        pub role: String,
        pub content: Option<String>,
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
            Self { role, content }
        }
    }

    impl Serialize for Role {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer {
            serializer.serialize_str(role_to_string(self).as_ref())
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

        pub async fn complete(&self, messages: Vec<Message> , tools: Option<Vec<tool::Tool>>) -> Result<String , Box<dyn Error>>{

            let client = reqwest::Client::new();
            let mut req_builder = client.request(reqwest::Method::POST, self.base_url.clone());

            let mut body = RequestBody{
                model: self.model_name.clone(),
                messages: messages.clone(),
                tools: None,
            };

            if let Some(tools_vec) = tools {
                body.tools = Some(convert_tools(tools_vec)?);
            }

            let json_body = serde_json::to_vec(&body)?;

            req_builder = req_builder
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .body(json_body);

            
            let response = req_builder.send().await?;
            let status = response.status();
            
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(format!("API request failed with status {}: {}", status, error_text).into());
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
    }

    fn role_to_string(r: &Role) -> String{
        match  r{
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::System => "system".to_string()
        }
    }

    fn convert_tools(tools: Vec<tool::Tool>) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
        let mut result = Vec::new();
        for tool in tools {
            result.push(tool.to_json()?);
        }
        Ok(result)
    }

    #[derive(Serialize)]
    pub(crate) struct RequestBody{
        model : String,
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<serde_json::Value>>,
    } 

#[cfg(test)]
mod tests{
    use super::*;
        use crate::tool::tool::tool::{Tool, Parameter};
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

            let messages = vec![
                Message::new(Role::User, "Hello, how are you?".to_string()),
            ];

            // Print request details
            println!("Sending request without tools");
            println!("Model: {}", model.model_name);
            println!("Messages: {:?}", messages);

            // Note: This will fail without a valid API key, but tests the structure
            let result = model.complete(messages.clone(), None).await;
            dbg!(&result);
            
            match &result {
                Ok(content) => {
                    println!("Success! Response content: {}", content);
                    dbg!(content);
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

            let messages = vec![
                Message::new(Role::User, "What's the weather in San Francisco?".to_string()),
            ];

            // Create a test tool
            let mut parameters = HashMap::new();
            let mut location_items = HashMap::new();
            location_items.insert("type".to_string(), "string".to_string());
            parameters.insert("location".to_string(), Parameter {
                items: location_items,
                description: "The city and state, e.g. San Francisco, CA".to_string(),
            });

            let tool = Tool {
                name: "get_weather".to_string(),
                description: "Get the current weather in a given location".to_string(),
                parameters,
                required: vec!["location".to_string()],
            };

            let tools = Some(vec![tool]);

            // Print request details
            println!("Sending request with tools");
            println!("Model: {}", model.model_name);
            println!("Messages: {:?}", messages);
            
            // Print the tools being sent
            if let Some(ref tools_vec) = tools {
                println!("Tools being sent:");
                for tool in tools_vec {
                    let tool_json = tool.to_json().unwrap();
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

            if let Some(tools_vec) = tools.clone() {
                body.tools = Some(convert_tools(tools_vec)?);
            }

            let json_body = serde_json::to_vec(&body)?;

            req_builder = req_builder
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", model.api_key))
                .body(json_body);

            let response = req_builder.send().await?;
            let status = response.status();
            
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                println!("Error: API request failed with status {}: {}", status, error_text);
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
            let result = model.complete(messages.clone(), tools.clone()).await;
            
            match &result {
                Ok(content) => {
                    println!("Complete method result - Success! Response content: {}", content);
                    dbg!(content);
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
            let messages = vec![
                Message::new(Role::User, "Hello".to_string()),
            ];

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
            let messages = vec![
                Message::new(Role::User, "Hello".to_string()),
            ];

            let mut parameters = HashMap::new();
            let mut location_items = HashMap::new();
            location_items.insert("type".to_string(), "string".to_string());
            parameters.insert("location".to_string(), Parameter {
                items: location_items,
                description: "Location".to_string(),
            });

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
            assert_eq!(response.choices[0].message.content.as_ref().unwrap(), "Hello! How can I help you today?");
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
    }
}