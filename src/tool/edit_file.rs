pub mod edit_file {
    //! Compatibility wrapper that exposes the existing `edit` tool under the
    //! `edit_file` name.
    use crate::tool::edit::edit::EditTool;
    use crate::tool::tool::tool::ToolCall;
    use serde_json;
    use std::error::Error;

    /// Wrapper around `EditTool` with a different exposed name.
    pub struct EditFileTool {
        inner: EditTool,
    }

    impl EditFileTool {
        pub fn new() -> Self {
            Self { inner: EditTool::new() }
        }
    }

    impl ToolCall for EditFileTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            let mut json = self.inner.get_json()?;
            if let Some(obj) = json.as_object_mut() {
                if let Some(function_obj) = obj.get_mut("function") {
                    if let Some(f) = function_obj.as_object_mut() {
                        f.insert(
                            "name".to_string(),
                            serde_json::Value::String("edit_file".to_string()),
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
            "edit_file"
        }
    }
}

