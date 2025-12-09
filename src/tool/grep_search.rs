pub mod grep_search {
    //! Thin wrapper that exposes the existing grep tool under the
    //! `grep_search` name for compatibility with external tool callers.
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::tool::tool::ToolCall;
    use serde_json;
    use std::error::Error;

    /// Wrapper around `GrepTool` but with a different exposed name.
    pub struct GrepSearchTool {
        inner: GrepTool,
    }

    impl GrepSearchTool {
        pub fn new() -> Self {
            let inner = GrepTool::new();
            Self { inner }
        }
    }

    impl ToolCall for GrepSearchTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            // Reuse inner JSON but change the function name
            let mut json = self.inner.get_json()?;
            if let Some(obj) = json.as_object_mut() {
                if let Some(function_obj) = obj.get_mut("function") {
                    if let Some(f) = function_obj.as_object_mut() {
                        f.insert(
                            "name".to_string(),
                            serde_json::Value::String("grep_search".to_string()),
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
            "grep_search"
        }
    }
}
