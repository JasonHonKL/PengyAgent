pub mod web_search {
    //! Compatibility wrapper exposing the existing `web` tool as `web_search`.
    use crate::tool::tool::tool::ToolCall;
    use crate::tool::web::web::WebTool;
    use serde_json;
    use std::error::Error;

    pub struct WebSearchTool {
        inner: WebTool,
    }

    impl WebSearchTool {
        pub fn new() -> Self {
            Self {
                inner: WebTool::new(),
            }
        }
    }

    impl ToolCall for WebSearchTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            let mut json = self.inner.get_json()?;
            if let Some(obj) = json.as_object_mut() {
                if let Some(function_obj) = obj.get_mut("function") {
                    if let Some(f) = function_obj.as_object_mut() {
                        f.insert(
                            "name".to_string(),
                            serde_json::Value::String("web_search".to_string()),
                        );
                    }
                }
            }
            Ok(json)
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            self.inner.run(arguments)
        }

        fn name(&self) -> &str {
            "web_search"
        }
    }
}
