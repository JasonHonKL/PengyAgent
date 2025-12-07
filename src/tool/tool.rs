pub mod tool {

    use std::collections::HashMap;
    use serde::Serialize;
    use serde_json;

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
    }

    impl Tool {
        pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            let mut properties = serde_json::Map::new();
            for (key, param) in &self.parameters {
                let mut prop = serde_json::Map::new();
                prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
                prop.insert("description".to_string(), serde_json::Value::String(param.description.clone()));
                if !param.items.is_empty() {
                    let mut items_obj = serde_json::Map::new();
                    for (k, v) in &param.items {
                        items_obj.insert(k.clone(), serde_json::Value::String(v.clone()));
                    }
                    prop.insert("items".to_string(), serde_json::Value::Object(items_obj));
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