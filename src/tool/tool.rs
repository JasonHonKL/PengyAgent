pub mod tool {

    use std::collections::HashMap;
    use serde::Serialize;
    use serde_json;
    use std::error::Error;

    pub trait ToolCall: Send + Sync {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error>;
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>>;
        fn name(&self) -> &str;
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Tool{
        pub name: String,
        pub description: String,
        pub parameters: HashMap<String , Parameter>,

        pub required: Vec<String>,
    }

    #[derive(Debug , Clone, Serialize)]
    pub struct Parameter{
        pub items: HashMap<String , String>,
        pub description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub enum_values: Option<Vec<String>>,
    }

    impl ToolCall for Tool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            let mut properties = serde_json::Map::new();
            for (key, param) in &self.parameters {
                let mut prop = serde_json::Map::new();
                
                // Determine type from items or default to string
                let param_type = param.items.get("type")
                    .cloned()
                    .unwrap_or_else(|| "string".to_string());
                prop.insert("type".to_string(), serde_json::Value::String(param_type.clone()));
                prop.insert("description".to_string(), serde_json::Value::String(param.description.clone()));
                
                // Handle enum values if present
                if let Some(ref enum_vals) = param.enum_values {
                    let enum_array: Vec<serde_json::Value> = enum_vals.iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect();
                    prop.insert("enum".to_string(), serde_json::Value::Array(enum_array));
                } else if param_type == "array" {
                    // Handle array type - look for item_type in items to define array item type
                    let item_type = param.items.get("item_type")
                        .cloned()
                        .unwrap_or_else(|| "string".to_string());
                    let mut items_obj = serde_json::Map::new();
                    items_obj.insert("type".to_string(), serde_json::Value::String(item_type));
                    prop.insert("items".to_string(), serde_json::Value::Object(items_obj));
                } else if !param.items.is_empty() {
                    // Keep backward compatibility with items
                    let mut items_obj = serde_json::Map::new();
                    for (k, v) in &param.items {
                        if k != "type" && k != "item_type" {  // Skip type and item_type as they're already handled
                            items_obj.insert(k.clone(), serde_json::Value::String(v.clone()));
                        }
                    }
                    if !items_obj.is_empty() {
                        prop.insert("items".to_string(), serde_json::Value::Object(items_obj));
                    }
                }
                properties.insert(key.clone(), serde_json::Value::Object(prop));
            }

            let mut parameters_obj = serde_json::Map::new();
            parameters_obj.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            parameters_obj.insert("properties".to_string(), serde_json::Value::Object(properties));
            if !self.required.is_empty() {
                let required_array: Vec<serde_json::Value> = self.required.iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect();
                parameters_obj.insert("required".to_string(), serde_json::Value::Array(required_array));
            }

            let mut function_obj = serde_json::Map::new();
            function_obj.insert("name".to_string(), serde_json::Value::String(self.name.clone()));
            function_obj.insert("description".to_string(), serde_json::Value::String(self.description.clone()));
            function_obj.insert("parameters".to_string(), serde_json::Value::Object(parameters_obj));

            let mut tool_obj = serde_json::Map::new();
            tool_obj.insert("type".to_string(), serde_json::Value::String("function".to_string()));
            tool_obj.insert("function".to_string(), serde_json::Value::Object(function_obj));

            Ok(serde_json::Value::Object(tool_obj))
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // For now, return a mock response based on the tool name
            // In a real implementation, this would call the actual tool function
            if self.name == "get_weather" {
                if let Some(location) = args.get("location").and_then(|v| v.as_str()) {
                    Ok(format!("The weather in {} is sunny, 72°F", location))
                } else {
                    Err("Missing location parameter".into())
                }
            } else {
                Ok(format!("Tool {} executed with arguments: {}", self.name, arguments))
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Tool {
        pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.get_json()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn test_tool_to_json() {
            let mut parameters = HashMap::new();
            let mut location_items = HashMap::new();
            location_items.insert("type".to_string(), "string".to_string());
            parameters.insert("location".to_string(), Parameter {
                items: location_items,
                description: "The city and state".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "get_weather".to_string(),
                description: "Get the weather".to_string(),
                parameters,
                required: vec!["location".to_string()],
            };

            let json = tool.to_json().unwrap();
            assert!(json.is_object());
            let obj = json.as_object().unwrap();
            assert_eq!(obj.get("type").unwrap().as_str().unwrap(), "function");
            assert!(obj.contains_key("function"));
            
            let function = obj.get("function").unwrap().as_object().unwrap();
            assert_eq!(function.get("name").unwrap().as_str().unwrap(), "get_weather");
            assert_eq!(function.get("description").unwrap().as_str().unwrap(), "Get the weather");
            assert!(function.contains_key("parameters"));
        }
    }
    
}